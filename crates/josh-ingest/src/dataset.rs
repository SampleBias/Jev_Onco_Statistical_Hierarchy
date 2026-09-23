//! Immutable local dataset bundles and compatibility import. No provider requests.
use crate::expression::{self, ImportedDataset};
use josh_core::sample::*;
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeSet,
    fs::{self, File, OpenOptions},
    io::{BufRead, BufReader, Read, Write},
    path::{Component, Path},
    time::{SystemTime, UNIX_EPOCH},
};

const MAX_MANIFEST: usize = 32 * 1024 * 1024;
const MAX_ARTIFACT: u64 = 256 * 1024 * 1024;

#[derive(Debug, thiserror::Error)]
pub enum DatasetError {
    #[error("dataset IO failed")]
    Io(#[from] std::io::Error),
    #[error("input exceeds the expression source, sample, gene or cell limit")]
    Limit,
    #[error(
        "unable to detect expression format; set delimiter, layout and column names explicitly"
    )]
    Detection,
    #[error(
        "invalid expression columns; use gene/expression with sample_id, or gene-by-sample wide layout"
    )]
    Columns,
    #[error("a two-column expression table requires --sample-id")]
    SampleId,
    #[error("invalid expression configuration or incompatible transform")]
    Configuration,
    #[error("malformed, non-UTF-8 or inconsistent-width CSV/TSV")]
    Format,
    #[error("no expression records found")]
    Empty,
    #[error("invalid HGNC mapping table; approved entries and identifier columns are required")]
    GeneMap,
    #[error("dataset bundle is incomplete, incompatible or has changed")]
    InvalidBundle,
    #[error("dataset serialization failed")]
    Serialization,
}

pub fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

pub fn artifact(path: &str, bytes: &[u8]) -> Artifact {
    Artifact {
        path: path.into(),
        sha256: format!("{:x}", Sha256::digest(bytes)),
        bytes: bytes.len() as u64,
    }
}

fn records_bytes(records: &[ExpressionRecord]) -> Result<Vec<u8>, DatasetError> {
    let mut bytes = Vec::new();
    for record in records {
        let start = bytes.len();
        serde_json::to_writer(&mut bytes, record).map_err(|_| DatasetError::Serialization)?;
        bytes.push(b'\n');
        if bytes.len() - start > 65_536 {
            return Err(DatasetError::Limit);
        }
        if bytes.len() as u64 > MAX_ARTIFACT {
            return Err(DatasetError::Limit);
        }
    }
    Ok(bytes)
}

pub fn records_artifact(
    path: &str,
    records: &[ExpressionRecord],
) -> Result<Artifact, DatasetError> {
    Ok(artifact(path, &records_bytes(records)?))
}

pub fn bounded_read(reader: impl Read, limit: usize) -> Result<Vec<u8>, DatasetError> {
    let mut bytes = Vec::new();
    reader.take(limit as u64 + 1).read_to_end(&mut bytes)?;
    if bytes.len() > limit {
        return Err(DatasetError::Limit);
    }
    Ok(bytes)
}

fn private_dir(path: &Path) -> Result<(), DatasetError> {
    let mut options = fs::DirBuilder::new();
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;
        options.mode(0o700);
    }
    options.create(path)?;
    Ok(())
}

fn save(path: &Path, bytes: &[u8]) -> Result<(), DatasetError> {
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options.open(path)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    Ok(())
}

pub fn write(root: &Path, data: &ImportedDataset) -> Result<(), DatasetError> {
    let serialized =
        serde_json::to_vec_pretty(&data.manifest).map_err(|_| DatasetError::Serialization)?;
    if serialized.len() > MAX_MANIFEST || data.records.len() != data.manifest.samples.len() {
        return Err(DatasetError::Limit);
    }
    private_dir(root)?;
    private_dir(&root.join("source"))?;
    private_dir(&root.join("samples"))?;
    save(&root.join("source/input"), &data.source)?;
    if let Some(map) = &data.mapping_source {
        save(&root.join("source/gene-map.tsv"), map)?;
    }
    for (i, records) in data.records.iter().enumerate() {
        save(
            &root.join(format!("samples/{i:06}.jsonl")),
            &records_bytes(records)?,
        )?;
    }
    // Completion manifest is always last. Incomplete output is never accepted on load.
    save(&root.join("dataset.json"), &serialized)?;
    Ok(())
}

fn safe_path(root: &Path, relative: &str) -> Result<std::path::PathBuf, DatasetError> {
    let path = Path::new(relative);
    if relative.len() > 256
        || path.as_os_str().is_empty()
        || path
            .components()
            .any(|c| !matches!(c, Component::Normal(_)))
    {
        return Err(DatasetError::InvalidBundle);
    }
    if fs::symlink_metadata(root)?.file_type().is_symlink() {
        return Err(DatasetError::InvalidBundle);
    }
    let mut current = root.to_path_buf();
    for component in path.components() {
        current.push(component.as_os_str());
        if fs::symlink_metadata(&current)?.file_type().is_symlink() {
            return Err(DatasetError::InvalidBundle);
        }
    }
    if !fs::metadata(&current)?.is_file() {
        return Err(DatasetError::InvalidBundle);
    }
    Ok(current)
}

fn verify_artifact(root: &Path, expected: &Artifact) -> Result<(), DatasetError> {
    if expected.bytes > MAX_ARTIFACT || !josh_core::valid_sha256(&expected.sha256) {
        return Err(DatasetError::InvalidBundle);
    }
    let mut file = File::open(safe_path(root, &expected.path)?)?;
    let mut hash = Sha256::new();
    let mut buffer = [0u8; 65_536];
    let mut total = 0;
    loop {
        let n = file.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        total += n as u64;
        if total > expected.bytes {
            return Err(DatasetError::InvalidBundle);
        }
        hash.update(&buffer[..n]);
    }
    if total != expected.bytes || format!("{:x}", hash.finalize()) != expected.sha256 {
        return Err(DatasetError::InvalidBundle);
    }
    Ok(())
}

pub fn read(root: &Path) -> Result<DatasetManifest, DatasetError> {
    let bytes = bounded_read(File::open(safe_path(root, "dataset.json")?)?, MAX_MANIFEST)?;
    let manifest: DatasetManifest =
        serde_json::from_slice(&bytes).map_err(|_| DatasetError::InvalidBundle)?;
    if manifest.dataset_schema_version != 1
        || !valid_label(&manifest.dataset_id)
        || manifest.samples.is_empty()
        || manifest.samples.len() > expression::MAX_SAMPLES
        || !matches!(
            manifest.pipeline_version.as_str(),
            EXPRESSION_PIPELINE | "legacy-case-adapter-v1"
        )
        || manifest.source.artifact.path != "source/input"
    {
        return Err(DatasetError::InvalidBundle);
    }
    verify_artifact(root, &manifest.source.artifact)?;
    if let Some(map) = &manifest.gene_map {
        if map.artifact.path != "source/gene-map.tsv" || !valid_label(&map.release) {
            return Err(DatasetError::InvalidBundle);
        }
        verify_artifact(root, &map.artifact)?;
    }
    let mut ids = BTreeSet::new();
    for (i, sample) in manifest.samples.iter().enumerate() {
        if sample.sample_schema_version != SAMPLE_SCHEMA_VERSION
            || !valid_label(&sample.sample_id)
            || !ids.insert(&sample.sample_id)
            || sample.assays.len() != 1
        {
            return Err(DatasetError::InvalidBundle);
        }
        let assay = &sample.assays[0];
        let expected = if manifest.pipeline_version == EXPRESSION_PIPELINE {
            format!("samples/{i:06}.jsonl")
        } else {
            "source/input".into()
        };
        if assay.artifact.path != expected {
            return Err(DatasetError::InvalidBundle);
        }
        verify_artifact(root, &assay.artifact)?;
    }
    Ok(manifest)
}

pub fn read_records(root: &Path, sample: &Sample) -> Result<Vec<ExpressionRecord>, DatasetError> {
    let assay = sample.assays.first().ok_or(DatasetError::InvalidBundle)?;
    if assay.modality != Modality::Expression {
        return Ok(vec![]);
    }
    verify_artifact(root, &assay.artifact)?;
    let file = File::open(safe_path(root, &assay.artifact.path)?)?;
    let mut reader = BufReader::new(file);
    let mut records = Vec::new();
    loop {
        let mut line = Vec::new();
        (&mut reader).take(65_537).read_until(b'\n', &mut line)?;
        if line.is_empty() {
            break;
        }
        if line.len() > 65_536 || records.len() >= expression::MAX_GENES {
            return Err(DatasetError::Limit);
        }
        let record: ExpressionRecord =
            serde_json::from_slice(&line).map_err(|_| DatasetError::InvalidBundle)?;
        if record.raw_expression.is_some_and(|v| !v.is_finite())
            || record
                .transformed_expression
                .is_some_and(|v| !v.is_finite())
            || record.source_record < 2
            || record.source_column == 0
        {
            return Err(DatasetError::InvalidBundle);
        }
        records.push(record);
    }
    Ok(records)
}

/// Rebuild derived expression artifacts from the archived source, map and configuration.
/// Acquisition timestamps intentionally do not participate in deterministic comparisons.
pub fn reproduce(root: &Path) -> Result<DatasetManifest, DatasetError> {
    let saved = read(root)?;
    let config = saved
        .expression_config
        .clone()
        .ok_or(DatasetError::Configuration)?;
    let source = bounded_read(
        File::open(safe_path(root, &saved.source.artifact.path)?)?,
        expression::MAX_SOURCE,
    )?;
    let mapping = saved
        .gene_map
        .as_ref()
        .map(|m| {
            Ok::<_, DatasetError>((
                bounded_read(
                    File::open(safe_path(root, &m.artifact.path)?)?,
                    expression::MAX_SOURCE,
                )?,
                m.release.clone(),
            ))
        })
        .transpose()?;
    let options = expression::ImportOptions {
        dataset_id: saved.dataset_id.clone(),
        source_name: saved.source_name.clone(),
        study: saved.study.clone(),
        accession: saved.accession.clone(),
        citation: saved.citation.clone(),
        data_class: saved.samples[0].data_class.clone(),
        config,
    };
    let rebuilt = expression::import(source, &options, mapping)?;
    if serde_json::to_value(&saved.samples).map_err(|_| DatasetError::Serialization)?
        != serde_json::to_value(&rebuilt.manifest.samples)
            .map_err(|_| DatasetError::Serialization)?
    {
        return Err(DatasetError::InvalidBundle);
    }
    Ok(saved)
}

pub fn import_legacy(
    bytes: Vec<u8>,
    dataset_id: &str,
    original_name: &str,
    root: &Path,
) -> Result<DatasetManifest, DatasetError> {
    if bytes.len() > josh_core::MAX_CASE_BYTES
        || !valid_label(dataset_id)
        || !valid_label(original_name)
    {
        return Err(DatasetError::Configuration);
    }
    let case: josh_core::Case = serde_json::from_slice(&bytes).map_err(|_| DatasetError::Format)?;
    case.validate().map_err(|_| DatasetError::Format)?;
    let sample_id = case
        .metadata
        .as_ref()
        .and_then(|m| m.sample_id.clone())
        .unwrap_or_else(|| case.case_id.clone());
    let source = artifact("source/input", &bytes);
    let manifest = DatasetManifest {
        dataset_schema_version: 1, dataset_id: dataset_id.into(), source_name: original_name.into(),
        study: None, accession: None, citation: None, imported_at_unix_seconds: now(), pipeline_version: "legacy-case-adapter-v1".into(),
        source: SourceAsset { original_name: original_name.into(), artifact: source.clone() }, gene_map: None, expression_config: None, detection: None,
        samples: vec![Sample { sample_schema_version: 1, sample_id, data_class: case.data_class.clone(),
            patient_group_id: case.metadata.as_ref().and_then(|m| m.patient_id.clone()),
            assays: vec![Assay { assay_id: "legacy-evidence".into(), modality: Modality::LegacyAnnotations, artifact: source, qc: None }] }],
        data_dictionary: vec![],
        notices: vec![format!("Legacy case schema {} preserved byte-for-byte, including clinical context and review history. Findings are annotations, not inferred numeric expression.", case.schema_version),
            "Sample identity uses metadata.sample_id when present, otherwise the original case_id; original identity remains in source/input.".into()],
    };
    let serialized =
        serde_json::to_vec_pretty(&manifest).map_err(|_| DatasetError::Serialization)?;
    private_dir(root)?;
    private_dir(&root.join("source"))?;
    save(&root.join("source/input"), &bytes)?;
    save(&root.join("dataset.json"), &serialized)?;
    Ok(manifest)
}

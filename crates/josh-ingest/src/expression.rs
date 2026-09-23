//! Local expression adapters. Large measurements never enter the legacy Case contract.
use crate::dataset::{self, DatasetError};
use josh_core::sample::*;
use josh_features::GeneMapper;
use std::collections::{BTreeMap, BTreeSet};

pub const MAX_SOURCE: usize = 64 * 1024 * 1024;
pub const MAX_CELLS: usize = 1_000_000;
pub const MAX_SAMPLES: usize = 512;
pub const MAX_GENES: usize = 100_000;

#[derive(Clone)]
pub struct ImportOptions {
    pub dataset_id: String,
    pub source_name: String,
    pub study: Option<String>,
    pub accession: Option<String>,
    pub citation: Option<String>,
    pub data_class: josh_core::DataClass,
    pub config: ExpressionConfig,
}

impl Default for ImportOptions {
    fn default() -> Self {
        Self {
            dataset_id: "dataset".into(),
            source_name: "local file".into(),
            study: None,
            accession: None,
            citation: None,
            data_class: josh_core::DataClass::DeidentifiedResearch,
            config: ExpressionConfig::default(),
        }
    }
}

pub struct ImportedDataset {
    pub manifest: DatasetManifest,
    pub records: Vec<Vec<ExpressionRecord>>,
    pub source: Vec<u8>,
    pub mapping_source: Option<Vec<u8>>,
}

/// An adapter proposes a detection; explicit configuration can correct that proposal.
pub trait DatasetAdapter {
    fn detect(&self, bytes: &[u8], config: &ExpressionConfig) -> Result<Detection, DatasetError>;
    fn parse(
        &self,
        bytes: &[u8],
        config: &ExpressionConfig,
        detection: &Detection,
    ) -> Result<BTreeMap<String, Vec<ExpressionRecord>>, DatasetError>;
}

pub struct ExpressionTableAdapter;

fn delimiter_byte(value: Delimiter) -> u8 {
    if value == Delimiter::Csv { b',' } else { b'\t' }
}

fn headers(bytes: &[u8], delimiter: Delimiter) -> Result<csv::StringRecord, DatasetError> {
    Ok(csv::ReaderBuilder::new()
        .delimiter(delimiter_byte(delimiter))
        .from_reader(bytes)
        .headers()
        .map_err(|_| DatasetError::Format)?
        .clone())
}

impl DatasetAdapter for ExpressionTableAdapter {
    fn detect(&self, bytes: &[u8], config: &ExpressionConfig) -> Result<Detection, DatasetError> {
        if bytes.is_empty() || bytes.len() > MAX_SOURCE {
            return Err(DatasetError::Limit);
        }
        let delimiter = match config.delimiter {
            Delimiter::Auto => {
                let candidates: Vec<_> = [Delimiter::Csv, Delimiter::Tsv]
                    .into_iter()
                    .filter(|d| {
                        headers(bytes, *d).is_ok_and(|h| {
                            h.len() >= 2 && h.iter().any(|v| v == config.gene_column)
                        })
                    })
                    .collect();
                if candidates.len() != 1 {
                    return Err(DatasetError::Detection);
                }
                candidates[0]
            }
            d => d,
        };
        let headers = headers(bytes, delimiter)?;
        if headers.len() < 2
            || headers.len() > MAX_SAMPLES + 1
            || headers.iter().any(|v| !valid_label(v))
            || headers.iter().collect::<BTreeSet<_>>().len() != headers.len()
            || !headers.iter().any(|v| v == config.gene_column)
        {
            return Err(DatasetError::Columns);
        }
        let has_value = headers.iter().any(|v| v == config.value_column);
        let layout = match config.layout {
            MatrixLayout::Auto if has_value => MatrixLayout::Long,
            MatrixLayout::Auto => MatrixLayout::Wide,
            layout => layout,
        };
        let sample_columns = if layout == MatrixLayout::Long {
            if !has_value
                || headers.iter().any(|v| {
                    v != config.gene_column && v != config.value_column && v != config.sample_column
                })
            {
                return Err(DatasetError::Columns);
            }
            if !headers.iter().any(|v| v == config.sample_column)
                && config.single_sample_id.is_none()
            {
                return Err(DatasetError::SampleId);
            }
            vec![]
        } else {
            if config.single_sample_id.is_some() {
                return Err(DatasetError::Configuration);
            }
            headers
                .iter()
                .filter(|v| *v != config.gene_column)
                .map(String::from)
                .collect()
        };
        Ok(Detection { delimiter, layout, modality: Modality::Expression,
            columns: headers.iter().map(String::from).collect(), sample_columns,
            notices: vec!["Expression table candidate; units, platform and genome are declarations, not inferred from numerical values.".into(),
                "Wide layout treats every non-gene column as a sample. Correct --layout or columns before import if this is not your matrix.".into()] })
    }

    fn parse(
        &self,
        bytes: &[u8],
        config: &ExpressionConfig,
        detection: &Detection,
    ) -> Result<BTreeMap<String, Vec<ExpressionRecord>>, DatasetError> {
        let mut reader = csv::ReaderBuilder::new()
            .delimiter(delimiter_byte(detection.delimiter))
            .from_reader(bytes);
        let columns = reader.headers().map_err(|_| DatasetError::Format)?.clone();
        let position = |name: &str| columns.iter().position(|v| v == name);
        let gene = position(&config.gene_column).ok_or(DatasetError::Columns)?;
        let value = position(&config.value_column);
        let sample = position(&config.sample_column);
        let mut samples: BTreeMap<String, Vec<ExpressionRecord>> = BTreeMap::new();
        let mut cells = 0;
        for (n, row) in reader.records().enumerate() {
            let row = row.map_err(|_| DatasetError::Format)?;
            if n >= MAX_CELLS || !valid_label(&row[gene]) {
                return Err(DatasetError::Limit);
            }
            let mut push = |sample_id: &str, column: usize| -> Result<(), DatasetError> {
                if !valid_label(sample_id) || row[column].len() > 128 {
                    return Err(DatasetError::Limit);
                }
                if samples.len() >= MAX_SAMPLES && !samples.contains_key(sample_id) {
                    return Err(DatasetError::Limit);
                }
                cells += 1;
                if cells > MAX_CELLS {
                    return Err(DatasetError::Limit);
                }
                let records = samples.entry(sample_id.into()).or_default();
                if records.len() >= MAX_GENES {
                    return Err(DatasetError::Limit);
                }
                records.push(ExpressionRecord {
                    original_gene_id: row[gene].into(),
                    original_value: row[column].into(),
                    source_record: n as u64 + 2,
                    source_column: column + 1,
                    raw_expression: None,
                    transformed_expression: None,
                    mapping: GeneMapping {
                        status: MappingStatus::NotAttempted,
                        gene: None,
                        candidates: vec![],
                    },
                });
                Ok(())
            };
            if detection.layout == MatrixLayout::Long {
                let sample_id = sample
                    .map(|i| &row[i])
                    .or(config.single_sample_id.as_deref())
                    .ok_or(DatasetError::SampleId)?;
                push(sample_id, value.ok_or(DatasetError::Columns)?)?;
            } else {
                for (column, name) in columns.iter().enumerate().filter(|(i, _)| *i != gene) {
                    push(name, column)?;
                }
            }
        }
        if samples.is_empty() {
            return Err(DatasetError::Empty);
        }
        Ok(samples)
    }
}

pub fn import(
    bytes: Vec<u8>,
    options: &ImportOptions,
    mapping: Option<(Vec<u8>, String)>,
) -> Result<ImportedDataset, DatasetError> {
    validate_options(options)?;
    let adapter = ExpressionTableAdapter;
    let detection = adapter.detect(&bytes, &options.config)?;
    let mut parsed = adapter.parse(&bytes, &options.config, &detection)?;
    let mapper = mapping
        .as_ref()
        .map(|(data, _)| GeneMapper::from_tsv(data))
        .transpose()
        .map_err(|_| DatasetError::GeneMap)?;
    if mapping
        .as_ref()
        .is_some_and(|(_, release)| !valid_label(release))
    {
        return Err(DatasetError::Configuration);
    }
    let mut config = options.config.clone();
    config.delimiter = detection.delimiter;
    config.layout = detection.layout;
    let mut samples = Vec::new();
    let mut records = Vec::new();
    for (sample_id, mut data) in std::mem::take(&mut parsed) {
        let qc = josh_features::process(&mut data, &config, mapper.as_ref());
        let path = format!("samples/{:06}.jsonl", samples.len());
        let artifact = dataset::records_artifact(&path, &data)?;
        samples.push(Sample {
            sample_schema_version: SAMPLE_SCHEMA_VERSION,
            sample_id,
            data_class: options.data_class.clone(),
            patient_group_id: None,
            assays: vec![Assay {
                assay_id: "expression-1".into(),
                modality: Modality::Expression,
                artifact,
                qc: Some(qc),
            }],
        });
        records.push(data);
    }
    let source = dataset::artifact("source/input", &bytes);
    let gene_map = mapping.as_ref().map(|(bytes, release)| MappingAsset {
        release: release.clone(),
        artifact: dataset::artifact("source/gene-map.tsv", bytes),
    });
    let dictionary = vec![
        DictionaryField { name: "sample_id".into(), meaning: "Original sample identifier; patient identity is optional.".into(), original_column: (config.layout == MatrixLayout::Long).then(|| config.sample_column.clone()), units: None },
        DictionaryField { name: "original_gene_id".into(), meaning: "Unmodified source gene ID; mapped HGNC identity is stored separately.".into(), original_column: Some(config.gene_column.clone()), units: None },
        DictionaryField { name: "raw_expression".into(), meaning: "Parsed measurement; missing/invalid values remain null and are distinguished in QC and the original value.".into(), original_column: (config.layout == MatrixLayout::Long).then(|| config.value_column.clone()), units: Some(config.units) },
        DictionaryField { name: "transformed_expression".into(), meaning: format!("Derived value using {:?}; raw values remain intact. This is not platform harmonization.", config.transform), original_column: None, units: Some(if config.transform == Transform::Identity { config.units } else { ExpressionUnit::Log2 }) },
        DictionaryField { name: "source_record/source_column".into(), meaning: "One-based parser record (header = 1) and original column. Resolves against source/input.".into(), original_column: None, units: None },
    ];
    let manifest = DatasetManifest {
        dataset_schema_version: 1, dataset_id: options.dataset_id.clone(), source_name: options.source_name.clone(),
        study: options.study.clone(), accession: options.accession.clone(), citation: options.citation.clone(),
        imported_at_unix_seconds: dataset::now(), pipeline_version: EXPRESSION_PIPELINE.into(),
        source: SourceAsset { original_name: options.source_name.clone(), artifact: source }, gene_map,
        expression_config: Some(config), detection: Some(detection), samples,
        data_dictionary: dictionary,
        notices: vec!["Local import and exploration only; reference comparison and molecular Jev inference are not implemented in this milestone.".into(),
            "Per-sample QC includes at most 1000 issue details; summary counts cover every record.".into()],
    };
    Ok(ImportedDataset {
        manifest,
        records,
        source: bytes,
        mapping_source: mapping.map(|(data, _)| data),
    })
}

fn validate_options(options: &ImportOptions) -> Result<(), DatasetError> {
    let c = &options.config;
    if [
        &options.dataset_id,
        &options.source_name,
        &c.gene_column,
        &c.value_column,
        &c.sample_column,
    ]
    .into_iter()
    .any(|v| !valid_label(v))
        || [
            &options.study,
            &options.accession,
            &options.citation,
            &c.single_sample_id,
            &c.reference_genome,
            &c.platform,
        ]
        .into_iter()
        .flatten()
        .any(|v| !valid_label(v))
        || c.gene_column == c.value_column
        || c.gene_column == c.sample_column
        || c.value_column == c.sample_column
        || c.organism != "Homo sapiens"
        || !josh_features::valid_transform(c)
    {
        return Err(DatasetError::Configuration);
    }
    Ok(())
}

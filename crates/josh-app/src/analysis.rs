//! Shared guided-workflow prerequisites, imports, archive loading and reports.
use crate::{
    molecular, report,
    workflows::{self, AppError},
};
use josh_core::{DataClass, molecular::*};
use josh_explain::Archive;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

fn invalid(message: &'static str) -> AppError {
    josh_core::ValidationError(message).into()
}

pub struct Loaded {
    pub features: FeatureSet,
    pub inference: Option<InferenceRun>,
    pub archive: Option<Archive>,
    pub root: Option<PathBuf>,
}

/// Reopenable inference-only export; legacy bare inference files still require
/// their features.json sidecar. An absent explanation is not a fake checkpoint.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RunExport {
    pub kind: String,
    pub schema_version: u32,
    pub features: FeatureSet,
    pub inference: InferenceRun,
}
pub fn export_run(features: &FeatureSet, inference: &InferenceRun) -> Result<String, AppError> {
    inference.verify(features)?;
    workflows::pretty(&RunExport {
        kind: "josh_sample_run".into(),
        schema_version: 1,
        features: features.clone(),
        inference: inference.clone(),
    })
}

/// A directory can contain inference alone or an optional complete/incomplete explanation.
pub fn load(path: &Path) -> Result<Loaded, AppError> {
    if path == Path::new("-") {
        return Err(invalid(
            "choose a file or run directory, not terminal stdin",
        ));
    }
    if path.is_dir() {
        let features = molecular::load_features(&path.join("features.json"))?;
        let inference: InferenceRun = workflows::read_json(&path.join("inference.json"), 2 * 1024 * 1024)
            .map_err(|_| invalid("no valid completed inference in this directory; inspect run-status.json before starting a new request"))?;
        inference.verify(&features)?;
        let archive_path = ["explanation.json", "checkpoint.json"]
            .iter()
            .map(|f| path.join(f))
            .find(|p| p.exists());
        let archive = archive_path
            .as_deref()
            .map(molecular::load_archive)
            .transpose()?;
        if let Some(a) = &archive
            && (hash(&a.features)? != hash(&features)? || hash(&a.inference)? != hash(&inference)?)
        {
            return Err(invalid(
                "run directory contains mismatched inference and explanation",
            ));
        }
        return Ok(Loaded {
            features,
            inference: Some(inference),
            archive,
            root: Some(path.into()),
        });
    }
    let value: serde_json::Value = workflows::read_json(path, 64 * 1024 * 1024)?;
    if value.get("kind").and_then(|v| v.as_str()) == Some("josh_sample_run") {
        let run: RunExport = serde_json::from_value(value)?;
        if run.schema_version != 1 {
            return Err(invalid("unsupported sample run schema"));
        }
        run.inference.verify(&run.features)?;
        return Ok(Loaded {
            features: run.features,
            inference: Some(run.inference),
            archive: None,
            root: None,
        });
    }
    if value.get("inference").is_some() {
        let a = molecular::load_archive(path)?;
        Ok(Loaded {
            features: a.features.clone(),
            inference: Some(a.inference.clone()),
            archive: Some(a),
            root: path.parent().map(Path::to_path_buf),
        })
    } else if value.get("request_sha256").is_some() {
        let root = path
            .parent()
            .ok_or_else(|| invalid("inference requires its features.json sidecar"))?;
        let f = molecular::load_features(&root.join("features.json"))?;
        let r: InferenceRun = workflows::read_json(path, 2 * 1024 * 1024)?;
        r.verify(&f)?;
        Ok(Loaded {
            features: f,
            inference: Some(r),
            archive: None,
            root: Some(root.into()),
        })
    } else {
        Ok(Loaded {
            features: molecular::load_features(path)?,
            inference: None,
            archive: None,
            root: None,
        })
    }
}

pub fn export_markdown(path: &Path) -> Result<String, AppError> {
    let loaded = load(path)?;
    report::markdown(
        &loaded.features,
        loaded
            .inference
            .as_ref()
            .ok_or_else(|| invalid("analyze the loaded data before exporting a report"))?,
        loaded.archive.as_ref(),
    )
}

pub fn table_format(path: &Path) -> Option<josh_ingest::molecular::Format> {
    use josh_ingest::molecular::Format;
    match path.extension()?.to_str()?.to_ascii_lowercase().as_str() {
        "csv" => Some(Format::Csv),
        "tsv" => Some(Format::Tsv),
        "maf" => Some(Format::Maf),
        "vcf" => Some(Format::Vcf),
        _ => None,
    }
}

pub fn import_table(
    path: &Path,
    options: &josh_ingest::molecular::Options,
) -> Result<FeatureSet, AppError> {
    let format =
        table_format(path).ok_or_else(|| invalid("supported tables: .csv, .tsv, .maf or .vcf"))?;
    Ok(josh_ingest::molecular::import(std::fs::File::open(path)?, format, options)?.features)
}

/// Bounded, local sample discovery. This is not biological validation; the strict
/// importer validates the selected sample before it can become the active input.
pub fn table_samples(path: &Path) -> Result<Vec<(String, usize)>, AppError> {
    use josh_ingest::molecular::{Format, MAX_BYTES};
    use std::io::Read;
    let format = table_format(path).ok_or("Unsupported table format")?;
    let mut bytes = Vec::new();
    std::fs::File::open(path)?
        .take(MAX_BYTES as u64 + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() > MAX_BYTES {
        return Err("Table exceeds the 16 MiB import limit.".into());
    }
    let mut counts = std::collections::BTreeMap::new();
    if matches!(format, Format::Vcf) {
        let source = std::str::from_utf8(&bytes)?;
        let header = source
            .lines()
            .find(|l| l.starts_with("#CHROM\t"))
            .ok_or("VCF header missing.")?;
        let columns: Vec<_> = header.split('\t').collect();
        if columns.len() != 10 {
            return Err(
                "Only annotated single-sample VCF is supported. Export one sample first.".into(),
            );
        }
        counts.insert(
            columns[9].to_owned(),
            source
                .lines()
                .filter(|l| !l.starts_with('#') && !l.is_empty())
                .count(),
        );
    } else {
        let mut reader = csv::ReaderBuilder::new()
            .delimiter(if matches!(format, Format::Csv) {
                b','
            } else {
                b'\t'
            })
            .comment(matches!(format, Format::Maf).then_some(b'#'))
            .from_reader(bytes.as_slice());
        let column = if matches!(format, Format::Maf) {
            "Tumor_Sample_Barcode"
        } else {
            "sample_id"
        };
        let index = reader
            .headers()?
            .iter()
            .position(|h| h == column)
            .ok_or("No sample identifier column found. Check the import format in Help.")?;
        for row in reader.records() {
            let row = row?;
            let id = row
                .get(index)
                .filter(|s| !s.trim().is_empty())
                .ok_or("A row has no sample identifier.")?;
            *counts.entry(id.to_owned()).or_insert(0) += 1;
        }
    }
    if counts.is_empty() {
        return Err("No sample identifiers found in this table.".into());
    }
    if counts.len() > 1000 {
        return Err("Open a table with at most 1,000 samples in the workbench.".into());
    }
    Ok(counts.into_iter().collect())
}

#[derive(Serialize)]
pub struct Readiness {
    pub ready: bool,
    pub sends_to_provider: bool,
    pub model: &'static str,
    pub key_configured: bool,
    pub features: usize,
    pub observed: usize,
    pub unavailable: usize,
    pub request_bytes: Option<usize>,
    pub blockers: Vec<String>,
}
impl Readiness {
    pub fn text(&self) -> String {
        let mut s = format!(
            "{}\n{} observed / {} unavailable observations\nJev {} · credential {}\nOne request per analysis; explanations are optional.\n",
            if self.ready {
                "Ready to analyze"
            } else {
                "Setup needed"
            },
            self.observed,
            self.unavailable,
            self.model,
            if self.key_configured {
                "configured (not authenticated yet)"
            } else {
                "missing"
            }
        );
        for b in &self.blockers {
            s.push_str(&format!("\n- {b}"));
        }
        s
    }
}
pub fn readiness(f: &FeatureSet, t: &TaxonomyDefinition) -> Readiness {
    let observed = f
        .features
        .iter()
        .filter(|f| f.status == MeasurementStatus::Observed)
        .count();
    let key_configured = workflows::key_configured();
    let mut blockers = vec![];
    let request_bytes = match prepare(f, t) {
        Ok(r) => serde_json::to_vec(&r).ok().map(|v| v.len()),
        Err(e) => {
            blockers.push(e.to_string());
            None
        }
    };
    if f.data_class != DataClass::Synthetic {
        blockers.push("Live Jev currently accepts synthetic data only. Real research data can be inspected locally; provider eligibility is still required.".into());
    }
    if !key_configured {
        blockers.push("Load a TypeSafe Jev key from Menu > Set TypeSafe Jev API key for this session, or set TYPESAFE_API_KEY before launching. A .env file is not loaded automatically. The offline demo needs no key.".into());
    }
    Readiness {
        ready: blockers.is_empty(),
        sends_to_provider: false,
        model: josh_core::MODEL,
        key_configured,
        features: f.features.len(),
        observed,
        unavailable: f.features.len() - observed,
        request_bytes,
        blockers,
    }
}

/// New destinations only; the actual write still uses create_new/create_dir to avoid races.
pub fn check_destination(path: &Path) -> Result<(), AppError> {
    if path.as_os_str().is_empty() || path.exists() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::AlreadyExists,
            "choose a new run directory; existing runs are protected",
        )
        .into());
    }
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    if !parent.is_dir() {
        return Err(invalid(
            "the parent folder does not exist; create it or choose an existing parent",
        ));
    }
    // An empty temporary file checks real writability without changing a run.
    tempfile::NamedTempFile::new_in(parent)?;
    Ok(())
}

pub fn suggested_run_path() -> PathBuf {
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    PathBuf::from(format!("josh-run-{stamp}"))
}

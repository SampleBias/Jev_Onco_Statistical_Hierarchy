//! Export System One requests for the GENERATED gut fixture collection
//! (fixtures/gut-generated-v1/*.tsv) without provider calls.
//!
//! Usage: prepare_gut_fixture <repo-root> <out-dir>
//!   repo-root: the JOSH repo (locates fixtures/gut-generated-v1 and writes
//!              out-dir relative to cwd otherwise).
use josh_app::{
    molecular,
    workflows::AppError,
};
use josh_core::molecular::{self as core, Modality};
use josh_ingest::molecular::{self as ingest, Format, Options};
use josh_core::DataClass;
use serde_json::json;
use std::path::PathBuf;

fn main() -> Result<(), AppError> {
    let repo = std::env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .ok_or("Provide the JOSH repo root (containing fixtures/gut-generated-v1)")?;
    let out = std::env::args_os()
        .nth(2)
        .map(PathBuf::from)
        .ok_or("Provide a NEW output directory")?;
    std::fs::create_dir(&out)?;
    let fixture_dir = repo.join("fixtures").join("gut-generated-v1");
    let taxonomy = core::onconpc_taxonomy();
    let mut paths: Vec<PathBuf> = std::fs::read_dir(&fixture_dir)?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().map(|x| x == "tsv").unwrap_or(false))
        .collect();
    paths.sort();
    if paths.is_empty() {
        return Err(AppError::from(format!("no TSV fixtures in {}", fixture_dir.display())));
    }
    let mut inventory = Vec::new();
    let mut failures = 0usize;
    for path in &paths {
        let stem = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("sample")
            .to_string();
        let file = std::fs::File::open(path)
            .map_err(|e| AppError::from(format!("open {}: {e}", path.display())))?;
        // sample_id lives IN the file (IDs are not aligned with filenames) — read it.
        let raw = std::fs::read_to_string(path)
            .map_err(|e| AppError::from(format!("read {}: {e}", path.display())))?;
        let sample_id = raw
            .lines()
            .nth(1)
            .and_then(|l| l.split('\t').next())
            .unwrap_or("")
            .trim()
            .to_string();
        if sample_id.is_empty() {
            return Err(AppError::from(format!("no sample_id in {}", path.display())));
        }
        let options = Options {
            sample_id: sample_id.clone(),
            patient_group_id: sample_id.replace("SYNTH-", "SYNTH-P"),
            data_class: DataClass::Synthetic,
            source_id: format!("gut-generated-v1-{stem}"),
            assay: "synthetic-assay-v1".into(),
            reference_build: None,
        };
        let report: ingest::Report = match ingest::import(file, Format::Tsv, &options) {
            Ok(r) => r,
            Err(e) => {
                failures += 1;
                eprintln!("import failed for {}: {e}", path.display());
                continue;
            }
        };
        let mut features = report.features;
        // keep everything: the full-workup track is the training view
        features.features.retain(|f| {
            !matches!(f.modality, Modality::Signature) || true
        });
        features.validate()?;
        molecular::write_new(&out.join(format!("{stem}.features.json")), &features)?;
        let request = core::prepare_versioned(&features, &taxonomy, core::PROMPT)?;
        molecular::write_new(&out.join(format!("{stem}.request.json")), &request)?;
        inventory.push(json!({
            "sample": stem,
            "source_sha256": report.source_sha256,
            "accepted_records": report.accepted_records,
            "notices": report.notices,
        }));
    }
    molecular::write_new(
        &out.join("inventory.json"),
        &json!({
            "collection": "gut-generated-v1",
            "prompt_version": core::PROMPT,
            "prepared": inventory.len(),
            "failures": failures,
            "samples": inventory,
        }),
    )?;
    println!(
        "Prepared {} generated workups ({} import failures) — prompt {} — no provider calls.",
        inventory.len(),
        failures,
        core::PROMPT
    );
    Ok(())
}

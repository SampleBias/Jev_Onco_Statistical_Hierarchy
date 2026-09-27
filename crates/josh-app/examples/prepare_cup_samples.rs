//! Export built-in CUP inputs and frozen baseline/current requests without provider calls.
use josh_app::{
    cohort::{Entry, Manifest},
    molecular,
    prompt_comparison::Comparison,
    samples,
    workflows::AppError,
};
use josh_core::molecular::{self as core, Modality};
use serde_json::{Value, json};
use std::path::PathBuf;

fn main() -> Result<(), AppError> {
    let root = std::env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .ok_or("Provide a NEW output directory")?;
    std::fs::create_dir(&root)?;
    let scenarios: Value = serde_json::from_str(include_str!(
        "../../../fixtures/cup-realistic-v2/scenarios.json"
    ))?;
    molecular::write_new(&root.join("scenarios.json"), &scenarios)?;
    let taxonomy = core::onconpc_taxonomy();
    let mut inventory = Vec::new();
    for track in ["full", "genomic", "pathology"] {
        let mut entries = Vec::new();
        let mut labeled = Vec::new();
        for i in 0..samples::SAMPLES.len() {
            let mut features = samples::load(i)?;
            features.features.retain(|f| match track {
                "genomic" => matches!(
                    f.modality,
                    Modality::Mutation
                        | Modality::CopyNumber
                        | Modality::Signature
                        | Modality::Demographic
                ),
                "pathology" => matches!(
                    f.modality,
                    Modality::Ihc | Modality::Histology | Modality::Demographic
                ),
                _ => true,
            });
            features.validate()?;
            let stem = format!("sample-{:03}-{track}", i + 1);
            molecular::write_new(&root.join(format!("{stem}.json")), &features)?;
            let mut hashes = Vec::new();
            for (suffix, version) in [("v3", core::PREVIOUS_PROMPT), ("v5", core::V5_PROMPT)] {
                let request = core::prepare_versioned(&features, &taxonomy, version)?;
                molecular::write_new(
                    &root.join(format!("{stem}-{suffix}-request.json")),
                    &request,
                )?;
                hashes.push(json!({"version": version, "sha256": core::hash(&request)?, "bytes": serde_json::to_vec(&request)?.len()}));
            }
            inventory.push(json!({"input":format!("{stem}.json"), "patient_group_id":features.patient_group_id, "feature_sha256":core::hash(&features)?, "features":features.features.len(), "requests":hashes}));
            // Only the five unambiguous author-intended in-taxonomy cases enter
            // the execution smoke-test metrics. All ten retain request previews.
            let scenario = &scenarios["scenarios"][i];
            if scenario["exercise"] == "coherent_panel" {
                entries.push(Entry {
                    features: format!("{stem}.json").into(),
                    partition: "development".into(),
                    truth: scenario["candidates"][0]
                        .as_str()
                        .ok_or("missing scenario candidate")?
                        .into(),
                });
                labeled.push(features);
            }
        }
        let manifest = Manifest {
            schema_version: 1,
            protocol_id: format!("synthetic-cup-v2-{track}-v3-v5"),
            taxonomy: taxonomy.clone(),
            cases: entries,
        };
        molecular::write_new(&root.join(format!("{track}-manifest.json")), &manifest)?;
        Comparison::new(manifest, labeled)?.prepare(&root.join(format!("{track}-comparison")))?;
    }
    molecular::write_new(
        &root.join("inventory.json"),
        &json!({
            "provider_calls":0, "distinct_patients":10, "input_views":30, "request_previews":60,
            "label_scope":"Five author-intended in-taxonomy scenarios per track; development smoke tests only. Other cases require qualitative review.",
            "paired_views":"Full, genomic and pathology views reuse the same ten patients; never split views across partitions.",
            "inputs":inventory
        }),
    )?;
    println!(
        "Prepared ten patients, thirty input views and sixty v3/v5 requests at {}. No provider calls.",
        root.display()
    );
    Ok(())
}

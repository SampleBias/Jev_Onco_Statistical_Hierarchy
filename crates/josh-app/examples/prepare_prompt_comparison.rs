//! Prepare the bundled scenarios for an engineering comparison, without provider calls.
use josh_app::{
    cohort::{Entry, Manifest},
    molecular,
    prompt_comparison::Comparison,
    samples,
    workflows::AppError,
};
use josh_core::molecular::{self as core, onconpc_taxonomy};
use std::path::PathBuf;

fn main() -> Result<(), AppError> {
    let root = std::env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .ok_or("Provide a NEW output directory")?;
    std::fs::create_dir(&root)?;
    for (i, name) in [
        "001-full",
        "001-genomic",
        "002-full",
        "002-genomic",
        "003-sparse",
    ]
    .iter()
    .enumerate()
    {
        molecular::write_new(
            &root.join(format!("sample-{name}.json")),
            &samples::load(i)?,
        )?;
    }
    for (track, indices) in [("full", [0, 2]), ("genomic", [1, 3])] {
        let manifest = Manifest {
            schema_version: 1,
            protocol_id: format!("synthetic-engineering-{track}-v3"),
            taxonomy: onconpc_taxonomy(),
            cases: [("001", "NSCLC"), ("002", "COADREAD")]
                .iter()
                .map(|(id, truth)| Entry {
                    features: format!("sample-{id}-{track}.json").into(),
                    partition: "development".into(),
                    truth: (*truth).into(),
                })
                .collect(),
        };
        let path = root.join(format!("{track}-manifest.json"));
        molecular::write_new(&path, &manifest)?;
        let features = indices
            .into_iter()
            .map(samples::load)
            .collect::<Result<Vec<_>, _>>()?;
        Comparison::new(manifest, features)?.prepare(&root.join(format!("{track}-prepared")))?;
    }
    let sparse = samples::load(4)?;
    molecular::write_new(
        &root.join("sparse-v2-request.json"),
        &core::prepare_versioned(&sparse, &onconpc_taxonomy(), core::LEGACY_PROMPT)?,
    )?;
    molecular::write_new(
        &root.join("sparse-v3-request.json"),
        &core::prepare(&sparse, &onconpc_taxonomy())?,
    )?;
    let request = core::prepare(&sparse, &onconpc_taxonomy())?;
    let mut response = molecular::demo_response(&request);
    if let josh_core::Answer::Choice { probabilities, .. } = response
        .answers
        .get_mut("primary_site")
        .expect("fixture choice")
    {
        for p in probabilities.values_mut() {
            *p *= 0.99;
        }
    }
    let bytes = serde_json::to_vec(&response)?;
    let Err(josh_jev::Error::Response(diagnostic)) =
        josh_jev::decode_response(&request, &bytes, 200)
    else {
        return Err("offline fault injection should reject the altered probability total".into());
    };
    molecular::write_new(
        &root.join("offline-fault-diagnostic.json"),
        &serde_json::json!({
            "source":"Injected local fixture; not a response from live Jev",
            "expected_code":"probability_sum","diagnostic":diagnostic
        }),
    )?;
    molecular::write_new(
        &root.join("scope.json"),
        &serde_json::json!({
            "provider_calls":0,"patients":3,"labeled_scenarios_per_track":2,
            "label_source":"Invented scenario intent in fixtures/synthetic-jev/README.md; not patient ground truth",
            "sparse_case":"No known origin label; not included in accuracy metrics",
            "partitions":"development only; no held-out cancer benchmark supplied",
            "tracks":"Full and genomic reuse the same patients; compare separately, never split between partitions"
        }),
    )?;
    println!(
        "Prepared both comparison tracks and sparse-case requests at {}. No provider calls.",
        root.display()
    );
    Ok(())
}

//! Build a self-contained, offline demo using the same services as the CLI/TUI.
use clap::Parser;
use josh_app::{
    data, molecular,
    workflows::{self, AppError},
};
use std::{
    fs,
    path::{Path, PathBuf},
};

#[derive(Parser)]
#[command(about = "Create a clean synthetic demo bundle locally; no Jev requests or key required")]
struct Args {
    /// A NEW destination. Existing directories and files are never overwritten.
    #[arg(long, default_value = "results/basic-demo")]
    out_dir: PathBuf,
}

fn save(path: &Path, content: &str) -> Result<(), AppError> {
    workflows::save_new(path, content.trim_end_matches('\n'))
}

#[tokio::main]
async fn main() -> Result<(), AppError> {
    let args = Args::parse();
    let root = args.out_dir;
    if let Some(parent) = root.parent().filter(|p| !p.as_os_str().is_empty()) {
        fs::create_dir_all(parent)?;
    }
    fs::create_dir(&root)?;
    let inputs = root.join("inputs");
    let molecular_dir = root.join("molecular");
    fs::create_dir(&inputs)?;
    fs::create_dir(&molecular_dir)?;
    for (name, content) in [
        (
            "molecular-complete.tsv",
            include_str!("../../../fixtures/basic-demo/molecular-complete.tsv"),
        ),
        (
            "molecular-missing.tsv",
            include_str!("../../../fixtures/basic-demo/molecular-missing.tsv"),
        ),
        (
            "expression.tsv",
            include_str!("../../../fixtures/expression/synthetic-expression.tsv"),
        ),
        (
            "hgnc-six-gene.tsv",
            include_str!("../../../fixtures/expression/hgnc-subset.tsv"),
        ),
        (
            "hgnc-provenance.json",
            include_str!("../../../fixtures/expression/hgnc-subset.provenance.json"),
        ),
    ] {
        save(&inputs.join(name), content)?;
    }
    let mut cases = Vec::new();
    for (name, sample, patient) in [
        ("complete", "DEMO-MOL-01", "DEMO-PAT-01"),
        ("missing", "DEMO-MOL-02", "DEMO-PAT-02"),
    ] {
        let output = molecular::execute(molecular::Command::Import {
            input: inputs.join(format!("molecular-{name}.tsv")),
            input_format: molecular::InputFormat::Tsv,
            sample: sample.into(),
            patient_group: patient.into(),
            source_id: format!("basic-demo-{name}-v1"),
            assay: "invented-panel".into(),
            reference_build: None,
            synthetic: true,
        })
        .await?;
        let features_path = molecular_dir.join(format!("{name}.json"));
        molecular::write_new(&features_path, &output.value)?;
        let features = molecular::load_features(&features_path)?;
        let preview = molecular::execute(molecular::Command::Prepare {
            features: features_path,
            taxonomy: None,
        })
        .await?;
        molecular::write_new(
            &molecular_dir.join(format!("{name}-request.json")),
            &preview.value,
        )?;
        cases.push(serde_json::json!({
            "sample_id":sample, "features":features.features.len(), "observed_groups":features.groups().len(),
            "feature_sha256":josh_core::molecular::hash(&features)?,
            "source_sha256":features.features[0].source.sha256,
        }));
    }
    let expression = root.join("expression");
    let imported = data::execute(data::DatasetCommand::Import {
        input: inputs.join("expression.tsv"),
        expression: data::ExpressionArgs {
            delimiter: data::Separator::Tsv,
            layout: data::Layout::Wide,
            gene_column: "gene".into(),
            value_column: "expression".into(),
            sample_column: "sample_id".into(),
            sample_id: None,
            gene_ids: data::GeneIds::Symbol,
            units: data::Units::Tpm,
            transform: data::Scaling::Log2OnePlus,
            genome: None,
            platform: Some("invented-demonstration".into()),
        },
        dataset_id: "basic-demo-expression-v1".into(),
        out_dir: expression.clone(),
        gene_map: Some(inputs.join("hgnc-six-gene.tsv")),
        gene_map_release: Some("hgnc-six-gene-fixture-2026-09-22".into()),
        study: None,
        accession: None,
        citation: Some("Invented basic functionality demonstration".into()),
        synthetic: true,
    })?;
    if imported.blocked {
        return Err("demo expression import failed QC".into());
    }
    let verified = data::execute(data::DatasetCommand::Verify {
        directory: expression,
        reproduce: true,
    })?;
    if verified.blocked {
        return Err("demo expression reproduction failed".into());
    }
    molecular::write_new(&root.join("expression-verification.json"), &verified.value)?;
    let charts = root.join("charts");
    println!("Preparing offline analytical charts (128 cached coalitions)…");
    molecular::execute(molecular::Command::Demo {
        out_dir: charts.clone(),
    })
    .await?;
    let archive = molecular::load_archive(&charts.join("explanation.json"))?;
    save(
        &charts.join("explanation.svg"),
        &josh_app::molecular_charts::svg(&archive)?,
    )?;
    save(
        &charts.join("contributions.csv"),
        &josh_app::molecular_charts::csv(&archive)?,
    )?;
    save(
        &root.join("START_HERE.md"),
        include_str!("../../../fixtures/basic-demo/README.md"),
    )?;
    // The final manifest marks completion; partial failures never advertise a ready bundle.
    molecular::write_new(
        &root.join("demo-manifest.json"),
        &serde_json::json!({
            "schema_version":1, "demo_version":"basic-demo-v1", "application_version":env!("CARGO_PKG_VERSION"),
            "status":"ready", "data_class":"synthetic", "network_requests":0,
            "molecular_cases":cases, "expression":{"samples":2,"genes_per_sample":6,"reproduced":true},
            "charts":{"source":"mock","sample_id":archive.features.sample_id,"result":archive.result},
            "limitations":["Invented measurements; no cancer ground truth.","Charts use the fixed built-in analytical fixture, not predictions for edited demo inputs.","The real six-gene HGNC dictionary maps identifiers only; it is not a cancer reference."]
        }),
    )?;
    println!("Ready: {}", root.display());
    println!(
        "2 molecular samples · 2 expression samples · circular/scatter/waterfall archive · SVG/CSV exports"
    );
    println!(
        "No key read and no provider calls. Open START_HERE.md for commands and expected results."
    );
    Ok(())
}

use serde_json::Value;
use std::process::{Command, Output};
fn run(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_josh"))
        .args(args)
        .env_remove("TYPESAFE_API_KEY")
        .output()
        .unwrap()
}
fn fixture(name: &str) -> String {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/expression")
        .join(name)
        .display()
        .to_string()
}
fn import(root: &str) -> Output {
    run(&[
        "dataset",
        "import",
        &fixture("synthetic-expression.tsv"),
        "--dataset-id",
        "cli-test",
        "--units",
        "tpm",
        "--transform",
        "log2-one-plus",
        "--gene-map",
        &fixture("hgnc-subset.tsv"),
        "--gene-map-release",
        "fixture-v1",
        "--synthetic",
        "--out-dir",
        root,
    ])
}

#[test]
fn full_offline_dataset_workflow_and_reproduction() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("dataset");
    let root = root.to_str().unwrap();
    let output = import(root);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let value: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["samples"].as_array().unwrap().len(), 2);
    assert!(
        run(&["dataset", "verify", root, "--reproduce"])
            .status
            .success()
    );
    let output = run(&[
        "dataset",
        "explore",
        root,
        "--sample",
        "DEMO-EXPR-01",
        "--gene",
        "TP53",
    ]);
    assert!(output.status.success());
    let value: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["records"][0]["mapping"]["gene"]["symbol"], "TP53");
    assert!(
        run(&["dataset", "inspect", root, "--format", "text"])
            .status
            .success()
    );
    let exported = run(&[
        "dataset",
        "export",
        root,
        "--sample",
        "DEMO-EXPR-01",
        "--kind",
        "tsv",
    ]);
    assert!(exported.status.success());
    let text = String::from_utf8(exported.stdout).unwrap();
    assert!(text.starts_with("original_gene_id\thgnc_id\tsymbol"));
    assert!(text.contains("TP53\tHGNC:11998\tTP53"));
}

#[test]
fn importing_into_existing_directory_preserves_the_first_bundle() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("dataset");
    let root = root.to_str().unwrap();
    assert!(import(root).status.success());
    let before = std::fs::read(std::path::Path::new(root).join("dataset.json")).unwrap();
    let output = import(root);
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("output_exists"));
    assert_eq!(
        before,
        std::fs::read(std::path::Path::new(root).join("dataset.json")).unwrap()
    );
}

#[test]
fn invalid_expression_is_archived_but_exits_with_qc_failure() {
    let temp = tempfile::tempdir().unwrap();
    let source = temp.path().join("bad.tsv");
    std::fs::write(&source, "gene\texpression\nTP53\t-3\n").unwrap();
    let root = temp.path().join("dataset");
    let output = run(&[
        "dataset",
        "import",
        source.to_str().unwrap(),
        "--dataset-id",
        "bad",
        "--sample-id",
        "S",
        "--units",
        "tpm",
        "--out-dir",
        root.to_str().unwrap(),
    ]);
    assert_eq!(output.status.code(), Some(3));
    let value: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["samples"][0]["assays"][0]["qc"]["invalid_values"], 1);
    assert!(root.join("source/input").is_file());
}

#[test]
fn missing_sample_selection_is_actionable_and_exports_do_not_overwrite() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("dataset");
    let root = root.to_str().unwrap();
    assert!(import(root).status.success());
    let output = run(&["dataset", "explore", root]);
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("sample_selection"));
    let dest = temp.path().join("export.json");
    std::fs::write(&dest, "existing").unwrap();
    let output = run(&[
        "dataset",
        "export",
        root,
        "--sample",
        "DEMO-EXPR-01",
        "--output",
        dest.to_str().unwrap(),
    ]);
    assert!(!output.status.success());
    assert_eq!(std::fs::read_to_string(dest).unwrap(), "existing");
}

#[test]
fn expression_configuration_errors_do_not_appear_as_internal_errors() {
    let temp = tempfile::tempdir().unwrap();
    let source = temp.path().join("single.tsv");
    std::fs::write(&source, "gene\texpression\nTP53\t3\n").unwrap();
    let output = run(&["dataset", "detect", source.to_str().unwrap()]);
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("dataset_configuration"));
}

#[test]
fn new_contracts_validate_real_import_outputs() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("dataset");
    let root = root.to_str().unwrap();
    let output = import(root);
    assert!(output.status.success());
    let value: Value = serde_json::from_slice(&output.stdout).unwrap();
    let schema = run(&["schema", "dataset"]);
    let schema: Value = serde_json::from_slice(&schema.stdout).unwrap();
    assert!(jsonschema::validator_for(&schema).unwrap().is_valid(&value));
    let sample_schema = run(&["schema", "sample"]);
    let sample_schema: Value = serde_json::from_slice(&sample_schema.stdout).unwrap();
    assert!(
        jsonschema::validator_for(&sample_schema)
            .unwrap()
            .is_valid(&value["samples"][0])
    );
}

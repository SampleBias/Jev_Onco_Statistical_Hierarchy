use serde_json::Value;
use std::io::Write;
use std::process::{Command, Stdio};

fn cli() -> Command {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_nexus"));
    cmd.env_remove("TYPESAFE_API_KEY");
    cmd
}
fn import(path: &std::path::Path, root: &std::path::Path) -> std::process::Output {
    cli()
        .arg("import")
        .arg(path)
        .args([
            "--input-format",
            "csv",
            "--source-id",
            "synthetic-v1",
            "--out-dir",
        ])
        .arg(root)
        .output()
        .unwrap()
}

#[test]
fn import_export_validate_prepare_and_inspect_work_end_to_end() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("input.csv");
    let root = dir.path().join("run");
    std::fs::write(
        &input,
        include_str!("../../../fixtures/import/synthetic-findings.csv"),
    )
    .unwrap();
    let output = import(&input, &root);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let report: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["accepted_cases"], 3);
    let case = root.join("cases/DEMO-001.json");
    let output = cli().arg("validate").arg(&case).output().unwrap();
    assert!(output.status.success());
    let output = cli().arg("prepare").arg(&case).output().unwrap();
    assert!(output.status.success());
    let request: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(request["state"]["age_years"], Value::Null);
    assert_eq!(request["state"]["age_lower_bound_exclusive"], 89);
    assert!(request["state"].get("metadata").is_none());
    assert!(request["state"]["findings"][0].get("source").is_none());
    let output = cli()
        .arg("batch")
        .arg(&root)
        .args(["--format", "text"])
        .output()
        .unwrap();
    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("3 accepted"));
    let second = import(&input, &root);
    assert_eq!(second.status.code(), Some(1));
    let error: Value = serde_json::from_slice(&second.stderr).unwrap();
    assert_eq!(error["error"]["code"], "output_exists");
}

#[test]
fn partial_import_writes_report_and_exits_three() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("partial.csv");
    let root = dir.path().join("run");
    let csv = include_str!("../../../fixtures/import/synthetic-findings.csv")
        .replace("CK7,positive,positive", "CK7,positive,negative");
    std::fs::write(&input, csv).unwrap();
    let output = import(&input, &root);
    assert_eq!(output.status.code(), Some(3));
    assert!(output.stderr.is_empty());
    let report: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["status"], "partial");
    assert_eq!(report["rejected_records"], 2);
    assert!(!root.join("cases/DEMO-001.json").exists());
    assert!(root.join("manifest.json").exists());
}

#[test]
fn stdin_migration_and_new_schema_commands_work_without_credentials() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("run");
    let mut child = cli()
        .args([
            "import",
            "-",
            "--input-format",
            "jsonl",
            "--source-id",
            "stdin-v1",
            "--out-dir",
        ])
        .arg(&root)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(include_bytes!(
            "../../../fixtures/import/synthetic-cases.jsonl"
        ))
        .unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(output.status.success());
    let report: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["accepted_cases"], 2);
    for schema in ["import-report", "splits", "labels"] {
        let output = cli().args(["schema", schema]).output().unwrap();
        assert!(output.status.success());
        let _: Value = serde_json::from_slice(&output.stdout).unwrap();
    }
}

#[test]
fn import_argument_and_header_failures_are_sanitized_and_create_no_bundle() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("invalid.csv");
    let root = dir.path().join("run");
    std::fs::write(&input, "patient-private-column\nprivate-value\n").unwrap();
    let output = import(&input, &root);
    assert_eq!(output.status.code(), Some(1));
    let error: Value = serde_json::from_slice(&output.stderr).unwrap();
    assert_eq!(error["error"]["code"], "invalid_import_columns");
    assert!(!String::from_utf8_lossy(&output.stderr).contains("private"));
    assert!(!root.exists());
    let output = cli()
        .arg("import")
        .arg(&input)
        .args(["--input-format", "csv", "--source-id", "id", "--out-dir"])
        .arg(&root)
        .arg("--output")
        .arg(dir.path().join("report.json"))
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(1));
    assert!(!root.exists());
}

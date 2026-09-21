use serde_json::Value;
use std::{
    io::Write,
    process::{Command, Stdio},
};

fn cli() -> Command {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_nexus"));
    cmd.env_remove("TYPESAFE_API_KEY");
    cmd
}

#[test]
fn standalone_demo_works_outside_the_checkout() {
    let dir = tempfile::tempdir().unwrap();
    let output = cli()
        .args(["demo"])
        .current_dir(dir.path())
        .output()
        .unwrap();
    assert!(output.status.success());
    let result: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(result["source"], "mock");
    assert_eq!(result["status"], "abstained");
    assert_eq!(result["rankings"].as_array().unwrap().len(), 14);
}

#[test]
fn stdin_supports_shell_pipelines() {
    let example = cli().arg("example").output().unwrap();
    let mut child = cli()
        .args(["validate", "-"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(&example.stdout)
        .unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(output.status.success());
    let value: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["valid"], true);
}

#[test]
fn exports_never_overwrite_an_existing_file() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("case.json");
    assert!(
        cli()
            .arg("example")
            .arg("--output")
            .arg(&file)
            .status()
            .unwrap()
            .success()
    );
    let original = std::fs::read(&file).unwrap();
    let second = cli()
        .arg("demo")
        .arg("--output")
        .arg(&file)
        .output()
        .unwrap();
    assert!(!second.status.success());
    assert!(String::from_utf8_lossy(&second.stderr).contains("already exists"));
    assert_eq!(std::fs::read(&file).unwrap(), original);
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(
            std::fs::metadata(file).unwrap().permissions().mode() & 0o777,
            0o600
        );
    }
}

#[test]
fn text_output_marks_mock_scores_and_all_outcomes() {
    let output = cli().args(["demo", "--format", "text"]).output().unwrap();
    assert!(output.status.success());
    let text = String::from_utf8(output.stdout).unwrap();
    assert!(text.contains("MOCK / NO PREDICTION"));
    assert!(text.contains("abstained"));
    for origin in nexus_core::taxonomy().keys() {
        assert!(text.contains(origin));
    }
}

#[test]
fn doctor_does_not_expose_credentials() {
    let output = cli()
        .env("TYPESAFE_API_KEY", "dummy-do-not-print-me")
        .arg("doctor")
        .output()
        .unwrap();
    assert!(output.status.success());
    let text = String::from_utf8(output.stdout).unwrap();
    assert!(!text.contains("dummy-do-not-print-me"));
    let value: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(value["api_key_configured"], true);
    assert_eq!(value["provider_check"], "not_attempted");
}

#[test]
fn help_version_and_noninteractive_tui_have_clear_behavior() {
    let help = cli().arg("--help").output().unwrap();
    assert!(help.status.success());
    assert!(String::from_utf8_lossy(&help.stdout).contains("tui"));
    assert!(cli().arg("--version").status().unwrap().success());
    let tui = cli().arg("tui").stdin(Stdio::null()).output().unwrap();
    assert_eq!(tui.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&tui.stderr).contains("interactive terminal"));
}

#[test]
fn malformed_input_errors_do_not_echo_case_content() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("bad.json");
    std::fs::write(&file, r#"{"private-value": "do-not-echo"}"#).unwrap();
    let output = cli().arg("validate").arg(file).output().unwrap();
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    assert!(!String::from_utf8_lossy(&output.stderr).contains("do-not-echo"));
}

use serde_json::Value;
use std::{
    io::Write,
    process::{Command, Stdio},
};

fn cli() -> Command {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_josh"));
    cmd.env_remove("TYPESAFE_API_KEY");
    cmd
}

#[test]
fn clinical_template_and_guidance_work_offline_with_json_and_text_exports() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("clinical.json");
    let template = cli()
        .args(["example", "--clinical", "--output"])
        .arg(&path)
        .output()
        .unwrap();
    assert!(template.status.success(), "{:?}", template.stderr);
    let case: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    assert_eq!(case["schema_version"], 3);
    for format in ["json", "text"] {
        let output = cli()
            .arg("guidance")
            .arg(&path)
            .args(["--format", format])
            .output()
            .unwrap();
        assert!(output.status.success(), "{:?}", output.stderr);
        if format == "json" {
            let report: Value = serde_json::from_slice(&output.stdout).unwrap();
            assert_eq!(report["research_only"], true);
            assert_eq!(report["scope"], "in_scope");
            assert!(report["items"].as_array().unwrap().len() > 20);
        } else {
            let text = String::from_utf8(output.stdout).unwrap();
            assert!(text.contains("NICE"));
            assert!(text.contains("https://www.nice.org.uk/"));
        }
    }
    let schema = cli().args(["schema", "guidance"]).output().unwrap();
    assert!(schema.status.success());
    assert!(
        serde_json::from_slice::<Value>(&schema.stdout)
            .unwrap()
            .is_object()
    );
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
    for origin in josh_core::taxonomy().keys() {
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
    let help = cli().args(["tui", "--help"]).output().unwrap();
    let help = String::from_utf8_lossy(&help.stdout);
    assert!(help.contains("[INPUT]"));
    for removed in [
        "--legacy",
        "--workbench",
        "--molecular",
        "--explanation",
        "--case",
        "--batch",
        "--dataset",
    ] {
        assert!(
            !help.contains(removed),
            "separate TUI mode still advertised: {removed}"
        );
    }
    let with_input = cli()
        .args(["tui", "fixtures/clinical-case.json"])
        .stdin(Stdio::null())
        .output()
        .unwrap();
    assert_eq!(with_input.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&with_input.stderr).contains("interactive terminal"));
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
    let error: Value = serde_json::from_slice(&output.stderr).unwrap();
    assert_eq!(error["error"]["code"], "invalid_json");
}

#[test]
fn structured_errors_cover_arguments_io_validation_and_credentials() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("case.json");
    let mut case = serde_json::to_value(josh_app::workflows::example_case()).unwrap();
    case["schema_version"] = 2.into();
    std::fs::write(&file, case.to_string()).unwrap();
    let path = file.to_str().unwrap();
    for (args, code, exit) in [
        (vec!["bad-private-argument"], "invalid_arguments", 2),
        (
            vec!["validate", "/missing-do-not-echo.json"],
            "file_not_found",
            1,
        ),
        (vec!["validate", path], "invalid_case", 1),
        (vec!["serve", "--output", path], "invalid_arguments", 1),
    ] {
        let output = cli().args(args).output().unwrap();
        assert_eq!(output.status.code(), Some(exit));
        assert!(output.stdout.is_empty());
        let error: Value = serde_json::from_slice(&output.stderr).unwrap();
        assert_eq!(error["error"]["code"], code);
        assert!(!String::from_utf8_lossy(&output.stderr).contains("private-argument"));
        assert!(!String::from_utf8_lossy(&output.stderr).contains("do-not-echo"));
    }
    std::fs::write(
        &file,
        serde_json::to_vec(&josh_app::workflows::example_case()).unwrap(),
    )
    .unwrap();
    let output = cli().args(["classify", path]).output().unwrap();
    let error: Value = serde_json::from_slice(&output.stderr).unwrap();
    assert_eq!(error["error"]["code"], "missing_api_key");
}

#[test]
fn stdin_rejects_trailing_json_and_checks_raw_byte_limit() {
    let original = serde_json::to_string(&josh_app::workflows::example_case()).unwrap();
    let exact = format!(
        "{original}{}",
        " ".repeat(josh_core::MAX_CASE_BYTES - original.len())
    );
    for (input, expected) in [
        (format!("{original} {{}}"), Some("invalid_json")),
        (format!("{exact} "), Some("input_too_large")),
        (exact, None),
    ] {
        let mut child = cli()
            .args(["validate", "-"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        child
            .stdin
            .take()
            .unwrap()
            .write_all(input.as_bytes())
            .unwrap();
        let output = child.wait_with_output().unwrap();
        if let Some(code) = expected {
            assert_eq!(output.status.code(), Some(1));
            let error: Value = serde_json::from_slice(&output.stderr).unwrap();
            assert_eq!(error["error"]["code"], code);
        } else {
            assert!(output.status.success());
        }
    }
}

#[test]
fn schema_command_exports_machine_readable_contracts() {
    for kind in [
        "case",
        "jev-request",
        "jev-response",
        "result",
        "error",
        "openapi",
    ] {
        let output = cli().args(["schema", kind]).output().unwrap();
        assert!(output.status.success());
        let doc: Value = serde_json::from_slice(&output.stdout).unwrap();
        assert!(doc.get("$schema").is_some() || doc.get("openapi").is_some());
    }
}

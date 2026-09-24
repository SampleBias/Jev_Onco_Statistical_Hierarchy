use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use josh_app::{analysis, molecular, molecular_tui::App, report};
use josh_core::{JevRequest, JevResponse, Source, molecular::*};
use josh_explain::Evaluator;
use std::path::{Path, PathBuf};

fn fixture(path: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(path)
}
fn key(app: &mut App, code: KeyCode) {
    app.key(KeyEvent::new(code, KeyModifiers::NONE));
}
fn clear(app: &mut App) {
    app.key(KeyEvent::new(KeyCode::Char('u'), KeyModifiers::CONTROL));
}
fn enter(app: &mut App, value: &str) {
    clear(app);
    app.paste(value);
    key(app, KeyCode::Enter);
}
fn render(app: &mut App) -> String {
    let mut t = ratatui::Terminal::new(ratatui::backend::TestBackend::new(120, 40)).unwrap();
    t.draw(|f| app.draw(f)).unwrap();
    t.backend()
        .buffer()
        .content
        .iter()
        .map(|c| c.symbol())
        .collect()
}
struct Transport {
    calls: usize,
    fail: bool,
}
impl Evaluator for Transport {
    async fn evaluate(&mut self, request: &JevRequest) -> Result<JevResponse, josh_explain::Error> {
        self.calls += 1;
        if self.fail {
            Err(josh_explain::Error::Provider)
        } else {
            Ok(molecular::demo_response(request))
        }
    }
}

#[tokio::test]
async fn imported_data_runs_once_archives_reopens_and_exports_without_a_second_call() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("analysis");
    let f = analysis::import_table(
        &fixture("fixtures/basic-demo/molecular-missing.tsv"),
        &josh_ingest::molecular::Options {
            sample_id: "DEMO-MOL-02".into(),
            patient_group_id: "P1".into(),
            source_id: "fixture".into(),
            assay: "invented-panel".into(),
            reference_build: Some("GRCh38".into()),
            data_class: josh_core::DataClass::Synthetic,
        },
    )
    .unwrap();
    let mut transport = Transport {
        calls: 0,
        fail: false,
    };
    let r = molecular::infer_with_evaluator(&root, &f, &onconpc_taxonomy(), &mut transport)
        .await
        .unwrap();
    assert_eq!(transport.calls, 1);
    let saved = std::fs::read_to_string(root.join("report.md")).unwrap();
    assert!(saved.contains("Ranked outcomes"));
    assert!(saved.contains("Unknown"));
    assert!(saved.contains("Not tested"));
    assert!(saved.contains("Unavailable"));
    assert!(saved.contains("Not requested"));
    assert!(saved.contains(&r.request_sha256));
    assert!(!saved.contains("TYPESAFE_API_KEY"));
    assert_eq!(
        analysis::load(&root)
            .unwrap()
            .inference
            .unwrap()
            .request_sha256,
        r.request_sha256
    );
    assert_eq!(
        analysis::export_markdown(&root).unwrap().trim(),
        saved.trim()
    );
    let state: serde_json::Value =
        serde_json::from_slice(&std::fs::read(root.join("run-status.json")).unwrap()).unwrap();
    assert_eq!(state["status"], "complete");
    assert!(
        molecular::infer_with_evaluator(&root, &f, &onconpc_taxonomy(), &mut transport)
            .await
            .is_err()
    );
    assert_eq!(transport.calls, 1);
    // Inference-only archives must show results and export those results, never the input.
    let mut app = App::new(Some(root.clone()), None).unwrap();
    let screen = render(&mut app);
    assert!(screen.contains("RANKED OUTCOMES"));
    assert!(!screen.contains("\"probabilities\""));
    let export = temp.path().join("export.md");
    key(&mut app, KeyCode::Char('s'));
    enter(&mut app, export.to_str().unwrap());
    assert_eq!(
        std::fs::read_to_string(&export).unwrap().trim(),
        saved.trim()
    );
    // Loading new input invalidates the prior inference and export action.
    key(&mut app, KeyCode::Char('l'));
    enter(
        &mut app,
        fixture("fixtures/molecular/synthetic-features.json")
            .to_str()
            .unwrap(),
    );
    key(&mut app, KeyCode::Char('s'));
    assert!(render(&mut app).contains("Analyze first"));
    assert_eq!(transport.calls, 1);
}

#[tokio::test]
async fn failed_provider_attempt_is_recorded_never_retried_or_replaced_by_demo() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("failed");
    let mut transport = Transport {
        calls: 0,
        fail: true,
    };
    assert!(
        molecular::infer_with_evaluator(
            &root,
            &molecular::example(),
            &onconpc_taxonomy(),
            &mut transport
        )
        .await
        .is_err()
    );
    assert_eq!(transport.calls, 1);
    assert!(root.join("request.json").exists());
    assert!(!root.join("inference.json").exists());
    assert!(!root.join("report.md").exists());
    assert!(
        std::fs::read_to_string(root.join("run-status.json"))
            .unwrap()
            .contains("failed_or_uncertain")
    );
    assert!(analysis::load(&root).is_err());
    assert!(
        molecular::infer_with_evaluator(
            &root,
            &molecular::example(),
            &onconpc_taxonomy(),
            &mut transport
        )
        .await
        .is_err()
    );
    assert_eq!(transport.calls, 1);
}

#[test]
fn table_form_preserves_errors_requires_explicit_metadata_and_never_sends_on_load() {
    let mut app = App::new(None, None).unwrap();
    assert!(render(&mut app).contains("START HERE"));
    key(&mut app, KeyCode::Char('a'));
    assert!(!app.busy());
    key(&mut app, KeyCode::Char('l'));
    enter(
        &mut app,
        fixture("fixtures/basic-demo/molecular-complete.tsv")
            .to_str()
            .unwrap(),
    );
    assert!(
        render(&mut app).contains("Import settings") || render(&mut app).contains("Import · Tab")
    );
    key(&mut app, KeyCode::Enter);
    assert!(render(&mut app).contains("Edit settings"));
    for (i, value) in [
        "DEMO-MOL-01",
        "P1",
        "demo",
        "invented",
        "GRCh38",
        "synthetic",
    ]
    .iter()
    .enumerate()
    {
        clear(&mut app);
        app.paste(value);
        if i < 5 {
            key(&mut app, KeyCode::Tab);
        }
    }
    key(&mut app, KeyCode::Enter);
    assert_eq!(app.features.sample_id, "DEMO-MOL-01");
    assert_eq!(app.features.data_class, josh_core::DataClass::Synthetic);
    assert!(!app.busy());
    assert!(app.archive.is_none());
    // A failed replacement must leave the already-loaded sample intact.
    key(&mut app, KeyCode::Char('l'));
    enter(&mut app, "/does-not-exist.json");
    assert_eq!(app.features.sample_id, "DEMO-MOL-01");
    key(&mut app, KeyCode::Esc);
    assert!(render(&mut app).contains("DEMO-MOL-01"));
}

#[test]
fn report_rejects_tampering_and_escapes_untrusted_markdown() {
    let mut f = molecular::example();
    f.features[0].name = "[link](evil)|<script> heading".into();
    let t = onconpc_taxonomy();
    let r = interpret(
        &f,
        &t,
        molecular::demo_response(&prepare(&f, &t).unwrap()),
        Source::Replay,
    )
    .unwrap();
    let text = report::markdown(&f, &r, None).unwrap();
    assert!(text.contains("UNVERIFIED REPLAY"));
    assert!(text.contains("&#124;&lt;script&gt; heading"));
    assert!(!text.contains("[link]"));
    assert_eq!(report::cell("a\nb\u{1b}"), "a b ");
    f.features[0].name = "different".into();
    assert!(report::markdown(&f, &r, None).is_err());
}

#[test]
fn readiness_cli_is_offline_actionable_and_never_prints_credentials() {
    let result = std::process::Command::new(env!("CARGO_BIN_EXE_josh"))
        .args([
            "molecular",
            "check",
            fixture("fixtures/molecular/synthetic-features.json")
                .to_str()
                .unwrap(),
        ])
        .env_remove("TYPESAFE_API_KEY")
        .output()
        .unwrap();
    assert!(result.status.success());
    let check: serde_json::Value = serde_json::from_slice(&result.stdout).unwrap();
    assert_eq!(check["ready"], false);
    assert_eq!(check["sends_to_provider"], false);
    assert!(
        check["blockers"][0]
            .as_str()
            .unwrap()
            .contains("TYPESAFE_API_KEY")
    );
    let result = std::process::Command::new(env!("CARGO_BIN_EXE_josh"))
        .args([
            "molecular",
            "check",
            fixture("fixtures/molecular/synthetic-features.json")
                .to_str()
                .unwrap(),
        ])
        .env("TYPESAFE_API_KEY", "do-not-leak-this-key")
        .output()
        .unwrap();
    assert!(!String::from_utf8_lossy(&result.stdout).contains("do-not-leak"));
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&result.stdout).unwrap()["ready"],
        true
    );
}

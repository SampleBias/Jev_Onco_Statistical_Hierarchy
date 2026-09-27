use serde_json::Value;
use std::{path::Path, process::Command};
fn cli(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_josh"))
        .args(args)
        .env_remove("TYPESAFE_API_KEY")
        .output()
        .unwrap()
}
#[test]
fn full_offline_demo_exports_reconciled_figures_and_protects_existing_files() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("demo");
    let o = cli(&["molecular", "demo", "--out-dir", root.to_str().unwrap()]);
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    let a: Value = serde_json::from_slice(&o.stdout).unwrap();
    assert_eq!(a["inference"]["source"], "mock");
    assert_eq!(a["inference"]["status"], "abstained");
    assert_eq!(a["result"]["evaluations"], 129);
    assert!(a["result"]["additivity_residual"].as_f64().unwrap().abs() < 1e-10);
    let report = std::fs::read_to_string(root.join("explanation-report.md")).unwrap();
    assert!(report.contains("ANALYTICAL DEMO"));
    assert!(report.contains("percentage points"));
    assert!(report.contains("Provenance"));
    let export = tmp.path().join("report.md");
    let markdown = cli(&[
        "molecular",
        "export",
        root.to_str().unwrap(),
        "--output",
        export.to_str().unwrap(),
    ]);
    assert!(
        markdown.status.success(),
        "{}",
        String::from_utf8_lossy(&markdown.stderr)
    );
    assert_eq!(std::fs::read_to_string(&export).unwrap(), report);
    assert!(
        !cli(&[
            "molecular",
            "export",
            root.to_str().unwrap(),
            "--output",
            export.to_str().unwrap()
        ])
        .status
        .success()
    );
    assert_eq!(std::fs::read_to_string(&export).unwrap(), report);
    let archive = root.join("explanation.json");
    let svg = tmp.path().join("figure.svg");
    let o = cli(&[
        "molecular",
        "export",
        archive.to_str().unwrap(),
        "--kind",
        "svg",
        "--output",
        svg.to_str().unwrap(),
    ]);
    assert!(o.status.success());
    let text = std::fs::read_to_string(&svg).unwrap();
    assert!(text.starts_with("<svg"));
    assert!(text.contains("ANALYTICAL DEMO"));
    assert!(text.contains("SBS4"));
    assert!(text.contains("FAT1"));
    let before = std::fs::read(&svg).unwrap();
    assert!(
        !cli(&[
            "molecular",
            "export",
            archive.to_str().unwrap(),
            "--output",
            svg.to_str().unwrap()
        ])
        .status
        .success()
    );
    assert_eq!(std::fs::read(&svg).unwrap(), before);
    let o = cli(&[
        "molecular",
        "inspect",
        archive.to_str().unwrap(),
        "--format",
        "text",
    ]);
    assert!(o.status.success());
    assert!(String::from_utf8_lossy(&o.stdout).contains("NO CANCER PREDICTION"));
    assert!(
        !cli(&["molecular", "demo", "--out-dir", root.to_str().unwrap()])
            .status
            .success()
    );
    for name in [
        "molecular-features.schema.json",
        "molecular-inference.schema.json",
        "explanation.schema.json",
    ] {
        let docs = josh_app::contracts::documents();
        let value = match name {
            "molecular-features.schema.json" => &a["features"],
            "molecular-inference.schema.json" => &a["inference"],
            _ => &a,
        };
        jsonschema::validator_for(&docs[name])
            .unwrap()
            .validate(value)
            .unwrap();
    }
}
#[test]
fn prepare_is_offline_and_live_requires_key_before_creating_run() {
    let file = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/molecular/synthetic-features.json");
    let o = cli(&["molecular", "prepare", file.to_str().unwrap()]);
    assert!(o.status.success());
    let v: Value = serde_json::from_slice(&o.stdout).unwrap();
    assert_eq!(v["sends_to_provider"], false);
    assert_eq!(
        v["request"]["questions"]["primary_site"]["criteria"]
            .as_object()
            .unwrap()
            .len(),
        24
    );
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("run");
    let o = cli(&[
        "molecular",
        "run",
        file.to_str().unwrap(),
        "--out-dir",
        root.to_str().unwrap(),
    ]);
    assert!(!o.status.success());
    assert!(!root.exists());
    assert!(String::from_utf8_lossy(&o.stderr).contains("missing_api_key"));
}
#[tokio::test]
async fn terminal_charts_resize_select_and_do_not_invoke_provider() {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    use ratatui::{Terminal, backend::TestBackend};
    let mut app = josh_app::molecular_tui::App::new(None, None).unwrap();
    app.key(KeyEvent::new(KeyCode::Char('d'), KeyModifiers::NONE));
    for _ in 0..500 {
        app.poll().await;
        if !app.busy() {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }
    assert!(!app.busy());
    assert!(app.archive.as_ref().unwrap().result.is_some());
    // The guided workflow lands on readable results; Tab opens the first chart.
    app.key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE));
    for (w, h) in [(140, 45), (120, 40), (80, 24), (60, 20), (20, 8), (1, 1)] {
        let mut t = Terminal::new(TestBackend::new(w, h)).unwrap();
        t.draw(|f| app.draw(f)).unwrap();
        app.key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));
    }
    let mut t = Terminal::new(TestBackend::new(140, 45)).unwrap();
    t.draw(|f| app.draw(f)).unwrap();
    let text = t
        .backend()
        .buffer()
        .content
        .iter()
        .map(|c| c.symbol())
        .collect::<String>();
    assert!(text.contains("ANALYTICAL DEMO"));
    assert!(text.contains("Signed contribution"));
    assert!(text.contains("Attribution magnitude"));
    app.key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE));
    app.key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE));
    t.draw(|f| app.draw(f)).unwrap();
    let text = t
        .backend()
        .buffer()
        .content
        .iter()
        .map(|c| c.symbol())
        .collect::<String>();
    assert!(text.contains("Waterfall"));
    let old = app.archive.as_ref().unwrap().config.target_class.clone();
    app.key(KeyEvent::new(KeyCode::Char('t'), KeyModifiers::NONE));
    assert_ne!(app.archive.as_ref().unwrap().config.target_class, old);
    assert!(!app.busy());
}
#[test]
fn indexed_fasta_handles_line_boundaries() {
    let dir = tempfile::tempdir().unwrap();
    let fasta = dir.path().join("test.fa");
    let fai = dir.path().join("test.fa.fai");
    std::fs::write(&fasta, b">1\nACGT\nACGT\n").unwrap();
    std::fs::write(&fai, b"1\t8\t3\t4\t5\n").unwrap();
    let mut f = josh_app::signature_workflow::Fasta::open(&fasta, &fai).unwrap();
    assert_eq!(f.context("1", 4).unwrap(), *b"GTA");
    assert!(f.context("1", 1).is_err());
    assert!(f.context("chr1", 4).is_err());
}

#[tokio::test]
async fn zero_and_tied_contributions_have_honest_empty_and_remainder_views() {
    use josh_core::{Source, molecular::*};
    use josh_explain::{Archive, Config, Progress};
    use ratatui::{Terminal, backend::TestBackend};
    for call in [0, 1] {
        let mut f = josh_app::molecular::example();
        let template = f.features[3].clone();
        f.features = (0..12)
            .map(|i| {
                let mut feature = template.clone();
                feature.id = format!("copy-{i:02}");
                feature.group = feature.id.clone();
                feature.name = format!("<Gene & {i}>");
                feature.value = Some(FeatureValue::CopyNumber {
                    gene: format!("GENE{i}"),
                    call,
                });
                feature
            })
            .collect();
        let t = onconpc_taxonomy();
        let r = interpret(
            &f,
            &t,
            josh_app::molecular::demo_response(&prepare(&f, &t).unwrap()),
            Source::Replay,
        )
        .unwrap();
        let mut a = Archive::new(
            f,
            r,
            Config {
                permutation_pairs: 2,
                ..Config::default()
            },
            None,
        )
        .unwrap();
        josh_explain::run(
            &mut a,
            &mut josh_app::molecular::AnalyticalDemo,
            &Progress::default(),
            |_| Ok(()),
        )
        .await
        .unwrap();
        a.validate().unwrap();
        let svg = josh_app::molecular_charts::svg(&a).unwrap();
        assert!(svg.contains("UNVERIFIED REPLAY"));
        assert!(svg.contains("Remaining 2 groups"));
        assert!(svg.contains("&lt;Gene &amp;"));
        assert!(!svg.contains("NaN"));
        if call == 0 {
            assert!(svg.contains("No nonzero attribution magnitude"));
        } else {
            assert!(svg.contains("magnitude 5.0000 pp"));
        }
        let mut terminal = Terminal::new(TestBackend::new(140, 45)).unwrap();
        for page in 0..3 {
            terminal
                .draw(|frame| josh_app::molecular_charts::draw(frame, frame.area(), &a, 11, page))
                .unwrap();
        }
    }
}

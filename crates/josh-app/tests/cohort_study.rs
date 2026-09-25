use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use josh_app::{
    study,
    study_charts::{self, Matrix, Normalization},
    study_tui, terminal,
};
use josh_features::study as stats;
use ratatui::{Terminal, backend::TestBackend};
use std::collections::BTreeMap;

fn key(c: char) -> KeyEvent {
    KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE)
}
#[tokio::test]
async fn failed_and_cancelled_calculations_preserve_the_previous_study() {
    let mut data = study::demo();
    data.study_id = "preserve-me".into();
    let report = stats::analyze(&data, 0, 42).unwrap();
    let mut app = study_tui::App::default();
    app.loaded = Some((data, report));
    let dir = tempfile::tempdir().unwrap();
    app.open(&dir.path().join("missing.json"));
    for _ in 0..200 {
        if !app.busy() {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        app.poll().await;
    }
    assert!(!app.busy());
    assert_eq!(app.loaded.as_ref().unwrap().0.study_id, "preserve-me");
    app.key(key('d'));
    assert!(app.busy());
    app.cancel_job();
    for _ in 0..200 {
        if !app.busy() {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        app.poll().await;
    }
    assert!(!app.busy());
    assert_eq!(app.loaded.as_ref().unwrap().0.study_id, "preserve-me");
}
#[test]
fn empty_classification_all_failures_and_no_events_have_honest_unavailable_states() {
    let mut data = study::demo();
    data.records.clear();
    for r in &mut data.outcomes.as_mut().unwrap().observations {
        r.event = false;
    }
    let report = stats::analyze(&data, 100, 42).unwrap();
    assert!(report.evaluation.is_none());
    assert!(report.thresholds.is_empty());
    assert!(report.bootstrap.is_none());
    let s = report.survival.as_ref().unwrap();
    assert!(s.log_rank.is_none() && s.subtype_log_rank.is_none() && s.cox.is_none());
    assert!(s.by_class.iter().all(|c| c.median.is_none()));
    let mut data = study::demo();
    data.outcomes = None;
    for r in &mut data.records {
        r.probabilities = None;
        r.failure = Some("unavailable".into());
        r.accepted = false;
    }
    let report = stats::analyze(&data, 100, 42).unwrap();
    assert_eq!(report.evaluation.as_ref().unwrap().failed, 528);
    assert_eq!(report.evaluation.as_ref().unwrap().top1_all_eligible, 0.);
    assert!(
        report
            .thresholds
            .iter()
            .all(|t| t.retained == 0 && t.weighted_f1.is_none())
    );
    assert_eq!(report.bootstrap.unwrap().top1.upper, 0.);
}
#[test]
fn study_schema_and_offline_cli_roundtrip_protect_exports() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("study.json");
    let archive = dir.path().join("archive.json");
    let svg = dir.path().join("plot.svg");
    let cli = |args: &[&str]| {
        std::process::Command::new(env!("CARGO_BIN_EXE_josh"))
            .env_remove("TYPESAFE_API_KEY")
            .args(args)
            .output()
            .unwrap()
    };
    let result = cli(&["study", "demo", "--output", input.to_str().unwrap()]);
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let data = study::load(&input).unwrap();
    let schema = &josh_app::contracts::documents()["cohort-study.schema.json"];
    assert!(
        jsonschema::validator_for(schema)
            .unwrap()
            .is_valid(&serde_json::to_value(data).unwrap())
    );
    for (kind, path) in [("json", &archive), ("svg", &svg)] {
        let result = cli(&[
            "study",
            "export",
            input.to_str().unwrap(),
            "--kind",
            kind,
            "--bootstrap",
            "100",
            "--output",
            path.to_str().unwrap(),
        ]);
        assert!(
            result.status.success(),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
    }
    assert!(study::load(&archive).is_ok());
    let original = std::fs::read(&svg).unwrap();
    let result = cli(&[
        "study",
        "export",
        input.to_str().unwrap(),
        "--kind",
        "svg",
        "--output",
        svg.to_str().unwrap(),
    ]);
    assert!(!result.status.success());
    assert_eq!(std::fs::read(&svg).unwrap(), original);
}
#[test]
fn complete_demo_is_explicit_and_statistics_are_finite_and_reproducible() {
    let data = study::demo();
    let a = stats::analyze(&data, 100, 42).unwrap();
    let b = stats::analyze(&data, 100, 42).unwrap();
    assert_eq!(
        serde_json::to_value(&a).unwrap(),
        serde_json::to_value(&b).unwrap()
    );
    assert_eq!(a.evaluation.as_ref().unwrap().eligible, 528);
    assert!(a.evaluation.as_ref().unwrap().failed > 0);
    assert_eq!(a.survival.as_ref().unwrap().patients, 180);
    assert!(
        a.survival.as_ref().unwrap().cox.is_some(),
        "{:?}",
        a.survival.as_ref().unwrap().cox_unavailable
    );
    assert!(a.bootstrap.is_some());
    assert_eq!(a.survival.as_ref().unwrap().weighted_concordance.len(), 2);
    assert!(a.limitations[0].contains("SYNTHETIC"));
    assert!(
        a.thresholds
            .windows(2)
            .all(|w| w[1].retained <= w[0].retained)
    );
    assert!(
        a.thresholds
            .iter()
            .all(|t| t.weighted_f1.is_none_or(|f| (0.0..=1.0).contains(&f)))
    );
}
#[test]
fn matrices_preserve_failures_unknowns_and_normalization_denominators() {
    let data = study::demo();
    let report = stats::analyze(&data, 0, 42).unwrap();
    let m = Matrix::new(&data, &report, false).unwrap();
    assert_eq!(m.rows.len(), 22);
    assert_eq!(m.columns.len(), 25);
    assert_eq!(m.counts.iter().flatten().sum::<usize>(), 528);
    for i in 0..m.rows.len() {
        assert!(
            ((0..m.columns.len())
                .map(|j| m.value(i, j, Normalization::Recall))
                .sum::<f64>()
                - 1.)
                .abs()
                < 1e-10
        );
    }
    for j in 0..m.columns.len() {
        let sum = (0..m.rows.len())
            .map(|i| m.value(i, j, Normalization::Precision))
            .sum::<f64>();
        assert!(sum == 0. || (sum - 1.).abs() < 1e-10);
    }
}
#[test]
fn broad_groups_sum_probability_mass_without_dropping_extra_outcomes() {
    let mut data = study::demo();
    data.broad_groups = data
        .taxonomy
        .classes
        .iter()
        .enumerate()
        .map(|(i, c)| (c.id.clone(), format!("group-{}", i % 2)))
        .collect();
    data.broad_group_version = Some("explicit-test-map".into());
    let mapped = stats::mapped_records(&data);
    for r in mapped {
        if let Some(p) = r.probabilities {
            assert!((p.values().sum::<f64>() - 1.).abs() < 1e-12);
            assert!(p.contains_key(&data.taxonomy.unknown_id));
            assert!(p.contains_key(&data.taxonomy.other_id));
        }
    }
    let report = stats::analyze(&data, 0, 42).unwrap();
    assert_eq!(report.broad_evaluation.unwrap().eligible, 528);
    data.broad_groups.remove("NSCLC");
    assert!(stats::validate(&data).is_err());
}
#[test]
fn patient_leakage_unknown_labels_and_invalid_outcomes_are_rejected() {
    let mut data = study::demo();
    data.records[1].patient_group_id = data.records[0].patient_group_id.clone();
    data.records[1].partition = "development".into();
    assert!(stats::validate(&data).is_err());
    let mut data = study::demo();
    data.outcomes.as_mut().unwrap().observations[0].predicted_class = "unlisted".into();
    assert!(stats::validate(&data).is_err());
    let mut data = study::demo();
    data.records[0].truth = "unlisted".into();
    assert!(stats::validate(&data).is_err());
}
#[test]
fn bootstrap_resamples_patient_clusters_and_retains_failed_samples() {
    let mut data = study::demo();
    data.records.truncate(4);
    for (i, r) in data.records.iter_mut().enumerate() {
        r.truth = "NSCLC".into();
        r.patient_group_id = format!("P{}", i / 2);
        r.probabilities = if i < 2 {
            Some(BTreeMap::from([("NSCLC".into(), 1.)]))
        } else {
            None
        };
        r.failure = if i < 2 { None } else { Some("error".into()) };
        r.accepted = i < 2;
    }
    let rows: Vec<_> = data.records.iter().collect();
    let classes = std::collections::BTreeSet::from(["NSCLC".into()]);
    let b = stats::bootstrap(&rows, &classes, 200, 123).unwrap();
    assert_eq!(b.patients, 2);
    assert_eq!(b.top1.estimate, 0.5);
    assert_eq!(b.top1.lower, 0.);
    assert_eq!(b.top1.upper, 1.);
}
#[test]
fn all_views_render_on_wide_and_tiny_terminals_and_exports_escape_input() {
    let mut data = study::demo();
    data.title = "Study <tag> & results".into();
    let report = stats::analyze(&data, 0, 42).unwrap();
    let mut app = study_tui::App::default();
    app.loaded = Some((data.clone(), report.clone()));
    for (w, h) in [
        (190, 64),
        (140, 48),
        (120, 40),
        (80, 24),
        (40, 12),
        (5, 3),
        (1, 1),
    ] {
        let mut terminal = Terminal::new(TestBackend::new(w, h)).unwrap();
        for page in 0..6 {
            app.page = page;
            terminal.draw(|f| app.draw_area(f, f.area())).unwrap();
        }
        app.key(key('i'));
        app.key(key('w'));
        app.page = 3;
        terminal.draw(|f| app.draw_area(f, f.area())).unwrap();
    }
    let svg = study_charts::svg(&data, &report);
    assert!(svg.contains("Study &lt;tag&gt; &amp; results"));
    assert!(svg.contains("SYNTHETIC DEMONSTRATION"));
    assert!(!svg.contains("NaN"));
    assert!(!svg.contains("Infinity"));
    assert!(svg.contains("<polyline"));
    assert!(study::markdown(&data, &report).contains("not cancer performance"));
}
#[tokio::test]
async fn shared_loader_routes_study_preserves_sections_and_rejects_archive_tampering() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("study.json");
    let data = study::demo();
    std::fs::write(&path, serde_json::to_vec(&data).unwrap()).unwrap();
    let mut app = terminal::App::new(Some(path)).unwrap();
    assert_eq!(app.section(), terminal::Section::Cohort);
    app.key(KeyEvent::new(KeyCode::F(2), KeyModifiers::NONE));
    for _ in 0..500 {
        app.poll().await;
        if !app.busy() {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }
    assert!(!app.busy());
    assert_eq!(app.section(), terminal::Section::Analysis);
    app.key(KeyEvent::new(KeyCode::F(5), KeyModifiers::NONE));
    let mut t = Terminal::new(TestBackend::new(140, 48)).unwrap();
    t.draw(|f| app.draw(f)).unwrap();
    let text = t
        .backend()
        .buffer()
        .content
        .iter()
        .map(|c| c.symbol())
        .collect::<String>();
    assert!(text.contains("SYNTHETIC DEMONSTRATION"));
    assert!(text.contains("CONFUSION"));
    let report = stats::analyze(&data, 0, 42).unwrap();
    let archive = tmp.path().join("report.json");
    std::fs::write(&archive, study::archive(&data, &report).unwrap()).unwrap();
    assert!(study::load(&archive).is_ok());
    let mut saved: serde_json::Value =
        serde_json::from_str(&study::archive(&data, &report).unwrap()).unwrap();
    saved["study"]["model"] = "changed".into();
    std::fs::write(&archive, serde_json::to_vec(&saved).unwrap()).unwrap();
    assert!(study::load(&archive).is_err());
}

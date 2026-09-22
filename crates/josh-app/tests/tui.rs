use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use josh_app::tui::App;
use ratatui::{Terminal, backend::TestBackend};

fn press(app: &mut App, code: KeyCode) -> bool {
    app.key(KeyEvent::new(code, KeyModifiers::NONE))
}

fn type_text(app: &mut App, text: &str) {
    for ch in text.chars() {
        press(app, KeyCode::Char(ch));
    }
}

fn add_review(app: &mut App) {
    press(app, KeyCode::Char('7'));
    press(app, KeyCode::Home);
    press(app, KeyCode::Char('a'));
    type_text(
        app,
        "reviewer-1 | 2026-09-22 | acknowledged | Synthetic scope review only",
    );
    press(app, KeyCode::Enter);
}

fn screen(app: &mut App, width: u16, height: u16) -> String {
    let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
    terminal.draw(|frame| app.draw(frame)).unwrap();
    let buffer = terminal.backend().buffer();
    (0..height)
        .map(|y| {
            (0..width)
                .map(|x| buffer[(x, y)].symbol())
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn evidence_request_and_mock_result_are_navigable() {
    let mut app = App::new(None).unwrap();
    assert!(screen(&mut app, 110, 40).contains("SYNTHETIC-001"));
    press(&mut app, KeyCode::Char('2'));
    let mut request = screen(&mut app, 110, 45);
    assert!(request.contains("Request preview"));
    for _ in 0..8 {
        press(&mut app, KeyCode::PageDown);
        request.push_str(&screen(&mut app, 110, 45));
    }
    assert!(request.contains("primary_site"));
    press(&mut app, KeyCode::Char('d'));
    let result = screen(&mut app, 110, 45);
    assert!(result.contains("MOCK / NO PREDICTION"));
    assert!(result.contains("abstained"));
    assert!(result.contains("insufficient_evidence"));
}

#[test]
fn reloading_clears_a_previous_result() {
    let mut app = App::new(None).unwrap();
    press(&mut app, KeyCode::Char('d'));
    press(&mut app, KeyCode::Char('r'));
    press(&mut app, KeyCode::Char('3'));
    assert!(screen(&mut app, 100, 30).contains("No result yet"));
}

#[test]
fn open_errors_preserve_current_case_and_show_feedback() {
    let mut app = App::new(None).unwrap();
    press(&mut app, KeyCode::Char('o'));
    press(&mut app, KeyCode::Enter);
    assert!(screen(&mut app, 110, 30).contains("enter a file path"));
    press(&mut app, KeyCode::Esc);
    assert!(screen(&mut app, 110, 30).contains("SYNTHETIC-001"));
}

#[test]
fn keyboard_export_writes_a_complete_mock_result() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("result.json");
    let mut app = App::new(None).unwrap();
    press(&mut app, KeyCode::Char('d'));
    press(&mut app, KeyCode::Char('s'));
    for ch in path.to_str().unwrap().chars() {
        press(&mut app, KeyCode::Char(ch));
    }
    press(&mut app, KeyCode::Enter);
    let value: serde_json::Value = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
    assert_eq!(value["source"], "mock");
    assert_eq!(value["rankings"].as_array().unwrap().len(), 14);
}

#[test]
fn narrow_terminals_and_exit_keys_work() {
    let mut app = App::new(None).unwrap();
    assert!(screen(&mut app, 40, 8).contains("Enlarge terminal"));
    assert!(press(&mut app, KeyCode::Char('q')));
    assert!(app.key(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL)));
    assert!(App::new(Some("-".into())).is_err());
}

#[test]
fn failed_open_keeps_previous_case_and_result_for_trailing_or_oversized_input() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("invalid.json");
    for content in [
        format!(
            "{} {{}}",
            serde_json::to_string(&josh_app::workflows::example_case()).unwrap()
        ),
        "x".repeat(josh_core::MAX_CASE_BYTES + 1),
    ] {
        std::fs::write(&path, content).unwrap();
        let mut app = App::new(None).unwrap();
        press(&mut app, KeyCode::Char('d'));
        press(&mut app, KeyCode::Char('o'));
        for ch in path.to_str().unwrap().chars() {
            press(&mut app, KeyCode::Char(ch));
        }
        press(&mut app, KeyCode::Enter);
        let error = screen(&mut app, 110, 40);
        assert!(error.contains("JSON schema") || error.contains("byte limit"));
        press(&mut app, KeyCode::Esc);
        press(&mut app, KeyCode::Char('3'));
        let result = screen(&mut app, 110, 40);
        assert!(result.contains("SYNTHETIC-001"));
        assert!(result.contains("MOCK / NO PREDICTION"));
    }
}

fn make_bundle(root: &std::path::Path) {
    let options = josh_ingest::Options {
        format: josh_ingest::InputFormat::Csv,
        source_id: "synthetic-v1".into(),
        split_seed: "test".into(),
        holdout_institution: None,
    };
    let mut bundle = josh_ingest::import(
        include_bytes!("../../../fixtures/import/synthetic-findings.csv").as_slice(),
        &options,
    )
    .unwrap();
    josh_ingest::bundle::write(root, &mut bundle).unwrap();
}

#[test]
fn batch_browsing_shows_sources_censored_age_and_quality_and_clears_results() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("run");
    make_bundle(&root);
    let mut app = App::from_batch(root).unwrap();
    let evidence = screen(&mut app, 120, 40);
    assert!(evidence.contains("DEMO-001"));
    assert!(evidence.contains("Age: >89"));
    assert!(evidence.contains("Source: synthetic-v1"));
    assert!(evidence.contains("Batch case 1 / 3"));
    press(&mut app, KeyCode::Char('d'));
    press(&mut app, KeyCode::Char(']'));
    press(&mut app, KeyCode::Char('3'));
    assert!(screen(&mut app, 120, 40).contains("No result yet"));
    press(&mut app, KeyCode::Char('1'));
    assert!(screen(&mut app, 120, 40).contains("DEMO-002"));
    press(&mut app, KeyCode::Char('5'));
    let quality = screen(&mut app, 120, 40);
    assert!(quality.contains("3 accepted"));
    assert!(quality.contains("not_tested"));
    press(&mut app, KeyCode::Char('['));
    assert!(screen(&mut app, 120, 40).contains("DEMO-001"));
}

#[test]
fn changed_batch_case_cannot_replace_current_case_or_result() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("run");
    make_bundle(&root);
    let mut app = App::from_batch(root.clone()).unwrap();
    press(&mut app, KeyCode::Char('d'));
    let path = root.join("cases/DEMO-002.json");
    let mut case: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    case["findings"][0]["value"] = "edited".into();
    std::fs::write(&path, serde_json::to_vec(&case).unwrap()).unwrap();
    press(&mut app, KeyCode::Char(']'));
    let result = screen(&mut app, 120, 40);
    assert!(result.contains("MOCK / NO PREDICTION"));
    assert!(result.contains("DEMO-001"));
    assert!(result.contains("has changed"));
}

#[test]
fn batch_dialog_and_report_export_work() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("run");
    make_bundle(&root);
    let mut app = App::new(None).unwrap();
    press(&mut app, KeyCode::Char('b'));
    for ch in root.to_str().unwrap().chars() {
        press(&mut app, KeyCode::Char(ch));
    }
    press(&mut app, KeyCode::Enter);
    assert!(screen(&mut app, 120, 40).contains("DEMO-001"));
    press(&mut app, KeyCode::Char('5'));
    press(&mut app, KeyCode::Char('s'));
    let export = dir.path().join("report-copy.json");
    for ch in export.to_str().unwrap().chars() {
        press(&mut app, KeyCode::Char(ch));
    }
    press(&mut app, KeyCode::Enter);
    let report: serde_json::Value =
        serde_json::from_slice(&std::fs::read(export).unwrap()).unwrap();
    assert_eq!(report["accepted_cases"], 3);
}

#[test]
fn themed_visuals_preserve_all_outcomes_and_distinct_ihc_statuses() {
    let mut app = App::new(None).unwrap();
    press(&mut app, KeyCode::Char('6'));
    assert!(screen(&mut app, 120, 36).contains("No result yet"));
    press(&mut app, KeyCode::Char('d'));
    press(&mut app, KeyCode::Char('6'));
    let scores = screen(&mut app, 120, 36);
    assert!(scores.contains("MOCK / NO PREDICTION"));
    assert!(scores.contains("fixed 0–100%"));
    for origin in josh_core::taxonomy().keys() {
        assert!(scores.contains(origin), "{origin}\n{scores}");
    }
    press(&mut app, KeyCode::Right);
    let ihc = screen(&mut app, 120, 36);
    for text in [
        "+ POS", "- NEG", "~ EQV", "NT", "? UNK", "CK7", "CK20", "TTF-1", "ER", "PLAP",
    ] {
        assert!(ihc.contains(text), "{text}\n{ihc}");
    }
    press(&mut app, KeyCode::Right);
    let pathway = screen(&mut app, 120, 36);
    assert!(pathway.contains("ProvisionalCup"));
    assert_eq!(pathway.matches("RECORDED HERE").count(), 1);
    press(&mut app, KeyCode::Right);
    let timeline = screen(&mut app, 120, 36);
    assert!(timeline.contains("Relative day"));
    assert!(timeline.contains("Planned"));
    press(&mut app, KeyCode::Right);
    let evidence = screen(&mut app, 120, 36);
    assert!(evidence.contains("Recorded findings by type"));
    assert!(evidence.contains("not diagnostic adequacy"));
}

#[test]
fn every_view_handles_small_terminals_and_scroll_limits() {
    let mut app = App::new(None).unwrap();
    press(&mut app, KeyCode::Char('d'));
    for (width, height) in [(1, 1), (40, 8), (48, 12), (60, 20), (80, 24), (120, 40)] {
        for tab in '1'..='8' {
            press(&mut app, KeyCode::Char(tab));
            for _ in 0..5 {
                screen(&mut app, width, height);
                press(&mut app, KeyCode::PageDown);
                press(&mut app, KeyCode::Right);
            }
        }
    }
    press(&mut app, KeyCode::Char('6'));
    let mut seen = String::new();
    // Return to Scores regardless of where the previous loop ended.
    for _ in 0..5 {
        for _ in 0..15 {
            seen.push_str(&screen(&mut app, 80, 24));
            press(&mut app, KeyCode::Down);
        }
        press(&mut app, KeyCode::Right);
    }
    for origin in josh_core::taxonomy().keys() {
        assert!(seen.contains(origin), "{origin}");
    }
}

#[test]
fn local_reviews_export_reload_and_protect_unsaved_work() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = App::new(None).unwrap();
    press(&mut app, KeyCode::Char('d'));
    add_review(&mut app);
    assert!(screen(&mut app, 120, 40).contains("UNSAVED REVIEWS"));
    for key in ['r', 'o', 'b', '[', ']'] {
        press(&mut app, KeyCode::Char(key));
        assert!(screen(&mut app, 120, 40).contains("Unsaved reviews"));
    }
    assert!(!press(&mut app, KeyCode::Char('q')));
    assert!(screen(&mut app, 40, 8).contains("Unsaved reviews"));
    press(&mut app, KeyCode::Char('n'));
    press(&mut app, KeyCode::Char('3'));
    assert!(screen(&mut app, 120, 40).contains("No result yet"));
    press(&mut app, KeyCode::Char('8'));
    press(&mut app, KeyCode::Char('s'));
    let file = dir.path().join("reviewed-case.json");
    type_text(&mut app, file.to_str().unwrap());
    press(&mut app, KeyCode::Enter);
    assert!(!screen(&mut app, 120, 40).contains("UNSAVED REVIEWS"));
    let case = josh_app::workflows::load_case(&file).unwrap();
    assert_eq!(case.clinical.as_ref().unwrap().reviews.len(), 1);
    assert!(josh_core::guidance::evaluate(&case).unwrap().items[0].review_is_current);
    let mut reloaded = App::new(Some(file)).unwrap();
    press(&mut reloaded, KeyCode::Char('8'));
    let mut history = screen(&mut reloaded, 120, 40);
    for _ in 0..10 {
        press(&mut reloaded, KeyCode::PageDown);
        history.push_str(&screen(&mut reloaded, 120, 40));
    }
    assert!(history.contains("reviewer-1"));
    assert!(press(&mut app, KeyCode::Char('q')));
}

#[test]
fn reports_and_visual_exports_do_not_clear_unsaved_review_warning() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = App::new(None).unwrap();
    add_review(&mut app);
    for tab in ['7', '6'] {
        press(&mut app, KeyCode::Char(tab));
        press(&mut app, KeyCode::Char('s'));
        let file = dir.path().join(format!("export-{tab}.json"));
        type_text(&mut app, file.to_str().unwrap());
        press(&mut app, KeyCode::Enter);
        let value: serde_json::Value =
            serde_json::from_slice(&std::fs::read(file).unwrap()).unwrap();
        if tab == '7' {
            assert_eq!(value["research_only"], true);
            assert_eq!(
                value["clinical_context"]["reviews"][0]["assessment"]["reviewer_id"],
                "reviewer-1"
            );
        } else {
            assert!(value["case"]["clinical"].is_object());
            assert!(value["guidance"]["items"].is_array());
            assert!(value["result"].is_null());
        }
        assert!(screen(&mut app, 120, 40).contains("UNSAVED REVIEWS"));
    }
    assert!(!app.key(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL)));
    assert!(press(&mut app, KeyCode::Char('y')));
}

#[test]
fn guidance_details_can_scroll_to_source_links_without_changing_selected_rule() {
    let mut app = App::new(None).unwrap();
    press(&mut app, KeyCode::Char('7'));
    let first = screen(&mut app, 80, 24);
    assert!(first.contains("scope"));
    let mut text = first;
    for _ in 0..10 {
        press(&mut app, KeyCode::PageDown);
        text.push_str(&screen(&mut app, 80, 24));
    }
    assert!(text.contains("https://www.nice.org.uk/guidance/cg104"));
    add_review(&mut app);
    assert!(screen(&mut app, 120, 40).contains("UNSAVED REVIEWS"));
}

#[test]
fn legacy_cases_show_unknown_clinical_context_not_a_completed_pathway() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("legacy.json");
    std::fs::write(&file, include_str!("../../../fixtures/synthetic-case.json")).unwrap();
    let mut app = App::new(Some(file)).unwrap();
    press(&mut app, KeyCode::Char('6'));
    press(&mut app, KeyCode::Right);
    press(&mut app, KeyCode::Right);
    let pathway = screen(&mut app, 120, 40);
    assert!(pathway.contains("Unassessed"));
    assert!(!pathway.contains("RECORDED HERE"));
    press(&mut app, KeyCode::Right);
    assert!(screen(&mut app, 120, 40).contains("No structured investigation"));
    press(&mut app, KeyCode::Char('7'));
    press(&mut app, KeyCode::Char('a'));
    assert!(screen(&mut app, 120, 40).contains("requires a schema 3"));
}

#[test]
fn missing_timeline_dates_are_not_plotted_at_zero() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("undated.json");
    let mut case = josh_app::workflows::clinical_example_case();
    for investigation in &mut case.clinical.as_mut().unwrap().investigations {
        investigation.day = None;
    }
    std::fs::write(&file, serde_json::to_vec(&case).unwrap()).unwrap();
    let mut app = App::new(Some(file)).unwrap();
    press(&mut app, KeyCode::Char('6'));
    for _ in 0..3 {
        press(&mut app, KeyCode::Right);
    }
    let timeline = screen(&mut app, 120, 40);
    assert!(timeline.contains("Dates unknown"));
    assert!(timeline.contains("unknown"));
    assert!(!timeline.contains("+0"));
}

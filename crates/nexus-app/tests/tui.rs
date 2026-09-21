use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use nexus_app::tui::App;
use ratatui::{Terminal, backend::TestBackend};

fn press(app: &mut App, code: KeyCode) -> bool {
    app.key(KeyEvent::new(code, KeyModifiers::NONE))
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
            serde_json::to_string(&nexus_app::workflows::example_case()).unwrap()
        ),
        "x".repeat(nexus_core::MAX_CASE_BYTES + 1),
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

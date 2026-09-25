use super::*;
use ratatui::{Terminal, backend::TestBackend};
fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(name)
}
fn press(app: &mut App, code: KeyCode) {
    app.key(KeyEvent::new(code, KeyModifiers::NONE));
}
fn screen(app: &mut App, w: u16, h: u16) -> String {
    let mut t = Terminal::new(TestBackend::new(w, h)).unwrap();
    t.draw(|f| app.draw(f)).unwrap();
    t.backend()
        .buffer()
        .content
        .chunks(w as usize)
        .map(|r| r.iter().map(|c| c.symbol()).collect::<String>())
        .collect::<Vec<_>>()
        .join("\n")
}
fn click(app: &mut App, id: Control) {
    screen(app, 80, 24);
    let rect = app
        .navigation
        .iter()
        .find(|(_, c)| *c == id)
        .expect("visible control")
        .0;
    app.mouse(MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column: rect.x + rect.width / 2,
        row: rect.y,
        modifiers: KeyModifiers::NONE,
    });
}
fn sample(app: &mut App, i: usize) {
    app.finish_browse(Outcome::Sample(i));
}
async fn settle(app: &mut App) {
    for _ in 0..1000 {
        app.poll().await;
        if !app.busy() {
            return;
        }
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
    panic!("job timeout");
}
#[test]
fn starts_empty_with_one_toolbar_and_no_unrelated_demo_patients() {
    let mut app = App::new(None).unwrap();
    let text = screen(&mut app, 80, 24);
    assert!(matches!(app.session.input, Input::Empty));
    assert!(!app.busy());
    assert!(app.history.is_empty());
    assert!(text.contains("Try a sample"));
    assert!(!text.contains("SYNTHETIC-001"));
    assert_eq!(app.actions().len(), 3);
    assert!(
        !app.actions()
            .iter()
            .any(|a| a.label == "Clinical" || a.label == "Expression" || a.label == "Record")
    );
}
#[test]
fn keyboard_first_sample_needs_no_paths_or_typed_identifiers() {
    let mut app = App::new(None).unwrap();
    screen(&mut app, 80, 24);
    press(&mut app, KeyCode::Tab);
    assert_eq!(app.focus, Some(Control::TrySample));
    press(&mut app, KeyCode::Enter);
    assert!(app.browser.is_some());
    press(&mut app, KeyCode::Enter);
    assert!(app.browser.is_none());
    assert_eq!(
        app.session.analysis().unwrap().features.sample_id,
        "SYNTH-001"
    );
    assert_eq!(app.session.page, Page::Data);
    assert!(!app.session.analysis().unwrap().has_result());
    assert!(!app.busy());
    assert_eq!(app.actions().len(), 5);
    assert_eq!(app.primary().id, Control::Run);
    click(&mut app, Control::Page(Page::Results));
    assert!(screen(&mut app, 80, 24).contains("No result yet"));
}
#[test]
fn data_and_results_are_distinct_and_new_samples_preserve_history() {
    let mut app = App::new(None).unwrap();
    sample(&mut app, 0);
    let first = app.session.context();
    sample(&mut app, 2);
    assert_ne!(first, app.session.context());
    assert_eq!(app.history.len(), 1);
    assert!(!app.session.analysis().unwrap().has_result());
    app.activate(Control::Restore(0));
    assert_eq!(app.session.context(), first);
    assert_eq!(app.history.len(), 1);
    assert!(screen(&mut app, 100, 30).contains("INPUT EVIDENCE"));
    press(&mut app, KeyCode::Char('v'));
    assert!(!screen(&mut app, 100, 30).contains("INPUT EVIDENCE"));
}
#[test]
fn cancelled_or_invalid_import_cannot_replace_the_active_sample() {
    let mut app = App::new(None).unwrap();
    sample(&mut app, 0);
    let original = app.session.context();
    app.open(&fixture("fixtures/basic-demo/molecular-complete.tsv"))
        .unwrap();
    assert!(app.draft.is_some());
    let e = app.editor().unwrap();
    assert_eq!(e.fields[0].value, "DEMO-MOL-01");
    assert!(e.notice.contains("1 samples"));
    assert_eq!(e.fields[5].value, "deidentified_research");
    assert_eq!(app.session.context(), original);
    app.key(KeyEvent::new(KeyCode::Char('u'), KeyModifiers::CONTROL));
    app.paste("bad-id");
    app.key(KeyEvent::new(KeyCode::Enter, KeyModifiers::CONTROL));
    assert!(app.draft.is_some());
    assert_eq!(app.session.context(), original);
    assert!(!app.busy());
    press(&mut app, KeyCode::Esc);
    assert!(app.draft.is_none());
    assert_eq!(app.session.context(), original);
    assert!(app.open(Path::new("/missing/input.json")).is_err());
    assert_eq!(app.session.context(), original);
}
#[test]
fn multi_sample_table_picker_keeps_original_hash_and_row_numbers() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("multi.tsv");
    let bytes=b"sample_id\tid\tname\tmodality\tvalue\tunits\tstatus\nA\tage\tAge\tdemographic\t50\tyears\tobserved\nB\tage\tAge\tdemographic\t60\tyears\tobserved\n";
    std::fs::write(&path, bytes).unwrap();
    let mut app = App::new(None).unwrap();
    app.open(&path).unwrap();
    press(&mut app, KeyCode::F(6));
    assert!(matches!(app.menu, Some((Menu::ImportSamples, _))));
    press(&mut app, KeyCode::Down);
    press(&mut app, KeyCode::Enter);
    assert_eq!(app.editor().unwrap().fields[0].value, "B");
    assert_eq!(app.editor().unwrap().fields[1].value, "B");
    app.key(KeyEvent::new(KeyCode::Enter, KeyModifiers::CONTROL));
    assert!(app.draft.is_none());
    let f = &app.session.analysis().unwrap().features;
    assert_eq!(f.sample_id, "B");
    assert_eq!(f.features.len(), 1);
    assert_eq!(f.features[0].source.record, 3);
    assert_eq!(
        f.features[0].source.sha256,
        josh_core::molecular::bytes_hash(bytes)
    );
    assert_eq!(f.data_class, josh_core::DataClass::DeidentifiedResearch);
    assert!(!app.busy());
}
#[test]
fn guided_import_submit_is_clickable_and_does_not_send() {
    let mut app = App::new(None).unwrap();
    app.open(&fixture("fixtures/basic-demo/molecular-complete.tsv"))
        .unwrap();
    screen(&mut app, 80, 24);
    let rect = app
        .form
        .hits
        .iter()
        .find(|(_, f)| *f == FieldFocus::Submit)
        .unwrap()
        .0;
    app.mouse(MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column: rect.x + rect.width / 2,
        row: rect.y + 1,
        modifiers: KeyModifiers::NONE,
    });
    assert!(app.draft.is_none());
    assert!(app.session.ready());
    assert!(!app.busy());
    assert!(!app.session.analysis().unwrap().has_result());
}
#[test]
fn all_controls_are_reachable_in_both_tab_directions_and_mouse_works() {
    let mut app = App::new(None).unwrap();
    sample(&mut app, 0);
    for page in [Page::Data, Page::Results] {
        app.session.set_page(page);
        screen(&mut app, 80, 24);
        let controls: Vec<_> = app.navigation.iter().map(|(_, c)| *c).collect();
        app.focus = None;
        for id in &controls {
            press(&mut app, KeyCode::Tab);
            assert_eq!(app.focus, Some(*id));
        }
        press(&mut app, KeyCode::Tab);
        assert_eq!(app.focus, controls.first().copied());
        for id in controls.iter().rev() {
            press(&mut app, KeyCode::BackTab);
            assert_eq!(app.focus, Some(*id));
        }
    }
    click(&mut app, Control::Page(Page::Data));
    assert_eq!(app.session.page, Page::Data);
    click(&mut app, Control::Menu(Menu::Workspace));
    press(&mut app, KeyCode::Down);
    press(&mut app, KeyCode::Enter);
    assert!(app.studies);
    press(&mut app, KeyCode::F(2));
    assert!(!app.studies);
    assert_eq!(
        app.session.analysis().unwrap().features.sample_id,
        "SYNTH-001"
    );
}
#[test]
fn legacy_records_are_explicit_and_cannot_start_another_classifier_or_review() {
    let mut app = App::new(Some(fixture("fixtures/clinical-case.json"))).unwrap();
    let text = screen(&mut app, 100, 30);
    assert!(text.contains("LEGACY"));
    assert!(app.session.analysis().is_none());
    app.activate(Control::Detail(6));
    app.inner_key(key('a'));
    assert!(app.editor().is_none());
    assert!(!app.busy());
    app.activate(Control::Run);
    assert!(!app.busy());
    assert!(app.editor().is_none());
    app.activate(Control::Export);
    assert_eq!(app.editor().unwrap().title, "Export clinical data");
}
#[test]
fn help_and_request_preview_do_not_submit_or_change_inputs() {
    let mut app = App::new(None).unwrap();
    sample(&mut app, 0);
    let before = app.session.context();
    app.activate(Control::Request);
    assert!(
        app.inspection
            .as_ref()
            .unwrap()
            .0
            .contains(josh_core::MODEL)
    );
    app.paste("should not alter input");
    press(&mut app, KeyCode::Esc);
    app.activate(Control::Help);
    assert!(app.guide.open);
    press(&mut app, KeyCode::Esc);
    assert_eq!(app.session.context(), before);
    assert!(!app.busy());
}
#[tokio::test]
async fn demo_is_opt_in_and_confirmation_defaults_to_cancel() {
    let mut app = App::new(None).unwrap();
    sample(&mut app, 0);
    let before = app.session.context();
    press(&mut app, KeyCode::Char('d'));
    screen(&mut app, 80, 24);
    assert_eq!(app.form.focus, Some(FieldFocus::Cancel));
    press(&mut app, KeyCode::Enter);
    assert!(app.confirm.is_none());
    assert_eq!(app.session.context(), before);
    assert!(!app.busy());
    press(&mut app, KeyCode::Char('d'));
    press(&mut app, KeyCode::Char('y'));
    assert!(app.busy());
    press(&mut app, KeyCode::F(5));
    settle(&mut app).await;
    press(&mut app, KeyCode::F(2));
    assert_eq!(app.session.page, Page::Results);
    assert!(app.session.analysis().unwrap().has_explanation());
    assert_eq!(app.history.len(), 1);
    assert!(screen(&mut app, 120, 40).contains("RANKED OUTCOMES"));
}
#[tokio::test]
async fn every_chart_is_preserved_in_one_selector_and_renders_small() {
    let mut app = App::new(None).unwrap();
    press(&mut app, KeyCode::Char('d'));
    settle(&mut app).await;
    let views = app.menu_actions(Menu::Details);
    assert_eq!(views.len(), 5);
    for action in views {
        app.activate(action.id);
        for (w, h) in [(120, 40), (80, 24), (60, 20), (20, 8), (1, 1)] {
            let rendered = screen(&mut app, w, h);
            if w >= 80 {
                assert!(
                    rendered.lines().next().unwrap().contains(josh_core::MODEL),
                    "chart content overwrote header: {rendered}"
                );
            }
        }
    }
    app.activate(Control::Detail(4));
    let text = screen(&mut app, 80, 24);
    assert!(text.contains("Category / feature ring"));
    assert!(text.contains("Signed contribution"));
    assert!(text.contains("uncalibrated"));
}
#[tokio::test]
async fn expression_dataset_and_cancellation_preserve_prior_sample() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("dataset");
    let data = josh_ingest::expression::import(
        include_bytes!("../../../../fixtures/expression/synthetic-expression.tsv").to_vec(),
        &Default::default(),
        None,
    )
    .unwrap();
    josh_ingest::dataset::write(&path, &data).unwrap();
    let mut app = App::new(None).unwrap();
    sample(&mut app, 0);
    let before = app.session.context();
    app.open(&path).unwrap();
    app.activate(Control::CancelImport);
    settle(&mut app).await;
    assert_eq!(app.session.context(), before);
    assert!(app.draft.is_none());
    app.open(&path).unwrap();
    settle(&mut app).await;
    assert!(app.session.data().unwrap().has_input());
    assert!(app.session.analysis().is_none());
    assert_eq!(app.history.len(), 1);
    app.activate(Control::Restore(0));
    assert_eq!(app.session.context(), before);
}
#[test]
fn menus_on_tiny_screens_and_empty_lists_do_not_panic() {
    let mut app = App::new(None).unwrap();
    for menu in [
        Menu::Main,
        Menu::Workspace,
        Menu::History,
        Menu::Details,
        Menu::Samples,
        Menu::ImportSamples,
    ] {
        app.menu = Some((menu, 0));
        for (w, h) in [(80, 24), (20, 8), (1, 1)] {
            screen(&mut app, w, h);
        }
        press(&mut app, KeyCode::Down);
        press(&mut app, KeyCode::Esc);
    }
}
#[test]
fn expression_import_exposes_basic_fields_and_preserves_advanced_options() {
    let mut app = App::new(None).unwrap();
    app.open(&fixture("fixtures/expression/synthetic-expression.tsv"))
        .unwrap();
    let e = app.editor().unwrap();
    assert_eq!(e.fields.len(), 5);
    assert!(!e.fields[1].value.is_empty());
    app.submit_editor_key(KeyEvent::new(KeyCode::F(7), KeyModifiers::NONE));
    assert_eq!(app.editor().unwrap().fields.len(), 15);
    assert!(!app.current().ready());
    assert!(!app.busy());
    press(&mut app, KeyCode::Esc);
    assert!(matches!(app.session.input, Input::Empty));
}

#[tokio::test]
async fn failed_expression_worker_restores_settings_and_previous_sample() {
    let mut app = App::new(None).unwrap();
    sample(&mut app, 0);
    let previous = app.session.context();
    app.open(&fixture("fixtures/expression/synthetic-expression.tsv"))
        .unwrap();
    // A destination collision must preserve both the input and editable settings.
    screen(&mut app, 80, 24);
    app.form.focus = Some(FieldFocus::Field(1));
    app.key(KeyEvent::new(KeyCode::Char('u'), KeyModifiers::CONTROL));
    app.paste(std::env::temp_dir().to_str().unwrap());
    app.key(KeyEvent::new(KeyCode::Enter, KeyModifiers::CONTROL));
    settle(&mut app).await;
    assert!(app.draft.is_some());
    assert_eq!(app.session.context(), previous);
    assert!(app.editor().is_some());
    assert_eq!(
        app.editor().unwrap().fields[1].value,
        std::env::temp_dir().to_str().unwrap()
    );
    press(&mut app, KeyCode::Esc);
    assert_eq!(app.session.context(), previous);
}

#[test]
fn paste_expression_is_a_staged_input_and_cancel_is_lossless() {
    let mut app = App::new(None).unwrap();
    sample(&mut app, 0);
    let before = app.session.context();
    app.activate(Control::PasteExpression);
    app.paste("gene\texpression\nTP53\t3\n");
    press(&mut app, KeyCode::Enter);
    assert_eq!(app.editor().unwrap().title, "Import expression");
    assert_eq!(app.session.context(), before);
    press(&mut app, KeyCode::Esc);
    assert!(app.draft.is_none());
    assert_eq!(app.session.context(), before);
    assert!(!app.busy());
}

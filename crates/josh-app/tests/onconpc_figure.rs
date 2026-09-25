use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use josh_app::{molecular_tui, terminal};
use ratatui::{Terminal, backend::TestBackend};

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}
fn text(t: &Terminal<TestBackend>) -> String {
    t.backend()
        .buffer()
        .content
        .iter()
        .map(|c| c.symbol())
        .collect()
}

#[tokio::test]
async fn paired_figure_is_discoverable_in_the_shared_workspace_and_preserves_other_views() {
    let mut app = terminal::App::new(None).unwrap();
    let mut t = Terminal::new(TestBackend::new(120, 40)).unwrap();
    t.draw(|f| app.draw(f)).unwrap();
    assert!(text(&t).contains("Try a sample"));
    app.key(key(KeyCode::Char('p')));
    t.draw(|f| app.draw(f)).unwrap();
    assert!(text(&t).contains("No result for this input"));
    assert!(!app.busy());
    app.key(key(KeyCode::Char('d')));
    for _ in 0..500 {
        app.poll().await;
        if !app.busy() {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }
    assert!(!app.busy());
    app.key(key(KeyCode::Char('p')));
    for (w, h) in [
        (160, 50),
        (120, 40),
        (100, 30),
        (80, 24),
        (60, 55),
        (60, 20),
        (20, 8),
        (1, 1),
    ] {
        let mut t = Terminal::new(TestBackend::new(w, h)).unwrap();
        t.draw(|f| app.draw(f)).unwrap();
        if w >= 80 && h >= 24 || w == 60 && h == 55 {
            let screen = text(&t);
            assert!(
                screen.contains("Category / feature ring"),
                "{w}x{h}: {screen}"
            );
            assert!(screen.contains("Signed contribution"), "{w}x{h}: {screen}");
            assert!(screen.contains("ANALYTICAL DEMO"));
            assert!(screen.contains("uncalibrated"));
            // Canvas coordinate rounding must not cause one label to erase another.
            for label in [
                "1 SBS4", "2 SBS24", "3 MDM2", "4 FAT1", "5 KRAS", "6 CREBBP", "7 Age",
            ] {
                assert!(
                    screen.contains(label),
                    "missing label {label} at {w}x{h}: {screen}"
                );
            }
        }
    }
    // Arrow navigation retains every chart; Tab now traverses all workbench controls.
    let mut large = Terminal::new(TestBackend::new(160, 50)).unwrap();
    for (code, expected) in [
        (KeyCode::Char('v'), "RANKED OUTCOMES"),
        (KeyCode::Right, "Attribution magnitude"),
        (KeyCode::Right, "Signed contribution"),
        (KeyCode::Right, "Waterfall · baseline"),
        (KeyCode::Right, "Category / feature ring"),
        (KeyCode::Left, "Waterfall · baseline"),
    ] {
        app.key(key(code));
        large.draw(|f| app.draw(f)).unwrap();
        assert!(text(&large).contains(expected), "{expected}");
    }
    // Results has one chart selector instead of five permanent chart tabs.
    app.mouse(MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column: 8,
        row: 4,
        modifiers: KeyModifiers::NONE,
    });
    app.key(key(KeyCode::Down)); // paired ring + scatter
    app.key(key(KeyCode::Enter));
    large.draw(|f| app.draw(f)).unwrap();
    assert!(text(&large).contains("Category / feature ring"));
    app.key(key(KeyCode::F(5)));
    app.key(key(KeyCode::F(2)));
    large.draw(|f| app.draw(f)).unwrap();
    assert!(text(&large).contains("Category / feature ring"));
    assert!(!app.busy());
}

#[tokio::test]
async fn figure_selection_retargeting_and_export_use_the_same_archived_values() {
    let mut app = molecular_tui::App::new(None, None).unwrap();
    app.key(key(KeyCode::Char('d')));
    for _ in 0..500 {
        app.poll().await;
        if !app.busy() {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }
    assert!(!app.busy());
    let before = app.archive.as_ref().unwrap().clone();
    app.key(key(KeyCode::Char('p')));
    app.key(key(KeyCode::Down));
    let mut t = Terminal::new(TestBackend::new(160, 50)).unwrap();
    t.draw(|f| app.draw(f)).unwrap();
    let second = josh_app::molecular_charts::ranked(&before)[1];
    assert!(text(&t).contains(&format!("#2 {}", second.label)));
    assert_eq!(
        josh_core::molecular::hash(app.archive.as_ref().unwrap()).unwrap(),
        josh_core::molecular::hash(&before).unwrap()
    );
    assert_eq!(
        josh_app::molecular_charts::svg(app.archive.as_ref().unwrap()).unwrap(),
        josh_app::molecular_charts::svg(&before).unwrap()
    );
    app.key(key(KeyCode::Char('t')));
    t.draw(|f| app.draw(f)).unwrap();
    let after = app.archive.as_ref().unwrap();
    assert_ne!(after.config.target_class, before.config.target_class);
    assert!(text(&t).contains(&format!("({}) · raw score", after.config.target_class)));
    assert_eq!(after.evaluations.len(), before.evaluations.len());
    assert!(!app.busy());
    // Inspecting a group below the top ten must show its actual rank on the plot.
    let mut many = before.clone();
    let r = many.result.as_mut().unwrap();
    for i in 0..8 {
        let mut extra = r.attributions[0].clone();
        extra.group = format!("extra-{i}");
        extra.label = format!("Extra {i}");
        extra.contribution = 0.00001 * (i + 1) as f64;
        r.attributions.push(extra);
    }
    t.draw(|f| josh_app::molecular_charts::draw_onconpc(f, f.area(), &many, 14))
        .unwrap();
    let screen = text(&t);
    assert!(screen.contains("#15 Extra 0"));
    assert!(screen.contains("15 Extra 0"));
    assert!(screen.contains("omitted: 5"));
    // Empty and zero-contribution archives retain an honest empty state on every layout.
    let mut zero = before;
    zero.result
        .as_mut()
        .unwrap()
        .attributions
        .iter_mut()
        .for_each(|r| r.contribution = 0.0);
    t.draw(|f| josh_app::molecular_charts::draw_onconpc(f, f.area(), &zero, usize::MAX))
        .unwrap();
    assert!(text(&t).contains("No nonzero attribution magnitude"));
    zero.result.as_mut().unwrap().attributions.clear();
    for (w, h) in [(80, 24), (20, 8), (1, 1)] {
        let mut t = Terminal::new(TestBackend::new(w, h)).unwrap();
        t.draw(|f| josh_app::molecular_charts::draw_onconpc(f, f.area(), &zero, usize::MAX))
            .unwrap();
    }
}

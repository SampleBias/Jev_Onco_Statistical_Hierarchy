//! Capture the actual paired-figure terminal cells, for offline visual review.
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{Terminal, backend::TestBackend, style::Color};
use std::{fmt::Write, fs::OpenOptions, io::Write as _};

fn color(c: Color) -> String {
    match c {
        Color::Rgb(r, g, b) => format!("#{r:02x}{g:02x}{b:02x}"),
        Color::Black => "#000000".into(),
        Color::White => "#ffffff".into(),
        _ => "#859bb0".into(),
    }
}
fn xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}
fn capture(
    app: &mut josh_app::terminal::App,
    root: &std::path::Path,
    name: &str,
    w: u16,
    h: u16,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut t = Terminal::new(TestBackend::new(w, h))?;
    t.draw(|f| app.draw(f))?;
    let b = t.backend().buffer();
    let mut svg = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{}\" height=\"{}\"><rect width=\"100%\" height=\"100%\" fill=\"#0a111c\"/><g font-family=\"DejaVu Sans Mono\" font-size=\"14\">",
        w * 9,
        h * 18
    );
    for y in 0..h {
        for x in 0..w {
            let cell = &b[(x, y)];
            write!(
                svg,
                "<rect x=\"{}\" y=\"{}\" width=\"9\" height=\"18\" fill=\"{}\"/>",
                x * 9,
                y * 18,
                color(cell.bg)
            )?;
        }
    }
    for y in 0..h {
        for x in 0..w {
            let cell = &b[(x, y)];
            if cell.symbol() != " " {
                write!(
                    svg,
                    "<text x=\"{}\" y=\"{}\" fill=\"{}\">{}</text>",
                    x * 9,
                    y * 18 + 14,
                    color(cell.fg),
                    xml(cell.symbol())
                )?;
            }
        }
    }
    svg.push_str("</g></svg>");
    OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(root.join(format!("{name}-{w}x{h}.svg")))?
        .write_all(svg.as_bytes())?;

    Ok(())
}
fn press(app: &mut josh_app::terminal::App, code: KeyCode) {
    app.key(KeyEvent::new(code, KeyModifiers::NONE));
}
async fn settle(app: &mut josh_app::terminal::App) {
    while app.busy() {
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        app.poll().await;
    }
}
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let root = std::path::PathBuf::from(
        std::env::args()
            .nth(1)
            .ok_or("existing output directory required")?,
    );
    let mut app = josh_app::terminal::App::new(None)?;
    press(&mut app, KeyCode::Char('d'));
    settle(&mut app).await;
    press(&mut app, KeyCode::Char('p'));
    for (w, h) in [(160, 50), (120, 40), (80, 24), (60, 55)] {
        capture(&mut app, &root, "onconpc", w, h)?;
    }
    for (w, h) in [(120, 40), (80, 24), (60, 20)] {
        let mut app = josh_app::terminal::App::new(None)?;
        capture(&mut app, &root, "welcome", w, h)?;
        press(&mut app, KeyCode::Char('l'));
        capture(&mut app, &root, "browser", w, h)?;
        press(&mut app, KeyCode::Esc);
        app.key(KeyEvent::new(KeyCode::Char('b'), KeyModifiers::CONTROL));
        capture(&mut app, &root, "samples", w, h)?;
        press(&mut app, KeyCode::Enter);
        capture(&mut app, &root, "input", w, h)?;
        // Opening/canceling the confirmation does not send a provider request.
        press(&mut app, KeyCode::Char('a'));
        capture(&mut app, &root, "analyze-confirmation", w, h)?;
        press(&mut app, KeyCode::Esc);
        let repo = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        app.open(&repo.join("fixtures/expression/synthetic-expression.tsv"))?;
        capture(&mut app, &root, "expression-import", w, h)?;
        press(&mut app, KeyCode::Esc);
        app.open(&repo.join("fixtures/clinical-case.json"))?;
        capture(&mut app, &root, "legacy-record", w, h)?;
        press(&mut app, KeyCode::F(5));
        press(&mut app, KeyCode::Char('d'));
        settle(&mut app).await;
        capture(&mut app, &root, "cohort", w, h)?;
    }
    Ok(())
}

//! Render actual Ratatui buffers for visual inspection, without a running terminal.
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use josh_app::{study, terminal};
use ratatui::{Terminal, backend::TestBackend, style::Color};
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
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let root =
        std::path::PathBuf::from(std::env::args().nth(1).ok_or("output directory required")?);
    let data = study::demo();
    let report = josh_features::study::analyze(&data, 200, 42)?;
    std::fs::write(
        root.join("study.json"),
        serde_json::to_string_pretty(&data)?,
    )?;
    let mut app = terminal::App::new(Some(root.join("study.json")))?;
    while app.busy() {
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        app.poll().await;
    }
    let (w, h) = (190, 64);
    let mut terminal = Terminal::new(TestBackend::new(w, h))?;
    for page in 0..6 {
        app.key(KeyEvent::new(
            KeyCode::Char(char::from(b'1' + page)),
            KeyModifiers::NONE,
        ));
        terminal.draw(|f| app.draw(f))?;
        let buffer = terminal.backend().buffer();
        let mut svg = format!(
            "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{}\" height=\"{}\"><rect width=\"100%\" height=\"100%\" fill=\"#0a111c\"/><g font-family=\"DejaVu Sans Mono\" font-size=\"14\">",
            w * 9,
            h * 18
        );
        for y in 0..h {
            for x in 0..w {
                let cell = &buffer[(x, y)];
                svg.push_str(&format!(
                    "<rect x=\"{}\" y=\"{}\" width=\"9\" height=\"18\" fill=\"{}\"/>",
                    x * 9,
                    y * 18,
                    color(cell.bg)
                ));
            }
        }
        for y in 0..h {
            for x in 0..w {
                let cell = &buffer[(x, y)];
                if cell.symbol() != " " {
                    svg.push_str(&format!(
                        "<text x=\"{}\" y=\"{}\" fill=\"{}\">{}</text>",
                        x * 9,
                        y * 18 + 14,
                        color(cell.fg),
                        xml(cell.symbol())
                    ));
                }
            }
        }
        svg.push_str("</g></svg>");
        std::fs::write(root.join(format!("tui-{page}.svg")), svg)?;
    }
    std::fs::write(
        root.join("study.svg"),
        josh_app::study_charts::svg(&data, &report),
    )?;
    Ok(())
}

//! Shared visual language; inspired by Ratatui's Demo2 and Table examples.
use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    widgets::{Block, BorderType},
};
pub const BG: Color = Color::Rgb(10, 17, 28);
pub const PANEL: Color = Color::Rgb(17, 28, 42);
pub const RAISED: Color = Color::Rgb(28, 43, 61);
pub const TEXT: Color = Color::Rgb(216, 229, 241);
pub const MUTED: Color = Color::Rgb(133, 155, 176);
pub const ACCENT: Color = Color::Rgb(85, 224, 204);
pub const BLUE: Color = Color::Rgb(111, 164, 255);
pub const GOLD: Color = Color::Rgb(245, 198, 104);
pub const RED: Color = Color::Rgb(247, 130, 144);
pub fn panel(title: impl Into<ratatui::text::Line<'static>>) -> Block<'static> {
    Block::bordered()
        .border_type(BorderType::Rounded)
        .title(title)
        .style(Style::default().bg(PANEL).fg(TEXT))
        .border_style(Style::default().fg(RAISED))
        .title_style(Style::default().fg(ACCENT).add_modifier(Modifier::BOLD))
}
pub fn shadow(frame: &mut Frame, area: Rect) {
    let screen = frame.area();
    let shadow = Rect::new(
        area.x.saturating_add(1),
        area.y.saturating_add(1),
        area.width,
        area.height,
    )
    .intersection(screen);
    frame.render_widget(
        Block::default().style(Style::default().bg(Color::Black)),
        shadow,
    );
}

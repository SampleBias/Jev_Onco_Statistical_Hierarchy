use ratatui::{
    style::{Color, Style},
    widgets::Block,
};

pub const BACKGROUND: Color = Color::Rgb(12, 22, 38);
pub const PANEL: Color = Color::Rgb(18, 34, 53);
pub const TEXT: Color = Color::Rgb(221, 235, 245);
pub const MUTED: Color = Color::Rgb(145, 168, 188);
pub const TEAL: Color = Color::Rgb(83, 210, 185);
pub const CYAN: Color = Color::Rgb(99, 207, 239);
pub const BLUE: Color = Color::Rgb(118, 163, 245);
pub const VIOLET: Color = Color::Rgb(177, 157, 238);
pub const AMBER: Color = Color::Rgb(235, 195, 119);
pub const ROSE: Color = Color::Rgb(239, 149, 166);
pub const SERIES: [Color; 4] = [TEAL, CYAN, BLUE, VIOLET];

pub fn panel(title: impl Into<ratatui::text::Line<'static>>, accent: Color) -> Block<'static> {
    Block::bordered()
        .border_type(ratatui::widgets::BorderType::Rounded)
        .title(title)
        .border_style(Style::default().fg(accent))
        .style(Style::default().fg(TEXT).bg(PANEL))
}

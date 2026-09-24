use ratatui::{
    style::{Color, Style},
    widgets::Block,
};

pub const BACKGROUND: Color = crate::ui::BG;
pub const PANEL: Color = crate::ui::PANEL;
pub const TEXT: Color = crate::ui::TEXT;
pub const MUTED: Color = crate::ui::MUTED;
pub const TEAL: Color = crate::ui::ACCENT;
pub const CYAN: Color = Color::Rgb(99, 207, 239);
pub const BLUE: Color = crate::ui::BLUE;
pub const VIOLET: Color = Color::Rgb(177, 157, 238);
pub const AMBER: Color = crate::ui::GOLD;
pub const ROSE: Color = crate::ui::RED;
pub const SERIES: [Color; 4] = [TEAL, CYAN, BLUE, VIOLET];

pub fn panel(title: impl Into<ratatui::text::Line<'static>>, accent: Color) -> Block<'static> {
    Block::bordered()
        .border_type(ratatui::widgets::BorderType::Rounded)
        .title(title)
        .border_style(Style::default().fg(accent))
        .style(Style::default().fg(TEXT).bg(PANEL))
}

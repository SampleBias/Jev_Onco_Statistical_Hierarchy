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
/// Identical focus/selection language for every interactive control.
pub fn button(
    frame: &mut Frame,
    area: Rect,
    label: &str,
    focused: bool,
    selected: bool,
    enabled: bool,
) {
    use ratatui::widgets::Paragraph;
    let style = if !enabled {
        Style::default().fg(MUTED).bg(PANEL)
    } else if focused {
        Style::default().fg(BG).bg(ACCENT).bold()
    } else if selected {
        Style::default().fg(ACCENT).bg(RAISED).bold()
    } else {
        Style::default().fg(TEXT).bg(PANEL)
    };
    let available = area
        .width
        .saturating_sub(if area.height >= 3 { 2 } else { 0 }) as usize;
    let text = if focused && ratatui::text::Line::from(label).width() + 2 <= available {
        format!("> {label}")
    } else {
        label.into()
    };
    let widget = Paragraph::new(text).centered().style(style);
    if area.height >= 3 {
        frame.render_widget(
            widget.block(
                Block::bordered()
                    .border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(if focused { ACCENT } else { RAISED })),
            ),
            area,
        );
    } else {
        frame.render_widget(widget, area);
    }
}
pub fn popup(area: Rect, max_width: u16, max_height: u16) -> Rect {
    let width = area.width.saturating_sub(2).min(max_width);
    let height = area.height.saturating_sub(2).min(max_height);
    Rect::new(
        area.x + (area.width - width) / 2,
        area.y + (area.height - height) / 2,
        width,
        height,
    )
}
pub fn tail(text: &str, width: usize) -> String {
    let mut result = String::new();
    for ch in text.chars().rev() {
        let candidate = format!("{ch}{result}");
        if ratatui::text::Line::from(candidate.as_str()).width() > width {
            break;
        }
        result = candidate;
    }
    result
}
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

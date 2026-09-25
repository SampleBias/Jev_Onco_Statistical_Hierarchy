//! A common, keyboard- and mouse-accessible presentation for existing section forms.
use crate::{ui, workflows};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::Style,
    widgets::{Clear, Paragraph, Wrap},
};

pub struct Field {
    pub label: String,
    pub value: String,
}
pub struct Editor {
    pub title: String,
    pub notice: String,
    pub fields: Vec<Field>,
    pub submit_label: String,
    pub submit: KeyEvent,
    pub auxiliary: Option<(&'static str, KeyEvent)>,
    pub status: String,
}
impl Editor {
    pub fn new(title: &str, notice: &str, label: &str, status: &str) -> Self {
        Self {
            title: title.into(),
            notice: notice.into(),
            fields: vec![],
            submit_label: label.into(),
            submit: KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE),
            auxiliary: None,
            status: status.into(),
        }
    }
    pub fn field(mut self, label: impl Into<String>, value: impl Into<String>) -> Self {
        self.fields.push(Field {
            label: label.into(),
            value: value.into(),
        });
        self
    }
    pub fn controls(&self) -> Vec<Focus> {
        let mut items: Vec<_> = (0..self.fields.len()).map(Focus::Field).collect();
        if self.auxiliary.is_some() {
            items.push(Focus::Auxiliary);
        }
        items.extend([Focus::Submit, Focus::Cancel]);
        items
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Focus {
    Field(usize),
    Auxiliary,
    Submit,
    Cancel,
}

#[derive(Default)]
pub struct Form {
    title: String,
    pub focus: Option<Focus>,
    pub hits: Vec<(Rect, Focus)>,
}
impl Form {
    pub fn sync(&mut self, editor: &Editor) {
        if self.title != editor.title || self.focus.is_none() {
            self.title = editor.title.clone();
            // Confirmation-only dialogs default to Cancel, never a billed action.
            self.focus = Some(if editor.fields.is_empty() {
                Focus::Cancel
            } else {
                Focus::Field(0)
            });
        }
    }
    pub fn clear(&mut self) {
        *self = Self::default();
    }
    pub fn cycle(&mut self, editor: &Editor, backwards: bool) {
        self.sync(editor);
        let controls = editor.controls();
        let i = controls
            .iter()
            .position(|x| Some(*x) == self.focus)
            .unwrap_or(0);
        self.focus =
            Some(controls[(i + if backwards { controls.len() - 1 } else { 1 }) % controls.len()]);
    }
    pub fn activation(&self, editor: &Editor) -> Option<KeyEvent> {
        match self.focus? {
            Focus::Submit => Some(editor.submit),
            Focus::Cancel => Some(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE)),
            Focus::Auxiliary => editor.auxiliary.map(|(_, k)| k),
            Focus::Field(_) => None,
        }
    }
    pub fn draw(&mut self, frame: &mut Frame, editor: &Editor) {
        self.sync(editor);
        self.hits.clear();
        let area = ui::popup(frame.area(), 100, 24);
        ui::shadow(frame, area);
        frame.render_widget(Clear, area);
        let panel = ui::panel(format!(" {} ", editor.title));
        let inner = panel.inner(area);
        frame.render_widget(panel, area);
        let [notice, fields, status, actions, hint] = Layout::vertical([
            Constraint::Length(if inner.height >= 16 { 4 } else { 3 }),
            Constraint::Min(0),
            Constraint::Length(2),
            Constraint::Length(3),
            Constraint::Length(1),
        ])
        .areas(inner);
        frame.render_widget(
            Paragraph::new(editor.notice.clone()).wrap(Wrap { trim: false }),
            notice,
        );
        let visible = (fields.height / 3).max(1) as usize;
        let active = match self.focus {
            Some(Focus::Field(i)) => i,
            _ => editor.fields.len().saturating_sub(1),
        };
        let start = active.saturating_sub(visible.saturating_sub(1));
        for (i, f) in editor.fields.iter().enumerate().skip(start).take(visible) {
            let rect = Rect::new(
                fields.x,
                fields.y + ((i - start) * 3) as u16,
                fields.width,
                3,
            )
            .intersection(fields);
            let focused = self.focus == Some(Focus::Field(i));
            let value = workflows::display_text(&f.value);
            // Tail scrolling keeps the insertion point visible for long paths and Unicode.
            let tail = ui::tail(&value, rect.width.saturating_sub(3) as usize);
            frame.render_widget(
                Paragraph::new(tail.clone()).block(
                    ui::panel(format!(
                        " {}{} ",
                        f.label,
                        if focused { " · editing" } else { "" }
                    ))
                    .border_style(Style::default().fg(if focused {
                        ui::ACCENT
                    } else {
                        ui::RAISED
                    })),
                ),
                rect,
            );
            if focused && rect.width > 2 && rect.height > 2 {
                frame.set_cursor_position((
                    rect.x
                        + 1
                        + ratatui::text::Line::from(tail)
                            .width()
                            .min(rect.width.saturating_sub(3) as usize)
                            as u16,
                    rect.y + 1,
                ));
            }
            self.hits.push((rect, Focus::Field(i)));
        }
        frame.render_widget(
            Paragraph::new(workflows::display_text(&editor.status))
                .style(Style::default().fg(ui::GOLD))
                .wrap(Wrap { trim: false }),
            status,
        );
        let mut buttons = vec![];
        if let Some((label, _)) = editor.auxiliary {
            buttons.push((label, Focus::Auxiliary));
        }
        buttons.push((editor.submit_label.as_str(), Focus::Submit));
        buttons.push(("Cancel", Focus::Cancel));
        let rects = Layout::horizontal(vec![
            Constraint::Ratio(1, buttons.len() as u32);
            buttons.len()
        ])
        .split(actions);
        for ((label, id), rect) in buttons.into_iter().zip(rects.iter()) {
            ui::button(frame, *rect, label, self.focus == Some(id), false, true);
            self.hits.push((*rect, id));
        }
        frame.render_widget(
            Paragraph::new(
                "Tab / Shift-Tab move · Enter activate · Ctrl-U clear field · Esc cancel",
            )
            .style(Style::default().fg(ui::MUTED)),
            hint,
        );
    }
}

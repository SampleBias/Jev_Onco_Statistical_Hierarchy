//! Offline, searchable guide for the unified terminal workspace.
use crate::ui;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Margin, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Clear, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState, Wrap},
};
use regex::{Regex, RegexBuilder};

const MANUAL: &str = concat!(
    include_str!("../../../docs/USER_GUIDE.md"),
    "\n\n",
    include_str!("../../../docs/data/MOLECULAR_GUIDE.md"),
    "\n\n",
    include_str!("../../../docs/data/EXPRESSION_GUIDE.md"),
    "\n\n",
    include_str!("../../../docs/data/REFERENCE_GUIDE.md"),
    "\n\n",
    include_str!("../../../docs/data/COHORT_GUIDE.md"),
    "\n\n",
    include_str!("../../../docs/references/ONCONPC_GUIDE.md"),
    "\n\n",
    include_str!("../../../docs/assessment/ONCONPC_PARITY.md"),
    "\n\n",
    include_str!("../../../docs/SOURCES.md"),
    "\n\n",
    include_str!("../../../docs/TERMINAL_GUIDE.md")
);

#[derive(Default)]
pub struct Guide {
    pub open: bool,
    query: String,
    draft: Option<String>,
    regex: Option<Regex>,
    matches: Vec<usize>,
    selected: usize,
    scroll: usize,
    page: usize,
    total: usize,
    filtered: bool,
    error: Option<String>,
    jump: Option<usize>,
    controls: Vec<(Rect, KeyCode)>,
    focus: Option<usize>,
}
impl Guide {
    /// Text editors keep printable g; F1/Ctrl+g are unconditional shortcuts.
    pub fn key(&mut self, key: KeyEvent, editing: bool) -> bool {
        let control = key.modifiers.contains(KeyModifiers::CONTROL);
        let global = key.code == KeyCode::F(1) || (control && key.code == KeyCode::Char('g'));
        if !self.open {
            if global || (!editing && key.code == KeyCode::Char('g')) {
                self.open = true;
                return true;
            }
            return false;
        }
        if global || (control && key.code == KeyCode::Char('c')) {
            self.open = false;
            return true;
        }
        if matches!(key.code, KeyCode::Tab | KeyCode::BackTab) {
            let backwards =
                key.code == KeyCode::BackTab || key.modifiers.contains(KeyModifiers::SHIFT);
            self.focus = Some(match self.focus {
                Some(i) => (i + if backwards { 4 } else { 1 }) % 5,
                None => {
                    if backwards {
                        4
                    } else {
                        0
                    }
                }
            });
            return true;
        }
        if key.code == KeyCode::Enter
            && let Some(i) = self.focus
        {
            self.activate(i);
            return true;
        }
        if let Some(draft) = &mut self.draft {
            match key.code {
                KeyCode::Esc => self.draft = None,
                KeyCode::Enter => {
                    let query = self.draft.take().unwrap();
                    self.search(query);
                }
                KeyCode::Backspace => {
                    draft.pop();
                }
                KeyCode::Char('u') if control => draft.clear(),
                KeyCode::Char(c)
                    if !control && !c.is_control() && draft.len() + c.len_utf8() <= 256 =>
                {
                    draft.push(c)
                }
                _ => {}
            }
            return true;
        }
        match key.code {
            KeyCode::Esc | KeyCode::Char('g' | 'q') => self.open = false,
            KeyCode::Char('/') => self.draft = Some(self.query.clone()),
            KeyCode::Char('t') => self.search("^## ".into()),
            KeyCode::Char('c') => self.search(String::new()),
            KeyCode::Char('n') | KeyCode::Down if self.filtered => self.next(false),
            KeyCode::Char('N') | KeyCode::Up if self.filtered => self.next(true),
            KeyCode::Char('n') => self.next(false),
            KeyCode::Char('N') => self.next(true),
            KeyCode::Enter if !self.matches.is_empty() => {
                self.filtered = !self.filtered;
                self.jump = Some(self.matches[self.selected]);
                self.scroll = 0;
            }
            KeyCode::Down | KeyCode::Char('j') => self.scroll = self.scroll.saturating_add(1),
            KeyCode::Up | KeyCode::Char('k') => self.scroll = self.scroll.saturating_sub(1),
            KeyCode::PageDown => self.scroll = self.scroll.saturating_add(self.page.max(1)),
            KeyCode::PageUp => self.scroll = self.scroll.saturating_sub(self.page.max(1)),
            KeyCode::Home => self.scroll = 0,
            KeyCode::End => self.scroll = self.total.saturating_sub(self.page),
            _ => {}
        }
        true
    }
    pub fn paste(&mut self, text: &str) {
        if let Some(draft) = &mut self.draft
            && draft.len() + text.len() <= 256
            && !text.chars().any(char::is_control)
        {
            draft.push_str(text);
        }
    }
    fn activate(&mut self, index: usize) {
        self.focus = None;
        match index {
            0 => {
                if let Some(draft) = self.draft.take() {
                    self.search(draft);
                } else {
                    self.draft = Some(self.query.clone());
                }
            }
            1 => {
                self.draft = None;
                self.search("^## ".into());
            }
            2 => self.next(false),
            3 => {
                self.draft = None;
                self.search(String::new());
            }
            _ => self.open = false,
        }
    }
    pub fn mouse(&mut self, event: MouseEvent) {
        match event.kind {
            MouseEventKind::Down(MouseButton::Left) => {
                if let Some(i) = self
                    .controls
                    .iter()
                    .position(|(r, _)| r.contains((event.column, event.row).into()))
                {
                    self.activate(i);
                }
            }
            MouseEventKind::ScrollDown => self.scroll = self.scroll.saturating_add(3),
            MouseEventKind::ScrollUp => self.scroll = self.scroll.saturating_sub(3),
            _ => (),
        }
    }
    fn search(&mut self, query: String) {
        let compiled = if query.is_empty() {
            Ok(None)
        } else {
            RegexBuilder::new(&query)
                .case_insensitive(true)
                .size_limit(256 * 1024)
                .dfa_size_limit(512 * 1024)
                .build()
                .map(Some)
        };
        match compiled {
            Err(e) => {
                self.error = Some(e.to_string());
                self.draft = Some(query);
            }
            Ok(regex) => {
                self.matches = MANUAL
                    .lines()
                    .enumerate()
                    .filter(|(_, l)| regex.as_ref().is_some_and(|r| r.is_match(l)))
                    .map(|(i, _)| i)
                    .collect();
                self.regex = regex;
                self.query = query;
                self.selected = 0;
                self.scroll = 0;
                self.filtered = !self.query.is_empty();
                self.error = None;
                self.jump = None;
            }
        }
    }
    fn next(&mut self, previous: bool) {
        if self.matches.is_empty() {
            return;
        }
        let n = self.matches.len();
        self.selected = if previous {
            (self.selected + n - 1) % n
        } else {
            (self.selected + 1) % n
        };
        self.jump = Some(self.matches[self.selected]);
    }
    fn line(&self, number: usize, text: &str, selected: bool) -> Line<'static> {
        let base = if text.starts_with('#') {
            Style::default().fg(ui::ACCENT).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(ui::TEXT)
        };
        let mut spans = vec![Span::styled(
            format!("{:>4} │ ", number + 1),
            Style::default().fg(if selected { ui::GOLD } else { ui::MUTED }),
        )];
        let mut last = 0;
        if let Some(regex) = &self.regex {
            for m in regex.find_iter(text).take(128) {
                spans.push(Span::styled(text[last..m.start()].to_string(), base));
                spans.push(Span::styled(
                    text[m.start()..m.end()].to_string(),
                    Style::default()
                        .fg(ui::BG)
                        .bg(ui::GOLD)
                        .add_modifier(Modifier::BOLD),
                ));
                last = m.end();
            }
        }
        spans.push(Span::styled(text[last..].to_string(), base));
        Line::from(spans).style(if selected && self.filtered {
            Style::default().bg(ui::RAISED)
        } else {
            Style::default()
        })
    }
    pub fn draw(&mut self, frame: &mut Frame) {
        if !self.open {
            return;
        }
        let screen = frame.area();
        let area = screen.inner(Margin {
            horizontal: if screen.width > 60 { 2 } else { 0 },
            vertical: if screen.height > 12 { 1 } else { 0 },
        });
        ui::shadow(frame, area);
        frame.render_widget(Clear, area);
        let block = ui::panel(" USER GUIDE · offline reference ");
        let inner = block.inner(area);
        frame.render_widget(block, area);
        let [search, body, footer, actions] = Layout::vertical([
            Constraint::Length(2),
            Constraint::Min(0),
            Constraint::Length(2),
            Constraint::Length(1),
        ])
        .areas(inner);
        let label = self.draft.as_deref().unwrap_or(&self.query);
        frame.render_widget(
            Paragraph::new(format!(
                "/ {label}{}\n{} matching lines · {}",
                if self.draft.is_some() { "▏" } else { "" },
                self.matches.len(),
                if self.filtered {
                    "grep results"
                } else {
                    "full guide"
                }
            ))
            .style(Style::default().fg(ui::ACCENT)),
            search,
        );
        let text_area = Rect {
            width: body.width.saturating_sub(1),
            ..body
        };
        let all: Vec<_> = MANUAL.lines().collect();
        let indices: Vec<_> = if self.filtered {
            self.matches.clone()
        } else {
            (0..all.len()).collect()
        };
        let lines: Vec<_> = indices
            .iter()
            .map(|i| self.line(*i, all[*i], self.matches.get(self.selected) == Some(i)))
            .collect();
        if let Some(target) = self.jump.take() {
            let prefix = indices.iter().position(|i| *i == target).unwrap_or(0);
            self.scroll = Paragraph::new(lines[..prefix].to_vec())
                .wrap(Wrap { trim: false })
                .line_count(text_area.width.max(1));
        }
        let paragraph = Paragraph::new(if lines.is_empty() {
            vec![Line::from(
                "No matches. Press / to change the pattern, or c to clear.",
            )]
        } else {
            lines
        })
        .wrap(Wrap { trim: false });
        self.total = paragraph.line_count(text_area.width.max(1));
        self.page = body.height as usize;
        self.scroll = self.scroll.min(self.total.saturating_sub(self.page));
        frame.render_widget(
            paragraph.scroll((self.scroll.min(u16::MAX as usize) as u16, 0)),
            text_area,
        );
        if self.total > self.page && self.page > 0 {
            frame.render_stateful_widget(
                Scrollbar::new(ScrollbarOrientation::VerticalRight)
                    .thumb_style(Style::default().fg(ui::ACCENT))
                    .track_style(Style::default().fg(ui::RAISED))
                    .begin_symbol(None)
                    .end_symbol(None),
                body,
                &mut ScrollbarState::new(self.total)
                    .position(self.scroll)
                    .viewport_content_length(self.page),
            );
        }
        let help = if let Some(e) = &self.error {
            format!("Invalid pattern: {e}")
        } else if self.draft.is_some() {
            "Enter search · Esc cancel editing · Ctrl-U clear · regex, case insensitive".into()
        } else {
            "/ Search regex · n/N next/previous · Enter context/results · t Contents\n↑/↓ or j/k scroll · PgUp/PgDn · Home/End · c Clear · g/Esc Close".into()
        };
        frame.render_widget(
            Paragraph::new(help)
                .style(Style::default().fg(ui::GOLD))
                .wrap(Wrap { trim: false }),
            footer,
        );
        self.controls.clear();
        for (i, ((label, key), rect)) in [
            ("Search", KeyCode::Char('/')),
            ("Contents", KeyCode::Char('t')),
            ("Next match", KeyCode::Char('n')),
            ("Clear", KeyCode::Char('c')),
            ("Close", KeyCode::F(1)),
        ]
        .into_iter()
        .zip(
            Layout::horizontal([Constraint::Ratio(1, 5); 5])
                .split(actions)
                .iter(),
        )
        .enumerate()
        {
            ui::button(frame, *rect, label, self.focus == Some(i), false, true);
            self.controls.push((*rect, key));
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn key(c: char) -> KeyEvent {
        KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE)
    }
    #[test]
    fn searches_regex_with_line_context_and_handles_invalid_patterns() {
        let mut g = Guide::default();
        g.search("gene.*map|provenance".into());
        assert!(g.matches.len() > 5);
        g.next(false);
        assert_eq!(g.selected, 1);
        g.search("[".into());
        assert!(g.error.is_some());
        assert_eq!(g.selected, 1);
        g.search("unlikely-no-match-xy123".into());
        assert!(g.matches.is_empty());
        g.next(false);
        g.search("^## ".into());
        assert!(g.matches.len() > 10);
    }
    #[test]
    fn research_reference_and_current_objective_audit_are_searchable_offline() {
        let mut g = Guide::default();
        for query in [
            "s41591-023-02482-6",
            "Objective-by-objective",
            "Reduce bloat",
            "polygenic",
        ] {
            g.search(query.into());
            assert!(
                !g.matches.is_empty(),
                "missing research guide content: {query}"
            );
        }
    }
    #[test]
    fn typing_g_is_preserved_and_global_shortcuts_work_in_editors() {
        let mut g = Guide::default();
        assert!(!g.key(key('g'), true));
        assert!(g.key(KeyEvent::new(KeyCode::F(1), KeyModifiers::NONE), true));
        assert!(g.open);
        g.key(key('/'), true);
        g.key(key('g'), true);
        assert_eq!(g.draft.as_deref(), Some("g"));
        g.key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE), true);
        assert!(g.open);
        g.key(key('g'), true);
        assert!(!g.open);
    }
    #[test]
    fn guide_is_usable_on_small_terminals_and_searches_are_highlighted() {
        use ratatui::{Terminal, backend::TestBackend};
        let mut g = Guide {
            open: true,
            ..Guide::default()
        };
        g.search("TP53".into());
        for (w, h) in [(1, 1), (20, 6), (60, 18), (120, 40)] {
            let mut t = Terminal::new(TestBackend::new(w, h)).unwrap();
            t.draw(|f| g.draw(f)).unwrap();
        }
        assert!(!g.matches.is_empty());
    }
}

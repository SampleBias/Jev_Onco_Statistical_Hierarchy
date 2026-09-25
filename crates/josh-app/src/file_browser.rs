//! Shared local picker. Listing never reads file contents or sends provider requests.
use crate::{samples, ui, workflows};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::Style,
    widgets::{Clear, Paragraph, Wrap},
};
use std::{
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

const MAX_ENTRIES: usize = 10_000;
#[derive(Clone, Debug)]
enum Target {
    Path(PathBuf),
    Sample(usize),
}
struct Entry {
    label: String,
    detail: String,
    target: Target,
    directory: bool,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Place {
    Folder,
    Samples,
    Recent,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Focus {
    Folder,
    Samples,
    Recent,
    Parent,
    Hidden,
    Path,
    Filter,
    List,
    Open,
    Cancel,
}
const ORDER: [Focus; 10] = [
    Focus::Folder,
    Focus::Samples,
    Focus::Recent,
    Focus::Parent,
    Focus::Hidden,
    Focus::Path,
    Focus::Filter,
    Focus::List,
    Focus::Open,
    Focus::Cancel,
];

#[derive(Debug, PartialEq, Eq)]
pub enum Outcome {
    None,
    Close,
    Open(PathBuf),
    Sample(usize),
}
pub struct Browser {
    cwd: PathBuf,
    place: Place,
    entries: Vec<Entry>,
    selected: usize,
    offset: usize,
    visible_rows: usize,
    path: String,
    filter: String,
    focus: Focus,
    hidden: bool,
    recent: Vec<PathBuf>,
    pub status: String,
    controls: Vec<(Rect, Focus)>,
    rows: Vec<(Rect, usize)>,
    last_click: Option<(usize, Instant)>,
    reference_only: bool,
    truncated: bool,
}
impl Browser {
    pub fn new(cwd: PathBuf, recent: Vec<PathBuf>, samples: bool, reference_only: bool) -> Self {
        let mut b = Self {
            cwd,
            place: if samples {
                Place::Samples
            } else {
                Place::Folder
            },
            entries: vec![],
            selected: 0,
            offset: 0,
            visible_rows: 1,
            path: String::new(),
            filter: String::new(),
            focus: if samples { Focus::List } else { Focus::Path },
            hidden: false,
            recent,
            status: String::new(),
            controls: vec![],
            rows: vec![],
            last_click: None,
            reference_only,
            truncated: false,
        };
        b.refresh();
        b
    }
    pub fn directory(&self) -> &Path {
        &self.cwd
    }
    fn refresh(&mut self) {
        self.truncated = false;
        let result = match self.place {
            Place::Samples => Ok(samples::SAMPLES
                .iter()
                .enumerate()
                .map(|(i, s)| Entry {
                    label: s.name.into(),
                    detail: s.description.into(),
                    target: Target::Sample(i),
                    directory: false,
                })
                .collect()),
            Place::Recent => Ok(self
                .recent
                .iter()
                .map(|p| Entry {
                    label: workflows::display_text(&p.display().to_string()),
                    detail: "Recently opened in this session".into(),
                    directory: p.is_dir(),
                    target: Target::Path(p.clone()),
                })
                .collect()),
            Place::Folder => self.list_directory(),
        };
        match result {
            Ok(entries) => {
                self.entries = entries;
                self.status = if self.truncated {
                    "Large folder: showing at most 10,000 entries. Use a direct path for other files.".into()
                } else {
                    "Select an item, then Open. Loading is local; analysis is a separate action."
                        .into()
                };
            }
            Err(e) => {
                self.entries.clear();
                self.status = format!(
                    "Cannot browse this folder: {e}. Enter another path or choose Samples."
                );
            }
        }
        self.selected = 0;
        self.offset = 0;
        self.last_click = None;
    }
    fn list_directory(&mut self) -> std::io::Result<Vec<Entry>> {
        let mut entries = vec![];
        for (index, entry) in std::fs::read_dir(&self.cwd)?
            .take(MAX_ENTRIES + 1)
            .enumerate()
        {
            if index == MAX_ENTRIES {
                self.truncated = true;
                break;
            }
            let entry = entry?;
            let p = entry.path();
            let name = entry.file_name().to_string_lossy().into_owned();
            if !self.hidden && name.starts_with('.') {
                continue;
            }
            let metadata = match std::fs::metadata(&p) {
                Ok(m) => m,
                Err(_) => continue,
            };
            let directory = metadata.is_dir();
            let ext = p
                .extension()
                .and_then(|s| s.to_str())
                .unwrap_or_default()
                .to_ascii_lowercase();
            if !directory
                && (!metadata.is_file()
                    || !matches!(ext.as_str(), "json" | "csv" | "tsv" | "maf" | "vcf"))
            {
                continue;
            }
            let bundle = directory && Self::bundle(&p);
            let detail = if bundle {
                "Saved run / data bundle · Open loads it".into()
            } else if directory {
                "Folder · Enter to browse".into()
            } else {
                format!("{} · {} bytes", ext.to_ascii_uppercase(), metadata.len())
            };
            entries.push(Entry {
                label: format!(
                    "{}{}",
                    workflows::display_text(&name),
                    if directory { "/" } else { "" }
                ),
                detail,
                target: Target::Path(p),
                directory,
            });
        }
        entries.sort_by(|a, b| {
            b.directory
                .cmp(&a.directory)
                .then_with(|| a.label.to_lowercase().cmp(&b.label.to_lowercase()))
                .then(a.label.cmp(&b.label))
        });
        Ok(entries)
    }
    fn bundle(p: &Path) -> bool {
        ["features.json", "dataset.json", "manifest.json"]
            .iter()
            .any(|name| p.join(name).is_file())
    }
    fn visible(&self) -> Vec<usize> {
        let needle = self.filter.to_lowercase();
        self.entries
            .iter()
            .enumerate()
            .filter(|(_, e)| {
                format!("{} {}", e.label, e.detail)
                    .to_lowercase()
                    .contains(&needle)
            })
            .map(|(i, _)| i)
            .collect()
    }
    fn change_place(&mut self, place: Place) {
        if self.reference_only && place == Place::Samples {
            self.status =
                "Reference comparison needs a reference-release JSON, not a patient sample.".into();
            return;
        }
        self.place = place;
        self.path.clear();
        self.filter.clear();
        self.focus = Focus::List;
        self.refresh();
    }
    fn enter_directory(&mut self, path: PathBuf) {
        // A failed navigation does not strand the picker or replace the current listing.
        match std::fs::read_dir(&path) {
            Ok(_) => {
                self.cwd = path;
                self.change_place(Place::Folder);
            }
            Err(e) => self.status = format!("Cannot open folder: {e}"),
        }
    }
    fn resolve(&self) -> Result<PathBuf, String> {
        let raw = self.path.trim().trim_matches('"').trim_matches('\'');
        if raw.is_empty() {
            return Err("Select a file or enter its path.".into());
        }
        let p = if raw == "~" || raw.starts_with("~/") {
            let home = std::env::var_os("HOME")
                .ok_or("Home directory is unavailable; use an absolute path.")?;
            PathBuf::from(home).join(raw.strip_prefix("~/").unwrap_or_default())
        } else {
            PathBuf::from(raw)
        };
        Ok(if p.is_absolute() { p } else { self.cwd.join(p) })
    }
    fn open_path(&mut self, p: PathBuf) -> Outcome {
        if p.is_dir() && !Self::bundle(&p) {
            self.enter_directory(p);
            Outcome::None
        } else if p.exists() {
            Outcome::Open(p)
        } else {
            self.status = "That path does not exist. Choose an item or correct the path.".into();
            Outcome::None
        }
    }
    fn open(&mut self) -> Outcome {
        if !self.path.trim().is_empty() {
            return match self.resolve() {
                Ok(p) => self.open_path(p),
                Err(e) => {
                    self.status = e;
                    Outcome::None
                }
            };
        }
        let visible = self.visible();
        let target = visible
            .get(self.selected)
            .map(|i| self.entries[*i].target.clone());
        match target {
            Some(Target::Sample(i)) => Outcome::Sample(i),
            Some(Target::Path(p)) => self.open_path(p),
            None => {
                self.status = "No item selected. Clear the filter or choose another folder.".into();
                Outcome::None
            }
        }
    }
    fn activate(&mut self, focus: Focus) -> Outcome {
        match focus {
            Focus::Folder => self.change_place(Place::Folder),
            Focus::Samples => self.change_place(Place::Samples),
            Focus::Recent => self.change_place(Place::Recent),
            Focus::Parent => {
                if let Some(parent) = self.cwd.parent() {
                    self.enter_directory(parent.into());
                }
            }
            Focus::Hidden => {
                self.hidden = !self.hidden;
                self.refresh();
            }
            Focus::Open | Focus::Path | Focus::List => return self.open(),
            Focus::Cancel => return Outcome::Close,
            Focus::Filter => self.focus = Focus::List,
        }
        Outcome::None
    }
    pub fn paste(&mut self, text: &str) {
        let value = text.trim().trim_matches('"').trim_matches('\'');
        let dest = if self.focus == Focus::Filter {
            &mut self.filter
        } else {
            self.focus = Focus::Path;
            &mut self.path
        };
        if !value.chars().any(char::is_control) && dest.len() + value.len() <= 4096 {
            dest.push_str(value);
            self.selected = 0;
        } else {
            self.status = "Enter one path, at most 4096 bytes, without control characters.".into();
        }
    }
    pub fn key(&mut self, key: KeyEvent) -> Outcome {
        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
        match key.code {
            KeyCode::Esc => return Outcome::Close,
            KeyCode::Tab | KeyCode::BackTab => {
                let i = ORDER.iter().position(|x| *x == self.focus).unwrap_or(0);
                self.focus = ORDER[(i + if key.code == KeyCode::BackTab
                    || key.modifiers.contains(KeyModifiers::SHIFT)
                {
                    ORDER.len() - 1
                } else {
                    1
                }) % ORDER.len()];
            }
            KeyCode::Enter => return self.activate(self.focus),
            KeyCode::Char(' ') if !matches!(self.focus, Focus::Path | Focus::Filter) => {
                return self.activate(self.focus);
            }
            KeyCode::Char('l') if ctrl => self.focus = Focus::Path,
            KeyCode::Char('/') if !matches!(self.focus, Focus::Path | Focus::Filter) => {
                self.focus = Focus::Filter
            }
            KeyCode::Char('u') if ctrl => {
                if self.focus == Focus::Filter {
                    self.filter.clear();
                    self.selected = 0;
                } else {
                    self.path.clear();
                }
            }
            KeyCode::Down | KeyCode::Up | KeyCode::PageDown | KeyCode::PageUp => {
                self.focus = Focus::List;
                self.path.clear();
                let step = if matches!(key.code, KeyCode::PageDown | KeyCode::PageUp) {
                    self.visible_rows
                } else {
                    1
                };
                self.selected = if matches!(key.code, KeyCode::Down | KeyCode::PageDown) {
                    (self.selected + step).min(self.visible().len().saturating_sub(1))
                } else {
                    self.selected.saturating_sub(step)
                };
            }
            KeyCode::Home if self.focus == Focus::List => self.selected = 0,
            KeyCode::End if self.focus == Focus::List => {
                self.selected = self.visible().len().saturating_sub(1)
            }
            KeyCode::Backspace if self.focus == Focus::List => return self.activate(Focus::Parent),
            KeyCode::Right if self.focus == Focus::List => {
                if let Some(entry) = self.visible().get(self.selected).map(|i| &self.entries[*i])
                    && entry.directory
                    && let Target::Path(p) = &entry.target
                {
                    self.enter_directory(p.clone());
                }
            }
            KeyCode::Backspace => {
                if self.focus == Focus::Filter {
                    self.filter.pop();
                    self.selected = 0;
                } else if self.focus == Focus::Path {
                    self.path.pop();
                }
            }
            KeyCode::Char(c) if !ctrl && !c.is_control() => {
                let dest = if self.focus == Focus::Filter {
                    self.selected = 0;
                    &mut self.filter
                } else if self.focus == Focus::Path {
                    &mut self.path
                } else {
                    return Outcome::None;
                };
                if dest.len() + c.len_utf8() <= 4096 {
                    dest.push(c);
                }
            }
            _ => (),
        }
        Outcome::None
    }
    pub fn mouse(&mut self, event: MouseEvent) -> Outcome {
        match event.kind {
            MouseEventKind::Down(MouseButton::Left) => {
                if let Some((_, focus)) = self
                    .controls
                    .iter()
                    .find(|(r, _)| r.contains((event.column, event.row).into()))
                {
                    let focus = *focus;
                    self.focus = focus;
                    if !matches!(focus, Focus::Path | Focus::Filter) {
                        return self.activate(focus);
                    }
                } else if let Some((_, i)) = self
                    .rows
                    .iter()
                    .find(|(r, _)| r.contains((event.column, event.row).into()))
                {
                    let i = *i;
                    self.selected = i;
                    self.focus = Focus::List;
                    self.path.clear();
                    if self.last_click.is_some_and(|(previous, t)| {
                        previous == i && t.elapsed() < Duration::from_millis(400)
                    }) {
                        self.last_click = None;
                        return self.open();
                    }
                    self.last_click = Some((i, Instant::now()));
                }
            }
            MouseEventKind::ScrollDown | MouseEventKind::ScrollUp => {
                return self.key(KeyEvent::new(
                    if event.kind == MouseEventKind::ScrollDown {
                        KeyCode::Down
                    } else {
                        KeyCode::Up
                    },
                    KeyModifiers::NONE,
                ));
            }
            _ => (),
        }
        Outcome::None
    }
    pub fn draw(&mut self, frame: &mut Frame) {
        let area = ui::popup(frame.area(), 112, 34);
        ui::shadow(frame, area);
        frame.render_widget(Clear, area);
        let title = if self.reference_only {
            " Browse reference release · local files "
        } else {
            " Browse data · load locally, then choose an analysis "
        };
        let panel = ui::panel(title);
        let inner = panel.inner(area);
        frame.render_widget(panel, area);
        let [
            places,
            location,
            path,
            filter,
            list,
            detail,
            status,
            actions,
            hint,
        ] = Layout::vertical([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(3),
            Constraint::Length(1),
            Constraint::Min(1),
            Constraint::Length(2),
            Constraint::Length(2),
            Constraint::Length(3),
            Constraint::Length(1),
        ])
        .areas(inner);
        self.controls.clear();
        self.rows.clear();
        for ((id, label), rect) in [
            (Focus::Folder, "Folder"),
            (Focus::Samples, "Samples"),
            (Focus::Recent, "Recent"),
            (Focus::Parent, "Up folder"),
            (
                Focus::Hidden,
                if self.hidden {
                    "Hide hidden"
                } else {
                    "Show hidden"
                },
            ),
        ]
        .into_iter()
        .zip(
            Layout::horizontal([Constraint::Ratio(1, 5); 5])
                .split(places)
                .iter(),
        ) {
            ui::button(
                frame,
                *rect,
                label,
                self.focus == id,
                matches!(
                    (id, self.place),
                    (Focus::Folder, Place::Folder)
                        | (Focus::Samples, Place::Samples)
                        | (Focus::Recent, Place::Recent)
                ),
                !self.reference_only || id != Focus::Samples,
            );
            self.controls.push((*rect, id));
        }
        let location_text = match self.place {
            Place::Folder => workflows::display_text(&self.cwd.display().to_string()),
            Place::Samples => {
                "Built-in synthetic inputs · no files to prepare · no prewritten predictions".into()
            }
            Place::Recent => "Recently opened · this session only".into(),
        };
        frame.render_widget(
            Paragraph::new(location_text).style(Style::default().fg(ui::MUTED)),
            location,
        );
        let path_value = ui::tail(&self.path, path.width.saturating_sub(3) as usize);
        frame.render_widget(
            Paragraph::new(path_value.clone()).block(
                ui::panel(" Path (optional) · paste a file or folder path ").border_style(
                    Style::default().fg(if self.focus == Focus::Path {
                        ui::ACCENT
                    } else {
                        ui::RAISED
                    }),
                ),
            ),
            path,
        );
        self.controls.push((path, Focus::Path));
        frame.render_widget(
            Paragraph::new(format!("Filter: {}", workflows::display_text(&self.filter))).style(
                Style::default().fg(if self.focus == Focus::Filter {
                    ui::ACCENT
                } else {
                    ui::MUTED
                }),
            ),
            filter,
        );
        self.controls.push((filter, Focus::Filter));
        if self.focus == Focus::Path && path.width > 2 && path.height > 2 {
            frame.set_cursor_position((
                path.x
                    + 1
                    + ratatui::text::Line::from(path_value)
                        .width()
                        .min(path.width.saturating_sub(3) as usize) as u16,
                path.y + 1,
            ));
        }
        let visible = self.visible();
        self.selected = self.selected.min(visible.len().saturating_sub(1));
        self.visible_rows = list.height.max(1) as usize;
        if self.selected < self.offset {
            self.offset = self.selected;
        }
        if self.selected >= self.offset + self.visible_rows {
            self.offset = self.selected + 1 - self.visible_rows;
        }
        for (position, index) in visible
            .iter()
            .enumerate()
            .skip(self.offset)
            .take(self.visible_rows)
        {
            let rect = Rect::new(
                list.x,
                list.y + (position - self.offset) as u16,
                list.width,
                1,
            );
            let e = &self.entries[*index];
            let selected = position == self.selected;
            let label = format!("{} {}", if selected { ">" } else { " " }, e.label);
            frame.render_widget(
                Paragraph::new(label).style(if selected {
                    Style::default()
                        .fg(ui::BG)
                        .bg(if self.focus == Focus::List {
                            ui::ACCENT
                        } else {
                            ui::BLUE
                        })
                } else {
                    Style::default().fg(ui::TEXT)
                }),
                rect,
            );
            self.rows.push((rect, position));
        }
        if visible.is_empty() {
            frame.render_widget(
                Paragraph::new("No matching items. Clear Filter, use Path, or choose Samples."),
                list,
            );
        }
        let text = visible
            .get(self.selected)
            .map(|i| self.entries[*i].detail.as_str())
            .unwrap_or("Supported: JSON, CSV, TSV, MAF, VCF and saved run / data folders.");
        frame.render_widget(
            Paragraph::new(text)
                .wrap(Wrap { trim: false })
                .style(Style::default().fg(ui::BLUE)),
            detail,
        );
        frame.render_widget(
            Paragraph::new(workflows::display_text(&self.status))
                .wrap(Wrap { trim: false })
                .style(Style::default().fg(ui::GOLD)),
            status,
        );
        for ((id, label), rect) in [(Focus::Open, "Open selected"), (Focus::Cancel, "Cancel")]
            .into_iter()
            .zip(
                Layout::horizontal([Constraint::Ratio(1, 2); 2])
                    .split(actions)
                    .iter(),
            )
        {
            ui::button(frame, *rect, label, self.focus == id, false, true);
            self.controls.push((*rect, id));
        }
        frame.render_widget(Paragraph::new("↑/↓ select · Enter open · Tab focus · / filter · → browse inside bundle · Esc cancel").style(Style::default().fg(ui::MUTED)),hint);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn key(b: &mut Browser, code: KeyCode) -> Outcome {
        b.key(KeyEvent::new(code, KeyModifiers::NONE))
    }
    #[test]
    fn built_in_inputs_are_one_selection_away_and_do_not_require_files() {
        let mut b = Browser::new(PathBuf::from("/missing"), vec![], true, false);
        assert_eq!(key(&mut b, KeyCode::Enter), Outcome::Sample(0));
        key(&mut b, KeyCode::Down);
        assert_eq!(key(&mut b, KeyCode::Enter), Outcome::Sample(1));
    }
    #[test]
    fn directories_bundles_filter_and_paths_with_spaces_work() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();
        std::fs::create_dir(root.join("folder")).unwrap();
        std::fs::create_dir(root.join("saved run")).unwrap();
        std::fs::write(root.join("saved run/features.json"), "{}").unwrap();
        std::fs::write(root.join("sample one.json"), "{}").unwrap();
        std::fs::write(root.join(".hidden.json"), "{}").unwrap();
        let mut b = Browser::new(root.into(), vec![], false, false);
        assert_eq!(b.entries.len(), 3);
        b.paste("\"sample one.json\"");
        assert_eq!(
            key(&mut b, KeyCode::Enter),
            Outcome::Open(root.join("sample one.json"))
        );
        b.path.clear();
        b.focus = Focus::Filter;
        b.paste("saved run");
        key(&mut b, KeyCode::Enter);
        assert_eq!(
            key(&mut b, KeyCode::Enter),
            Outcome::Open(root.join("saved run"))
        );
        key(&mut b, KeyCode::Right);
        assert_eq!(b.cwd, root.join("saved run"));
        key(&mut b, KeyCode::Backspace);
        assert_eq!(b.cwd, root);
        b.path = "missing.json".into();
        b.focus = Focus::Path;
        assert_eq!(key(&mut b, KeyCode::Enter), Outcome::None);
        assert!(b.status.contains("does not exist"));
        assert_eq!(b.cwd, root);
    }
    #[test]
    fn tiny_screens_unicode_and_mouse_never_panic_or_send() {
        let mut b = Browser::new(PathBuf::from("/missing"), vec![], true, false);
        b.path = "界🧬".repeat(100);
        for (w, h) in [(120, 40), (80, 24), (60, 20), (20, 8), (1, 1)] {
            let mut t = ratatui::Terminal::new(ratatui::backend::TestBackend::new(w, h)).unwrap();
            t.draw(|f| b.draw(f)).unwrap();
        }
    }
}

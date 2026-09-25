//! Cohort observatory: offline study analysis with one shared terminal lifecycle.
use crate::{
    study,
    study_charts::{self as charts, PlotState},
    ui, workflows,
};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use josh_core::DataClass;
use josh_features::study::{Report, Study};
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::Style,
    text::{Line, Span},
    widgets::{Block, Clear, Paragraph, Tabs, Wrap},
};
use std::path::{Path, PathBuf};

const TABS: [&str; 6] = [
    "Overview",
    "Confusion",
    "Survival",
    "Treatment",
    "Calibration",
    "Protocol",
];
type Loaded = (Study, Report);
#[derive(Default)]
pub struct App {
    pub loaded: Option<Loaded>,
    pub page: usize,
    pub plot: PlotState,
    scroll: usize,
    status: String,
    export_path: Option<String>,
    pending: Option<tokio::task::JoinHandle<Result<Loaded, String>>>,
    discard_pending: bool,
    tab_hits: Vec<Rect>,
}
impl App {
    pub fn busy(&self) -> bool {
        self.pending.is_some()
    }
    pub fn editing(&self) -> bool {
        self.export_path.is_some()
    }
    pub(crate) fn views(&self) -> Vec<(usize, &'static str)> {
        TABS.into_iter().enumerate().collect()
    }
    pub(crate) fn select_view(&mut self, page: usize) {
        self.page = page.min(TABS.len() - 1);
        self.scroll = 0;
    }
    pub(crate) fn status(&self) -> String {
        self.status.clone()
    }
    pub(crate) fn context(&self) -> String {
        self.loaded
            .as_ref()
            .map(|(d, _)| {
                if d.data_class == DataClass::Synthetic {
                    "SYNTHETIC DEMONSTRATION · invented cohort, no clinical performance".into()
                } else {
                    "FROZEN COHORT · offline research statistics, not clinical validation".into()
                }
            })
            .unwrap_or(
                "Browse a cohort-study JSON, or choose Offline demo. No provider calls.".into(),
            )
    }
    pub(crate) fn editor(&self) -> Option<crate::editor::Editor> {
        self.export_path.as_ref().map(|p|crate::editor::Editor::new("Export cohort","Save a figure (.svg), Markdown report (.md) or reopenable archive (.json). Existing files are protected.","Export",&self.status).field("New output file",p))
    }
    pub fn cancel_job(&mut self) {
        if self.busy() {
            self.discard_pending = true;
            self.status = "Finishing the local calculation; its result will be discarded.".into();
        }
    }
    fn begin(&mut self, input: Option<PathBuf>) {
        if self.busy() {
            return;
        }
        self.discard_pending = false;
        self.status = "Reading cohort and computing statistics locally…".into();
        self.pending = Some(tokio::task::spawn_blocking(move || {
            let data = match input {
                Some(path) => study::load(&path).map_err(|e| e.to_string())?,
                None => study::demo(),
            };
            let report =
                josh_features::study::analyze(&data, 200, 42).map_err(|e| e.to_string())?;
            Ok((data, report))
        }));
    }
    pub fn open(&mut self, path: &Path) {
        self.begin(Some(path.into()));
    }
    pub async fn poll(&mut self) {
        if self.pending.as_ref().is_some_and(|p| p.is_finished()) {
            let result = self.pending.take().unwrap().await;
            if self.discard_pending {
                self.status = "Calculation discarded; previous cohort preserved.".into();
                return;
            }
            match result {
                Ok(Ok(loaded)) => {
                    self.loaded = Some(loaded);
                    self.plot = PlotState::default();
                    self.scroll = 0;
                    self.status="Cohort ready · 1–6 views · arrows inspect · i confidence · w IPTW · s export".into();
                }
                Ok(Err(e)) => self.status = e,
                Err(e) => self.status = format!("Local calculation failed: {e}"),
            }
        }
    }
    pub fn key(&mut self, key: KeyEvent) -> bool {
        if let Some(path) = &mut self.export_path {
            match key.code {
                KeyCode::Esc => self.export_path = None,
                KeyCode::Backspace => {
                    path.pop();
                }
                KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => path.clear(),
                KeyCode::Char(c)
                    if !c.is_control()
                        && !key.modifiers.contains(KeyModifiers::CONTROL)
                        && path.len() + c.len_utf8() <= 4096 =>
                {
                    path.push(c)
                }
                KeyCode::Enter => {
                    let target = PathBuf::from(path.trim());
                    let kind = match target.extension().and_then(|e| e.to_str()) {
                        Some("svg") => Some(study::ExportKind::Svg),
                        Some("md") => Some(study::ExportKind::Markdown),
                        Some("json") => Some(study::ExportKind::Json),
                        _ => None,
                    };
                    if let (Some(kind), Some((data, report))) = (kind, &self.loaded) {
                        let result = study::export(data, report, kind)
                            .and_then(|s| workflows::save_new(&target, &s));
                        match result {
                            Ok(()) => {
                                self.status = format!("Exported {}", target.display());
                                self.export_path = None;
                            }
                            Err(e) => self.status = e.to_string(),
                        }
                    } else {
                        self.status = "Choose a new .svg, .md or .json file.".into();
                    }
                }
                _ => (),
            }
            return false;
        }
        match key.code {
            KeyCode::Char('d') if !self.busy() => self.begin(None),
            KeyCode::Char('x') => self.cancel_job(),
            KeyCode::Char('s') if self.loaded.is_some() && !self.busy() => {
                self.export_path = Some(String::new())
            }
            KeyCode::Char(c @ '1'..='6') => {
                self.page = (c as u8 - b'1') as usize;
                self.scroll = 0;
            }
            KeyCode::Tab => {
                self.page = (self.page + 1) % TABS.len();
                self.scroll = 0;
            }
            KeyCode::BackTab => {
                self.page = (self.page + TABS.len() - 1) % TABS.len();
                self.scroll = 0;
            }
            KeyCode::Char('n') => self.plot.normalization = self.plot.normalization.next(),
            KeyCode::Char('b') => {
                if self
                    .loaded
                    .as_ref()
                    .is_some_and(|(_, r)| r.broad_evaluation.is_some())
                {
                    self.plot.broad = !self.plot.broad;
                    self.plot.row = 0;
                    self.plot.column = 0;
                } else {
                    self.status="No broad-group mapping supplied. Add a complete, versioned mapping to the study.".into();
                }
            }
            KeyCode::Char('i') => self.plot.confidence = !self.plot.confidence,
            KeyCode::Char('w') => self.plot.weighted = !self.plot.weighted,
            KeyCode::Char('[') => self.plot.curve = self.plot.curve.saturating_sub(1),
            KeyCode::Char(']') => {
                let n = self
                    .loaded
                    .as_ref()
                    .and_then(|(_, r)| r.survival.as_ref())
                    .map_or(1, |s| s.by_class.len());
                self.plot.curve = (self.plot.curve + 1).min(n.saturating_sub(1));
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if self.page <= 1 {
                    self.plot.row = self.plot.row.saturating_sub(1);
                } else {
                    self.scroll = self.scroll.saturating_sub(1);
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if self.page <= 1 {
                    let n = self
                        .loaded
                        .as_ref()
                        .and_then(|(d, r)| charts::Matrix::new(d, r, self.plot.broad))
                        .map_or(1, |m| m.rows.len());
                    self.plot.row = (self.plot.row + 1).min(n.saturating_sub(1));
                } else {
                    self.scroll = self.scroll.saturating_add(1).min(1000);
                }
            }
            KeyCode::Left => self.plot.column = self.plot.column.saturating_sub(1),
            KeyCode::Right => {
                let n = self
                    .loaded
                    .as_ref()
                    .and_then(|(d, r)| charts::Matrix::new(d, r, self.plot.broad))
                    .map_or(1, |m| m.columns.len());
                self.plot.column = (self.plot.column + 1).min(n.saturating_sub(1));
            }
            KeyCode::Home => {
                self.scroll = 0;
                self.plot.row = 0;
                self.plot.column = 0;
            }
            KeyCode::PageDown => self.scroll = self.scroll.saturating_add(10).min(1000),
            KeyCode::PageUp => self.scroll = self.scroll.saturating_sub(10),
            _ => (),
        }
        false
    }
    pub fn paste(&mut self, text: &str) {
        if let Some(path) = &mut self.export_path {
            let value = text.trim().trim_matches('"').trim_matches('\'');
            if !value.chars().any(char::is_control) && path.len() + value.len() <= 4096 {
                path.push_str(value);
            }
        }
    }
    pub fn mouse(&mut self, event: MouseEvent) {
        if self.editing() {
            return;
        }
        if event.kind == MouseEventKind::Down(MouseButton::Left) {
            if let Some(i) = self
                .tab_hits
                .iter()
                .position(|r| r.contains((event.column, event.row).into()))
            {
                self.page = i;
                self.scroll = 0;
            }
        } else if matches!(
            event.kind,
            MouseEventKind::ScrollDown | MouseEventKind::ScrollUp
        ) {
            self.key(KeyEvent::new(
                if event.kind == MouseEventKind::ScrollDown {
                    KeyCode::Down
                } else {
                    KeyCode::Up
                },
                KeyModifiers::NONE,
            ));
        }
    }
    pub fn draw_area(&mut self, frame: &mut Frame, area: Rect) {
        self.draw_workspace(frame, area, false);
    }
    pub(crate) fn draw_workspace(&mut self, frame: &mut Frame, area: Rect, embedded: bool) {
        frame.render_widget(
            Block::default().style(Style::default().bg(ui::BG).fg(ui::TEXT)),
            area,
        );
        let [heading, tabs, body, status] = Layout::vertical([
            Constraint::Length(if embedded { 0 } else { 2 }),
            Constraint::Length(if embedded { 0 } else { 3 }),
            Constraint::Min(1),
            Constraint::Length(if embedded { 0 } else { 2 }),
        ])
        .areas(area);
        let (subtitle, color) = self
            .loaded
            .as_ref()
            .map(|(d, _)| {
                if d.data_class == DataClass::Synthetic {
                    (
                        "SYNTHETIC DEMONSTRATION · invented data, no clinical performance",
                        ui::GOLD,
                    )
                } else {
                    ("FROZEN COHORT · offline research analysis", ui::MUTED)
                }
            })
            .unwrap_or((
                "CLASSIFICATION  /  SURVIVAL  /  TREATMENT OUTCOMES",
                ui::MUTED,
            ));
        frame.render_widget(
            Paragraph::new(vec![
                Line::from(vec![
                    Span::styled(
                        " COHORT OBSERVATORY",
                        Style::default().fg(ui::ACCENT).bold(),
                    ),
                    Span::styled(
                        "  ·  OncoNPC research views",
                        Style::default().fg(ui::MUTED),
                    ),
                ]),
                Line::styled(format!(" {subtitle}"), Style::default().fg(color)),
            ]),
            heading,
        );
        let labels: Vec<_> = TABS
            .iter()
            .enumerate()
            .map(|(i, t)| {
                if area.width < 95 {
                    format!("{} {}", i + 1, t.chars().take(3).collect::<String>())
                } else {
                    format!("{} {t}", i + 1)
                }
            })
            .collect();
        let mut x = tabs.x + 1;
        self.tab_hits = labels
            .iter()
            .map(|l| {
                let r = Rect::new(x, tabs.y + 1, l.len() as u16 + 2, 1).intersection(tabs);
                x = x.saturating_add(l.len() as u16 + 3);
                r
            })
            .collect();
        frame.render_widget(
            Tabs::new(labels)
                .select(self.page)
                .divider(" ")
                .block(ui::panel(" STUDY VIEWS "))
                .style(Style::default().fg(ui::MUTED))
                .highlight_style(Style::default().fg(ui::BG).bg(ui::ACCENT).bold()),
            tabs,
        );
        if let Some((data, report)) = &self.loaded {
            let [cards, plots] = Layout::vertical([
                Constraint::Length(if body.height >= 16 { 4 } else { 0 }),
                Constraint::Min(1),
            ])
            .areas(body);
            charts::metrics(frame, cards, data, report);
            match self.page {
                0 if plots.width >= 115 && plots.height >= 28 => {
                    let [matrix, curves] = Layout::horizontal([
                        Constraint::Percentage(52),
                        Constraint::Percentage(48),
                    ])
                    .areas(plots);
                    let [subtype, treatment] =
                        Layout::vertical([Constraint::Ratio(1, 2); 2]).areas(curves);
                    charts::heatmap(frame, matrix, data, report, &self.plot);
                    charts::survival(frame, subtype, data, report, &self.plot, false);
                    charts::survival(frame, treatment, data, report, &self.plot, true);
                }
                0 | 1 => charts::heatmap(frame, plots, data, report, &self.plot),
                2 => charts::survival(frame, plots, data, report, &self.plot, false),
                3 if plots.width >= 100 => {
                    let [curves, details] = Layout::horizontal([
                        Constraint::Percentage(65),
                        Constraint::Percentage(35),
                    ])
                    .areas(plots);
                    charts::survival(frame, curves, data, report, &self.plot, true);
                    charts::treatment_details(frame, details, report, self.scroll);
                }
                3 => {
                    let [curves, details] =
                        Layout::vertical([Constraint::Percentage(65), Constraint::Percentage(35)])
                            .areas(plots);
                    charts::survival(frame, curves, data, report, &self.plot, true);
                    charts::treatment_details(frame, details, report, self.scroll);
                }
                4 => charts::calibration(frame, plots, report),
                _ => {
                    let text = study::markdown(data, report);
                    let inner = charts::panel(frame, plots, " PROTOCOL / METRICS / REFERENCES ");
                    let paragraph = Paragraph::new(text).wrap(Wrap { trim: false });
                    self.scroll = self.scroll.min(
                        paragraph
                            .line_count(inner.width.max(1))
                            .saturating_sub(inner.height as usize),
                    );
                    frame.render_widget(
                        paragraph
                            .scroll((self.scroll.min(u16::MAX as usize) as u16, 0))
                            .style(Style::default().fg(ui::TEXT)),
                        inner,
                    );
                }
            }
        } else {
            let inner = charts::panel(frame, body, " COHORT RESEARCH ");
            let text = "\n  FROM PREDICTIONS TO EVIDENCE\n\n  A  Classification    Confusion heatmap · recall · patient-level confidence intervals\n  B  Survival          Kaplan–Meier curves · censoring · delayed entry · risk tables\n  C  Treatment         Concordance · adjusted Cox · IPTW and balance diagnostics\n\n  d   Explore an invented 22-class demonstration\n  l   Load a cohort-study JSON file\n\n  s exports SVG, Markdown or a reopenable JSON archive.\n  Studies are analyzed locally. No provider request is made.\n\n  Reference: Moon et al. OncoNPC (2022 / 2023).\n  The source study's results do not establish Jev performance.";
            frame.render_widget(
                Paragraph::new(text)
                    .wrap(Wrap { trim: false })
                    .style(Style::default().fg(ui::TEXT)),
                inner,
            );
        }
        frame.render_widget(Paragraph::new(format!("{}{}\n1–6 views · d demo · l load · s export · n normalize · i CI · w IPTW · F1 help",if self.busy(){"● COMPUTING  "}else{""},workflows::display_text(&self.status))).style(Style::default().fg(if self.busy(){ui::GOLD}else{ui::MUTED})),status);
        if !embedded && let Some(path) = &self.export_path {
            let width = area.width.saturating_sub(4).min(90);
            let height = area.height.min(9);
            let rect = Rect::new(
                area.x + (area.width - width) / 2,
                area.y + (area.height - height) / 2,
                width,
                height,
            );
            ui::shadow(frame, rect);
            frame.render_widget(Clear, rect);
            frame.render_widget(Paragraph::new(format!("Save to a new .svg, .md or .json file.\n\n{}\n\nEnter save · Ctrl+u clear · Esc cancel\n{}",workflows::display_text(path),workflows::display_text(&self.status))).wrap(Wrap{trim:false}).block(ui::panel(" EXPORT COHORT ")),rect);
        }
    }
}

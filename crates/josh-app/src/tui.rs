use crate::workflows::{self, AppError};
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use josh_core::errors::{ErrorCode, ErrorEnvelope};
use josh_core::{Case, DataClass, ResultRecord, prepare};
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::Style,
    widgets::{Block, Clear, Paragraph, Tabs, Wrap},
};
use std::{io::IsTerminal, path::PathBuf, time::Duration};

mod guidance_view;
mod theme;
mod visuals;
const TAB_COUNT: usize = 8;
const CLINICAL_HELP: &str = "NEW VIEWS\n\n6 / v  Visualizations: left/right changes Scores, IHC, Pathway, Timeline, Evidence\n7      NICE guidance: up/down selects a rule; a records a review\n8      Clinical context and review history; s saves the complete case\nOn Guidance, s exports the source-linked report. On Visuals, s exports its data.\nClinical assertions are edited in standalone JSON and reloaded; absent facts stay unknown.\nReview entries need reviewer | YYYY-MM-DD | action | reason.\nActions: acknowledged, deferred, not_applicable, departed.\nSave unsaved reviews before loading another case; quitting asks before discarding them.\n\n";

const HELP: &str = "GETTING STARTED\n\nThe workbench opens a bundled synthetic case by default.\nPress o to load case JSON, or r to reload the current file.\nReview Evidence, then inspect the exact payload in Request.\n\nKEYS\n\n1 / 2 / 3 / 4 / 5  Evidence / Request / Results / Help / Import\nTab / Shift-Tab  Next / previous tab\nUp / Down, j/k   Scroll\nPgUp / PgDn     Scroll one page\nHome            Back to top\no               Open case JSON\nb               Open an import bundle directory\n[ / ]           Previous / next case in the bundle\n5               Import quality report\nr               Reload case and clear previous results\nd               Offline demo; always labeled MOCK\nc               Confirm sending the synthetic case to Jev\ns               Save Request or Results as JSON to a NEW file\nq / Ctrl-C      Quit and restore the terminal\nEsc             Close a dialog\n\nLIVE CLASSIFICATION\n\nSet TYPESAFE_API_KEY before launching. Keys are never shown.\nA live call sends evidence to api.typesafe.ai and may incur charges.\nOnly declared synthetic cases are accepted in this first build.\nDuring a request, case changes and additional runs are disabled.\nQuitting cancels local waiting; Jev may already have received the request.\n\nREADING RESULTS\n\nRaw probabilities and provider confidence are different quantities.\nNo clinical calibration is established. All results require review.\nThe demo uses a uniform distribution and does not classify cancer.\nAll 14 outcomes remain visible; scrolling never changes probability mass.\n\nEXPORTS\n\nOn Request, s exports the payload without credentials.\nOn Results, s exports the complete result with provenance.\nOn Import, s exports the complete quality report.\nExisting files are protected. Parent folders must already exist.\nEdit standalone case JSON in your editor, then reload.\nImported bundle cases are fingerprint-checked; make a standalone copy before editing.";

#[derive(Debug, PartialEq, Eq)]
enum Dialog {
    None,
    Open,
    OpenBatch,
    Save,
    ConfirmLive,
    Review,
    DiscardReviews,
}

pub struct App {
    case: Case,
    path: Option<PathBuf>,
    result: Option<ResultRecord>,
    tab: usize,
    scroll: u16,
    dialog: Dialog,
    input: String,
    status: String,
    error: bool,
    pending: Option<tokio::task::JoinHandle<Result<ResultRecord, String>>>,
    batch: Option<BatchSession>,
    visual_page: usize,
    dirty_reviews: bool,
    detail_scroll: u16,
}

struct BatchSession {
    root: PathBuf,
    report: josh_ingest::ImportReport,
    index: usize,
}

impl App {
    pub fn new(path: Option<PathBuf>) -> Result<Self, AppError> {
        if path.as_ref().is_some_and(|p| p.as_os_str() == "-") {
            return Err(
                "TUI case input must be a file; stdin is reserved for keyboard controls".into(),
            );
        }
        let case = match &path {
            Some(p) => workflows::load_case(p)?,
            None => workflows::clinical_example_case(),
        };
        Ok(Self {
            case,
            path,
            result: None,
            tab: 0,
            scroll: 0,
            dialog: Dialog::None,
            input: String::new(),
            status: "Ready. Review the case, then press d for an offline demo. Press ? for help."
                .into(),
            error: false,
            pending: None,
            batch: None,
            visual_page: 0,
            dirty_reviews: false,
            detail_scroll: 0,
        })
    }

    pub fn from_batch(root: PathBuf) -> Result<Self, AppError> {
        let mut app = Self::new(None)?;
        app.load_batch(root)?;
        Ok(app)
    }

    fn load_batch(&mut self, root: PathBuf) -> Result<(), AppError> {
        let report = josh_ingest::bundle::read_report(&root)?;
        let entry = report
            .cases
            .first()
            .ok_or_else(|| ErrorEnvelope::new(ErrorCode::NoImportedCases))?;
        let case = josh_ingest::bundle::read_case(&root, entry)?;
        self.path = Some(root.join("cases").join(format!("{}.json", case.case_id)));
        self.case = case;
        self.result = None;
        self.batch = Some(BatchSession {
            root,
            report,
            index: 0,
        });
        self.select_tab(0);
        self.message(
            "Import bundle verified. Use [ and ] for cases; 5 shows the quality report.",
            false,
        );
        Ok(())
    }

    fn batch_case(&mut self, offset: isize) -> Result<(), AppError> {
        let batch = self
            .batch
            .as_ref()
            .ok_or("open an import bundle with b first")?;
        let index = batch
            .index
            .saturating_add_signed(offset)
            .min(batch.report.cases.len() - 1);
        let case = josh_ingest::bundle::read_case(&batch.root, &batch.report.cases[index])?;
        self.path = Some(
            batch
                .root
                .join("cases")
                .join(format!("{}.json", case.case_id)),
        );
        self.case = case;
        self.result = None;
        self.batch.as_mut().expect("batch is open").index = index;
        self.select_tab(0);
        self.message("Case fingerprint verified. Previous result cleared.", false);
        Ok(())
    }

    fn message(&mut self, text: impl Into<String>, error: bool) {
        self.status = text.into();
        self.error = error;
    }
    fn select_tab(&mut self, tab: usize) {
        self.tab = tab;
        self.scroll = 0;
        self.detail_scroll = 0;
    }

    fn load(&mut self, path: Option<PathBuf>) -> Result<(), AppError> {
        if path.as_ref().is_some_and(|p| p.as_os_str() == "-") {
            return Err("open a file; stdin is reserved for keyboard controls".into());
        }
        let case = match &path {
            Some(p) => workflows::load_case(p)?,
            None => workflows::clinical_example_case(),
        };
        self.case = case;
        self.path = path;
        self.result = None;
        self.batch = None;
        self.select_tab(0);
        self.message("Case loaded and validated. Previous result cleared.", false);
        Ok(())
    }

    fn exported(&self) -> Result<String, AppError> {
        if self.tab == 7 {
            let pretty = workflows::pretty(&self.case)?;
            if pretty.len() < josh_core::MAX_CASE_BYTES {
                return Ok(pretty);
            }
            let compact = serde_json::to_string(&self.case)?;
            if compact.len() >= josh_core::MAX_CASE_BYTES {
                return Err(
                    "Case is at the file byte limit; reduce context before exporting.".into(),
                );
            }
            return Ok(compact);
        }
        if self.tab == 6 {
            return workflows::pretty(&josh_core::guidance::evaluate(&self.case)?);
        }
        if self.tab == 5 {
            return workflows::pretty(
                &serde_json::json!({"case":self.case,"result":self.result,"guidance":josh_core::guidance::evaluate(&self.case)?}),
            );
        }
        if self.tab == 4 {
            return workflows::pretty(
                &self
                    .batch
                    .as_ref()
                    .ok_or("open an import bundle with b first")?
                    .report,
            );
        }
        if self.tab == 1 {
            return workflows::pretty(&prepare(&self.case)?);
        }
        if self.tab == 2 {
            return workflows::pretty(
                self.result
                    .as_ref()
                    .ok_or("run a demo or classification before saving results")?,
            );
        }
        Err("select Request, Results, Import, Visuals, Guidance or Review before saving".into())
    }

    fn record_review_input(&mut self) -> Result<(), AppError> {
        let fields: Vec<_> = self.input.splitn(4, '|').map(str::trim).collect();
        if fields.len() != 4 {
            return Err("use reviewer | YYYY-MM-DD | action | reason".into());
        }
        let action = match fields[2] {
            "acknowledged" => josh_core::clinical::ReviewAction::Acknowledged,
            "deferred" => josh_core::clinical::ReviewAction::Deferred,
            "not_applicable" => josh_core::clinical::ReviewAction::NotApplicable,
            "departed" => josh_core::clinical::ReviewAction::Departed,
            _ => {
                return Err(
                    "action must be acknowledged, deferred, not_applicable or departed".into(),
                );
            }
        };
        let report = josh_core::guidance::evaluate(&self.case)?;
        let rule = report
            .items
            .get(self.scroll as usize)
            .ok_or("select a guidance rule first")?;
        josh_core::guidance::record_review(
            &mut self.case,
            &rule.rule_id,
            action,
            fields[3].into(),
            josh_core::clinical::Assessment {
                reviewer_id: fields[0].into(),
                recorded_on: fields[1].into(),
                source: "Local JOSH review entry; identity is self-reported".into(),
            },
        )?;
        self.result = None;
        self.dirty_reviews = true;
        self.message("Review recorded locally; result cleared. UNSAVED: press 8 then s to save a new case file.",false);
        Ok(())
    }

    fn start_live(&mut self) {
        let case = self.case.clone();
        self.result = None;
        self.pending = Some(tokio::spawn(async move {
            workflows::classify(&case)
                .await
                .map_err(|e| crate::errors::envelope(&e).to_string())
        }));
        self.select_tab(2);
        self.message(
            "Sending synthetic evidence to Jev… Browse tabs while waiting.",
            false,
        );
    }

    /// Returns true on exit. A live request starts only after the confirmation key.
    pub fn key(&mut self, key: KeyEvent) -> bool {
        if key.kind != KeyEventKind::Press {
            return false;
        }
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
            if self.dirty_reviews {
                self.dialog = Dialog::DiscardReviews;
                return false;
            }
            return true;
        }
        if self.dialog == Dialog::DiscardReviews {
            match key.code {
                KeyCode::Char('y' | 'Y') => return true,
                KeyCode::Esc | KeyCode::Char('n' | 'N') => self.dialog = Dialog::None,
                _ => {}
            }
            return false;
        }
        if self.dialog == Dialog::ConfirmLive {
            match key.code {
                KeyCode::Char('y' | 'Y') => {
                    self.dialog = Dialog::None;
                    self.start_live();
                }
                KeyCode::Esc | KeyCode::Char('n' | 'N') => {
                    self.dialog = Dialog::None;
                    self.message("Live request canceled; nothing sent.", false);
                }
                _ => {}
            }
            return false;
        }
        if self.dialog != Dialog::None {
            match key.code {
                KeyCode::Esc => {
                    self.dialog = Dialog::None;
                    self.input.clear();
                }
                KeyCode::Backspace => {
                    self.input.pop();
                }
                KeyCode::Char(c) if !c.is_control() => {
                    if self.input.len() < 4096 {
                        self.input.push(c);
                    }
                }
                KeyCode::Enter => {
                    let path = PathBuf::from(self.input.trim());
                    let result = if self.input.trim().is_empty() {
                        Err(if self.dialog == Dialog::Review {
                            "enter reviewer | YYYY-MM-DD | action | reason"
                        } else {
                            "enter a file path"
                        }
                        .into())
                    } else if self.dialog == Dialog::Open {
                        self.load(Some(path.clone()))
                    } else if self.dialog == Dialog::OpenBatch {
                        self.load_batch(path.clone())
                    } else if self.dialog == Dialog::Review {
                        self.record_review_input()
                    } else {
                        self.exported()
                            .and_then(|content| workflows::save_new(&path, &content))
                    };
                    match result {
                        Ok(()) => {
                            if self.dialog == Dialog::Save {
                                if self.tab == 7 {
                                    self.path = Some(path);
                                    self.batch = None;
                                    self.dirty_reviews = false;
                                }
                                self.message("JSON saved to a new file.", false);
                            }
                            self.dialog = Dialog::None;
                            self.input.clear();
                        }
                        Err(e) => self.message(e.to_string(), true),
                    }
                }
                _ => {}
            }
            return false;
        }
        if self.tab == 6
            && matches!(
                key.code,
                KeyCode::Up | KeyCode::Down | KeyCode::Char('j' | 'k')
            )
        {
            self.detail_scroll = 0;
        }
        match key.code {
            KeyCode::Char('q') => {
                if self.dirty_reviews {
                    self.dialog = Dialog::DiscardReviews;
                } else {
                    return true;
                }
            }
            KeyCode::Tab => self.select_tab((self.tab + 1) % TAB_COUNT),
            KeyCode::BackTab => self.select_tab((self.tab + TAB_COUNT - 1) % TAB_COUNT),
            KeyCode::Char('1'..='8') => {
                if let KeyCode::Char(c) = key.code {
                    self.select_tab((c as u8 - b'1') as usize);
                }
            }
            KeyCode::Char('?') => self.select_tab(3),
            KeyCode::Char('v') => self.select_tab(5),
            KeyCode::Right | KeyCode::Left if self.tab == 5 => {
                self.visual_page = (self.visual_page
                    + if key.code == KeyCode::Right {
                        1
                    } else {
                        visuals::VIEW_COUNT - 1
                    })
                    % visuals::VIEW_COUNT;
                self.scroll = 0;
            }
            KeyCode::Down | KeyCode::Char('j') => self.scroll = self.scroll.saturating_add(1),
            KeyCode::Up | KeyCode::Char('k') => self.scroll = self.scroll.saturating_sub(1),
            KeyCode::Right | KeyCode::PageDown if self.tab == 6 => {
                self.detail_scroll = self.detail_scroll.saturating_add(6)
            }
            KeyCode::Left | KeyCode::PageUp if self.tab == 6 => {
                self.detail_scroll = self.detail_scroll.saturating_sub(6)
            }
            KeyCode::PageDown => self.scroll = self.scroll.saturating_add(12),
            KeyCode::PageUp => self.scroll = self.scroll.saturating_sub(12),
            KeyCode::Home => {
                self.scroll = 0;
                self.detail_scroll = 0;
            }
            KeyCode::Char('o' | 'b' | '[' | ']' | 'r') if self.dirty_reviews => {
                self.message(
                    "Unsaved reviews: press 8 then s to save a new case before changing cases.",
                    true,
                );
            }
            KeyCode::Char('o' | 'b' | '[' | ']' | 'r' | 'd' | 'c' | 'a')
                if self.pending.is_some() =>
            {
                self.message(
                    "Jev run in progress. Wait before changing the case or starting another run.",
                    true,
                )
            }
            KeyCode::Char('o') => {
                self.dialog = Dialog::Open;
                self.input.clear();
            }
            KeyCode::Char('a') if self.tab == 6 => {
                if self.case.clinical.is_none() {
                    self.message(
                        "Recording reviews requires a schema 3 case (josh example --clinical).",
                        true,
                    );
                } else {
                    self.dialog = Dialog::Review;
                    self.input.clear();
                }
            }
            KeyCode::Char('b') => {
                self.dialog = Dialog::OpenBatch;
                self.input.clear();
            }
            KeyCode::Char('[' | ']') => {
                let offset = if key.code == KeyCode::Char('[') {
                    -1
                } else {
                    1
                };
                if let Err(e) = self.batch_case(offset) {
                    self.message(e.to_string(), true);
                }
            }
            KeyCode::Char('r') => {
                let loaded = if self.batch.is_some() {
                    self.batch_case(0)
                } else {
                    self.load(self.path.clone())
                };
                if let Err(e) = loaded {
                    self.message(e.to_string(), true);
                }
            }
            KeyCode::Char('d') => match workflows::demo(&self.case) {
                Ok(result) => {
                    self.result = Some(result);
                    self.select_tab(2);
                    self.message("MOCK complete. Uniform synthetic scores; no prediction and no network request.", false);
                }
                Err(e) => self.message(e.to_string(), true),
            },
            KeyCode::Char('c') => {
                if self.case.data_class != DataClass::Synthetic {
                    self.message("Live requests currently accept synthetic cases only.", true);
                } else if !self.case.has_observed_evidence() {
                    self.message("Add observed findings before a live request.", true);
                } else if !workflows::key_configured() {
                    self.message(
                        "TYPESAFE_API_KEY is missing. Set it before launch, or press d for a demo.",
                        true,
                    );
                } else {
                    self.dialog = Dialog::ConfirmLive;
                }
            }
            KeyCode::Char('s') => match self.exported() {
                Ok(_) => {
                    self.dialog = Dialog::Save;
                    self.input.clear();
                }
                Err(e) => self.message(e.to_string(), true),
            },
            _ => {}
        }
        false
    }

    async fn poll_result(&mut self) {
        if self.pending.as_ref().is_some_and(|task| task.is_finished()) {
            let task = self.pending.take().expect("finished task exists");
            match task.await {
                Ok(Ok(result)) => {
                    self.result = Some(result);
                    self.message("Jev result received. Research review is required.", false);
                }
                Ok(Err(error)) => self.message(error, true),
                Err(_) => self.message(
                    "Jev task stopped unexpectedly; no result was recorded.",
                    true,
                ),
            }
        }
    }

    fn body(&self) -> String {
        match self.tab {
            0 => {
                let mut lines = vec![format!("Case: {}", self.case.case_id), format!("Data: {:?}", self.case.data_class),
                    format!("Age: {} | Sex at birth: {}", self.case.age_years.map(|a| a.to_string()).or_else(|| self.case.age_lower_bound_exclusive.map(|a| format!(">{a}"))).unwrap_or("unknown".into()), self.case.sex_at_birth.as_ref().map(|s| format!("{s:?}")).unwrap_or("unknown".into())),
                    format!("Specimen: {}", workflows::display_text(self.case.specimen_site.as_deref().unwrap_or("unknown"))),
                    format!("Input: {}", self.path.as_ref().map(|p| workflows::display_text(&p.display().to_string())).unwrap_or("bundled synthetic example".into())),
                    String::new(), format!("FINDINGS ({})", self.case.findings.len()), String::new()];
                if let Some(batch) = &self.batch {
                    lines.insert(1, format!("Batch case {} / {} | [ previous  ] next | 5 quality report", batch.index + 1, batch.report.cases.len()));
                }
                for finding in &self.case.findings {
                    lines.push(format!("{}  {:?} / {}", finding.id, finding.kind, workflows::display_text(&finding.name)));
                    lines.push(format!("    {}", workflows::display_text(&finding.value)));
                    if let Some(observation) = &finding.observation {
                        lines.push(format!("    Status: {:?} | Assay: {} | Units: {}", observation.status, workflows::display_text(observation.assay.as_deref().unwrap_or("unknown")), workflows::display_text(observation.units.as_deref().unwrap_or("unspecified"))));
                    }
                    if let Some(source) = &finding.source {
                        lines.push(format!("    Source: {} / record {} / {}", source.source_id, source.record, workflows::display_text(&source.field)));
                    }
                    lines.push(String::new());
                }
                lines.push("Open JSON with o. Edit it in your editor and press r to reload.".into());
                lines.join("\n")
            }
            1 => prepare(&self.case).map_err(|e| e.to_string()).and_then(|r| workflows::pretty(&r).map_err(|e| e.to_string())).unwrap_or_else(|e| e),
            2 => self.result.as_ref().map(workflows::result_text).unwrap_or_else(|| if self.pending.is_some() { "Waiting for Jev…\n\nTabs remain available. Case changes are paused until the request finishes.".into() } else { "No result yet.\n\nPress d for an offline mock, or c to send a synthetic case to Jev.\nThe mock does not make a cancer prediction.".into() }),
            4 => self.batch.as_ref().map(|b| workflows::import_report_text(&b.report)).unwrap_or_else(|| "No import bundle open.\n\nCreate one with josh import (see --help), then press b to open its directory.\nThe report lists rejected records, missing fields and observation statuses.\nUse [ and ] to browse cases; s exports the report.".into()),
            7 => workflows::clinical_text(&self.case),
            _ => format!("{CLINICAL_HELP}{HELP}"),
        }
    }

    pub fn draw(&mut self, frame: &mut Frame) {
        let area = frame.area();
        frame.render_widget(
            Block::default().style(Style::default().bg(theme::BACKGROUND).fg(theme::TEXT)),
            area,
        );
        if area.width < 48 || area.height < 12 {
            frame.render_widget(
                Paragraph::new(
                    if self.dialog == Dialog::DiscardReviews {
                        "Unsaved reviews. Quit and discard?\ny: discard and quit; n: return"
                    } else {
                        "Jev Onco Statistical Hierarchy (JOSH)\nEnlarge terminal to at least 48 x 12.\nq / Ctrl-C: quit"
                    },
                ),
                area,
            );
            return;
        }
        let [header, tabs, body, status, footer] = Layout::vertical([
            Constraint::Length(3),
            Constraint::Length(1),
            Constraint::Min(3),
            Constraint::Length(2),
            Constraint::Length(2),
        ])
        .areas(area);
        let connection = if self.pending.is_some() {
            "Jev request running"
        } else if workflows::key_configured() {
            "API key configured"
        } else {
            "Offline ready | API key missing"
        };
        frame.render_widget(
            Paragraph::new(format!(
                "JEV ONCO STATISTICAL HIERARCHY (JOSH)  /  Research workbench\n{}  |  {connection}",
                josh_core::MODEL
            ))
            .style(Style::default().fg(theme::CYAN)),
            header,
        );
        frame.render_widget(
            Tabs::new(if area.width >= 105 {
                [
                    "1 Evidence",
                    "2 Request",
                    "3 Results",
                    "4 Help",
                    "5 Import",
                    "6 Visuals",
                    "7 Guidance",
                    "8 Review",
                ]
            } else {
                ["1", "2", "3", "4", "5", "6 Viz", "7 NICE", "8"]
            })
            .select(self.tab)
            .style(Style::default().fg(theme::MUTED))
            .highlight_style(
                Style::default()
                    .fg(theme::BACKGROUND)
                    .bg(theme::TEAL)
                    .bold(),
            ),
            tabs,
        );
        if self.tab == 5 {
            visuals::draw(
                frame,
                body,
                &self.case,
                self.result.as_ref(),
                self.visual_page,
                &mut self.scroll,
            );
        } else if self.tab == 6 {
            guidance_view::draw(
                frame,
                body,
                &self.case,
                &mut self.scroll,
                &mut self.detail_scroll,
            );
        } else {
            let paragraph = Paragraph::new(self.body()).wrap(Wrap { trim: false });
            let lines = paragraph.line_count(body.width.saturating_sub(2).max(1));
            let max_scroll = lines
                .saturating_sub(body.height.saturating_sub(2) as usize)
                .min(u16::MAX as usize) as u16;
            self.scroll = self.scroll.min(max_scroll);
            let title = match self.tab {
                0 => " Case evidence ",
                1 => " Request preview — not sent ",
                2 => " Results — research only ",
                4 => " Import quality report ",
                7 => " Clinical context / review history ",
                _ => " Help ",
            };
            frame.render_widget(
                paragraph
                    .scroll((self.scroll, 0))
                    .block(theme::panel(title, theme::BLUE)),
                body,
            );
        }
        frame.render_widget(
            Paragraph::new(format!(
                "{}{}",
                if self.dirty_reviews {
                    "UNSAVED REVIEWS · "
                } else {
                    ""
                },
                workflows::display_text(&self.status)
            ))
            .wrap(Wrap { trim: false })
            .style(Style::default().fg(if self.error {
                theme::ROSE
            } else {
                theme::AMBER
            })),
            status,
        );
        frame.render_widget(Paragraph::new("o Open  b Batch  [/] Case  d Demo  c Jev  s Save  q Quit\n6/v Visuals  7 Guidance  8 Review  ←/→ charts  ? Help").style(Style::default().fg(theme::MUTED)), footer);
        if self.dialog != Dialog::None {
            let popup = Rect {
                x: area.x + 2,
                y: area.y + area.height.saturating_sub(10) / 2,
                width: area.width.saturating_sub(4),
                height: 10.min(area.height),
            };
            let (title, text) = match self.dialog {
                Dialog::Review => (" Record review (local, not saved yet) ",format!("reviewer | YYYY-MM-DD | action | reason\nActions: acknowledged, deferred, not_applicable, departed\n{}\n\nEnter: record   Esc: cancel",workflows::display_text(&self.input))),
                Dialog::DiscardReviews => (" Unsaved review records ","Quit and discard unsaved reviews?\ny: discard and quit    n / Esc: return\nUse 8 then s to save the complete case to a new file.".into()),
                Dialog::ConfirmLive => (
                    " Send to Jev? ",
                    format!(
                        "Send {} findings from {} to api.typesafe.ai?\nA live API call may incur charges.\n\ny: send once    n / Esc: cancel",
                        self.case.findings.len(),
                        self.case.case_id
                    ),
                ),
                Dialog::Open => (
                    " Open case JSON ",
                    format!(
                        "Enter a file path (no shell expansion):\n{}\n\nEnter: load    Esc: cancel",
                        workflows::display_text(&self.input)
                    ),
                ),
                Dialog::OpenBatch => (
                    " Open import bundle ",
                    format!(
                        "Enter the import output directory:\n{}\n\nEnter: verify and open    Esc: cancel",
                        workflows::display_text(&self.input)
                    ),
                ),
                _ => (
                    " Save JSON to a new file ",
                    format!(
                        "Enter an output path (existing files are protected):\n{}\n\nEnter: save    Esc: cancel",
                        workflows::display_text(&self.input)
                    ),
                ),
            };
            frame.render_widget(Clear, popup);
            frame.render_widget(
                Paragraph::new(text).wrap(Wrap { trim: false }).block(
                    Block::bordered()
                        .title(title)
                        .border_style(Style::default().fg(theme::CYAN))
                        .style(Style::default().fg(theme::TEXT).bg(theme::PANEL)),
                ),
                popup,
            );
        }
    }
}

impl Drop for App {
    fn drop(&mut self) {
        if let Some(task) = &self.pending {
            task.abort();
        }
    }
}

struct RestoreTerminal;
impl Drop for RestoreTerminal {
    fn drop(&mut self) {
        ratatui::restore();
    }
}

pub async fn run(path: Option<PathBuf>, batch: Option<PathBuf>) -> Result<(), AppError> {
    if !std::io::stdin().is_terminal() || !std::io::stdout().is_terminal() {
        return Err(ErrorEnvelope::new(ErrorCode::TerminalRequired).into());
    }
    let mut app = match batch {
        Some(root) => App::from_batch(root)?,
        None => App::new(path)?,
    };
    let _restore = RestoreTerminal;
    let mut terminal = ratatui::try_init()?;
    loop {
        app.poll_result().await;
        terminal.draw(|frame| app.draw(frame))?;
        if event::poll(Duration::from_millis(80))?
            && let Event::Key(key) = event::read()?
            && app.key(key)
        {
            break;
        }
        tokio::task::yield_now().await;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn score_chart_uses_related_colors_and_keeps_zero_and_full_scale_labels() {
        use ratatui::{Terminal, backend::TestBackend};
        let mut app = App::new(None).unwrap();
        app.result = Some(workflows::demo(&app.case).unwrap());
        app.select_tab(5);
        let mut terminal = Terminal::new(TestBackend::new(120, 40)).unwrap();
        terminal.draw(|f| app.draw(f)).unwrap();
        for color in theme::SERIES {
            assert!(
                terminal
                    .backend()
                    .buffer()
                    .content
                    .iter()
                    .any(|cell| cell.fg == color)
            );
        }
        let result = app.result.as_mut().unwrap();
        for (index, ranking) in result.rankings.iter_mut().enumerate() {
            ranking.raw_probability = if index == 0 { 1.0 } else { 0.0 };
        }
        terminal.draw(|f| app.draw(f)).unwrap();
        let text: String = terminal
            .backend()
            .buffer()
            .content
            .iter()
            .map(|cell| cell.symbol())
            .collect();
        assert!(text.contains("100.0%"));
        assert!(text.contains("0.0%"));
        assert!(text.contains("insufficient_evidence"));
        assert!(text.contains("other_origin"));
    }

    #[test]
    fn canceling_live_confirmation_starts_no_task() {
        let mut app = App::new(None).unwrap();
        app.dialog = Dialog::ConfirmLive;
        app.key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
        assert_eq!(app.dialog, Dialog::None);
        assert!(app.pending.is_none());
        assert!(app.status.contains("nothing sent"));
    }

    #[tokio::test]
    async fn pending_request_blocks_case_changes_and_duplicate_runs() {
        let mut app = App::new(None).unwrap();
        app.pending = Some(tokio::spawn(std::future::pending()));
        for action in ['o', 'b', '[', ']', 'r', 'd', 'c', 'a'] {
            app.key(KeyEvent::new(KeyCode::Char(action), KeyModifiers::NONE));
            assert!(app.pending.is_some());
            assert_eq!(app.dialog, Dialog::None);
            assert!(app.result.is_none());
            assert!(app.status.contains("in progress"));
        }
    }
}

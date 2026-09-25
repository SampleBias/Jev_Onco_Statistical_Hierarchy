//! Single-sample workbench; cohort research is a secondary workspace.
use crate::{
    editor::{Editor, Focus as FieldFocus, Form},
    file_browser::{Browser, Outcome},
    molecular_tui,
    session::{Input, Page, Session},
    study_tui, tui, ui, workbench,
    workflows::{self, AppError},
};
use crossterm::event::{
    self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MouseButton, MouseEvent,
    MouseEventKind,
};
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::Style,
    widgets::{Block, Clear, Paragraph, Wrap},
};
use std::{
    io::{IsTerminal, Read},
    path::{Path, PathBuf},
    time::Duration,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Section {
    Analysis,
    Data,
    Clinical,
    Cohort,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Menu {
    Main,
    Workspace,
    Details,
    History,
    Samples,
    ImportSamples,
    EarlierRuns,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Control {
    Open,
    TrySample,
    Menu(Menu),
    Workbench,
    Studies,
    Page(Page),
    Run,
    Explain,
    Export,
    Cancel,
    Help,
    Quit,
    Demo,
    Request,
    PasteExpression,
    ExportData,
    ExportRequest,
    Reference,
    Compare,
    PrepareExpression,
    Detail(usize),
    Sample(usize),
    ImportSample(usize),
    Restore(usize),
    EarlierRun(usize),
    Content,
    Key(KeyCode),
    CancelImport,
}
struct Action {
    id: Control,
    label: String,
    enabled: bool,
    reason: &'static str,
}
impl Action {
    fn new(id: Control, label: impl Into<String>) -> Self {
        Self {
            id,
            label: label.into(),
            enabled: true,
            reason: "",
        }
    }
    fn when(mut self, enabled: bool, reason: &'static str) -> Self {
        self.enabled = enabled;
        self.reason = reason;
        self
    }
}
#[derive(Clone, Copy)]
enum Confirm {
    Demo,
    Expression,
}

pub struct App {
    session: Session,
    /// Import errors/cancellation must never replace the active sample.
    draft: Option<Session>,
    discard_import: bool,
    history: Vec<Session>,
    cohort: study_tui::App,
    studies: bool,
    guide: crate::guide::Guide,
    browser: Option<Browser>,
    browser_reference: bool,
    directory: PathBuf,
    recent: Vec<PathBuf>,
    status: String,
    navigation: Vec<(Rect, Control)>,
    focus: Option<Control>,
    menu: Option<(Menu, usize)>,
    menu_hits: Vec<(Rect, usize)>,
    form: Form,
    confirm: Option<Confirm>,
    inspection: Option<(String, u16)>,
    quitting: bool,
}
impl App {
    pub fn new(input: Option<PathBuf>) -> Result<Self, AppError> {
        let mut app = Self {
            session: Session::default(),
            draft: None,
            discard_import: false,
            history: vec![],
            cohort: study_tui::App::default(),
            studies: false,
            guide: crate::guide::Guide::default(),
            browser: None,
            browser_reference: false,
            directory: std::env::current_dir()?,
            recent: vec![],
            status: String::new(),
            navigation: vec![],
            focus: Some(Control::Open),
            menu: None,
            menu_hits: vec![],
            form: Form::default(),
            confirm: None,
            inspection: None,
            quitting: false,
        };
        if let Some(path) = input {
            app.open(&path)?;
        }
        Ok(app)
    }
    fn current(&self) -> &Session {
        self.draft.as_ref().unwrap_or(&self.session)
    }
    fn current_mut(&mut self) -> &mut Session {
        self.draft.as_mut().unwrap_or(&mut self.session)
    }
    pub fn section(&self) -> Section {
        if self.studies {
            Section::Cohort
        } else {
            match self.current().input {
                Input::Expression { .. } => Section::Data,
                Input::Legacy(_) => Section::Clinical,
                _ => Section::Analysis,
            }
        }
    }
    pub fn busy(&self) -> bool {
        self.session.busy() || self.draft.as_ref().is_some_and(Session::busy) || self.cohort.busy()
    }
    fn commit(&mut self, next: Session) {
        let old = std::mem::replace(&mut self.session, next);
        if old.ready() {
            self.history.push(old);
        }
        self.draft = None;
        self.studies = false;
        self.form.clear();
        self.focus = Some(Control::Page(self.session.page));
        self.status = "Sample loaded locally. Data and Results refer to this sample only.".into();
    }
    fn finish_import(&mut self) {
        if self.discard_import && self.draft.as_ref().is_none_or(|s| !s.busy()) {
            self.draft = None;
            self.discard_import = false;
            self.form.clear();
            self.status = "Import cancelled; previous sample preserved.".into();
            return;
        }
        if self
            .draft
            .as_ref()
            .is_some_and(|s| s.ready() && !s.busy() && !s.editing())
        {
            let next = self.draft.take().expect("ready import");
            self.commit(next);
        }
    }
    fn remember(&mut self, path: &Path) {
        let path = std::fs::canonicalize(path).unwrap_or_else(|_| path.into());
        if let Some(parent) = path.parent() {
            self.directory = parent.into();
        }
        self.recent.retain(|p| p != &path);
        self.recent.insert(0, path);
        self.recent.truncate(12);
    }
    fn browse(&mut self, samples: bool, reference: bool) {
        if self.busy() {
            self.status = "Wait for the current job or cancel it before opening data.".into();
            return;
        }
        self.menu = None;
        self.browser_reference = reference;
        self.browser = Some(Browser::new(
            self.directory.clone(),
            self.recent.clone(),
            samples,
            reference,
        ));
    }
    fn finish_browse(&mut self, outcome: Outcome) {
        let result = match outcome {
            Outcome::None => return,
            Outcome::Close => {
                if let Some(b) = self.browser.take() {
                    self.directory = b.directory().into();
                }
                return;
            }
            Outcome::Sample(i) => (|| {
                let mut a = molecular_tui::App::new(None, None)?;
                a.open_sample(i)?;
                self.commit(Session::molecular(a));
                Ok(())
            })(),
            Outcome::Open(path) => {
                if self.browser_reference {
                    if let Some(data) = self.current_mut().data_mut() {
                        data.open_reference(path.clone());
                        self.remember(&path);
                        Ok(())
                    } else {
                        Err(AppError::from(
                            "Open expression data before selecting its comparison reference.",
                        ))
                    }
                } else {
                    self.open(&path)
                }
            }
        };
        match result {
            Ok(()) => {
                self.browser = None;
                self.form.clear();
            }
            Err(e) => {
                if let Some(b) = &mut self.browser {
                    b.status = e.to_string();
                }
            }
        }
    }
    pub fn open(&mut self, path: &Path) -> Result<(), AppError> {
        if self.busy() {
            return Err("Wait for the current job before loading new data.".into());
        }
        self.status.clear();
        if path == Path::new("-") {
            return Err("Choose a file or folder; stdin is reserved for keyboard input.".into());
        }
        if path.is_dir() && path.join("dataset.json").is_file() {
            self.open_expression_dataset(path)?;
        } else if path.is_dir() && path.join("manifest.json").is_file() {
            self.commit(Session::new(Input::Legacy(Box::new(tui::App::from_batch(
                path.into(),
            )?))));
        } else if path.is_dir() {
            self.commit(Session::molecular(molecular_tui::App::new(
                Some(path.into()),
                None,
            )?));
        } else if let Some(format) = crate::analysis::table_format(path) {
            let molecular = match format {
                josh_ingest::molecular::Format::Maf | josh_ingest::molecular::Format::Vcf => true,
                _ => {
                    let mut reader = csv::ReaderBuilder::new()
                        .delimiter(if matches!(format, josh_ingest::molecular::Format::Csv) {
                            b','
                        } else {
                            b'\t'
                        })
                        .from_reader(std::fs::File::open(path)?.take(64 * 1024));
                    let headers = reader.headers().map_err(
                        |_| "Cannot read table headers. Use a CSV or TSV with a header row.",
                    )?;
                    if headers.iter().any(|v| v == "case_id") {
                        return Err("Legacy clinical table: import with josh import, then open its case bundle. Molecular tables use sample_id and modality columns.".into());
                    }
                    headers.iter().any(|v| v == "modality")
                }
            };
            let next = if molecular {
                let mut a = molecular_tui::App::new(None, None)?;
                a.open_path(path)?;
                Session::molecular(a)
            } else {
                let mut data = workbench::Workbench::empty()?;
                data.import_path(path);
                Session::expression(data)
            };
            self.draft = Some(next);
            self.studies = false;
            self.status = "Review import settings. Cancel keeps the previous sample.".into();
        } else {
            let value: serde_json::Value = workflows::read_json(path, 64 * 1024 * 1024)?;
            if value.get("sends_to_provider") == Some(&serde_json::Value::Bool(false))
                && value.get("request").is_some()
            {
                return Err("This is a request preview, not sample data. Open the input JSON or saved run folder.".into());
            }
            if matches!(
                value.get("kind").and_then(|v| v.as_str()),
                Some("josh_cohort_study" | "josh_cohort_report")
            ) {
                self.cohort.open(path);
                self.studies = true;
            } else if value.get("pipeline_version").and_then(|v| v.as_str())
                == Some(josh_core::reference::REFERENCE_PIPELINE)
                && value.get("release_id").is_some()
            {
                self.current_mut().data_mut().ok_or("This is a comparison reference, not a sample. Open expression data first, then Load reference.")?.open_reference(path.into());
            } else if value.get("case_id").is_some() {
                self.commit(Session::new(Input::Legacy(Box::new(tui::App::new(Some(
                    path.into(),
                ))?))));
            } else if path.file_name().is_some_and(|s| s == "dataset.json") {
                self.open_expression_dataset(path.parent().unwrap_or(Path::new(".")))?;
            } else if value.get("cases").is_some()
                && path.file_name().is_some_and(|s| s == "manifest.json")
            {
                self.commit(Session::new(Input::Legacy(Box::new(tui::App::from_batch(
                    path.parent().unwrap_or(Path::new(".")).into(),
                )?))));
            } else {
                self.commit(Session::molecular(molecular_tui::App::new(
                    Some(path.into()),
                    None,
                )?));
            }
        }
        self.remember(path);
        Ok(())
    }
    fn open_expression_dataset(&mut self, path: &Path) -> Result<(), AppError> {
        let mut data = workbench::Workbench::empty()?;
        data.open_dataset(path.into());
        self.draft = Some(Session::expression(data));
        self.studies = false;
        Ok(())
    }
    pub async fn poll(&mut self) {
        self.session.poll().await;
        if let Some(draft) = &mut self.draft {
            draft.poll().await;
        }
        self.cohort.poll().await;
        self.finish_import();
    }
    pub fn exit_ready(&self) -> bool {
        self.quitting && !self.busy()
    }
    fn cancel(&mut self) {
        self.session.cancel();
        if let Some(draft) = &mut self.draft {
            draft.cancel();
        }
        self.cohort.cancel_job();
        self.status = "Cancellation requested. Already-sent calls cannot be unsent; completed calls remain saved.".into();
    }
    fn request_quit(&mut self) {
        self.browser = None;
        self.menu = None;
        self.confirm = None;
        self.form.clear();
        self.quitting = true;
        self.cancel();
    }
    fn primary(&self) -> Action {
        if self.busy() {
            return Action::new(Control::Cancel, "Cancel job");
        }
        if self.studies {
            return if self.cohort.loaded.is_some() {
                Action::new(Control::Export, "Export study")
            } else {
                Action::new(Control::Demo, "Try study demo")
            };
        }
        let s = self.current();
        if s.legacy().is_some() {
            return Action::new(Control::Export, "Export record");
        }
        if let Some(a) = s.analysis().filter(|a| a.has_input()) {
            return if a.has_result() && a.can_explain() && !a.has_explanation() {
                Action::new(Control::Explain, "Explain result")
            } else if a.has_result() {
                Action::new(Control::Export, "Export report")
            } else {
                Action::new(Control::Run, "Run analysis")
            };
        }
        if let Some(data) = s.data().filter(|d| d.has_input()) {
            return if !data.has_reference() {
                Action::new(Control::Reference, "Load reference")
            } else if !data.has_comparison() {
                Action::new(Control::Compare, "Compare locally")
            } else {
                Action::new(Control::PrepareExpression, "Prepare analysis")
            };
        }
        Action::new(Control::Run, "Run analysis").when(false, "Open data or try a sample first.")
    }
    fn actions(&self) -> Vec<Action> {
        if self.draft.is_some() && !self.current().ready() {
            return vec![
                Action::new(Control::CancelImport, "Cancel import"),
                Action::new(Control::Open, "Open another…")
                    .when(!self.busy(), "Wait for the local job."),
                Action::new(Control::Menu(Menu::Main), "Menu"),
            ];
        }
        if !self.studies && !self.current().ready() {
            return vec![
                Action::new(Control::Open, "Open data…"),
                Action::new(Control::TrySample, "Try a sample"),
                Action::new(Control::Menu(Menu::Main), "Menu"),
            ];
        }
        let mut actions = if self.studies {
            vec![Action::new(
                Control::Menu(Menu::Details),
                format!("{} ▾", self.cohort.views()[self.cohort.page].1),
            )]
        } else {
            vec![
                Action::new(Control::Page(Page::Data), "Data"),
                Action::new(Control::Page(Page::Results), "Results"),
            ]
        };
        actions.extend([
            Action::new(Control::Open, "Open…")
                .when(!self.busy(), "Wait for or cancel the current job."),
            self.primary(),
            Action::new(Control::Menu(Menu::Main), "Menu"),
        ]);
        actions
    }
    fn menu_actions(&self, menu: Menu) -> Vec<Action> {
        let idle = !self.busy();
        let s = self.current();
        match menu {
            Menu::Workspace => vec![
                Action::new(Control::Workbench, "Single-sample workbench"),
                Action::new(Control::Studies, "Cohort studies (secondary workspace)"),
            ],
            Menu::History => self
                .history
                .iter()
                .enumerate()
                .rev()
                .map(|(i, s)| {
                    Action::new(Control::Restore(i), s.context())
                        .when(idle, "Wait for the current job.")
                })
                .collect(),
            Menu::EarlierRuns => s
                .previous_runs
                .iter()
                .enumerate()
                .rev()
                .map(|(i, a)| {
                    Action::new(
                        Control::EarlierRun(i),
                        format!("Earlier input · {}", a.context()),
                    )
                    .when(idle, "Wait for the current job.")
                })
                .collect(),
            Menu::Samples => s
                .data()
                .map(|d| {
                    d.samples()
                        .into_iter()
                        .enumerate()
                        .map(|(i, name)| {
                            Action::new(Control::Sample(i), name)
                                .when(idle, "Wait for the current job.")
                        })
                        .collect()
                })
                .unwrap_or_default(),
            Menu::ImportSamples => s
                .analysis()
                .map(|a| {
                    a.import_samples()
                        .into_iter()
                        .enumerate()
                        .map(|(i, id)| Action::new(Control::ImportSample(i), id))
                        .collect()
                })
                .unwrap_or_default(),
            Menu::Details => {
                let views = if self.studies {
                    self.cohort.views()
                } else if s.legacy().is_some() {
                    vec![
                        (0, "Evidence"),
                        (1, "Request (legacy contract)"),
                        (5, "Visualizations"),
                        (6, "Guidance (read-only legacy)"),
                        (7, "Saved review history"),
                        (4, "Import quality"),
                    ]
                } else if s.page == Page::Data {
                    s.data().map(|d| d.views()).unwrap_or_default()
                } else {
                    vec![
                        (0, "Summary"),
                        (4, "Paired ring + scatter"),
                        (1, "Feature ring"),
                        (2, "Feature scatter"),
                        (3, "Waterfall"),
                    ]
                };
                views
                    .into_iter()
                    .map(|(i, label)| Action::new(Control::Detail(i), label))
                    .collect()
            }
            Menu::Main => {
                let exportable = if self.studies {
                    self.cohort.loaded.is_some()
                } else {
                    s.analysis().is_some_and(|a| a.has_result())
                        || s.legacy().is_some()
                        || s.data().is_some_and(|d| d.has_input())
                };
                let mut actions = vec![
                    Action::new(Control::Export, "Export…").when(
                        idle && exportable,
                        "Load a result or dataset first; wait for the current job.",
                    ),
                    Action::new(Control::TrySample, "Try a synthetic sample…")
                        .when(idle, "Wait for the current job."),
                    Action::new(Control::PasteExpression, "Paste an expression table…")
                        .when(idle, "Wait for the current job."),
                    Action::new(
                        Control::Menu(Menu::History),
                        format!("Previous samples ({})…", self.history.len()),
                    )
                    .when(
                        !self.history.is_empty() && idle,
                        "No previous samples in this session, or a job is running.",
                    ),
                ];
                if !self.studies {
                    if s.analysis().is_some_and(|a| a.has_input()) {
                        actions.extend([
                            Action::new(Control::Run,"Run a new analysis…").when(idle,"Wait for the current job."),
                            Action::new(Control::Explain,"Generate explanation…").when(idle && s.analysis().is_some_and(|a| a.can_explain()),"A saved Jev inference is required; explanations use additional calls."),
                            Action::new(Control::Request,"Inspect exact Jev request (local)"),
                        ]);
                        if s.analysis().is_some_and(|a| a.has_explanation()) {
                            actions.push(
                                Action::new(
                                    Control::Key(KeyCode::Char('t')),
                                    "Change explained target (cached; no calls)",
                                )
                                .when(idle, "Wait for the current job."),
                            );
                        }
                    }
                    if let Some(data) = s.data() {
                        actions.push(
                            Action::new(
                                Control::ExportRequest,
                                "Export expression request preview (local)…",
                            )
                            .when(
                                idle && data.has_comparison(),
                                "Compare with a compatible reference first.",
                            ),
                        );
                        actions.extend([
                            Action::new(Control::Menu(Menu::Samples), "Select sample in dataset…")
                                .when(
                                    idle && data.has_input(),
                                    "Import a dataset first; wait for any job.",
                                ),
                            Action::new(Control::Reference, "Load comparison reference…")
                                .when(idle, "Wait for the current job."),
                            Action::new(Control::Compare, "Compare expression locally").when(
                                idle && data.has_reference(),
                                "Load a compatible reference first.",
                            ),
                            Action::new(
                                Control::PrepareExpression,
                                "Prepare expression-only analysis…",
                            )
                            .when(idle && data.has_comparison(), "Compare this sample first."),
                            Action::new(Control::Key(KeyCode::Char('/')), "Find gene…"),
                            Action::new(
                                Control::ExportData,
                                "Export expression manifest / comparison…",
                            )
                            .when(
                                idle && data.has_input(),
                                "Import data and wait for the current job.",
                            ),
                        ]);
                    }
                    if !s.previous_runs.is_empty() {
                        actions.push(Action::new(
                            Control::Menu(Menu::EarlierRuns),
                            "Earlier runs (previous input)…",
                        ));
                    }
                    if s.legacy().is_some() {
                        actions.push(Action::new(
                            Control::Menu(Menu::Details),
                            "Legacy records and visualizations…",
                        ));
                        actions.push(Action::new(
                            Control::Key(KeyCode::Left),
                            "Previous legacy visualization",
                        ));
                        actions.push(Action::new(
                            Control::Key(KeyCode::Right),
                            "Next legacy visualization",
                        ));
                        if s.legacy()
                            .is_some_and(|a| a.views().iter().any(|(i, _)| *i == 4))
                        {
                            actions.push(Action::new(
                                Control::Key(KeyCode::Char('[')),
                                "Previous case in legacy bundle",
                            ));
                            actions.push(Action::new(
                                Control::Key(KeyCode::Char(']')),
                                "Next case in legacy bundle",
                            ));
                        }
                    }
                    actions.push(
                        Action::new(
                            Control::Demo,
                            "Offline chart demonstration (invented scores)",
                        )
                        .when(idle, "Wait for the current job."),
                    );
                } else {
                    for (key, label) in [
                        ('n', "Confusion normalization"),
                        ('b', "Detailed / broad groups"),
                        ('i', "Survival intervals"),
                        ('w', "Supplied-propensity weighting"),
                        ('[', "Previous survival curve"),
                        (']', "Next survival curve"),
                    ] {
                        actions.push(
                            Action::new(Control::Key(KeyCode::Char(key)), label)
                                .when(self.cohort.loaded.is_some(), "Open a study first."),
                        );
                    }
                    actions.push(
                        Action::new(Control::Demo, "Offline cohort demonstration")
                            .when(idle, "Wait for the current job."),
                    );
                }
                if self.busy() {
                    actions.push(Action::new(
                        Control::Cancel,
                        "Cancel / finish current job safely",
                    ));
                }
                actions.extend([
                    Action::new(Control::Help, "Help, references and limitations"),
                    Action::new(Control::Quit, "Quit"),
                ]);
                actions
            }
        }
    }
    fn activate(&mut self, control: Control) {
        let disabled = self
            .actions()
            .into_iter()
            .chain(self.menu_actions(Menu::Main))
            .find(|a| a.id == control && !a.enabled);
        if let Some(a) = disabled {
            self.status = a.reason.into();
            return;
        }
        self.status.clear();
        match control {
            Control::Open => self.browse(false, false),
            Control::TrySample => self.browse(true, false),
            Control::Menu(menu) => {
                self.menu = Some((menu, 0));
                self.menu_hits.clear();
            }
            Control::Workbench => {
                self.studies = false;
                self.focus = Some(Control::Menu(Menu::Workspace));
            }
            Control::Studies => {
                self.studies = true;
                self.focus = Some(Control::Menu(Menu::Workspace));
            }
            Control::Page(page) => self.current_mut().set_page(page),
            Control::Run => {
                if let Some(a) = self.current_mut().analysis_mut() {
                    a.key(key('a'));
                }
            }
            Control::Explain => {
                if let Some(a) = self.current_mut().analysis_mut() {
                    a.key(key('e'));
                }
            }
            Control::Export => {
                if self.studies {
                    self.cohort.key(key('s'));
                } else if self.current().legacy().is_some() {
                    let a = self.current_mut().legacy_mut().expect("legacy");
                    a.set_view(7);
                    a.key(key('s'));
                } else if self.current().analysis().is_some_and(|a| a.has_result()) {
                    self.current_mut()
                        .analysis_mut()
                        .expect("analysis")
                        .key(key('s'));
                } else if let Some(d) = self.current_mut().data_mut() {
                    d.key(key('s'));
                }
            }
            Control::Cancel => self.cancel(),
            Control::CancelImport => {
                self.discard_import = true;
                if self.busy() {
                    self.cancel();
                } else {
                    self.finish_import();
                }
            }
            Control::Help => {
                self.guide
                    .key(KeyEvent::new(KeyCode::F(1), KeyModifiers::NONE), false);
            }
            Control::Quit => self.request_quit(),
            Control::Demo => {
                if self.studies {
                    self.cohort.key(key('d'));
                } else if self.current().ready() {
                    self.confirm = Some(Confirm::Demo);
                    self.form.clear();
                } else {
                    self.start_demo();
                }
            }
            Control::Request => {
                if let Some(a) = self.current().analysis() {
                    match a.request_preview() {
                        Ok(text) => self.inspection = Some((text, 0)),
                        Err(e) => self.status = e.to_string(),
                    }
                }
            }
            Control::PasteExpression => match workbench::Workbench::empty() {
                Ok(mut data) => {
                    data.key(key('p'));
                    self.draft = Some(Session::expression(data));
                    self.studies = false;
                    self.form.clear();
                }
                Err(e) => self.status = e.to_string(),
            },
            Control::ExportData => {
                self.current_mut().set_page(Page::Data);
                if let Some(data) = self.current_mut().data_mut() {
                    data.key(key('s'));
                }
            }
            Control::ExportRequest => {
                self.current_mut().set_page(Page::Data);
                if let Some(data) = self.current_mut().data_mut() {
                    data.key(key('e'));
                }
            }
            Control::Reference => self.browse(false, true),
            Control::Compare => {
                if let Some(d) = self.current_mut().data_mut() {
                    d.key(key('a'));
                }
            }
            Control::PrepareExpression => {
                self.confirm = Some(Confirm::Expression);
                self.form.clear();
            }
            Control::Detail(i) => {
                if self.studies {
                    self.cohort.select_view(i);
                } else if let Some(a) = self.current_mut().legacy_mut() {
                    a.set_view(i);
                } else if self.current().page == Page::Data {
                    if let Some(d) = self.current_mut().data_mut() {
                        d.select_view(i);
                    }
                } else {
                    self.current_mut().chart = i;
                }
            }
            Control::Sample(i) => {
                if let Some(d) = self.current_mut().data_mut() {
                    d.choose_sample(i);
                }
                self.current_mut().reconcile();
            }
            Control::ImportSample(i) => {
                if let Some(a) = self.current_mut().analysis_mut() {
                    a.choose_import_sample(i);
                }
            }
            Control::Restore(i) if !self.busy() && i < self.history.len() => {
                let next = self.history.remove(i);
                self.commit(next);
            }
            Control::EarlierRun(i) if !self.busy() && i < self.current().previous_runs.len() => {
                let a = self.current_mut().previous_runs.remove(i);
                self.commit(Session::molecular(a));
                self.status="Opened an earlier run with ITS original input, not the current expression sample.".into();
            }
            Control::Key(k) => {
                if k == KeyCode::Char('t') {
                    self.current_mut().set_page(Page::Results);
                }
                if k == KeyCode::Char('/') {
                    self.current_mut().set_page(Page::Data);
                }
                if matches!(k, KeyCode::Left | KeyCode::Right)
                    && let Some(a) = self.current_mut().legacy_mut()
                {
                    a.set_view(5);
                }
                self.inner_key(KeyEvent::new(k, KeyModifiers::NONE));
            }
            _ => (),
        }
    }
    fn start_demo(&mut self) {
        match molecular_tui::App::new(None, None) {
            Ok(mut a) => {
                a.key(key('d'));
                self.commit(Session::molecular(a));
                self.status.clear();
            }
            Err(e) => self.status = e.to_string(),
        }
    }
    fn editor(&self) -> Option<Editor> {
        if let Some(confirm) = self.confirm {
            let (title, notice, label) = match confirm {
                Confirm::Demo => (
                    "Open an offline demonstration?",
                    "Invented scores, not a prediction. The current sample remains in Menu > Previous samples. No provider calls.",
                    "Open chart demo",
                ),
                Confirm::Expression => (
                    "Use this expression comparison?",
                    "Prepare an expression-only analysis of the selected sample using the comparison's frozen taxonomy. No genomic or clinical scores are merged. This step is local; Run analysis asks separately before sending data.",
                    "Prepare profile",
                ),
            };
            let mut e = Editor::new(title, notice, label, &self.status);
            e.submit = key('y');
            return Some(e);
        }
        if self.studies {
            return self.cohort.editor();
        }
        let s = self.current();
        s.analysis()
            .and_then(|a| a.editor())
            .or_else(|| s.data().and_then(|d| d.editor()))
            .or_else(|| s.legacy().and_then(|a| a.editor()))
    }
    fn focus_field(&mut self, i: usize) {
        if self.current().analysis().is_some_and(|a| a.editing()) {
            self.current_mut()
                .analysis_mut()
                .expect("analysis")
                .focus_editor(i);
        } else if let Some(d) = self.current_mut().data_mut() {
            d.focus_editor(i);
        }
    }
    fn inner_key(&mut self, event: KeyEvent) {
        self.status.clear();
        if self.busy()
            && self.editor().is_none()
            && !matches!(
                event.code,
                KeyCode::Up
                    | KeyCode::Down
                    | KeyCode::PageUp
                    | KeyCode::PageDown
                    | KeyCode::Home
                    | KeyCode::End
                    | KeyCode::Left
                    | KeyCode::Right
            )
        {
            self.status = "Wait for the current job before changing evidence.".into();
            return;
        }
        if self.studies {
            self.cohort.key(event);
            return;
        }
        let s = self.current_mut();
        s.notice.clear();
        if s.analysis().is_some_and(|a| a.editing()) {
            s.analysis_mut().expect("analysis").key(event);
        } else if s.data().is_some_and(|d| d.editing()) {
            s.data_mut().expect("data").key(event);
        } else if let Some(a) = s.legacy_mut() {
            // Legacy records are inspectable/exportable, not a second classifier.
            if a.editing()
                || matches!(
                    event.code,
                    KeyCode::Up
                        | KeyCode::Down
                        | KeyCode::PageUp
                        | KeyCode::PageDown
                        | KeyCode::Home
                        | KeyCode::End
                        | KeyCode::Left
                        | KeyCode::Right
                        | KeyCode::Char('[' | ']')
                )
            {
                a.key(event);
            }
        } else if s.page == Page::Data && s.data().is_some() {
            s.data_mut().expect("data").key(event);
        } else if let Some(a) = s.analysis_mut() {
            a.key(event);
        }
        s.reconcile();
        self.finish_import();
    }
    fn submit_editor_key(&mut self, event: KeyEvent) {
        if event.code == KeyCode::F(6) && self.confirm.is_none() {
            self.menu = Some((Menu::ImportSamples, 0));
            return;
        }
        if let Some(confirm) = self.confirm {
            match event.code {
                KeyCode::Char('y' | 'Y') => {
                    self.confirm = None;
                    self.form.clear();
                    match confirm {
                        Confirm::Demo => self.start_demo(),
                        Confirm::Expression => {
                            if let Err(e) = self.current_mut().use_expression() {
                                self.status = e.to_string();
                            }
                        }
                    }
                }
                KeyCode::Esc | KeyCode::Char('n' | 'N') => {
                    self.confirm = None;
                    self.form.clear();
                }
                _ => (),
            }
        } else {
            self.inner_key(event);
            if event.code == KeyCode::Esc && self.draft.is_some() && !self.busy() {
                self.draft = None;
                self.form.clear();
                self.status = "Import cancelled; previous sample preserved.".into();
            }
        }
    }
    fn editor_key(&mut self, event: KeyEvent, editor: &Editor) {
        self.form.sync(editor);
        if let Some(FieldFocus::Field(i)) = self.form.focus {
            self.focus_field(i);
        }
        match event.code {
            KeyCode::Enter if event.modifiers.contains(KeyModifiers::CONTROL) => {
                self.submit_editor_key(editor.submit)
            }
            KeyCode::Tab | KeyCode::BackTab => self.form.cycle(
                editor,
                event.code == KeyCode::BackTab || event.modifiers.contains(KeyModifiers::SHIFT),
            ),
            KeyCode::Up | KeyCode::Down
                if matches!(self.form.focus, Some(FieldFocus::Field(_))) =>
            {
                self.form.cycle(editor, event.code == KeyCode::Up)
            }
            KeyCode::Enter => {
                if let Some(k) = self.form.activation(editor) {
                    self.submit_editor_key(k);
                } else if editor.fields.len() == 1 {
                    self.submit_editor_key(editor.submit);
                } else {
                    self.form.cycle(editor, false);
                }
            }
            KeyCode::Char(' ') if !matches!(self.form.focus, Some(FieldFocus::Field(_))) => {
                if let Some(k) = self.form.activation(editor) {
                    self.submit_editor_key(k);
                }
            }
            KeyCode::Esc | KeyCode::F(6) => self.submit_editor_key(event),
            _ if editor.fields.is_empty()
                || matches!(self.form.focus, Some(FieldFocus::Field(_))) =>
            {
                self.submit_editor_key(event)
            }
            _ => (),
        }
        if let Some(FieldFocus::Field(i)) = self.form.focus {
            self.focus_field(i);
        }
    }
    fn menu_key(&mut self, event: KeyEvent) {
        let Some((menu, index)) = self.menu else {
            return;
        };
        let actions = self.menu_actions(menu);
        let count = actions.len();
        if event.code == KeyCode::Esc {
            self.menu = None;
            return;
        }
        if count == 0 {
            return;
        }
        let i = index.min(count - 1);
        match event.code {
            KeyCode::Down | KeyCode::Tab => self.menu = Some((menu, (i + 1) % count)),
            KeyCode::Up | KeyCode::BackTab => self.menu = Some((menu, (i + count - 1) % count)),
            KeyCode::Home => self.menu = Some((menu, 0)),
            KeyCode::End => self.menu = Some((menu, count - 1)),
            KeyCode::Enter | KeyCode::Char(' ') => {
                if actions[i].enabled {
                    let id = actions[i].id;
                    self.menu = None;
                    self.activate(id);
                } else {
                    self.status = actions[i].reason.into();
                }
            }
            _ => (),
        }
    }
    pub fn key(&mut self, event: KeyEvent) -> bool {
        if event.kind != KeyEventKind::Press {
            return false;
        }
        if self.quitting {
            return self.exit_ready();
        }
        if self.guide.key(
            event,
            self.browser.is_some()
                || self.menu.is_some()
                || self.editor().is_some()
                || self.inspection.is_some(),
        ) {
            return false;
        }
        if event.modifiers.contains(KeyModifiers::CONTROL) && event.code == KeyCode::Char('c') {
            self.request_quit();
            return self.exit_ready();
        }
        if let Some((_, scroll)) = &mut self.inspection {
            match event.code {
                KeyCode::Esc => self.inspection = None,
                KeyCode::Down => *scroll = scroll.saturating_add(1),
                KeyCode::Up => *scroll = scroll.saturating_sub(1),
                KeyCode::PageDown => *scroll = scroll.saturating_add(10),
                KeyCode::PageUp => *scroll = scroll.saturating_sub(10),
                _ => (),
            };
            return false;
        }
        if let Some(b) = &mut self.browser {
            let result = b.key(event);
            self.finish_browse(result);
            return false;
        }
        if self.menu.is_some() {
            self.menu_key(event);
            return false;
        }
        if let Some(editor) = self.editor() {
            self.editor_key(event, &editor);
            return self.exit_ready();
        }
        match event.code{
            KeyCode::F(2)=>self.activate(Control::Workbench),KeyCode::F(5)=>self.activate(Control::Studies),
            KeyCode::F(3)=>{self.studies=false;self.current_mut().set_page(Page::Data);}
            KeyCode::F(4)=>self.status="Clinical evidence belongs to its sample. Open a legacy case to inspect records; no separate classifier is started.".into(),
            KeyCode::Tab|KeyCode::BackTab=>{
                let controls:Vec<_>=self.navigation.iter().map(|(_,id)|*id).collect();
                if !controls.is_empty(){let reverse=event.code==KeyCode::BackTab||event.modifiers.contains(KeyModifiers::SHIFT);let next=match self.focus.and_then(|f|controls.iter().position(|id|*id==f)){Some(i)=>(i+if reverse{controls.len()-1}else{1})%controls.len(),None=>if reverse{controls.len()-1}else{0}};self.focus=Some(controls[next]);}
            }
            KeyCode::Enter|KeyCode::Char(' ')=>{if let Some(f)=self.focus{self.activate(f);}}
            KeyCode::Char('q')=>self.request_quit(),KeyCode::Char('l'|'o')=>self.activate(Control::Open),
            KeyCode::Char('b') if event.modifiers.contains(KeyModifiers::CONTROL)=>self.activate(Control::TrySample),
            KeyCode::Char('m')=>self.activate(Control::Menu(Menu::Main)),KeyCode::Char('?')=>self.activate(Control::Help),
            KeyCode::Char('r') if !self.studies && self.current().data().is_some()=>self.activate(Control::Reference),
            KeyCode::Char('a'|'c') if !self.studies=>self.activate(self.primary().id),
            KeyCode::Char('s')=>self.activate(Control::Export),KeyCode::Char('e') if !self.studies=>self.activate(Control::Explain),
            KeyCode::Char('d')=>self.activate(Control::Demo),KeyCode::Char('x')=>self.activate(Control::Cancel),
            KeyCode::Char('v') if !self.studies=>{self.activate(Control::Page(Page::Results));self.current_mut().chart=0;self.focus=Some(Control::Content);}
            KeyCode::Char('p') if !self.studies=>{self.current_mut().set_page(Page::Results);self.current_mut().chart=4;self.focus=Some(Control::Content);}
            KeyCode::Char('1') if !self.studies=>self.activate(Control::Page(Page::Data)),KeyCode::Char('2') if !self.studies=>self.activate(Control::Page(Page::Results)),
            KeyCode::Left|KeyCode::Right if matches!(self.focus,Some(Control::Page(_)))=>{let next=if self.current().page==Page::Data{Page::Results}else{Page::Data};self.current_mut().set_page(next);self.focus=Some(Control::Page(next));}
            KeyCode::Left|KeyCode::Right if !self.studies && self.current().page==Page::Results && self.current().analysis().is_some()=>{let next=(self.current().chart+if event.code==KeyCode::Right{1}else{4})%5;self.current_mut().chart=next;self.current_mut().analysis_mut().expect("analysis").select_view(next);}
            KeyCode::Esc=>self.focus=None,_=>self.inner_key(event),
        }
        self.exit_ready()
    }
    pub fn paste(&mut self, text: &str) {
        if self.quitting
            || self.confirm.is_some()
            || self.inspection.is_some()
            || self.menu.is_some()
        {
            return;
        }
        if self.guide.open {
            self.guide.paste(text);
            return;
        }
        if let Some(b) = &mut self.browser {
            b.paste(text);
            return;
        }
        if let Some(editor) = self.editor() {
            self.form.sync(&editor);
            if let Some(FieldFocus::Field(i)) = self.form.focus {
                self.focus_field(i);
            } else {
                return;
            }
        } else {
            return;
        }
        if self.studies {
            self.cohort.paste(text);
            return;
        }
        let s = self.current_mut();
        if s.analysis().is_some_and(|a| a.editing()) {
            s.analysis_mut().expect("analysis").paste(text);
        } else if let Some(d) = s.data_mut() {
            d.paste(text.to_string());
        } else if let Some(a) = s.legacy_mut() {
            a.paste(text);
        }
    }
    pub fn mouse(&mut self, event: MouseEvent) {
        if self.quitting {
            return;
        }
        if self.guide.open {
            self.guide.mouse(event);
            return;
        }
        if let Some(b) = &mut self.browser {
            let result = b.mouse(event);
            self.finish_browse(result);
            return;
        }
        if self.inspection.is_some() {
            if matches!(
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
            return;
        }
        if let Some((menu, _)) = self.menu {
            if event.kind == MouseEventKind::Down(MouseButton::Left) {
                if let Some((_, i)) = self
                    .menu_hits
                    .iter()
                    .find(|(r, _)| r.contains((event.column, event.row).into()))
                {
                    let actions = self.menu_actions(menu);
                    if let Some(a) = actions.get(*i) {
                        if a.enabled {
                            let id = a.id;
                            self.menu = None;
                            self.activate(id);
                        } else {
                            self.status = a.reason.into();
                        }
                    }
                } else {
                    self.menu = None;
                }
            } else if matches!(
                event.kind,
                MouseEventKind::ScrollDown | MouseEventKind::ScrollUp
            ) {
                self.menu_key(KeyEvent::new(
                    if event.kind == MouseEventKind::ScrollDown {
                        KeyCode::Down
                    } else {
                        KeyCode::Up
                    },
                    KeyModifiers::NONE,
                ));
            }
            return;
        }
        if let Some(editor) = self.editor() {
            if event.kind == MouseEventKind::Down(MouseButton::Left)
                && let Some((_, focus)) = self
                    .form
                    .hits
                    .iter()
                    .find(|(r, _)| r.contains((event.column, event.row).into()))
            {
                let focus = *focus;
                self.form.focus = Some(focus);
                if let FieldFocus::Field(i) = focus {
                    self.focus_field(i);
                } else if let Some(k) = self.form.activation(&editor) {
                    self.submit_editor_key(k);
                }
            }
            return;
        }
        if event.kind == MouseEventKind::Down(MouseButton::Left)
            && let Some((_, control)) = self
                .navigation
                .iter()
                .find(|(r, _)| r.contains((event.column, event.row).into()))
        {
            let control = *control;
            self.focus = Some(control);
            self.activate(control);
            return;
        }
        if matches!(
            event.kind,
            MouseEventKind::ScrollDown | MouseEventKind::ScrollUp
        ) {
            self.inner_key(KeyEvent::new(
                if event.kind == MouseEventKind::ScrollDown {
                    KeyCode::Down
                } else {
                    KeyCode::Up
                },
                KeyModifiers::NONE,
            ));
        }
    }
    fn button(&mut self, frame: &mut Frame, area: Rect, action: Action) {
        let selected = matches!(action.id,Control::Page(p) if p==self.current().page);
        ui::button(
            frame,
            area,
            &workflows::display_text(&action.label),
            self.focus == Some(action.id),
            selected,
            action.enabled,
        );
        if area.width > 0 && area.height > 0 {
            self.navigation.push((area, action.id));
        }
    }
    fn draw_menu(&mut self, frame: &mut Frame) {
        let Some((menu, selected)) = self.menu else {
            return;
        };
        let actions = self.menu_actions(menu);
        let area = ui::popup(
            frame.area(),
            76,
            (actions.len() + 4).min(u16::MAX as usize) as u16,
        );
        ui::shadow(frame, area);
        frame.render_widget(Clear, area);
        let title = match menu {
            Menu::Main => " Menu ",
            Menu::Workspace => " Workspace ",
            Menu::Details => " Choose view ",
            Menu::History => " Previous samples · session-local ",
            Menu::Samples | Menu::ImportSamples => " Choose sample ",
            Menu::EarlierRuns => " Earlier input versions ",
        };
        let block = ui::panel(title);
        let inner = block.inner(area);
        frame.render_widget(block, area);
        self.menu_hits.clear();
        let count = inner.height.saturating_sub(1) as usize;
        let start = selected.saturating_sub(count.saturating_sub(1));
        for (i, a) in actions.iter().enumerate().skip(start).take(count) {
            let row = Rect::new(inner.x, inner.y + (i - start) as u16, inner.width, 1);
            ui::button(
                frame,
                row,
                &workflows::display_text(&a.label),
                selected == i,
                false,
                a.enabled,
            );
            self.menu_hits.push((row, i));
        }
        if inner.height > 0 {
            frame.render_widget(
                Paragraph::new("↑/↓ or Tab · Enter / click · Esc closes")
                    .style(Style::default().fg(ui::MUTED)),
                Rect::new(inner.x, inner.bottom() - 1, inner.width, 1),
            );
        }
    }
    pub fn draw(&mut self, frame: &mut Frame) {
        frame.render_widget(
            Block::default().style(Style::default().bg(ui::BG).fg(ui::TEXT)),
            frame.area(),
        );
        self.navigation.clear();
        let [header, toolbar, context, body, status, hint] = Layout::vertical([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(2),
            Constraint::Min(1),
            Constraint::Length(2),
            Constraint::Length(1),
        ])
        .areas(frame.area());
        let [workspace, brand] =
            Layout::horizontal([Constraint::Length(18), Constraint::Min(0)]).areas(header);
        self.button(
            frame,
            workspace,
            Action::new(
                Control::Menu(Menu::Workspace),
                if self.studies {
                    "Cohort studies ▾"
                } else {
                    "Workbench ▾"
                },
            ),
        );
        let credential = if workflows::key_configured() {
            "Key set · unverified"
        } else {
            "No key · local use"
        };
        frame.render_widget(
            Paragraph::new(format!("JOSH · {} · {credential}", josh_core::MODEL))
                .style(Style::default().fg(ui::ACCENT)),
            brand,
        );
        let actions = self.actions();
        if !actions.is_empty() {
            let rects = Layout::horizontal(vec![
                Constraint::Ratio(1, actions.len() as u32);
                actions.len()
            ])
            .split(toolbar);
            for (a, r) in actions.into_iter().zip(rects.iter()) {
                self.button(frame, *r, a);
            }
        }
        let label = if self.studies {
            self.cohort.context()
        } else {
            self.current().context()
        };
        frame.render_widget(
            Paragraph::new(workflows::display_text(&label))
                .wrap(Wrap { trim: false })
                .style(Style::default().fg(ui::BLUE)),
            context,
        );
        let mut content = body;
        if !self.studies
            && self.current().ready()
            && (self.current().page == Page::Results
                || self.current().data().is_some()
                || self.current().legacy().is_some())
        {
            let [selector, rest] =
                Layout::vertical([Constraint::Length(1), Constraint::Min(0)]).areas(body);
            content = rest;
            let text = if let Some(a) = self.current().legacy() {
                format!(
                    "Legacy view: {} ▾",
                    a.views()
                        .iter()
                        .find(|(i, _)| *i == a.view())
                        .map(|(_, l)| *l)
                        .unwrap_or("Evidence")
                )
            } else if self.current().page == Page::Data {
                format!(
                    "Expression: {} ▾",
                    self.current()
                        .data()
                        .map(|d| d.views()[d.view()].1)
                        .unwrap_or("Overview")
                )
            } else {
                format!(
                    "View: {} ▾",
                    [
                        "Summary",
                        "Feature ring",
                        "Feature scatter",
                        "Waterfall",
                        "Paired ring + scatter"
                    ][self.current().chart.min(4)]
                )
            };
            self.button(
                frame,
                Rect::new(
                    selector.x,
                    selector.y,
                    selector.width.min(40),
                    selector.height,
                ),
                Action::new(Control::Menu(Menu::Details), text),
            );
        }
        if self.studies {
            self.cohort.draw_workspace(frame, content, true);
        } else if let Some(a) = self.current_mut().legacy_mut() {
            a.draw_area(frame, content, true);
        } else if self.current().page == Page::Data {
            if let Some(d) = self.current_mut().data_mut() {
                d.draw_area(frame, content, true);
            } else if let Some(a) = self.current().analysis().filter(|a| a.has_input()) {
                a.draw_data(frame, content);
            } else {
                frame.render_widget(Paragraph::new("START WITH ONE SAMPLE\n\nOpen data…       Browse a file, dataset or saved run.\nTry a sample     Load invented evidence; no prediction yet.\n\nData    Review evidence, provenance and missing inputs.\nResults Inspect the prediction and its explanation.\n\nLoading is local. Jev requests need your confirmation.\nCohort research is available from Workbench ▾.\nF1 opens the guide and paper references.").wrap(Wrap{trim:false}).block(ui::panel(" Welcome ")),content);
            }
        } else {
            let chart = self.current().chart;
            if let Some(a) = self.current_mut().analysis_mut() {
                a.draw_results(frame, content, chart);
            } else {
                frame.render_widget(Paragraph::new("No result for this input.\n\nReview Data and its next action. Earlier results, if any, remain in Menu > Earlier runs.").wrap(Wrap{trim:false}).block(ui::panel(" Results ")),content);
            }
        }
        if content.width > 0 && content.height > 0 {
            self.navigation.push((content, Control::Content));
        }
        let message = if !self.status.is_empty() {
            self.status.clone()
        } else if self.studies {
            self.cohort.status()
        } else {
            self.current().status()
        };
        frame.render_widget(
            Paragraph::new(workflows::display_text(&message))
                .wrap(Wrap { trim: false })
                .style(Style::default().fg(ui::GOLD)),
            status,
        );
        frame.render_widget(
            Paragraph::new(if self.focus == Some(Control::Content) {
                "Content focused · arrows / wheel · Tab controls · m Menu · F1 Help"
            } else {
                "Tab focus · Enter select · o Open · m Menu · F1 Help · q Quit"
            })
            .style(
                Style::default().fg(if self.focus == Some(Control::Content) {
                    ui::ACCENT
                } else {
                    ui::MUTED
                }),
            ),
            hint,
        );
        if let Some(editor) = self.editor() {
            self.form.draw(frame, &editor);
        }
        self.draw_menu(frame);
        if let Some(b) = &mut self.browser {
            b.draw(frame);
        }
        if let Some((text, scroll)) = &self.inspection {
            let area = ui::popup(frame.area(), 110, 40);
            frame.render_widget(Clear, area);
            frame.render_widget(
                Paragraph::new(
                    text.lines()
                        .map(workflows::display_text)
                        .collect::<Vec<_>>()
                        .join("\n"),
                )
                .scroll((*scroll, 0))
                .wrap(Wrap { trim: false })
                .block(ui::panel(
                    " Exact Jev request · local only · ↑/↓ scroll · Esc closes ",
                )),
                area,
            );
        }
        self.guide.draw(frame);
    }
}
fn key(c: char) -> KeyEvent {
    KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE)
}

#[cfg(test)]
mod tests;
struct Restore;
impl Drop for Restore {
    fn drop(&mut self) {
        let _ = crossterm::execute!(
            std::io::stdout(),
            event::DisableBracketedPaste,
            event::DisableMouseCapture
        );
        ratatui::restore();
    }
}
pub async fn run(input: Option<PathBuf>) -> Result<(), AppError> {
    if !std::io::stdin().is_terminal() || !std::io::stdout().is_terminal() {
        return Err(josh_core::errors::ErrorEnvelope::new(
            josh_core::errors::ErrorCode::TerminalRequired,
        )
        .into());
    }
    let mut app = App::new(input)?;
    let _restore = Restore;
    let mut terminal = ratatui::try_init()?;
    crossterm::execute!(
        std::io::stdout(),
        event::EnableBracketedPaste,
        event::EnableMouseCapture
    )?;
    loop {
        app.poll().await;
        if app.exit_ready() {
            break;
        }
        terminal.draw(|f| app.draw(f))?;
        if event::poll(Duration::from_millis(40))? {
            match event::read()? {
                Event::Key(k) if app.key(k) => break,
                Event::Paste(t) => app.paste(&t),
                Event::Mouse(e) => app.mouse(e),
                _ => (),
            }
        }
    }
    Ok(())
}

//! Interactive molecular analysis with cancellable jobs and offline chart replay.
use crate::{
    molecular,
    workflows::{self, AppError},
};
use crossterm::event::{
    KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
};
use josh_core::{
    Source,
    molecular::{FeatureSet, InferenceRun, TaxonomyDefinition},
};
use josh_explain::{Archive, Config, Progress};
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::Style,
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
};
use std::{path::PathBuf, sync::atomic::Ordering, time::Instant};

#[derive(Clone, Copy)]
enum Dialog {
    Open,
    Import,
    Save,
    Live,
    Explain,
}
const IMPORT_LABELS: [&str; 6] = [
    "Sample ID",
    "Patient group ID",
    "Source ID",
    "Assay (fallback)",
    "Reference build (optional)",
    "Data class: synthetic / deidentified_research",
];
struct ImportForm {
    path: PathBuf,
    fields: [String; 6],
    active: usize,
}
type Job = tokio::task::JoinHandle<
    Result<(Option<Archive>, Option<InferenceRun>, Option<PathBuf>), String>,
>;
pub struct App {
    pub features: FeatureSet,
    pub archive: Option<Archive>,
    inference: Option<InferenceRun>,
    taxonomy: TaxonomyDefinition,
    run_root: Option<PathBuf>,
    selected: usize,
    page: usize,
    status: String,
    input: String,
    dialog: Option<Dialog>,
    pending: Option<Job>,
    progress: Progress,
    guide: crate::guide::Guide,
    started: Option<Instant>,
    evaluation_budget: usize,
    loaded: bool,
    import_form: Option<ImportForm>,
    buttons: Vec<(Rect, char)>,
    scroll: u16,
}
impl App {
    pub fn new(features: Option<PathBuf>, archive: Option<PathBuf>) -> Result<Self, AppError> {
        let loaded = archive
            .as_ref()
            .or(features.as_ref())
            .map(|p| crate::analysis::load(p))
            .transpose()?;
        let f = loaded
            .as_ref()
            .map(|l| l.features.clone())
            .unwrap_or_else(molecular::example);
        let inference = loaded.as_ref().and_then(|l| l.inference.clone());
        let taxonomy = inference
            .as_ref()
            .map(|r| r.taxonomy.clone())
            .unwrap_or_else(josh_core::molecular::onconpc_taxonomy);
        Ok(Self {
            features: f,
            archive: loaded.as_ref().and_then(|l| l.archive.clone()),
            inference,
            taxonomy,
            run_root: loaded.as_ref().and_then(|l| l.root.clone()),
            selected: 0,
            page: 0,
            status:
                "Load a molecular file or saved run. d tries the offline demo; g opens the guide."
                    .into(),
            input: String::new(),
            dialog: None,
            pending: None,
            progress: Progress::default(),
            guide: crate::guide::Guide::default(),
            started: None,
            evaluation_budget: 0,
            loaded: loaded.is_some(),
            import_form: None,
            buttons: Vec::new(),
            scroll: 0,
        })
    }
    pub fn busy(&self) -> bool {
        self.pending.is_some()
    }
    pub(crate) fn editing(&self) -> bool {
        self.dialog.is_some()
    }
    pub(crate) fn cancel_job(&self) {
        self.progress.cancelled.store(true, Ordering::Relaxed);
    }
    pub(crate) fn open_path(&mut self, path: &std::path::Path) -> Result<(), AppError> {
        if self.busy() {
            return Err("Wait for the current analysis before loading new data.".into());
        }
        self.input = path.display().to_string();
        self.submit(Dialog::Open)?;
        self.input.clear();
        Ok(())
    }
    pub(crate) fn load_button(&self, event: MouseEvent) -> bool {
        !self.editing()
            && event.kind == MouseEventKind::Down(MouseButton::Left)
            && self
                .buttons
                .iter()
                .any(|(r, key)| *key == 'l' && r.contains((event.column, event.row).into()))
    }
    pub fn paste(&mut self, text: &str) {
        if self.guide.open {
            self.guide.paste(text);
        } else if self.dialog.is_some_and(|d| !matches!(d, Dialog::Explain)) {
            let clean = text.trim().trim_matches('"').trim_matches('\'');
            let target = if matches!(self.dialog, Some(Dialog::Import)) {
                let form = self.import_form.as_mut().expect("import form");
                &mut form.fields[form.active]
            } else {
                &mut self.input
            };
            if !clean.chars().any(char::is_control) && target.len() + clean.len() <= 4096 {
                target.push_str(clean);
            } else {
                self.status = "Paste rejected: enter a single path of at most 4096 bytes.".into();
            }
        }
    }
    pub fn from_expression(
        e: &josh_core::reference::EvidencePackage,
        patient_group: &str,
    ) -> Result<Self, AppError> {
        let (features, taxonomy) = molecular::expression_features(e, patient_group)?;
        let mut app = Self::new(None, None)?;
        app.features = features;
        app.loaded = true;
        app.taxonomy = taxonomy;
        app.status="Expression comparison attached as one evidence group. c runs Jev; raw gene-level attribution is unavailable.".into();
        Ok(app)
    }
    pub async fn poll(&mut self) {
        if self.pending.as_ref().is_some_and(|h| h.is_finished()) {
            match self.pending.take().expect("pending").await {
                Ok(Ok((a, r, path))) => {
                    if let Some(a) = a {
                        self.features = a.features.clone();
                        self.taxonomy = a.inference.taxonomy.clone();
                        self.inference = Some(a.inference.clone());
                        self.archive = Some(a);
                    }
                    if let Some(r) = r {
                        self.inference = Some(r);
                        self.archive = None;
                    }
                    self.run_root = path;
                    self.selected = 0;
                    self.page = 0;
                    self.scroll = 0;
                    self.loaded = true;
                    self.status = self
                        .run_root
                        .as_ref()
                        .map(|p| {
                            format!(
                                "Analysis complete. Report saved: {} · s exports another copy",
                                p.join(if self.archive.is_some() {
                                    "explanation-report.md"
                                } else {
                                    "report.md"
                                })
                                .display()
                            )
                        })
                        .unwrap_or_else(|| {
                            "Offline demo complete. v Results · Tab Charts · s Export Markdown"
                                .into()
                        });
                }
                Ok(Err(e)) => {
                    self.status = e;
                    // Keep a completed inference usable even if explanation/report writing failed.
                    if let Some(root) = &self.run_root
                        && let Ok(loaded) = crate::analysis::load(root)
                        && let Ok(saved) = josh_core::molecular::hash(&loaded.features)
                        && let Ok(current) = josh_core::molecular::hash(&self.features)
                        && saved == current
                    {
                        self.inference = loaded.inference;
                        self.archive = loaded.archive;
                    }
                }
                Err(_) => {
                    self.status =
                        "Job cancelled or failed; saved checkpoints remain available.".into()
                }
            }
        }
    }
    fn start_demo(&mut self) {
        self.features = molecular::example();
        self.taxonomy = josh_core::molecular::onconpc_taxonomy();
        self.inference = None;
        self.archive = None;
        self.run_root = None;
        self.loaded = true;
        self.started = Some(Instant::now());
        self.evaluation_budget = 128;
        self.progress = Progress::default();
        let progress = self.progress.clone();
        self.pending = Some(tokio::spawn(async move {
            let task = async {
                let f = molecular::example();
                let t = josh_core::molecular::onconpc_taxonomy();
                let response = molecular::demo_response(&josh_core::molecular::prepare(&f, &t)?);
                let r = josh_core::molecular::interpret(&f, &t, response, Source::Mock)?;
                let mut a = Archive::new(
                    f,
                    r,
                    Config {
                        algorithm: josh_explain::Algorithm::Exact,
                        ..Config::default()
                    },
                    None,
                )?;
                josh_explain::run(
                    &mut a,
                    &mut molecular::AnalyticalDemo,
                    &progress,
                    |_| Ok(()),
                )
                .await?;
                Ok::<_, AppError>((Some(a), None, None))
            }
            .await;
            task.map_err(|_| "Analytical fixture failed numerical verification.".into())
        }));
        self.status = "Calculating an explicitly synthetic analytical demonstration…".into();
    }
    fn submit(&mut self, dialog: Dialog) -> Result<(), AppError> {
        if matches!(dialog, Dialog::Import) {
            let form = self.import_form.as_ref().ok_or("no import settings")?;
            let data_class = match form.fields[5].as_str() {
                "synthetic" => josh_core::DataClass::Synthetic,
                "deidentified_research" => josh_core::DataClass::DeidentifiedResearch,
                _ => return Err("data class must be synthetic or deidentified_research".into()),
            };
            let f = crate::analysis::import_table(
                &form.path,
                &josh_ingest::molecular::Options {
                    sample_id: form.fields[0].clone(),
                    patient_group_id: form.fields[1].clone(),
                    source_id: form.fields[2].clone(),
                    assay: form.fields[3].clone(),
                    reference_build: (!form.fields[4].is_empty()).then(|| form.fields[4].clone()),
                    data_class,
                },
            )?;
            let mut next = Self::new(None, None)?;
            next.features = f;
            next.loaded = true;
            next.status =
                "Data loaded. Review the observations and setup checks, then a Analyze.".into();
            *self = next;
            return Ok(());
        }
        let path = PathBuf::from(self.input.trim());
        if self.input.trim().is_empty() {
            return Err("enter a path".into());
        }
        match dialog {
            Dialog::Open => {
                if path.as_os_str() == "-" {
                    return Err("terminal input must be a file".into());
                }
                if crate::analysis::table_format(&path).is_some() {
                    if !path.is_file() {
                        return Err("input file does not exist".into());
                    }
                    self.import_form = Some(ImportForm {
                        path,
                        fields: Default::default(),
                        active: 0,
                    });
                    self.import_form.as_mut().expect("form").fields[5] =
                        "deidentified_research".into();
                    self.dialog = Some(Dialog::Import);
                    self.status =
                        "Complete the import settings. Only mark invented data synthetic.".into();
                } else {
                    *self = Self::new(Some(path), None)?;
                    self.status =
                        "Loaded and verified. a Analyze · v Results · s Export Markdown".into();
                }
            }
            Dialog::Save => {
                if matches!(
                    path.extension().and_then(|s| s.to_str()),
                    Some("md" | "markdown")
                ) {
                    let r = self
                        .inference
                        .as_ref()
                        .ok_or("analyze first; no result is available to report")?;
                    workflows::save_new(
                        &path,
                        &crate::report::markdown(&self.features, r, self.archive.as_ref())?,
                    )?;
                } else if let Some(a) = &self.archive {
                    let content = match path.extension().and_then(|s| s.to_str()) {
                        Some("svg") => crate::molecular_charts::svg(a)?,
                        Some("csv") => crate::molecular_charts::csv(a)?,
                        Some("json") => workflows::pretty(a)?,
                        _ => return Err("use .md, .svg, .csv or .json".into()),
                    };
                    workflows::save_new(&path, &content)?;
                } else if path.extension().is_some_and(|s| s == "json") {
                    molecular::write_new(
                        &path,
                        self.inference
                            .as_ref()
                            .ok_or("analyze first; no result to export")?,
                    )?;
                } else {
                    return Err("use .md for a report or .json for inference; charts require an explanation".into());
                }
                self.status = "Export saved to a new file.".into();
            }
            Dialog::Live => {
                if !self.loaded {
                    return Err("load data first or press d for the offline demo".into());
                }
                crate::analysis::check_destination(&path)?;
                josh_core::molecular::prepare(&self.features, &self.taxonomy)?;
                let _preflight = molecular::LiveEvaluator::new(self.features.data_class.clone())?;
                self.started = Some(Instant::now());
                self.evaluation_budget = 1;
                self.progress = Progress::default();
                let f = self.features.clone();
                let t = self.taxonomy.clone();
                self.run_root = Some(path.clone());
                self.pending = Some(tokio::spawn(async move {
                    let task = async {
                        let run = molecular::infer_and_save(&path, &f, &t).await?;
                        Ok::<_, AppError>((None, Some(run), Some(path)))
                    }
                    .await;
                    task.map_err(|e| {
                        format!(
                            "{}; inspect the run directory before retrying. No automatic retry.",
                            crate::errors::envelope(&e).error.message
                        )
                    })
                }));
                self.archive = None;
                self.inference = None;
                self.page = 0;
                self.status = "Jev request running; inputs remain fixed during this run.".into();
            }
            Dialog::Explain | Dialog::Import => {}
        }
        Ok(())
    }
    fn start_explanation(&mut self) -> Result<(), AppError> {
        let root = self
            .run_root
            .clone()
            .ok_or("run Jev first; no archived inference is loaded")?;
        let inference = self.inference.clone().ok_or("run Jev first")?;
        let target = josh_core::molecular::probabilities(&inference.response)?
            .iter()
            .max_by(|a, b| a.1.total_cmp(b.1))
            .ok_or("no scores")?
            .0
            .clone();
        let options = molecular::ExplainOptions {
            retry_uncertain: false,
            target,
            method: molecular::Method::Permutation,
            pairs: 16,
            seed: 42,
            max_evaluations: 512,
            max_input_tokens: 1_000_000,
            max_seconds: 600,
            background: None,
        };
        let archive = molecular::initialize_archive(&root, &options)?;
        if josh_core::molecular::hash(&archive.features)?
            != josh_core::molecular::hash(&self.features)?
            || josh_core::molecular::hash(&archive.inference)?
                != josh_core::molecular::hash(&inference)?
        {
            return Err(
                "the saved run changed on disk; reload it before starting an explanation".into(),
            );
        }
        if !archive.plan().fits_budget {
            return Err("this explanation exceeds the guided request budget; use molecular plan/explain to choose a suitable budget".into());
        }
        self.started = Some(Instant::now());
        self.evaluation_budget = options.max_evaluations;
        self.progress = Progress::default();
        let progress = self.progress.clone();
        self.pending = Some(tokio::spawn(async move {
            molecular::explain(&root, archive, &progress)
                .await
                .map(|a| (Some(a), None, Some(root)))
                .map_err(|e| e.to_string())
        }));
        self.status =
            "Computing masked-evidence Shapley. x cancels; completed calls are checkpointed."
                .into();
        Ok(())
    }
    pub fn key(&mut self, key: KeyEvent) -> bool {
        if key.kind != KeyEventKind::Press {
            return false;
        }
        if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
            self.progress.cancelled.store(true, Ordering::Relaxed);
            return true;
        }
        if self.guide.key(key, self.dialog.is_some()) {
            return false;
        }
        if matches!(self.dialog, Some(Dialog::Import)) {
            let form = self.import_form.as_mut().expect("import form");
            match key.code {
                KeyCode::Esc => {
                    self.dialog = None;
                    self.import_form = None;
                }
                KeyCode::Tab | KeyCode::Down => {
                    form.active = (form.active + 1) % IMPORT_LABELS.len()
                }
                KeyCode::BackTab | KeyCode::Up => {
                    form.active = (form.active + IMPORT_LABELS.len() - 1) % IMPORT_LABELS.len()
                }
                KeyCode::Backspace => {
                    form.fields[form.active].pop();
                }
                KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    form.fields[form.active].clear()
                }
                KeyCode::Enter => match self.submit(Dialog::Import) {
                    Ok(()) => self.dialog = None,
                    Err(e) => self.status = format!("Import: {e}. Edit settings and try again."),
                },
                KeyCode::Char(c)
                    if !c.is_control()
                        && !key.modifiers.contains(KeyModifiers::CONTROL)
                        && form.fields[form.active].len() < 4096 =>
                {
                    form.fields[form.active].push(c)
                }
                _ => (),
            }
            return false;
        }
        if let Some(dialog) = self.dialog {
            match key.code {
                KeyCode::Esc => {
                    self.dialog = None;
                    self.input.clear();
                }
                KeyCode::Enter => {
                    self.dialog = None;
                    let result = if matches!(dialog, Dialog::Explain) {
                        self.start_explanation()
                    } else {
                        self.submit(dialog)
                    };
                    if let Err(e) = result {
                        self.status = e.to_string();
                        self.dialog = Some(dialog);
                    } else {
                        self.input.clear();
                    }
                }
                KeyCode::Backspace => {
                    self.input.pop();
                }
                KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    self.input.clear()
                }
                KeyCode::Char(c) if !c.is_control() && self.input.len() < 4096 => {
                    self.input.push(c)
                }
                _ => (),
            }
            return false;
        }
        if matches!(key.code, KeyCode::Char('q') | KeyCode::Esc) {
            self.progress.cancelled.store(true, Ordering::Relaxed);
            return true;
        }
        if key.code == KeyCode::Char('x') {
            self.progress.cancelled.store(true, Ordering::Relaxed);
            self.status = if self.evaluation_budget == 1 && self.busy() {
                "One request is in flight; waiting for its result (20s timeout). A sent request cannot be unsent.".into()
            } else {
                "Cancellation requested. Completed explanation calls remain checkpointed.".into()
            };
            return false;
        }
        match key.code {
            KeyCode::Down | KeyCode::Char('j') => {
                self.scroll = self.scroll.saturating_add(1);
                self.selected = (self.selected + 1).min(
                    self.archive
                        .as_ref()
                        .and_then(|a| a.result.as_ref())
                        .map_or(0, |r| r.attributions.len().saturating_sub(1)),
                );
            }
            KeyCode::Up | KeyCode::Char('k') => {
                self.selected = self.selected.saturating_sub(1);
                self.scroll = self.scroll.saturating_sub(1);
            }
            KeyCode::PageDown => self.scroll = self.scroll.saturating_add(10),
            KeyCode::PageUp => self.scroll = self.scroll.saturating_sub(10),
            KeyCode::Home => self.scroll = 0,
            KeyCode::Tab | KeyCode::Right => {
                self.page = (self.page + 1) % 4;
                self.scroll = 0;
            }
            KeyCode::Left => {
                self.page = (self.page + 3) % 4;
                self.scroll = 0;
            }
            KeyCode::Char('v') => {
                self.page = 0;
                self.scroll = 0;
            }
            KeyCode::Char('t') if !self.busy() => {
                if let Some(a) = &mut self.archive {
                    let classes: Vec<_> = a.inference.taxonomy.criteria().keys().cloned().collect();
                    let next = (classes
                        .iter()
                        .position(|c| c == &a.config.target_class)
                        .unwrap_or(0)
                        + 1)
                        % classes.len();
                    match a.retarget(&classes[next]) {
                        Ok(()) => {
                            self.selected = 0;
                            self.status="Changed target class using cached distributions; no network request.".into();
                        }
                        Err(e) => self.status = e.to_string(),
                    }
                }
            }
            KeyCode::Char('d') if !self.busy() => self.start_demo(),
            KeyCode::Char('l' | 'o') if !self.busy() => {
                self.dialog = Some(Dialog::Open);
                self.input.clear();
            }
            KeyCode::Char('s') if !self.busy() => {
                if self.inference.is_none() {
                    self.status =
                        "Analyze first or load a saved run to export a Markdown report.".into();
                    return false;
                }
                self.dialog = Some(Dialog::Save);
                self.input = "report.md".into();
            }
            KeyCode::Char('a' | 'c') if !self.busy() => {
                if !self.loaded {
                    self.status = "Load data first (l), or try the offline demo (d).".into();
                    return false;
                }
                let check = crate::analysis::readiness(&self.features, &self.taxonomy);
                if !check.ready {
                    self.status = check.blockers.join(" ");
                    return false;
                }
                self.dialog = Some(Dialog::Live);
                self.input = crate::analysis::suggested_run_path().display().to_string();
            }
            KeyCode::Char('e') if !self.busy() => {
                if self
                    .inference
                    .as_ref()
                    .is_none_or(|r| r.source != Source::Jev)
                {
                    self.status = "Optional live explanations require a saved Jev inference. d includes offline demo charts.".into();
                    return false;
                }
                self.dialog = Some(Dialog::Explain);
                self.input.clear();
            }
            _ => (),
        }
        false
    }
    fn overview(&self) -> String {
        if !self.loaded {
            return "START HERE\n\n1. Load data: click Load or press l.\n   Molecular JSON, CSV/TSV, annotated MAF/VCF, or a saved run directory.\n2. Review the evidence and setup checks, then Analyze (a).\n3. Read ranked outcomes and evidence checks under Results (v).\n4. Export a clean Markdown report (s).\n\nTRY IT OFFLINE\nPress d for the bundled analytical demonstration. No key or network is needed.\n\nJEV SETUP\nLive analysis needs TYPESAFE_API_KEY in the launching environment and synthetic data.\nRun josh doctor --format text for local setup status.\n\nExpression/reference data: F3 in this workspace\nHelp and supported data formats: g opens the searchable guide.".into();
        }
        let mut lines = vec![];
        if let Some(r) = &self.inference {
            lines.push(format!(
                "{}\n{}",
                crate::report::source_label(r.source),
                if r.status == "abstained" {
                    "ABSTAINED — no origin assigned"
                } else {
                    "REVIEW REQUIRED — no autonomous diagnosis"
                }
            ));
            lines.push(
                "Raw scores are uncalibrated model outputs, not patient cancer probabilities.\n"
                    .into(),
            );
            for reason in &r.reasons {
                lines.push(format!("• {}", crate::report::reason(reason)));
            }
            lines.push("\nRANKED OUTCOMES                                      RAW SCORE".into());
            if let Ok(rankings) = crate::report::rankings(r) {
                for (i, (id, p)) in rankings.iter().enumerate() {
                    lines.push(format!(
                        "{:>2}. {:<48} {:>6.2}%",
                        i + 1,
                        crate::report::class_label(r, id),
                        p * 100.0
                    ));
                }
            }
            lines.push("\nEVIDENCE CHECKS (separate judgments)".into());
            for (id, label) in [
                ("evidence_sufficient", "Evidence sufficient"),
                ("conflicting_evidence", "Conflicting evidence"),
            ] {
                if let josh_core::Answer::Noul { noul } = r.response.answers[id] {
                    lines.push(format!("{label}: {:.2}%", noul * 100.0));
                }
            }
            lines.push(match self.archive.as_ref() {
                Some(a) if a.result.is_some() => "\nExplanation available: Tab switches charts; t changes target offline.".into(),
                Some(_) => "\nExplanation incomplete. Inference is available; resume with the CLI options recorded in the checkpoint.".into(),
                None => "\nExplanation not requested. e runs an optional budgeted explanation.".into(),
            });
            if let Some(root) = &self.run_root {
                lines.push(format!("Saved run: {}", root.display()));
            }
        } else {
            lines.push(crate::analysis::readiness(&self.features, &self.taxonomy).text());
        }
        lines.push(format!(
            "\nINPUT EVIDENCE · {} · {:?}",
            self.features.sample_id, self.features.data_class
        ));
        for f in &self.features.features {
            lines.push(format!(
                "{} · {} · {} · {}",
                f.name,
                f.modality.label(),
                crate::report::status_label(&f.status),
                f.value
                    .as_ref()
                    .map(|v| v.display())
                    .unwrap_or_else(|| "Unavailable".into())
            ));
        }
        lines
            .into_iter()
            .map(|l| {
                l.lines()
                    .map(workflows::display_text)
                    .collect::<Vec<_>>()
                    .join("\n")
            })
            .collect::<Vec<_>>()
            .join("\n")
    }
    pub fn mouse(&mut self, event: MouseEvent) {
        if self.dialog.is_some() || self.guide.open {
            return;
        }
        match event.kind {
            MouseEventKind::Down(MouseButton::Left) => {
                if let Some((_, key)) = self
                    .buttons
                    .iter()
                    .find(|(r, _)| r.contains((event.column, event.row).into()))
                {
                    self.key(KeyEvent::new(KeyCode::Char(*key), KeyModifiers::NONE));
                }
            }
            MouseEventKind::ScrollDown => self.scroll = self.scroll.saturating_add(3),
            MouseEventKind::ScrollUp => self.scroll = self.scroll.saturating_sub(3),
            _ => (),
        }
    }
    pub fn draw(&mut self, frame: &mut Frame) {
        self.draw_area(frame, frame.area(), false);
    }
    pub(crate) fn draw_area(&mut self, frame: &mut Frame, area: Rect, embedded: bool) {
        let [header, actions, body, footer] = Layout::vertical([
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Min(1),
            Constraint::Length(3),
        ])
        .areas(area);
        let subtitle = if !self.loaded {
            "Load data to begin, or press d for the offline demo.".into()
        } else {
            format!(
                "{} · {} observations · {}",
                self.features.sample_id,
                self.features.features.len(),
                self.inference
                    .as_ref()
                    .map(|r| crate::report::source_label(r.source))
                    .unwrap_or("Input loaded · no prediction")
            )
        };
        frame.render_widget(
            Paragraph::new(format!(
                "{}\n{}",
                if embedded {
                    "Analysis · Load → Analyze → Results → Markdown"
                } else {
                    "Jev Onco Statistical Hierarchy · Guided analysis"
                },
                workflows::display_text(&subtitle)
            ))
            .style(Style::default().fg(crate::ui::TEXT)),
            header,
        );
        self.buttons.clear();
        let areas = Layout::horizontal([Constraint::Ratio(1, 4); 4]).split(actions);
        for ((rect, label), key) in areas
            .iter()
            .zip([
                "1 Load [l]",
                "2 Analyze [a]",
                "3 Results [v]",
                "4 Export [s]",
            ])
            .zip(['l', 'a', 'v', 's'])
        {
            let disabled = self.busy()
                || (key == 'a' && !self.loaded)
                || (matches!(key, 's' | 'v') && self.inference.is_none());
            frame.render_widget(
                Paragraph::new(label)
                    .block(Block::default().borders(Borders::ALL))
                    .style(Style::default().fg(if disabled {
                        crate::ui::MUTED
                    } else {
                        crate::ui::ACCENT
                    })),
                *rect,
            );
            self.buttons.push((*rect, key));
        }
        if self.page > 0 && self.archive.as_ref().is_some_and(|a| a.result.is_some()) {
            crate::molecular_charts::draw(
                frame,
                body,
                self.archive.as_ref().expect("archive"),
                self.selected,
                self.page - 1,
            );
        } else {
            let content = self.overview();
            frame.render_widget(
                Paragraph::new(content)
                    .scroll((self.scroll, 0))
                    .wrap(Wrap { trim: false })
                    .block(
                        Block::default()
                            .borders(Borders::ALL)
                            .title(" Data / results · Up/Down scroll · Tab charts "),
                    ),
                body,
            );
        }
        let status = if self.busy() {
            format!(
                "{} · {}/{} evaluations · {}s elapsed · x Cancel",
                self.status,
                self.progress.completed.load(Ordering::Relaxed),
                self.evaluation_budget,
                self.started.map_or(0, |t| t.elapsed().as_secs())
            )
        } else {
            self.status.clone()
        };
        frame.render_widget(Paragraph::new(format!("{}\nl Load · a Analyze · v Results · s Markdown · d Demo · e Explain · g Guide · q Quit", workflows::display_text(&status))).wrap(Wrap {trim:false}).style(Style::default().fg(crate::ui::MUTED)),footer);
        if let Some(dialog) = self.dialog {
            let width = area.width.saturating_sub(2).min(100);
            let height = (if matches!(dialog, Dialog::Import) {
                20
            } else {
                13
            })
            .min(area.height);
            let rect = Rect::new(
                area.x + (area.width - width) / 2,
                area.y + (area.height - height) / 2,
                width,
                height,
            );
            let title = match dialog {
                Dialog::Open => "Load data file or saved run directory",
                Dialog::Import => "Import · Tab moves · Ctrl+u clears · Enter imports",
                Dialog::Save => "Export Markdown (.md); charts .svg/.csv or data .json",
                Dialog::Live => "Analyze with Jev · NEW run directory",
                Dialog::Explain => "Optional explanation · review request budget",
            };
            let notice = match dialog {
                Dialog::Live => {
                    "Enter sends ONE synthetic sample to api.typesafe.ai (may incur charges). Results and report.md are saved automatically to this new directory. Ctrl+u clears the path."
                }
                Dialog::Explain => {
                    "Enter starts up to 512 evaluations / 1,000,000 input tokens / 600 seconds. 16 paired permutations. Successful calls are checkpointed. Esc cancels this dialog."
                }
                Dialog::Import => {
                    "Import runs locally. Enter the exact sample ID in the table. Use synthetic only for invented data. Empty required fields must be completed."
                }
                Dialog::Open => {
                    "Molecular JSON, CSV/TSV, annotated MAF/VCF, or saved run folder. Enter loads; Esc cancels. Expression matrices: use Load or the Data section (F3)."
                }
                Dialog::Save => {
                    "Enter exports to a NEW file. .md includes results, evidence and provenance. Ctrl+u clears; Esc cancels. Existing files are protected."
                }
            };
            let content = if let Some(form) = self
                .import_form
                .as_ref()
                .filter(|_| matches!(dialog, Dialog::Import))
            {
                let rows = IMPORT_LABELS
                    .iter()
                    .enumerate()
                    .map(|(i, l)| {
                        format!(
                            "{} {}: {}",
                            if i == form.active { ">" } else { " " },
                            l,
                            workflows::display_text(&form.fields[i])
                        )
                    })
                    .collect::<Vec<_>>()
                    .join("\n");
                format!(
                    "{notice}\n\n{rows}\n\n{}",
                    workflows::display_text(&self.status)
                )
            } else {
                format!(
                    "{notice}\n\n{}\n\n{}",
                    workflows::display_text(&self.input),
                    workflows::display_text(&self.status)
                )
            };
            frame.render_widget(Clear, rect);
            frame.render_widget(
                Paragraph::new(content)
                    .wrap(Wrap { trim: false })
                    .block(Block::default().borders(Borders::ALL).title(title)),
                rect,
            );
        }
        self.guide.draw(frame);
    }
}
impl Drop for App {
    fn drop(&mut self) {
        self.progress.cancelled.store(true, Ordering::Relaxed);
        if let Some(job) = self.pending.take() {
            job.abort();
        }
    }
}

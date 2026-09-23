//! Interactive molecular analysis with cancellable jobs and offline chart replay.
use crate::{
    molecular,
    workflows::{self, AppError},
};
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use josh_core::{
    Source,
    molecular::{FeatureSet, InferenceRun, TaxonomyDefinition},
};
use josh_explain::{Archive, Config, Progress};
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Color, Style},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
};
use std::{
    io::IsTerminal,
    path::PathBuf,
    sync::atomic::Ordering,
    time::{Duration, Instant},
};

#[derive(Clone, Copy)]
enum Dialog {
    Open,
    Save,
    Live,
    Explain,
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
}
impl App {
    pub fn new(features: Option<PathBuf>, archive: Option<PathBuf>) -> Result<Self, AppError> {
        let a = archive
            .as_ref()
            .map(|p| molecular::load_archive(p))
            .transpose()?;
        let f = match (&a, features) {
            (Some(a), _) => a.features.clone(),
            (_, Some(p)) => molecular::load_features(&p)?,
            _ => molecular::example(),
        };
        let inference = a.as_ref().map(|a| a.inference.clone());
        let taxonomy = inference
            .as_ref()
            .map(|r| r.taxonomy.clone())
            .unwrap_or_else(josh_core::molecular::onconpc_taxonomy);
        Ok(Self {
            features: f,
            archive: a,
            inference,
            taxonomy,
            run_root: archive.and_then(|p| p.parent().map(|p| p.to_path_buf())),
            selected: 0,
            page: 0,
            status: "d Analytical chart demo · o Open features/archive · c Run Jev · g Guide"
                .into(),
            input: String::new(),
            dialog: None,
            pending: None,
            progress: Progress::default(),
            guide: crate::guide::Guide::default(),
            started: None,
            evaluation_budget: 0,
        })
    }
    pub fn busy(&self) -> bool {
        self.pending.is_some()
    }
    pub fn paste(&mut self, text: &str) {
        if self.guide.open {
            self.guide.paste(text);
        } else if self.dialog.is_some_and(|d| !matches!(d, Dialog::Explain)) {
            let clean = text.trim().trim_matches('"').trim_matches('\'');
            if !clean.chars().any(char::is_control) && self.input.len() + clean.len() <= 4096 {
                self.input.push_str(clean);
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
                    self.status="Complete. Arrows select evidence · Tab changes view · s exports · e explains a live run".into();
                }
                Ok(Err(e)) => self.status = e,
                Err(_) => {
                    self.status =
                        "Job cancelled or failed; saved checkpoints remain available.".into()
                }
            }
        }
    }
    fn start_demo(&mut self) {
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
        let path = PathBuf::from(self.input.trim());
        if self.input.trim().is_empty() {
            return Err("enter a path".into());
        }
        match dialog {
            Dialog::Open => {
                if path.as_os_str() == "-" {
                    return Err("terminal input must be a file".into());
                }
                let value: serde_json::Value = workflows::read_json(&path, 64 * 1024 * 1024)?;
                if value.get("inference").is_some() {
                    *self = Self::new(None, Some(path))?;
                } else {
                    *self = Self::new(Some(path), None)?;
                }
            }
            Dialog::Save => {
                if let Some(a) = &self.archive {
                    let content = match path.extension().and_then(|s| s.to_str()) {
                        Some("svg") => crate::molecular_charts::svg(a)?,
                        Some("csv") => crate::molecular_charts::csv(a)?,
                        _ => workflows::pretty(a)?,
                    };
                    workflows::save_new(&path, &content)?;
                } else {
                    molecular::write_new(&path, &self.features)?;
                }
                self.status = "Export saved to a new file.".into();
            }
            Dialog::Live => {
                if path.exists() {
                    return Err("choose a new run directory".into());
                }
                josh_core::molecular::prepare(&self.features, &self.taxonomy)?;
                let _preflight = molecular::LiveEvaluator::new(self.features.data_class.clone())?;
                self.started = Some(Instant::now());
                self.evaluation_budget = 1;
                self.progress = Progress::default();
                let f = self.features.clone();
                let t = self.taxonomy.clone();
                self.pending = Some(tokio::spawn(async move {
                    let task = async {
                        let run = molecular::infer_and_save(&path, &f, &t).await?;
                        Ok::<_, AppError>((None, Some(run), Some(path)))
                    }
                    .await;
                    task.map_err(|e| crate::errors::envelope(&e).error.message.to_string())
                }));
                self.archive = None;
                self.status = "Jev request running; inputs remain fixed during this run.".into();
            }
            Dialog::Explain => {}
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
        if let Some(dialog) = self.dialog {
            match key.code {
                KeyCode::Esc => {
                    self.dialog = None;
                    self.input.clear();
                }
                KeyCode::Enter => {
                    let result = if matches!(dialog, Dialog::Explain) {
                        self.start_explanation()
                    } else {
                        self.submit(dialog)
                    };
                    self.dialog = None;
                    self.input.clear();
                    if let Err(e) = result {
                        self.status = e.to_string();
                    }
                }
                KeyCode::Backspace => {
                    self.input.pop();
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
            self.status =
                "Cancellation requested. An in-flight provider call may already be billed.".into();
            return false;
        }
        match key.code {
            KeyCode::Down | KeyCode::Char('j') => {
                self.selected = (self.selected + 1).min(
                    self.archive
                        .as_ref()
                        .and_then(|a| a.result.as_ref())
                        .map_or(0, |r| r.attributions.len().saturating_sub(1)),
                );
            }
            KeyCode::Up | KeyCode::Char('k') => self.selected = self.selected.saturating_sub(1),
            KeyCode::Tab | KeyCode::Right => self.page = (self.page + 1) % 3,
            KeyCode::Left => self.page = (self.page + 2) % 3,
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
            KeyCode::Char('o') if !self.busy() => {
                self.dialog = Some(Dialog::Open);
                self.input.clear();
            }
            KeyCode::Char('s') if !self.busy() => {
                self.dialog = Some(Dialog::Save);
                self.input.clear();
            }
            KeyCode::Char('c') if !self.busy() => {
                self.dialog = Some(Dialog::Live);
                self.input.clear();
            }
            KeyCode::Char('e') if !self.busy() => {
                self.dialog = Some(Dialog::Explain);
                self.input.clear();
            }
            _ => (),
        }
        false
    }
    pub fn draw(&mut self, frame: &mut Frame) {
        let [header, body, footer] = Layout::vertical([
            Constraint::Length(4),
            Constraint::Min(1),
            Constraint::Length(3),
        ])
        .areas(frame.area());
        let badge = self
            .inference
            .as_ref()
            .map(|r| match r.source {
                Source::Mock => "ANALYTICAL DEMO · NO CANCER PREDICTION",
                Source::Replay => "UNVERIFIED REPLAY · UNCALIBRATED",
                Source::Jev => "JEV RESEARCH · UNCALIBRATED",
            })
            .unwrap_or("MOLECULAR INPUT · NO PREDICTION");
        let selected = self
            .archive
            .as_ref()
            .and_then(|a| a.result.as_ref())
            .map(|r| {
                format!(
                    "{} · raw score {:.4} · baseline {:.4}",
                    r.target_class, r.full_probability, r.baseline_probability
                )
            })
            .unwrap_or_else(|| {
                format!(
                    "{} · {} features · {} attribution groups",
                    self.features.sample_id,
                    self.features.features.len(),
                    self.features.groups().len()
                )
            });
        frame.render_widget(
            Paragraph::new(format!(
                "Jev Onco Statistical Hierarchy · Molecular explanations\n{badge}\n{selected}\n{}",
                self.inference
                    .as_ref()
                    .map(|r| format!(
                        "{} · {} · run {}",
                        self.features.sample_id,
                        r.status,
                        &r.request_sha256[..12]
                    ))
                    .unwrap_or_default()
            ))
            .style(Style::default().fg(Color::White)),
            header,
        );
        if let Some(a) = &self.archive {
            crate::molecular_charts::draw(frame, body, a, self.selected, self.page);
        } else {
            let rows = self
                .features
                .features
                .iter()
                .map(|f| {
                    format!(
                        "{:<18} {:<15} {}",
                        f.name,
                        f.modality.label(),
                        f.value
                            .as_ref()
                            .map(|v| v.display())
                            .unwrap_or_else(|| format!("{:?}", f.status))
                    )
                })
                .collect::<Vec<_>>()
                .join("\n");
            let inference = self
                .inference
                .as_ref()
                .map(|r| {
                    format!(
                        "\nJev result: {}\n{}",
                        r.status,
                        serde_json::to_string_pretty(&r.response.answers).unwrap_or_default()
                    )
                })
                .unwrap_or_default();
            frame.render_widget(Paragraph::new(format!("{rows}\n{inference}\n\nd Demo charts · c Classify · e Explain · o Open input or archive")).wrap(Wrap {trim:false}).block(Block::default().borders(Borders::ALL).title(" Molecular evidence ")),body);
        }
        let status = if self.busy() {
            format!(
                "{} · {}/{} completed · {}s elapsed · x Cancel",
                self.status,
                self.progress.completed.load(Ordering::Relaxed),
                self.evaluation_budget,
                self.started.map_or(0, |t| t.elapsed().as_secs())
            )
        } else {
            self.status.clone()
        };
        frame.render_widget(
            Paragraph::new(format!(
                "{status}\nArrows Select · Tab View · g Guide · s Save SVG/CSV/JSON · q Back/quit"
            ))
            .wrap(Wrap { trim: false })
            .style(Style::default().fg(Color::Gray)),
            footer,
        );
        if let Some(dialog) = self.dialog {
            let area = frame.area();
            let width = area.width.saturating_sub(4).min(90);
            let height = 10.min(area.height);
            let rect = Rect::new(
                area.x + (area.width - width) / 2,
                area.y + (area.height - height) / 2,
                width,
                height,
            );
            let title = match dialog {
                Dialog::Open => "Open feature set or explanation JSON",
                Dialog::Save => "Save NEW file (.svg, .csv, .json)",
                Dialog::Live => "Run Jev: enter a NEW run directory",
                Dialog::Explain => "Run masked-evidence explanation",
            };
            let notice = match dialog {
                Dialog::Live => {
                    "Enter sends one synthetic case to api.typesafe.ai and may incur charges. A new archive directory is required."
                }
                Dialog::Explain => {
                    "Enter starts up to 512 evaluations / 1,000,000 input tokens / 600 seconds. 16 paired permutations. Successful calls are saved; Esc cancels."
                }
                _ => "Enter accepts · Esc cancels",
            };
            frame.render_widget(Clear, rect);
            frame.render_widget(
                Paragraph::new(format!("{notice}\n\n{}", self.input))
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
struct Restore;
impl Drop for Restore {
    fn drop(&mut self) {
        let _ = crossterm::terminal::disable_raw_mode();
        let _ = crossterm::execute!(
            std::io::stdout(),
            event::DisableBracketedPaste,
            crossterm::terminal::LeaveAlternateScreen
        );
    }
}
pub async fn run(features: Option<PathBuf>, archive: Option<PathBuf>) -> Result<(), AppError> {
    if !std::io::stdin().is_terminal() || !std::io::stdout().is_terminal() {
        return Err("molecular TUI requires an interactive terminal".into());
    }
    let mut app = App::new(features, archive)?;
    crossterm::terminal::enable_raw_mode()?;
    let _restore = Restore;
    crossterm::execute!(
        std::io::stdout(),
        crossterm::terminal::EnterAlternateScreen,
        event::EnableBracketedPaste
    )?;
    let mut terminal =
        ratatui::Terminal::new(ratatui::backend::CrosstermBackend::new(std::io::stdout()))?;
    loop {
        app.poll().await;
        terminal.draw(|f| app.draw(f))?;
        if event::poll(Duration::from_millis(40))? {
            match event::read()? {
                Event::Key(key) if app.key(key) => break,
                Event::Paste(text) => app.paste(&text),
                _ => (),
            }
        }
    }
    Ok(())
}

//! Sample-first Ratatui workbench. All imports are local and no model call is hidden.
use crate::{
    data,
    workflows::{self, AppError},
};
use crossterm::{
    event::{
        self, DisableBracketedPaste, EnableBracketedPaste, Event, KeyCode, KeyEvent, KeyEventKind,
        KeyModifiers,
    },
    execute,
};
use josh_core::sample::*;
use josh_ingest::{dataset, expression};
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Color, Style},
    widgets::{Block, Borders, Cell, Clear, Paragraph, Row, Table, Tabs, Wrap},
};
use std::{
    io::IsTerminal,
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicU8, Ordering},
    },
    time::Duration,
};

const TABS: [&str; 7] = [
    "Samples",
    "Datasets",
    "Analyze",
    "Explore",
    "Models",
    "Reference",
    "Projects",
];
const LABELS: [&str; 14] = [
    "Source path",
    "New output directory",
    "Dataset ID",
    "Sample ID (two-column only)",
    "Units",
    "Layout",
    "Delimiter",
    "Gene column",
    "Value column",
    "Sample column",
    "HGNC TSV path",
    "HGNC release",
    "Platform",
    "Transform",
];
const MAX_PASTE: usize = 1024 * 1024;

struct Loaded {
    root: Option<PathBuf>,
    manifest: DatasetManifest,
    records: Vec<Vec<ExpressionRecord>>,
}

#[derive(Clone)]
struct ImportForm {
    values: Vec<String>,
    active: usize,
    pasted: Option<String>,
}
impl ImportForm {
    fn new(pasted: Option<String>) -> Self {
        Self {
            values: vec![
                String::new(),
                String::new(),
                "dataset-001".into(),
                String::new(),
                "unknown".into(),
                "auto".into(),
                "auto".into(),
                "gene".into(),
                "expression".into(),
                "sample_id".into(),
                String::new(),
                String::new(),
                String::new(),
                "identity".into(),
            ],
            active: 0,
            pasted,
        }
    }
    fn options(&self) -> Result<expression::ImportOptions, AppError> {
        let v = &self.values;
        let parsed = |value: &str| serde_json::Value::String(value.trim().replace('-', "_"));
        let config = ExpressionConfig {
            units: serde_json::from_value(parsed(&v[4])).map_err(|_| "Units: unknown, counts, tpm, fpkm, normalized, microarray_intensity, log2 or z_score")?,
            layout: serde_json::from_value(parsed(&v[5])).map_err(|_| "Layout: auto, long or wide")?,
            delimiter: serde_json::from_value(parsed(&v[6])).map_err(|_| "Delimiter: auto, csv or tsv")?,
            transform: serde_json::from_value(parsed(&v[13])).map_err(|_| "Transform: identity or log2_one_plus")?,
            gene_column: v[7].clone(), value_column: v[8].clone(), sample_column: v[9].clone(),
            single_sample_id: (!v[3].is_empty()).then(|| v[3].clone()),
            platform: (!v[12].is_empty()).then(|| v[12].clone()), ..ExpressionConfig::default()
        };
        Ok(expression::ImportOptions {
            dataset_id: v[2].clone(),
            source_name: if self.pasted.is_some() {
                "pasted-expression".into()
            } else {
                Path::new(&v[0])
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .into_owned()
            },
            config,
            ..Default::default()
        })
    }
}

enum Dialog {
    None,
    Open(String),
    Search(String),
    Save(String),
    Paste(String),
    Import(ImportForm),
}
enum JobResult {
    Dataset(Box<Loaded>),
    Preview { form: ImportForm, text: String },
}

struct Pending {
    handle: tokio::task::JoinHandle<Result<JobResult, String>>,
    state: Arc<AtomicU8>,
}

pub struct Workbench {
    loaded: Loaded,
    sample: usize,
    tab: usize,
    scroll: usize,
    query: String,
    visible: Vec<usize>,
    dialog: Dialog,
    status: String,
    pending: Option<Pending>,
    quit_after_job: bool,
}

fn demo() -> Result<Loaded, AppError> {
    let options = expression::ImportOptions {
        dataset_id: "synthetic-expression-demo".into(),
        source_name: "bundled synthetic expression".into(),
        data_class: josh_core::DataClass::Synthetic,
        config: ExpressionConfig {
            units: ExpressionUnit::Tpm,
            transform: Transform::Log2OnePlus,
            platform: Some("invented demonstration".into()),
            ..Default::default()
        },
        ..Default::default()
    };
    let data = expression::import(
        include_bytes!("../../../fixtures/expression/synthetic-expression.tsv").to_vec(),
        &options,
        Some((
            include_bytes!("../../../fixtures/expression/hgnc-subset.tsv").to_vec(),
            "hgnc-six-gene-fixture-2026-09-22".into(),
        )),
    )?;
    Ok(Loaded {
        root: None,
        manifest: data.manifest,
        records: data.records,
    })
}

fn load(root: PathBuf) -> Result<Loaded, AppError> {
    let manifest = dataset::read(&root)?;
    let mut records = Vec::new();
    let mut total = 0;
    for s in &manifest.samples {
        let data = dataset::read_records(&root, s)?;
        total += data.len();
        if total > expression::MAX_CELLS {
            return Err(dataset::DatasetError::Limit.into());
        }
        records.push(data);
    }
    Ok(Loaded {
        root: Some(root),
        manifest,
        records,
    })
}

impl Workbench {
    pub fn new() -> Result<Self, AppError> {
        let mut app = Self { loaded: demo()?, sample: 0, tab: 0, scroll: 0, query: String::new(), visible: vec![], dialog: Dialog::None,
            status: "Synthetic expression example. i Import file · p Paste data · o Open dataset · ? Help".into(), pending: None, quit_after_job: false };
        app.filter();
        Ok(app)
    }
    fn filter(&mut self) {
        let data = &self.loaded.records[self.sample];
        let query = self.query.to_ascii_lowercase();
        self.visible = data
            .iter()
            .enumerate()
            .filter(|(_, r)| {
                r.original_gene_id.to_ascii_lowercase().contains(&query)
                    || r.mapping.gene.as_ref().is_some_and(|g| {
                        g.symbol.to_ascii_lowercase().contains(&query)
                            || g.hgnc_id.to_ascii_lowercase().contains(&query)
                    })
            })
            .map(|(i, _)| i)
            .collect();
        self.visible.sort_by(|a, b| {
            data[*b]
                .raw_expression
                .unwrap_or(f64::NEG_INFINITY)
                .total_cmp(&data[*a].raw_expression.unwrap_or(f64::NEG_INFINITY))
                .then(data[*a].original_gene_id.cmp(&data[*b].original_gene_id))
        });
        self.scroll = 0;
    }
    fn open(&mut self, root: PathBuf) {
        let state = Arc::new(AtomicU8::new(0));
        let copy = state.clone();
        let handle = tokio::task::spawn_blocking(move || {
            let result = load(root).map_err(|e| e.to_string())?;
            if copy.load(Ordering::SeqCst) == 1 {
                return Err("Open canceled.".into());
            }
            Ok(JobResult::Dataset(Box::new(result)))
        });
        self.pending = Some(Pending { handle, state });
        self.status = "Verifying dataset artifacts… x cancels before replacing this view.".into();
    }
    fn import(&mut self, form: ImportForm) -> Result<(), AppError> {
        let options = form.options()?;
        if form.values[0] == "-" || form.values[10] == "-" {
            return Err("TUI input requires file paths; use p for pasted expression data.".into());
        }
        if form.values[1].is_empty() || (form.pasted.is_none() && form.values[0].is_empty()) {
            return Err("Set a source and new output directory.".into());
        }
        if form.values[10].is_empty() != form.values[11].is_empty() {
            return Err("Gene-map path and release must be supplied together.".into());
        }
        let state = Arc::new(AtomicU8::new(0));
        let worker_state = state.clone();
        let handle = tokio::task::spawn_blocking(move || {
            let result = (|| -> Result<Loaded, AppError> {
                let source = if let Some(pasted) = form.pasted {
                    pasted.into_bytes()
                } else {
                    data::input(Path::new(&form.values[0]))?
                };
                let mapping = if form.values[10].is_empty() {
                    None
                } else {
                    Some((
                        data::input(Path::new(&form.values[10]))?,
                        form.values[11].clone(),
                    ))
                };
                let imported = expression::import(source, &options, mapping)?;
                if worker_state
                    .compare_exchange(0, 2, Ordering::SeqCst, Ordering::SeqCst)
                    .is_err()
                {
                    return Err("Import canceled before writing a bundle.".into());
                }
                let root = PathBuf::from(&form.values[1]);
                dataset::write(&root, &imported)?;
                Ok(Loaded {
                    root: Some(root),
                    manifest: imported.manifest,
                    records: imported.records,
                })
            })();
            result
                .map(|loaded| JobResult::Dataset(Box::new(loaded)))
                .map_err(|e| e.to_string())
        });
        self.pending = Some(Pending { handle, state });
        self.status = "Importing locally… x cancels at the boundary before bundle writing.".into();
        Ok(())
    }
    fn preview(&mut self, form: ImportForm) {
        let state = Arc::new(AtomicU8::new(0));
        let copy = state.clone();
        let handle = tokio::task::spawn_blocking(move || {
            use expression::DatasetAdapter;
            let result = (|| -> Result<String, AppError> {
                let options = form.options()?;
                if form.pasted.is_none() && (form.values[0].is_empty() || form.values[0] == "-") {
                    return Err("Choose a file path or use the paste editor.".into());
                }
                let bytes = if let Some(pasted) = &form.pasted {
                    pasted.as_bytes().to_vec()
                } else {
                    data::input(Path::new(&form.values[0]))?
                };
                let detection =
                    expression::ExpressionTableAdapter.detect(&bytes, &options.config)?;
                Ok(format!(
                    "Detected {:?} / {:?}; {} columns, {} wide sample columns. Units: {:?}. Correct fields or Ctrl-S to import.",
                    detection.delimiter,
                    detection.layout,
                    detection.columns.len(),
                    detection.sample_columns.len(),
                    options.config.units
                ))
            })();
            if copy.load(Ordering::SeqCst) == 1 {
                return Err("Detection canceled.".into());
            }
            Ok(JobResult::Preview {
                form,
                text: result.unwrap_or_else(|e| e.to_string()),
            })
        });
        self.pending = Some(Pending { handle, state });
        self.status = "Detecting source structure locally…".into();
    }
    fn cancel(&mut self) {
        if let Some(job) = &self.pending {
            self.status = if job
                .state
                .compare_exchange(0, 1, Ordering::SeqCst, Ordering::SeqCst)
                .is_ok()
            {
                "Cancellation requested. Waiting for the current processing step; no bundle will be published.".into()
            } else {
                "Finishing the current job. A bundle write already in progress must complete."
                    .into()
            };
        }
    }
    pub async fn poll(&mut self) {
        if self
            .pending
            .as_ref()
            .is_some_and(|p| p.handle.is_finished())
        {
            let pending = self.pending.take().unwrap();
            match pending.handle.await {
                Ok(Ok(JobResult::Dataset(loaded))) => {
                    self.loaded = *loaded;
                    self.sample = 0;
                    self.query.clear();
                    self.filter();
                    self.status="Dataset loaded. Inspect QC before analysis; molecular inference is not implemented yet.".into();
                }
                Ok(Ok(JobResult::Preview { form, text })) => {
                    self.dialog = Dialog::Import(form);
                    self.status = text;
                }
                Ok(Err(message)) => self.status = message,
                Err(_) => self.status = "Dataset worker failed; current dataset preserved.".into(),
            }
        }
    }
    pub fn key(&mut self, key: KeyEvent) -> bool {
        if key.kind != KeyEventKind::Press {
            return false;
        }
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
            if self.pending.is_some() {
                self.quit_after_job = true;
                self.cancel();
                return false;
            }
            return true;
        }
        let mut submit = false;
        let mut preview = false;
        match &mut self.dialog {
            Dialog::Import(form) => match key.code {
                KeyCode::Esc => self.dialog = Dialog::None,
                KeyCode::Tab | KeyCode::Down => form.active = (form.active + 1) % LABELS.len(),
                KeyCode::BackTab | KeyCode::Up => {
                    form.active = (form.active + LABELS.len() - 1) % LABELS.len()
                }
                KeyCode::Backspace => {
                    form.values[form.active].pop();
                }
                KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    preview = true
                }
                KeyCode::Enter if key.modifiers.contains(KeyModifiers::CONTROL) => submit = true,
                KeyCode::Char('s') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    submit = true
                }
                KeyCode::Char(c)
                    if !c.is_control()
                        && !key.modifiers.contains(KeyModifiers::CONTROL)
                        && form.values[form.active].len() + c.len_utf8() <= 4096 =>
                {
                    form.values[form.active].push(c);
                }
                _ => {}
            },
            Dialog::None => {}
            dialog => {
                let text = match dialog {
                    Dialog::Open(s) | Dialog::Search(s) | Dialog::Save(s) | Dialog::Paste(s) => s,
                    _ => unreachable!(),
                };
                match key.code {
                    KeyCode::Esc => self.dialog = Dialog::None,
                    KeyCode::Enter => submit = true,
                    KeyCode::Char('s') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        submit = true
                    }
                    KeyCode::Backspace => {
                        text.pop();
                    }
                    KeyCode::Char(c)
                        if !c.is_control()
                            && !key.modifiers.contains(KeyModifiers::CONTROL)
                            && text.len() + c.len_utf8() <= MAX_PASTE =>
                    {
                        text.push(c);
                    }
                    _ => {}
                }
            }
        }
        if preview {
            if let Dialog::Import(form) = std::mem::replace(&mut self.dialog, Dialog::None) {
                self.preview(form);
            }
            return false;
        }
        if submit {
            let dialog = std::mem::replace(&mut self.dialog, Dialog::None);
            let result: Result<(), AppError> = match dialog {
                Dialog::Open(text) => { self.open(PathBuf::from(text.trim().trim_matches('"').trim_matches('\''))); Ok(()) },
                Dialog::Search(text) => { self.query=text; self.filter(); Ok(()) },
                Dialog::Save(text) => workflows::pretty(&self.loaded.manifest).and_then(|s|workflows::save_new(Path::new(&text), &s)).map(|()| self.status="Manifest exported. Use dataset export for complete sample measurements.".into()),
                Dialog::Paste(text) => { self.dialog=Dialog::Import(ImportForm::new(Some(text))); Ok(()) },
                Dialog::Import(form) => { let result=self.import(form.clone()); if result.is_err() { self.dialog=Dialog::Import(form); } result },
                Dialog::None => Ok(()),
            };
            if let Err(e) = result {
                self.status = e.to_string();
            }
            return false;
        }
        if !matches!(self.dialog, Dialog::None) {
            return false;
        }
        match key.code {
            KeyCode::Char('q') => { if self.pending.is_some() { self.quit_after_job=true; self.cancel(); } else { return true; } },
            KeyCode::Char('x') => self.cancel(),
            KeyCode::Char(c @ '1'..='7') => { self.tab=(c as u8-b'1') as usize; self.scroll=0; },
            KeyCode::Tab => {self.tab=(self.tab+1)%TABS.len();self.scroll=0;},
            KeyCode::BackTab => {self.tab=(self.tab+TABS.len()-1)%TABS.len();self.scroll=0;},
            KeyCode::Down | KeyCode::Char('j') => self.scroll=self.scroll.saturating_add(1),
            KeyCode::Up | KeyCode::Char('k') => self.scroll=self.scroll.saturating_sub(1),
            KeyCode::PageDown => self.scroll=self.scroll.saturating_add(20),
            KeyCode::PageUp => self.scroll=self.scroll.saturating_sub(20),
            KeyCode::Home => self.scroll=0,
            KeyCode::Char(']') if self.pending.is_none() => {self.sample=(self.sample+1).min(self.loaded.manifest.samples.len()-1);self.filter();},
            KeyCode::Char('[') if self.pending.is_none() => {self.sample=self.sample.saturating_sub(1);self.filter();},
            KeyCode::Char('o') if self.pending.is_none() => self.dialog=Dialog::Open(String::new()),
            KeyCode::Char('i') if self.pending.is_none() => self.dialog=Dialog::Import(ImportForm::new(None)),
            KeyCode::Char('p') if self.pending.is_none() => self.dialog=Dialog::Paste(String::new()),
            KeyCode::Char('/') => {self.tab=3;self.dialog=Dialog::Search(self.query.clone());},
            KeyCode::Char('s') => self.dialog=Dialog::Save(String::new()),
            KeyCode::Char('?') => self.status="1–7 views · [/] samples · / gene search · i file import · p paste table · o dataset · s manifest · x cancel · q quit. Import: Tab fields, Ctrl-D detect, Ctrl-S submit. CLI: josh dataset --help. Legacy: josh tui --legacy.".into(),
            _=>{}
        }
        false
    }
    pub fn paste(&mut self, text: String) {
        match &mut self.dialog {
            Dialog::Paste(value) if value.len()+text.len()<=MAX_PASTE => value.push_str(&text),
            Dialog::Import(form) => { let clean=text.trim().trim_matches('"').trim_matches('\''); if !clean.chars().any(char::is_control) && form.values[form.active].len()+clean.len()<4096 { form.values[form.active].push_str(clean); } },
            Dialog::Open(value) | Dialog::Save(value) | Dialog::Search(value) if !text.chars().any(char::is_control) && value.len()+text.len()<4096 => value.push_str(text.trim()),
            _ => self.status="Paste rejected or no editor open. Press p to paste an expression table (maximum 1 MiB).".into(),
        }
    }
    pub fn draw(&mut self, frame: &mut Frame) {
        let area = frame.area();
        let [header, nav, body, status, footer] = Layout::vertical([
            Constraint::Length(2),
            Constraint::Length(2),
            Constraint::Min(1),
            Constraint::Length(3),
            Constraint::Length(2),
        ])
        .areas(area);
        let s = &self.loaded.manifest.samples[self.sample];
        let class = if s.data_class == josh_core::DataClass::Synthetic {
            "SYNTHETIC DATA"
        } else {
            "RESEARCH DATA (declared)"
        };
        frame.render_widget(
            Paragraph::new(format!(
                "JOSH · MOLECULAR DATA WORKBENCH · {class}\n{} / {}",
                clean(&self.loaded.manifest.dataset_id),
                clean(&s.sample_id)
            ))
            .style(Style::default().fg(Color::Cyan)),
            header,
        );
        let tabs: Vec<_> = TABS
            .iter()
            .enumerate()
            .map(|(i, t)| {
                if area.width < 85 {
                    format!("{} {}", i + 1, &t[..3])
                } else {
                    format!("{} {t}", i + 1)
                }
            })
            .collect();
        frame.render_widget(
            Tabs::new(tabs)
                .select(self.tab)
                .highlight_style(Style::default().fg(Color::LightCyan)),
            nav,
        );
        if self.tab == 3 {
            self.draw_features(frame, body);
        } else {
            let text=match self.tab {
                0=>self.sample_text(),
                1=>format!("{}\n\nDATA DICTIONARY\n{}\n\n{}",data::summary(&self.loaded.manifest),self.loaded.manifest.data_dictionary.iter().map(|d|format!("{}: {}",d.name,d.meaning)).collect::<Vec<_>>().join("\n"),self.loaded.manifest.notices.join("\n")),
                2=>"DATA PIPELINE\n\n1 Import source                 COMPLETE\n2 Detect structure              RECORDED\n3 Parse measurements             COMPLETE\n4 Map genes                      See sample QC\n5 Apply declared transform       See configuration\n6 Quality control                See sample QC\n7 Compare compatible reference   NOT IMPLEMENTED\n8 Jev molecular inference        NOT IMPLEMENTED\n9 Ranked molecular analysis      NOT RUN\n\nExplore imported data without a key. No clinical profile is required.\nExisting summarized-case Jev workflows remain available through the legacy CLI/TUI.".into(),
                4=>format!("JEV\nPinned model: {}\nExpression pipeline: {}\n\nJev remains the origin classifier.\nMolecular evidence packaging and compatible-reference comparison are the next implementation step.\nNo trained cancer reference, calibrated probability or OOD detector is installed.\n\nLegacy request preview/classify commands are unchanged.",josh_core::MODEL,EXPRESSION_PIPELINE),
                5=>"REFERENCE CANCERS\n\nNo reference cohort loaded.\nGene dictionaries map identifiers; they are not known-primary tumor references.\n\nExpression values and the demo are not tissue-of-origin predictions.\nReference comparison and held-out validation remain required.".into(),
                _=>format!("LOCAL WORKSPACE\n\nOpen dataset: {}\n\nDataset bundles contain source files, the selected gene dictionary, sample measurements, QC and provenance.\nOpen another bundle with o; import with i or paste with p.\nA multi-dataset project catalog is planned.\n\nLegacy case access: josh tui --legacy or --case FILE.",self.loaded.root.as_ref().map(|p|p.display().to_string()).unwrap_or_else(||"bundled in-memory example".into())),
            };
            let text = clean_multiline(&text);
            let max_scroll = text
                .lines()
                .count()
                .saturating_sub(body.height.saturating_sub(2) as usize);
            self.scroll = self.scroll.min(max_scroll);
            frame.render_widget(
                Paragraph::new(text)
                    .block(Block::bordered().title(TABS[self.tab]))
                    .wrap(Wrap { trim: false })
                    .scroll((self.scroll.min(u16::MAX as usize) as u16, 0)),
                body,
            );
        }
        frame.render_widget(
            Paragraph::new(clean_multiline(&self.status))
                .style(Style::default().fg(Color::Yellow))
                .wrap(Wrap { trim: false }),
            status,
        );
        frame.render_widget(Paragraph::new("i Import · p Paste · o Open · [/] Sample · / Gene · s Export · q Quit\nTab views · ↑/↓ scroll · x Cancel job · ? Help"),footer);
        self.draw_dialog(frame, area);
    }
    fn sample_text(&self) -> String {
        let s = &self.loaded.manifest.samples[self.sample];
        let a = &s.assays[0];
        let mut lines = vec![
            format!(
                "SAMPLE {} ({}/{})",
                s.sample_id,
                self.sample + 1,
                self.loaded.manifest.samples.len()
            ),
            format!("Modality: {:?}", a.modality),
        ];
        if let Some(q) = &a.qc {
            lines.push(format!("\nQUALITY: {:?}\nRecords: {} | measured: {} | missing: {} | zeros: {}\nMapped: {} | ambiguous: {} | unmapped: {}\nInvalid values: {} | duplicate genes: {}\nGene-ID mapping rate: {}\nRaw range: {:?} .. {:?}",q.status,q.total_records,q.measured_values,q.missing_values,q.zero_values,q.mapped_records,q.ambiguous_records,q.unmapped_records,q.invalid_values,q.duplicate_gene_records,q.gene_id_mapping_rate.map(|v|format!("{:.1}% of all source records",v*100.0)).unwrap_or_else(||"not assessed".into()),q.minimum,q.maximum));
            for i in &q.issues {
                lines.push(format!("  {} (record {:?})", i.code, i.source_record));
            }
        }
        if let Some(c) = &self.loaded.manifest.expression_config {
            lines.push(format!(
                "\nUnits: {:?} | transform: {:?}\nPlatform: {} | genome: {}",
                c.units,
                c.transform,
                c.platform.as_deref().unwrap_or("unknown"),
                c.reference_genome.as_deref().unwrap_or("unknown")
            ));
        }
        lines.push(format!("\nPROVENANCE\nSource: {}\nSource hash: {}\nMeasurements: {}\nMeasurement hash: {}\nGene dictionary: {}\n\nMissing modalities: variants, structured IHC, copy number, methylation.\nReference compatibility: not assessed.\nNo molecular prediction has been run.",self.loaded.manifest.source_name,self.loaded.manifest.source.artifact.sha256,a.artifact.path,a.artifact.sha256,self.loaded.manifest.gene_map.as_ref().map(|m|m.release.as_str()).unwrap_or("not supplied")));
        lines.join("\n")
    }
    fn draw_features(&mut self, frame: &mut Frame, area: Rect) {
        let data = &self.loaded.records[self.sample];
        let height = area.height.saturating_sub(4) as usize;
        self.scroll = self
            .scroll
            .min(self.visible.len().saturating_sub(height.max(1)));
        let rows = self.visible.iter().skip(self.scroll).take(height).map(|i| {
            let r = &data[*i];
            Row::new(vec![
                Cell::from(clean(&r.original_gene_id)),
                Cell::from(
                    r.mapping
                        .gene
                        .as_ref()
                        .map(|g| g.symbol.clone())
                        .unwrap_or_else(|| format!("{:?}", r.mapping.status)),
                ),
                Cell::from(
                    r.raw_expression
                        .map(|v| format!("{v:.4}"))
                        .unwrap_or_else(|| "—".into()),
                ),
                Cell::from(
                    r.transformed_expression
                        .map(|v| format!("{v:.4}"))
                        .unwrap_or_else(|| "—".into()),
                ),
                Cell::from(format!("{}:{}", r.source_record, r.source_column)),
            ])
        });
        frame.render_widget(
            Table::new(
                rows,
                [
                    Constraint::Percentage(25),
                    Constraint::Percentage(25),
                    Constraint::Percentage(17),
                    Constraint::Percentage(18),
                    Constraint::Percentage(15),
                ],
            )
            .header(
                Row::new([
                    "Original gene",
                    "Canonical / status",
                    "Raw",
                    "Transformed",
                    "Record:col",
                ])
                .style(Style::default().fg(Color::Cyan)),
            )
            .block(Block::bordered().title(format!(
                "Explore · {} matches · / search · sorted by raw expression",
                self.visible.len()
            ))),
            area,
        );
    }
    fn draw_dialog(&self, frame: &mut Frame, area: Rect) {
        if matches!(self.dialog, Dialog::None) {
            return;
        }
        let inset = if area.width > 50 { 3 } else { 0 };
        let rect = Rect {
            x: area.x + inset,
            y: area.y + 1,
            width: area.width.saturating_sub(inset * 2),
            height: area.height.saturating_sub(2),
        };
        frame.render_widget(Clear, rect);
        let (title, text) = match &self.dialog {
            Dialog::Import(f) => {
                let visible = rect.height.saturating_sub(7) as usize;
                let start = f.active.saturating_sub(visible.saturating_sub(1));
                let fields = LABELS
                    .iter()
                    .enumerate()
                    .skip(start)
                    .take(visible)
                    .map(|(i, label)| {
                        format!(
                            "{} {label}: {}",
                            if i == f.active { ">" } else { " " },
                            clean(&f.values[i])
                        )
                    })
                    .collect::<Vec<_>>()
                    .join("\n");
                (
                    "Import expression",
                    format!(
                        "Tab/↑/↓ field · Ctrl-D detect · Ctrl-S import · Esc cancel\nWide matrix: clear Sample ID. Map/release optional.\nSource: {}\n{fields}\n\n{}",
                        if f.pasted.is_some() {
                            "pasted table"
                        } else {
                            "local file"
                        },
                        clean(&self.status)
                    ),
                )
            }
            Dialog::Open(s) => (
                "Open dataset directory",
                format!("{s}\n\nEnter to open · Esc cancel"),
            ),
            Dialog::Save(s) => (
                "Export manifest to NEW file",
                format!("{s}\n\nEnter to save · Esc cancel"),
            ),
            Dialog::Search(s) => (
                "Search original/canonical gene",
                format!("{s}\n\nEnter to search · empty shows all"),
            ),
            Dialog::Paste(s) => (
                "Paste CSV/TSV table",
                format!(
                    "{} bytes (max 1 MiB)\nPaste with your terminal; Enter continues to import settings.\n\n{}",
                    s.len(),
                    s.lines().take(12).collect::<Vec<_>>().join("\n")
                ),
            ),
            Dialog::None => unreachable!(),
        };
        frame.render_widget(
            Paragraph::new(clean_multiline(&text))
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title(title)
                        .border_style(Style::default().fg(Color::Cyan)),
                )
                .wrap(Wrap { trim: false }),
            rect,
        );
    }
}

fn clean(s: &str) -> String {
    workflows::display_text(s)
}
fn clean_multiline(s: &str) -> String {
    s.chars()
        .map(|c| {
            if c.is_control() && c != '\n' && c != '\t' {
                ' '
            } else {
                c
            }
        })
        .collect()
}
struct Restore;
impl Drop for Restore {
    fn drop(&mut self) {
        let _ = execute!(std::io::stdout(), DisableBracketedPaste);
        ratatui::restore();
    }
}
pub async fn run(root: Option<PathBuf>) -> Result<(), AppError> {
    if !std::io::stdin().is_terminal() || !std::io::stdout().is_terminal() {
        return Err(josh_core::errors::ErrorEnvelope::new(
            josh_core::errors::ErrorCode::TerminalRequired,
        )
        .into());
    }
    let mut app = Workbench::new()?;
    if let Some(root) = root {
        app.open(root);
    }
    let _restore = Restore;
    let mut terminal = ratatui::try_init()?;
    execute!(std::io::stdout(), EnableBracketedPaste)?;
    loop {
        app.poll().await;
        if app.quit_after_job && app.pending.is_none() {
            break;
        }
        terminal.draw(|f| app.draw(f))?;
        if event::poll(Duration::from_millis(80))? {
            match event::read()? {
                Event::Key(key) if app.key(key) => break,
                Event::Paste(text) => app.paste(text),
                _ => {}
            }
        }
        tokio::task::yield_now().await;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::{Terminal, backend::TestBackend};
    fn key(c: char) -> KeyEvent {
        KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE)
    }
    fn text(terminal: &Terminal<TestBackend>) -> String {
        terminal
            .backend()
            .buffer()
            .content
            .iter()
            .map(|c| c.symbol())
            .collect()
    }
    #[test]
    fn default_workbench_is_sample_first_and_all_views_render_on_small_terminals() {
        for (w, h) in [(120, 40), (60, 18), (28, 9), (5, 3)] {
            let mut app = Workbench::new().unwrap();
            let mut terminal = Terminal::new(TestBackend::new(w, h)).unwrap();
            for tab in 0..7 {
                app.tab = tab;
                terminal.draw(|f| app.draw(f)).unwrap();
            }
            app.dialog = Dialog::Import(ImportForm::new(None));
            terminal.draw(|f| app.draw(f)).unwrap();
        }
        let mut app = Workbench::new().unwrap();
        let mut terminal = Terminal::new(TestBackend::new(120, 40)).unwrap();
        terminal.draw(|f| app.draw(f)).unwrap();
        let t = text(&terminal);
        assert!(t.contains("SYNTHETIC DATA"));
        assert!(t.contains("Samples"));
        assert!(!t.contains("NICE"));
        assert!(t.contains("No molecular prediction"));
    }
    #[test]
    fn gene_search_uses_canonical_and_original_ids_and_changes_sample() {
        let mut app = Workbench::new().unwrap();
        app.query = "TP53".into();
        app.filter();
        assert_eq!(app.visible.len(), 1);
        app.key(key(']'));
        assert_eq!(app.sample, 1);
        assert_eq!(app.visible.len(), 1);
        assert_eq!(
            app.loaded.records[1][app.visible[0]].raw_expression,
            Some(4.1)
        );
        app.query.clear();
        app.filter();
        assert_eq!(app.visible.len(), 6);
    }
    #[test]
    fn bracketed_paste_retains_table_and_cancel_does_not_import() {
        let mut app = Workbench::new().unwrap();
        app.key(key('p'));
        app.paste("gene\texpression\nTP53\t3\n".into());
        app.key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
        assert!(
            matches!(&app.dialog,Dialog::Import(f) if f.pasted.as_deref()==Some("gene\texpression\nTP53\t3\n"))
        );
        app.key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
        assert!(app.pending.is_none());
        assert_eq!(app.loaded.manifest.dataset_id, "synthetic-expression-demo");
    }
    #[tokio::test]
    async fn failed_async_open_preserves_current_dataset() {
        let mut app = Workbench::new().unwrap();
        app.open(PathBuf::from("/missing-josh-dataset"));
        app.key(key('i'));
        assert!(matches!(app.dialog, Dialog::None));
        for _ in 0..200 {
            app.poll().await;
            if app.pending.is_none() {
                break;
            }
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
        assert!(app.pending.is_none());
        assert_eq!(app.loaded.manifest.dataset_id, "synthetic-expression-demo");
    }
    #[test]
    fn unknown_units_and_invalid_transform_are_explicit_in_import_settings() {
        let mut form = ImportForm::new(None);
        assert_eq!(
            form.options().unwrap().config.units,
            ExpressionUnit::Unknown
        );
        form.values[4] = "invented-unit".into();
        assert!(form.options().is_err());
    }
    #[tokio::test]
    async fn import_preview_preserves_editable_configuration_and_writes_nothing() {
        let mut app = Workbench::new().unwrap();
        let mut form = ImportForm::new(Some("gene\texpression\nTP53\t3\n".into()));
        form.values[3] = "sample".into();
        app.preview(form);
        for _ in 0..200 {
            app.poll().await;
            if app.pending.is_none() {
                break;
            }
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
        assert!(app.status.contains("Detected Tsv / Long"));
        assert!(matches!(&app.dialog, Dialog::Import(f) if f.values[3] == "sample"));
        assert_eq!(app.loaded.manifest.dataset_id, "synthetic-expression-demo");
    }
    #[test]
    fn tui_import_never_reads_keyboard_stdin_as_a_source() {
        let mut app = Workbench::new().unwrap();
        let mut form = ImportForm::new(None);
        form.values[0] = "-".into();
        assert!(app.import(form).is_err());
        assert!(app.pending.is_none());
    }
    #[test]
    fn oversized_paste_is_rejected_without_replacing_data() {
        let mut app = Workbench::new().unwrap();
        app.key(key('p'));
        app.paste("x".repeat(MAX_PASTE + 1));
        assert!(matches!(&app.dialog,Dialog::Paste(s) if s.is_empty()));
        assert!(app.status.contains("rejected"));
    }
}

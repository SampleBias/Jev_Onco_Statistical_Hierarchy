//! Sample-first Ratatui workbench. All imports are local and no model call is hidden.
use crate::{
    data,
    workflows::{self, AppError},
};
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use josh_core::sample::*;
use josh_ingest::{dataset, expression};
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Color, Style},
    widgets::{Block, Cell, Clear, Paragraph, Row, Table, Tabs, Wrap},
};
use std::{
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicU8, Ordering},
    },
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
    Reference(String),
    SaveRequest(String),
    Paste(String),
    Import(ImportForm),
}
enum JobResult {
    Reference(Box<josh_core::reference::ReferenceRelease>, String),
    Comparison(Box<josh_core::reference::EvidencePackage>),
    Dataset(Box<Loaded>),
    Preview { form: ImportForm, text: String },
}

struct Pending {
    handle: tokio::task::JoinHandle<Result<JobResult, String>>,
    state: Arc<AtomicU8>,
}

pub struct Workbench {
    guide: crate::guide::Guide,
    reference: Option<(josh_core::reference::ReferenceRelease, String)>,
    comparison: Option<josh_core::reference::EvidencePackage>,
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
    dataset::reproduce(&root)?;
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
        let mut app = Self {
            guide: crate::guide::Guide::default(),
            reference: None,
            comparison: None,
            loaded: demo()?,
            sample: 0,
            tab: 0,
            scroll: 0,
            query: String::new(),
            visible: vec![],
            dialog: Dialog::None,
            status:
                "Synthetic expression example. i Import file · p Paste data · l/o Load · ? Help"
                    .into(),
            pending: None,
            quit_after_job: false,
        };
        app.filter();
        Ok(app)
    }
    pub(crate) fn editing(&self) -> bool {
        !matches!(self.dialog, Dialog::None)
    }
    pub(crate) fn busy(&self) -> bool {
        self.pending.is_some()
    }
    pub(crate) fn cancel_job(&mut self) {
        self.cancel();
    }
    pub(crate) fn open_dataset(&mut self, path: PathBuf) {
        self.open(path);
    }
    pub(crate) fn import_path(&mut self, path: &Path) {
        let mut form = ImportForm::new(None);
        form.values[0] = path.display().to_string();
        self.dialog = Dialog::Import(form);
    }
    pub(crate) fn analysis_input(&self) -> Result<crate::molecular_tui::App, AppError> {
        let e = self
            .comparison
            .as_ref()
            .ok_or("Compare the selected sample with a reference first (r, then a).")?;
        let sample = &self.loaded.manifest.samples[self.sample];
        crate::molecular_tui::App::from_expression(
            e,
            sample
                .patient_group_id
                .as_deref()
                .unwrap_or(&sample.sample_id),
        )
    }
    fn open_reference(&mut self, path: PathBuf) {
        let state = Arc::new(AtomicU8::new(0));
        let copy = state.clone();
        let handle = tokio::task::spawn_blocking(move || {
            let (r, hash) = crate::reference::load(&path).map_err(|e| e.to_string())?;
            if copy.load(Ordering::SeqCst) == 1 {
                return Err("Reference open canceled.".into());
            }
            Ok(JobResult::Reference(Box::new(r), hash))
        });
        self.pending = Some(Pending { handle, state });
        self.status = "Validating reference release locally…".into();
    }
    fn compare(&mut self) {
        let Some((reference, hash)) = self.reference.clone() else {
            self.status = "Press r to open a reference JSON first.".into();
            return;
        };
        let manifest = self.loaded.manifest.clone();
        let records = self.loaded.records[self.sample].clone();
        let sample = self.sample;
        let state = Arc::new(AtomicU8::new(0));
        let copy = state.clone();
        let handle = tokio::task::spawn_blocking(move || {
            let evidence =
                crate::reference::compare(&reference, &hash, &manifest, sample, &records)
                    .map_err(|e| e.to_string())?;
            if copy.load(Ordering::SeqCst) == 1 {
                return Err("Comparison canceled.".into());
            }
            Ok(JobResult::Comparison(Box::new(evidence)))
        });
        self.pending = Some(Pending { handle, state });
        self.status = "Comparing compatible expression profiles locally…".into();
        self.tab = 2;
    }
    fn reference_text(&self) -> String {
        let library = match &self.reference {
            None => "REFERENCE LIBRARY\n\nPress r to open a curated reference JSON.\nBuild a reference: josh reference build DATASET --labels LABELS.tsv --release-id VERSION --citation SOURCE --output NEW.json\n\nGene dictionaries map identifiers; they are not tumor references.\nPress g for the complete guide, including label format and compatibility gates.".into(),
            Some((r,hash)) => format!("REFERENCE {}\nSHA256: {}\nCitation: {}\nSource dataset: {}\nSynthetic: {}\n\n{} samples · {} classes · {} common genes\nProcessing: TPM → log2(x+1) · {}\nMinimum overlap: {} genes / {:.0}%\n\n{}\n\nPress a to compare the selected sample.\nNo calibrated probabilities or validated OOD detector.\n{}",r.release_id,hash,r.citation,r.source_dataset_id,r.synthetic,r.members.len(),r.classes.len(),r.genes.len(),r.compatibility.platform,r.minimum_genes,r.minimum_overlap*100.,r.classes.iter().map(|c|format!("{}  (n={})",c.cancer_type,c.samples)).collect::<Vec<_>>().join("\n"),r.limitations.join("\n")),
        };
        format!(
            "RESEARCH REFERENCES\n\n\
             OncoNPC - Moon I et al.\n\
             Utilizing Electronic Health Records (EHR) and Tumor Panel Sequencing to Demystify Prognosis of Cancer of Unknown Primary (CUP) patients.\n\
             medRxiv, 2022, version 1 (preprint). DOI: 10.1101/2022.12.22.22283696\n\
             https://www.medrxiv.org/content/10.1101/2022.12.22.22283696v1.full-text\n\n\
             Published study: Moon I et al.\n\
             Machine learning for genetics-based classification and treatment response prediction in cancer of unknown primary.\n\
             Nature Medicine 29, 2057-2067 (2023).\n\
             https://doi.org/10.1038/s41591-023-02482-6\n\
             Publisher correction (cancer grouping):\n\
             https://doi.org/10.1038/s41591-023-02693-x\n\n\
             Research and visualization reference for JOSH. Published OncoNPC results do not validate Jev predictions.\n\
             Cohort figures: confusion matrix, survival by predicted cancer type, and survival by treatment concordance.\n\
             F5 Cohort: confusion heatmap, survival curves, adjusted Cox and supplied-propensity IPTW.\n\
             Press d there for a labeled synthetic demo; load frozen studies for local analysis.\n\
             Weighted survival intervals/tests and real cancer predictive validation remain unavailable.\n\
             Assessment: docs/assessment/ONCONPC_PARITY.md\n\n\
             {library}"
        )
    }
    fn analysis_text(&self) -> String {
        match &self.comparison {
            Some(e) => format!("{}\n\ns Export evidence · e Export exact Jev request preview\nm Open molecular inference and explanations for this sample\n\nLIMITATIONS\n{}",crate::reference::summary(e),e.limitations.join("\n")),
            None => "ANALYSIS PIPELINE\n\n● Import sample + preserve source\n● Parse expression + map identifiers\n● Record transform + QC\n○ Compare compatible reference [r then a]\n○ Prepare structured Jev request [e after comparison]\n○ Molecular Jev inference and explanations [m]\n\nCompare a reference first to carry this expression sample into molecular inference. After comparison, m uses this sample in the shared Analysis section.\nNo clinical profile is required. Explore without an API key.\nReference correlations remain numerical evidence, not cancer probabilities.".into(),
        }
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
            let canceled = pending.state.load(Ordering::SeqCst) == 1;
            let outcome = pending.handle.await;
            if canceled {
                self.status = "Job canceled; current workspace preserved.".into();
                return;
            }
            match outcome {
                Ok(Ok(JobResult::Dataset(loaded))) => {
                    self.comparison = None;
                    self.loaded = *loaded;
                    self.sample = 0;
                    self.query.clear();
                    self.filter();
                    self.status =
                        "Dataset loaded. Inspect QC; r opens a reference and a compares locally."
                            .into();
                }
                Ok(Ok(JobResult::Reference(reference, hash))) => {
                    self.reference = Some((*reference, hash));
                    self.comparison = None;
                    self.tab = 5;
                    self.scroll = 0;
                    self.status =
                        "Reference loaded. a compares the selected sample; g opens the guide."
                            .into();
                }
                Ok(Ok(JobResult::Comparison(evidence))) => {
                    self.status = format!(
                        "Comparison: {:?}. s exports evidence; e exports an offline Jev request preview.",
                        evidence.status
                    );
                    self.comparison = Some(*evidence);
                    self.tab = 2;
                    self.scroll = 0;
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
        if self.guide.key(key, !matches!(self.dialog, Dialog::None)) {
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
                    Dialog::Open(s)
                    | Dialog::Search(s)
                    | Dialog::Save(s)
                    | Dialog::Paste(s)
                    | Dialog::Reference(s)
                    | Dialog::SaveRequest(s) => s,
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
            let result: Result<(), AppError> = (|| match dialog {
                Dialog::Open(text) => {
                    self.open(PathBuf::from(
                        text.trim().trim_matches('"').trim_matches('\''),
                    ));
                    Ok(())
                }
                Dialog::Search(text) => {
                    self.query = text;
                    self.filter();
                    Ok(())
                }
                Dialog::Save(text) => {
                    let value = if self.tab == 2 {
                        self.comparison
                            .as_ref()
                            .map(serde_json::to_value)
                            .transpose()?
                    } else {
                        None
                    };
                    let value = value.unwrap_or(serde_json::to_value(&self.loaded.manifest)?);
                    workflows::pretty(&value)
                        .and_then(|s| workflows::save_new(Path::new(&text), &s))
                        .map(|()| self.status = "Export saved to a new file.".into())
                }
                Dialog::Reference(text) => {
                    self.open_reference(PathBuf::from(text.trim()));
                    Ok(())
                }
                Dialog::SaveRequest(text) => self
                    .comparison
                    .as_ref()
                    .ok_or_else(|| AppError::from("Compare a sample first."))
                    .and_then(|e| {
                        let value = crate::reference::prepared(e)?;
                        workflows::pretty(&value)
                            .and_then(|s| workflows::save_new(Path::new(&text), &s))
                    })
                    .map(|()| self.status = "Jev request preview exported. Nothing sent.".into()),
                Dialog::Paste(text) => {
                    self.dialog = Dialog::Import(ImportForm::new(Some(text)));
                    Ok(())
                }
                Dialog::Import(form) => {
                    let result = self.import(form.clone());
                    if result.is_err() {
                        self.dialog = Dialog::Import(form);
                    }
                    result
                }
                Dialog::None => Ok(()),
            })();
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
            KeyCode::Char(']') if self.pending.is_none() => {self.comparison=None;self.sample=(self.sample+1).min(self.loaded.manifest.samples.len()-1);self.filter();},
            KeyCode::Char('[') if self.pending.is_none() => {self.comparison=None;self.sample=self.sample.saturating_sub(1);self.filter();},
            KeyCode::Char('r') if self.pending.is_none() => self.dialog=Dialog::Reference(String::new()),
            KeyCode::Char('a') if self.pending.is_none() => self.compare(),
            KeyCode::Char('e') if self.pending.is_none() => self.dialog=Dialog::SaveRequest(String::new()),
            KeyCode::Char('o') if self.pending.is_none() => self.dialog=Dialog::Open(String::new()),
            KeyCode::Char('i') if self.pending.is_none() => self.dialog=Dialog::Import(ImportForm::new(None)),
            KeyCode::Char('p') if self.pending.is_none() => self.dialog=Dialog::Paste(String::new()),
            KeyCode::Char('/') => {self.tab=3;self.dialog=Dialog::Search(self.query.clone());},
            KeyCode::Char('s') => self.dialog=Dialog::Save(String::new()),
            KeyCode::Char('?') => self.status="m Molecular inference/charts · g Full guide · F1/Ctrl+g in forms · r Reference · a Compare · s Export · e Jev preview · x Cancel · q Quit".into(),
            _=>{}
        }
        false
    }
    pub fn paste(&mut self, text: String) {
        if self.guide.open {
            self.guide.paste(&text);
            return;
        }
        match &mut self.dialog {
            Dialog::Paste(value) if value.len()+text.len()<=MAX_PASTE => value.push_str(&text),
            Dialog::Import(form) => { let clean=text.trim().trim_matches('"').trim_matches('\''); if !clean.chars().any(char::is_control) && form.values[form.active].len()+clean.len()<4096 { form.values[form.active].push_str(clean); } },
            Dialog::Open(value) | Dialog::Save(value) | Dialog::Search(value) | Dialog::Reference(value) | Dialog::SaveRequest(value) if !text.chars().any(char::is_control) && value.len()+text.len()<4096 => value.push_str(text.trim()),
            _ => self.status="Paste rejected or no editor open. Press p to paste an expression table (maximum 1 MiB).".into(),
        }
    }
    pub fn draw(&mut self, frame: &mut Frame) {
        self.draw_area(frame, frame.area(), false);
    }
    pub(crate) fn draw_area(&mut self, frame: &mut Frame, area: Rect, embedded: bool) {
        use crate::ui;
        use ratatui::{
            style::Modifier,
            text::{Line, Span},
        };
        frame.render_widget(
            Block::default().style(Style::default().bg(ui::BG).fg(ui::TEXT)),
            area,
        );
        let [header, nav, body, status, footer] = Layout::vertical([
            Constraint::Length(4),
            Constraint::Length(3),
            Constraint::Min(1),
            Constraint::Length(2),
            Constraint::Length(2),
        ])
        .areas(area);
        let [brand, badge] = Layout::horizontal([
            Constraint::Min(0),
            Constraint::Length(if area.width >= 90 { 32 } else { 0 }),
        ])
        .areas(header);
        frame.render_widget(
            Paragraph::new(vec![
                Line::from(Span::styled(
                    if embedded {
                        " Expression data"
                    } else {
                        " Jev Onco Statistical Hierarchy"
                    },
                    Style::default().fg(ui::ACCENT).add_modifier(Modifier::BOLD),
                )),
                Line::from(Span::styled(
                    if embedded {
                        " Reference comparison and preparation"
                    } else {
                        " Molecular Data Workbench"
                    },
                    Style::default().fg(ui::TEXT),
                )),
                Line::from(Span::styled(
                    format!(
                        " {} / {}",
                        clean(&self.loaded.manifest.dataset_id),
                        clean(&self.loaded.manifest.samples[self.sample].sample_id)
                    ),
                    Style::default().fg(ui::MUTED),
                )),
            ]),
            brand,
        );
        frame.render_widget(
            Paragraph::new(vec![
                Line::from(vec![
                    Span::styled(
                        "  ● ",
                        Style::default().fg(if workflows::key_configured() {
                            ui::ACCENT
                        } else {
                            ui::MUTED
                        }),
                    ),
                    Span::styled("JEV Ready", Style::default().fg(ui::TEXT)),
                ]),
                Line::from(vec![
                    Span::styled(
                        "  ● ",
                        Style::default().fg(if self.pending.is_some() {
                            ui::GOLD
                        } else {
                            ui::ACCENT
                        }),
                    ),
                    Span::styled("Workspace", Style::default().fg(ui::TEXT)),
                ]),
            ]),
            badge,
        );
        let tabs: Vec<_> = TABS
            .iter()
            .enumerate()
            .map(|(i, t)| {
                if area.width < 90 {
                    format!("{} {}", i + 1, &t[..3])
                } else {
                    format!("{} {t}", i + 1)
                }
            })
            .collect();
        frame.render_widget(
            Tabs::new(tabs)
                .select(self.tab)
                .block(ui::panel(" NAVIGATE "))
                .divider(" ")
                .style(Style::default().fg(ui::MUTED))
                .highlight_style(
                    Style::default()
                        .fg(ui::BG)
                        .bg(ui::ACCENT)
                        .add_modifier(Modifier::BOLD),
                ),
            nav,
        );
        match self.tab {
            0 if body.width >= 80 && body.height >= 16 => self.draw_dashboard(frame, body),
            2 if self.comparison.is_some() && body.width >= 85 && body.height >= 14 => {
                self.draw_comparison(frame, body)
            }
            3 => self.draw_features(frame, body),
            _ => {
                let text = match self.tab {
                    0 => self.sample_text(),
                    1 => format!(
                        "{}\n\nDATA DICTIONARY\n{}\n\n{}",
                        data::summary(&self.loaded.manifest),
                        self.loaded
                            .manifest
                            .data_dictionary
                            .iter()
                            .map(|d| format!("{}: {}", d.name, d.meaning))
                            .collect::<Vec<_>>()
                            .join("\n"),
                        self.loaded.manifest.notices.join("\n")
                    ),
                    2 => self.analysis_text(),
                    4 => format!(
                        "JEV · STRUCTURED DECISIONS\n\nPinned model: {}\nExpression pipeline: {}\n\nJev remains the sole origin classifier. Reference correlations are numerical evidence, not class probabilities.\n\nPress m for molecular inference, circular and scatter explanations. Live calls accept declared synthetic data.\nNo calibrated cancer model or validated OOD detector is installed.\n\nPress g for the full guide and / inside the guide to search.",
                        josh_core::MODEL,
                        EXPRESSION_PIPELINE
                    ),
                    5 => self.reference_text(),
                    _ => format!(
                        "LOCAL WORKSPACE\n\nOpen dataset: {}\n\nDataset bundles preserve source files, gene dictionaries, measurements, QC and provenance.\n\nOpen a dataset with o; import with i or paste with p.\nA multi-dataset project catalog is planned.\n\nClinical evidence and review: F4 in this workspace.\nUser guide: g (F1/Ctrl+g inside forms).",
                        self.loaded
                            .root
                            .as_ref()
                            .map(|p| p.display().to_string())
                            .unwrap_or_else(|| "bundled in-memory example".into())
                    ),
                };
                let paragraph = Paragraph::new(clean_multiline(&text)).wrap(Wrap { trim: false });
                let panel = ui::panel(format!(" {} ", TABS[self.tab].to_uppercase()));
                let inner = panel.inner(body);
                let count = paragraph.line_count(inner.width.max(1));
                self.scroll = self.scroll.min(count.saturating_sub(inner.height as usize));
                frame.render_widget(
                    paragraph
                        .block(panel)
                        .scroll((self.scroll.min(u16::MAX as usize) as u16, 0)),
                    body,
                );
            }
        }
        frame.render_widget(
            Paragraph::new(clean_multiline(&self.status))
                .style(Style::default().fg(ui::GOLD))
                .wrap(Wrap { trim: false }),
            status,
        );
        frame.render_widget(Paragraph::new(vec![
            Line::from(vec![Span::styled(" g GUIDE ",Style::default().fg(ui::BG).bg(ui::ACCENT).add_modifier(Modifier::BOLD)),Span::raw("  i Import · p Paste · o Open · [/] Sample · / Gene · q Quit")]),
            Line::from(Span::styled("m Molecular charts · Tab Views · r Reference · a Compare · s Export · x Cancel · F1 Help",Style::default().fg(ui::MUTED)))
        ]),footer);
        // A guide overlay leaves underlying forms and ongoing jobs intact.
        self.draw_dialog(frame, area);
        self.guide.draw(frame);
    }
    fn draw_comparison(&mut self, frame: &mut Frame, area: Rect) {
        use crate::ui;
        use ratatui::text::{Line, Span};
        let e = self.comparison.as_ref().unwrap();
        let [scores, detail] =
            Layout::horizontal([Constraint::Percentage(55), Constraint::Percentage(45)])
                .areas(area);
        let mut lines = vec![
            Line::from(Span::styled(
                "Pearson r · fixed scale -1 ── 0 ── +1",
                Style::default().fg(ui::GOLD),
            )),
            Line::from("Numerical similarity, not probability"),
            Line::from(""),
        ];
        for c in &e.similarities {
            lines.push(Line::from(format!(
                "{} · n={}",
                clean(&c.cancer_type),
                c.reference_samples
            )));
            if let Some(r) = c.pearson_r {
                let n = (r.abs() * 16.).round() as usize;
                let left = if r < 0. {
                    format!("{}{}", " ".repeat(16 - n), "━".repeat(n))
                } else {
                    " ".repeat(16)
                };
                let right = if r >= 0. {
                    format!("{}{}", "━".repeat(n), " ".repeat(16 - n))
                } else {
                    " ".repeat(16)
                };
                lines.push(Line::from(vec![
                    Span::styled(
                        format!("{left}│{right}"),
                        Style::default().fg(if r >= 0. { ui::ACCENT } else { ui::BLUE }),
                    ),
                    Span::raw(format!(" {r:+.4}")),
                ]));
            } else {
                lines.push(Line::from(Span::styled(
                    "Undefined correlation",
                    Style::default().fg(ui::GOLD),
                )));
            }
            lines.push(Line::from(""));
        }
        if e.similarities.is_empty() {
            lines.push(Line::from("Comparison gates did not pass."));
        }
        let panel = ui::panel(" REFERENCE SIMILARITY ");
        let inner = panel.inner(scores);
        let report = format!(
            "{:?}\n\n{} / {} shared genes\nOverlap: {:.1}%\n\n{}\n\nOOD: not validated\nConflicts: expression only; not assessed\n\n{}\n\ns Save complete evidence\ne Save offline Jev request\ng Guide",
            e.status,
            e.common_genes,
            e.reference_genes,
            e.overlap_fraction * 100.,
            e.reasons.join("\n"),
            e.limitations.join("\n")
        );
        let detail_panel = ui::panel(" COVERAGE / UNCERTAINTY ");
        let detail_inner = detail_panel.inner(detail);
        let detail_text = Paragraph::new(clean_multiline(&report)).wrap(Wrap { trim: false });
        let max_scroll = lines.len().saturating_sub(inner.height as usize).max(
            detail_text
                .line_count(detail_inner.width.max(1))
                .saturating_sub(detail_inner.height as usize),
        );
        self.scroll = self.scroll.min(max_scroll);
        let scroll = self.scroll.min(u16::MAX as usize) as u16;
        frame.render_widget(
            Paragraph::new(lines).block(panel).scroll((scroll, 0)),
            scores,
        );
        frame.render_widget(detail_text.block(detail_panel).scroll((scroll, 0)), detail);
    }
    fn draw_dashboard(&mut self, frame: &mut Frame, area: Rect) {
        use crate::ui;
        use ratatui::{
            style::Modifier,
            text::{Line, Span},
            widgets::{LineGauge, Sparkline},
        };
        let [metrics, details] =
            Layout::vertical([Constraint::Length(4), Constraint::Min(0)]).areas(area);
        let cards = Layout::horizontal([Constraint::Ratio(1, 4); 4]).split(metrics);
        let sample = &self.loaded.manifest.samples[self.sample];
        let qc = sample.assays[0].qc.as_ref();
        for (i, (title, value, color)) in [
            (
                " SAMPLE ",
                format!(
                    "{} / {}",
                    self.sample + 1,
                    self.loaded.manifest.samples.len()
                ),
                ui::BLUE,
            ),
            (
                " MEASURED ",
                qc.map(|q| q.measured_values.to_string())
                    .unwrap_or_else(|| "—".into()),
                ui::ACCENT,
            ),
            (
                " MAPPED ",
                qc.and_then(|q| q.gene_id_mapping_rate)
                    .map(|r| format!("{:.1}%", r * 100.))
                    .unwrap_or_else(|| "—".into()),
                ui::ACCENT,
            ),
            (
                " QUALITY ",
                qc.map(|q| format!("{:?}", q.status))
                    .unwrap_or_else(|| "Unknown".into()),
                if qc.is_some_and(|q| q.status == QcStatus::Blocked) {
                    ui::RED
                } else {
                    ui::GOLD
                },
            ),
        ]
        .into_iter()
        .enumerate()
        {
            frame.render_widget(
                Paragraph::new(value)
                    .style(Style::default().fg(color).add_modifier(Modifier::BOLD))
                    .block(ui::panel(title)),
                cards[i],
            );
        }
        let [left, right] =
            Layout::horizontal([Constraint::Percentage(56), Constraint::Percentage(44)])
                .areas(details);
        let paragraph =
            Paragraph::new(clean_multiline(&self.sample_text())).wrap(Wrap { trim: false });
        let panel = ui::panel(" SAMPLE / PROVENANCE ");
        let inner = panel.inner(left);
        self.scroll = self.scroll.min(
            paragraph
                .line_count(inner.width.max(1))
                .saturating_sub(inner.height as usize),
        );
        frame.render_widget(
            paragraph
                .block(panel)
                .scroll((self.scroll.min(u16::MAX as usize) as u16, 0)),
            left,
        );
        let [quality, distribution, pipeline] = Layout::vertical([
            Constraint::Length(4),
            Constraint::Length(6),
            Constraint::Min(0),
        ])
        .areas(right);
        frame.render_widget(
            LineGauge::default()
                .block(ui::panel(" GENE ID COVERAGE "))
                .ratio(
                    qc.and_then(|q| q.gene_id_mapping_rate)
                        .unwrap_or(0.)
                        .clamp(0., 1.),
                )
                .label("Mapped / all records")
                .filled_style(Style::default().fg(ui::ACCENT))
                .unfilled_style(Style::default().fg(ui::RAISED)),
            quality,
        );
        let values: Vec<_> = self.loaded.records[self.sample]
            .iter()
            .filter_map(|r| r.raw_expression)
            .filter(|v| v.is_finite())
            .collect();
        let min = values.iter().copied().reduce(f64::min).unwrap_or(0.);
        let max = values.iter().copied().reduce(f64::max).unwrap_or(0.);
        let mut bins = [0u64; 16];
        for v in &values {
            let i = if max > min {
                (((v - min) / (max - min) * 16.) as usize).min(15)
            } else {
                0
            };
            bins[i] += 1;
        }
        let display_bins: Vec<_> = (0..distribution.width.saturating_sub(2) as usize)
            .map(|i| bins[(i * 16 / distribution.width.saturating_sub(2).max(1) as usize).min(15)])
            .collect();
        frame.render_widget(
            Sparkline::default()
                .data(&display_bins)
                .style(Style::default().fg(ui::BLUE))
                .block(ui::panel(format!(
                    " RAW HISTOGRAM · 16 bins · {min:.2}…{max:.2} "
                ))),
            distribution,
        );
        let phase = if self.comparison.is_some() {
            "● REFERENCE COMPARED"
        } else {
            "○ COMPARE REFERENCE"
        };
        frame.render_widget(
            Paragraph::new(vec![
                Line::from(Span::styled(
                    "● IMPORT → MAP → QC",
                    Style::default().fg(ui::ACCENT),
                )),
                Line::from(phase),
                Line::from("○ JEV · request preview only"),
                Line::from("○ REVIEW EVIDENCE + UNCERTAINTY"),
            ])
            .block(ui::panel(" ANALYSIS PATH "))
            .wrap(Wrap { trim: false }),
            pipeline,
        );
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
            format!("Data provenance: {:?}", s.data_class),
        ];
        if let Some(q) = &a.qc {
            lines.push(format!("\nQUALITY: {:?}\nRecords: {} | measured: {} | missing: {} | zeros: {}\nMapped: {} | ambiguous: {} | unmapped: {}\nInvalid values: {} | duplicate genes: {}\nGene-ID mapping rate: {}\nRaw range: {} .. {}",q.status,q.total_records,q.measured_values,q.missing_values,q.zero_values,q.mapped_records,q.ambiguous_records,q.unmapped_records,q.invalid_values,q.duplicate_gene_records,q.gene_id_mapping_rate.map(|v|format!("{:.1}% of all source records",v*100.0)).unwrap_or_else(||"not assessed".into()),q.minimum.map(|v|format!("{v:.4}")).unwrap_or_else(||"unknown".into()),q.maximum.map(|v|format!("{v:.4}")).unwrap_or_else(||"unknown".into())));
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
        lines.push(format!("\nPROVENANCE\nSource: {}\nSource hash: {}\nMeasurements: {}\nMeasurement hash: {}\nGene dictionary: {}\n\nThis section shows the selected dataset assay.\nReference comparison: Data > Analyze.\nMolecular inputs and results: F2 Analysis.",self.loaded.manifest.source_name,self.loaded.manifest.source.artifact.sha256,a.artifact.path,a.artifact.sha256,self.loaded.manifest.gene_map.as_ref().map(|m|m.release.as_str()).unwrap_or("not supplied")));
        lines.join("\n")
    }
    fn draw_features(&mut self, frame: &mut Frame, area: Rect) {
        let data = &self.loaded.records[self.sample];
        let height = area.height.saturating_sub(4) as usize;
        self.scroll = self
            .scroll
            .min(self.visible.len().saturating_sub(height.max(1)));
        let rows = self
            .visible
            .iter()
            .skip(self.scroll)
            .take(height)
            .enumerate()
            .map(|(index, i)| {
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
                .style(
                    Style::default()
                        .bg(if index % 2 == 0 {
                            crate::ui::PANEL
                        } else {
                            crate::ui::RAISED
                        })
                        .fg(crate::ui::TEXT),
                )
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
            .block(crate::ui::panel(format!(
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
        crate::ui::shadow(frame, rect);
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
            Dialog::Reference(s) => (
                "Open reference JSON",
                format!("{s}\n\nEnter to open · Esc cancel · F1 Guide"),
            ),
            Dialog::SaveRequest(s) => (
                "Save offline Jev request to NEW file",
                format!("{s}\n\nEnter to export · Esc cancel · F1 Guide"),
            ),
            Dialog::Save(s) => (
                if self.tab == 2 && self.comparison.is_some() {
                    "Export comparison evidence to NEW file"
                } else {
                    "Export manifest to NEW file"
                },
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
                .block(crate::ui::panel(format!(" {title} ")))
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
#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::{Terminal, backend::TestBackend};
    use std::time::Duration;
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
        assert!(t.contains("Jev Onco Statistical Hierarchy"));
        assert!(t.contains("Molecular Data Workbench"));
        assert!(!t.contains("SYNTHETIC DATA"));
        assert!(t.contains("Samples"));
        assert!(!t.contains("NICE"));
        assert!(t.contains("GENE ID COVERAGE"));
        assert!(t.contains("Synthetic"));
    }
    #[test]
    fn guide_preserves_forms_and_never_submits_or_edits_underlying_input() {
        let mut app = Workbench::new().unwrap();
        for tab in 0..7 {
            app.tab = tab;
            app.key(key('g'));
            assert!(app.guide.open);
            app.key(key('g'));
            assert!(!app.guide.open);
        }
        app.key(key('i'));
        app.paste("/tmp/gene-input.tsv".into());
        app.key(KeyEvent::new(KeyCode::F(1), KeyModifiers::NONE));
        app.key(key('/'));
        app.paste("gene.*map".into());
        app.key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
        app.key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
        assert!(app.pending.is_none());
        app.key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
        assert!(matches!(&app.dialog,Dialog::Import(f) if f.values[0]=="/tmp/gene-input.tsv"));
        app.key(key('g'));
        assert!(matches!(&app.dialog,Dialog::Import(f) if f.values[0].ends_with("tsvg")));
    }
    #[tokio::test]
    async fn guide_remains_available_during_pending_job_and_preserves_completion() {
        let mut app = Workbench::new().unwrap();
        app.open(PathBuf::from("/missing-reference-test"));
        app.key(key('g'));
        assert!(app.guide.open);
        for _ in 0..200 {
            app.poll().await;
            if app.pending.is_none() {
                break;
            }
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
        assert!(app.guide.open);
        assert!(app.pending.is_none());
        app.key(key('g'));
        assert_eq!(app.loaded.manifest.dataset_id, "synthetic-expression-demo");
    }
    #[tokio::test]
    async fn reference_jobs_comparison_export_and_sample_changes_stay_consistent() {
        use josh_core::reference::*;
        let mut app = Workbench::new().unwrap();
        let map = josh_features::reference::measured(&app.loaded.records[0]).unwrap();
        let genes: Vec<_> = map.values().map(|(g, _)| g.clone()).collect();
        let values: Vec<_> = map.values().map(|(_, v)| *v).collect();
        let r = ReferenceRelease {
            schema_version: 1,
            pipeline_version: REFERENCE_PIPELINE.into(),
            release_id: "UI-test".into(),
            citation: "synthetic test".into(),
            created_at_unix_seconds: 0,
            synthetic: true,
            source_dataset_id: "reference".into(),
            source_manifest_sha256: "a".repeat(64),
            source_input_sha256: "a".repeat(64),
            source_labels_sha256: "a".repeat(64),
            compatibility: Compatibility {
                organism: "Homo sapiens".into(),
                units: ExpressionUnit::Tpm,
                transform: Transform::Log2OnePlus,
                platform: "invented demonstration".into(),
                reference_genome: None,
                gene_map_sha256: app
                    .loaded
                    .manifest
                    .gene_map
                    .as_ref()
                    .unwrap()
                    .artifact
                    .sha256
                    .clone(),
            },
            minimum_genes: 3,
            minimum_overlap: 0.8,
            genes,
            members: vec![
                ReferenceMember {
                    sample_id: "REF-A".into(),
                    patient_group_id: "GROUP-A".into(),
                    class_id: "demo_a".into(),
                    measurement_sha256: "a".repeat(64),
                },
                ReferenceMember {
                    sample_id: "REF-B".into(),
                    patient_group_id: "GROUP-B".into(),
                    class_id: "demo_b".into(),
                    measurement_sha256: "b".repeat(64),
                },
            ],
            classes: vec![
                ReferenceClass {
                    class_id: "demo_a".into(),
                    cancer_type: "Demonstration A".into(),
                    samples: 1,
                    mean_expression: values.clone(),
                },
                ReferenceClass {
                    class_id: "demo_b".into(),
                    cancer_type: "Demonstration B".into(),
                    samples: 1,
                    mean_expression: values.into_iter().rev().collect(),
                },
            ],
            limitations: vec!["test only".into()],
        };
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("reference.json");
        std::fs::write(&path, serde_json::to_vec(&r).unwrap()).unwrap();
        app.open_reference(path);
        for _ in 0..200 {
            app.poll().await;
            if app.pending.is_none() {
                break;
            }
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
        assert!(app.reference.is_some());
        app.key(key('a'));
        for _ in 0..200 {
            app.poll().await;
            if app.pending.is_none() {
                break;
            }
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
        assert!(matches!(
            app.comparison.as_ref().unwrap().status,
            ComparisonStatus::Compared
        ));
        let analysis = app.analysis_input().unwrap();
        assert_eq!(
            analysis.features.sample_id,
            app.loaded.manifest.samples[app.sample].sample_id
        );
        assert_eq!(analysis.features.features.len(), 1);
        assert!(matches!(
            analysis.features.features[0].value,
            Some(josh_core::molecular::FeatureValue::Vector { .. })
        ));
        let mut terminal = Terminal::new(TestBackend::new(120, 40)).unwrap();
        terminal.draw(|f| app.draw(f)).unwrap();
        assert!(text(&terminal).contains("REFERENCE SIMILARITY"));
        let export = tmp.path().join("request.json");
        app.dialog = Dialog::SaveRequest(export.to_str().unwrap().into());
        app.key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
        let v: serde_json::Value = serde_json::from_slice(&std::fs::read(export).unwrap()).unwrap();
        assert_eq!(v["sends_to_provider"], false);
        assert!(v["request_sha256"].as_str().is_some());
        app.key(key(']'));
        assert!(app.comparison.is_none());
        assert!(app.reference.is_some());
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

//! Sample-scoped ownership. Different input contracts never share a prediction.
use crate::{molecular_tui, tui, workbench, workflows::AppError};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Page {
    #[default]
    Data,
    Results,
}

pub enum Input {
    Empty,
    Molecular(Box<molecular_tui::App>),
    Expression {
        data: Box<workbench::Workbench>,
        analysis: Option<Box<molecular_tui::App>>,
        binding: Option<String>,
    },
    /// Compatibility only: never converted into the molecular taxonomy.
    Legacy(Box<tui::App>),
}

pub struct Session {
    pub input: Input,
    pub page: Page,
    pub chart: usize,
    pub previous_runs: Vec<molecular_tui::App>,
    pub notice: String,
}
impl Default for Session {
    fn default() -> Self {
        Self::new(Input::Empty)
    }
}
impl Session {
    pub fn new(mut input: Input) -> Self {
        if let Input::Legacy(a) = &mut input {
            a.enter_compatibility();
        }
        let page = match &input {
            Input::Molecular(a) if a.has_result() => Page::Results,
            _ => Page::Data,
        };
        Self {
            input,
            page,
            chart: 0,
            previous_runs: vec![],
            notice: String::new(),
        }
    }
    pub fn molecular(a: molecular_tui::App) -> Self {
        Self::new(Input::Molecular(Box::new(a)))
    }
    pub fn expression(data: workbench::Workbench) -> Self {
        Self::new(Input::Expression {
            data: Box::new(data),
            analysis: None,
            binding: None,
        })
    }
    pub fn analysis(&self) -> Option<&molecular_tui::App> {
        match &self.input {
            Input::Molecular(a) => Some(a),
            Input::Expression { analysis, .. } => analysis.as_deref(),
            _ => None,
        }
    }
    pub fn analysis_mut(&mut self) -> Option<&mut molecular_tui::App> {
        match &mut self.input {
            Input::Molecular(a) => Some(a),
            Input::Expression { analysis, .. } => analysis.as_deref_mut(),
            _ => None,
        }
    }
    pub fn data(&self) -> Option<&workbench::Workbench> {
        match &self.input {
            Input::Expression { data, .. } => Some(data),
            _ => None,
        }
    }
    pub fn data_mut(&mut self) -> Option<&mut workbench::Workbench> {
        match &mut self.input {
            Input::Expression { data, .. } => Some(data),
            _ => None,
        }
    }
    pub fn legacy(&self) -> Option<&tui::App> {
        if let Input::Legacy(a) = &self.input {
            Some(a)
        } else {
            None
        }
    }
    pub fn legacy_mut(&mut self) -> Option<&mut tui::App> {
        if let Input::Legacy(a) = &mut self.input {
            Some(a)
        } else {
            None
        }
    }
    pub fn ready(&self) -> bool {
        match &self.input {
            Input::Empty => false,
            Input::Molecular(a) => a.has_input(),
            Input::Expression { data, .. } => data.has_input(),
            Input::Legacy(_) => true,
        }
    }
    pub fn busy(&self) -> bool {
        self.analysis().is_some_and(|a| a.busy())
            || self.data().is_some_and(|d| d.busy())
            || self.legacy().is_some_and(|a| a.busy())
    }
    pub fn editing(&self) -> bool {
        self.analysis().is_some_and(|a| a.editing())
            || self.data().is_some_and(|d| d.editing())
            || self.legacy().is_some_and(|a| a.editing())
    }
    pub fn context(&self) -> String {
        match &self.input {
            Input::Empty => "No sample loaded · research use only".into(),
            Input::Molecular(a) => a.context(),
            Input::Expression { data, .. } => data.context(),
            Input::Legacy(a) => format!("LEGACY · {} · separate clinical contract", a.context()),
        }
    }
    pub fn status(&self) -> String {
        if !self.notice.is_empty() {
            return self.notice.clone();
        }
        if let Some(a) = self
            .analysis()
            .filter(|a| a.busy() || a.editing() || self.page == Page::Results)
        {
            return a.status();
        }
        match &self.input {
            Input::Empty => {
                "Open data or try a synthetic sample. No data is sent until you confirm a request."
                    .into()
            }
            Input::Molecular(a) => a.status(),
            Input::Expression { data, .. } => data.status(),
            Input::Legacy(a) => a.status(),
        }
    }
    pub fn use_expression(&mut self) -> Result<(), AppError> {
        if self.busy() {
            return Err("Wait for the current job.".into());
        }
        let Input::Expression {
            data,
            analysis,
            binding,
        } = &mut self.input
        else {
            return Err("Open expression data first.".into());
        };
        let next = data.analysis_input()?;
        // The producer binds the exact selected sample; do not link on a display label.
        if data.sample_id() != Some(next.features.sample_id.as_str()) {
            return Err("Expression sample identity mismatch; no analysis was attached.".into());
        }
        let next_binding = data
            .binding()
            .ok_or("Expression provenance could not be verified.")?;
        if let Some(old) = analysis.take() {
            self.previous_runs.push(*old);
        }
        *analysis = Some(Box::new(next));
        *binding = Some(next_binding);
        self.page = Page::Data;
        self.notice = "Expression-only profile prepared for this sample. No genomic data or clinical probabilities were merged. Run analysis requires confirmation.".into();
        Ok(())
    }
    pub fn reconcile(&mut self) {
        if let Input::Expression {
            data,
            analysis,
            binding,
        } = &mut self.input
            && analysis.is_some()
            && *binding != data.binding()
        {
            self.previous_runs.push(*analysis.take().expect("present"));
            *binding = None;
            self.page = Page::Data;
            self.chart = 0;
            self.notice = "Evidence changed. Previous analysis moved to Menu > Earlier runs; it is not a result for this input. Compare and prepare this sample again.".into();
        }
    }
    pub async fn poll(&mut self) {
        let was_analysis_busy = self.analysis().is_some_and(|a| a.busy());
        if let Some(data) = self.data_mut() {
            data.poll().await;
        }
        if let Some(a) = self.analysis_mut() {
            a.poll().await;
        }
        if let Some(a) = self.legacy_mut() {
            a.poll_result().await;
        }
        self.reconcile();
        if was_analysis_busy && self.analysis().is_some_and(|a| !a.busy() && a.has_result()) {
            self.page = Page::Results;
            self.notice.clear();
        }
    }
    pub fn cancel(&mut self) {
        if let Some(a) = self.analysis() {
            a.cancel_job();
        }
        if let Some(d) = self.data_mut() {
            d.cancel_job();
        }
    }
    pub fn set_page(&mut self, page: Page) {
        self.page = page;
        if let Some(a) = self.analysis_mut() {
            a.key(KeyEvent::new(KeyCode::Home, KeyModifiers::NONE));
        }
        if let Some(a) = self.legacy_mut() {
            a.set_view(if page == Page::Data { 0 } else { 5 });
        }
    }
}

//! The single terminal application: shared navigation, loader, help and job lifecycle.
use crate::{
    molecular_tui, study_tui, tui, ui, workbench,
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

pub struct App {
    analysis: molecular_tui::App,
    data: workbench::Workbench,
    clinical: tui::App,
    cohort: study_tui::App,
    section: Section,
    guide: crate::guide::Guide,
    load_input: Option<String>,
    status: String,
    navigation: Vec<(Rect, Option<Section>)>,
    confirm_exit: bool,
    quitting: bool,
}

impl App {
    pub fn new(input: Option<PathBuf>) -> Result<Self, AppError> {
        let mut app = Self {
            analysis: molecular_tui::App::new(None, None)?,
            data: workbench::Workbench::new()?,
            clinical: tui::App::new(None)?,
            cohort: study_tui::App::default(),
            section: Section::Analysis,
            guide: crate::guide::Guide::default(),
            load_input: None,
            status: "l Load · F2 Analysis · F3 Data · F4 Clinical · F5 Cohort · F1 Help".into(),
            navigation: vec![],
            confirm_exit: false,
            quitting: false,
        };
        if let Some(path) = input {
            app.open(&path)?;
        }
        Ok(app)
    }
    pub fn section(&self) -> Section {
        self.section
    }
    pub fn busy(&self) -> bool {
        self.analysis.busy() || self.data.busy() || self.clinical.busy() || self.cohort.busy()
    }
    fn editing(&self) -> bool {
        match self.section {
            Section::Analysis => self.analysis.editing(),
            Section::Data => self.data.editing(),
            Section::Clinical => self.clinical.editing(),
            Section::Cohort => self.cohort.editing(),
        }
    }
    fn select(&mut self, section: Section) {
        self.section = section;
        self.status = "Sections share this session. Switching preserves inputs, forms, results and running jobs.".into();
    }
    fn begin_load(&mut self) {
        if self.busy() {
            self.status = "Wait for the current job before replacing data. You can browse all sections while it runs.".into();
        } else {
            self.load_input = Some(String::new());
        }
    }
    /// One loader for every supported input; interpretation remains in the existing validators.
    pub fn open(&mut self, path: &Path) -> Result<(), AppError> {
        if self.busy() {
            return Err("Wait for the current job before loading new data.".into());
        }
        if path == Path::new("-") {
            return Err(
                "Choose a file or directory; terminal stdin is reserved for keyboard input.".into(),
            );
        }
        if path.is_dir() {
            if path.join("dataset.json").is_file() {
                self.data.open_dataset(path.into());
                self.section = Section::Data;
            } else if path.join("manifest.json").is_file() {
                self.replace_clinical(tui::App::from_batch(path.into())?)?;
            } else {
                self.analysis.open_path(path)?;
                self.section = Section::Analysis;
            }
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
                        |_| "Cannot read table headers; use a CSV or TSV with a header row.",
                    )?;
                    if headers.iter().any(|v| v == "case_id") {
                        return Err("Case tables must first be imported with josh import; load the resulting bundle here.".into());
                    }
                    headers.iter().any(|v| matches!(v, "modality" | "id"))
                }
            };
            if molecular {
                self.analysis.open_path(path)?;
                self.section = Section::Analysis;
            } else {
                self.data.import_path(path);
                self.section = Section::Data;
            }
        } else {
            let value: serde_json::Value = workflows::read_json(path, 64 * 1024 * 1024)?;
            if matches!(
                value.get("kind").and_then(|v| v.as_str()),
                Some("josh_cohort_study" | "josh_cohort_report")
            ) {
                self.cohort.open(path);
                self.section = Section::Cohort;
            } else if value.get("case_id").is_some() {
                self.replace_clinical(tui::App::new(Some(path.into()))?)?;
            } else if path.file_name().is_some_and(|s| s == "dataset.json") {
                self.data
                    .open_dataset(path.parent().unwrap_or(Path::new(".")).into());
                self.section = Section::Data;
            } else if value.get("cases").is_some()
                && path.file_name().is_some_and(|s| s == "manifest.json")
            {
                self.replace_clinical(tui::App::from_batch(
                    path.parent().unwrap_or(Path::new(".")).into(),
                )?)?;
            } else {
                self.analysis.open_path(path)?;
                self.section = Section::Analysis;
            }
        }
        self.status = format!(
            "Opened {} in this workspace. F2/F3/F4/F5 switch sections.",
            workflows::display_text(&path.display().to_string())
        );
        Ok(())
    }
    fn replace_clinical(&mut self, next: tui::App) -> Result<(), AppError> {
        if self.clinical.unsaved_reviews() {
            return Err("Clinical reviews are unsaved. Use F4, then 8 and s to save before replacing the case.".into());
        }
        self.clinical = next;
        self.section = Section::Clinical;
        Ok(())
    }
    fn attach_comparison(&mut self) -> Result<(), AppError> {
        if self.busy() {
            return Err("Wait for the current job before changing the analysis input.".into());
        }
        let next = self.data.analysis_input()?;
        self.analysis = next;
        self.section = Section::Analysis;
        self.status =
            "Expression comparison attached to Analysis. Review this sample, then a runs Jev."
                .into();
        Ok(())
    }
    pub async fn poll(&mut self) {
        // Hidden sections keep progressing; switching tabs never drops a task or result.
        self.analysis.poll().await;
        self.data.poll().await;
        self.clinical.poll_result().await;
        self.cohort.poll().await;
    }
    pub fn exit_ready(&self) -> bool {
        self.quitting && !self.busy()
    }
    fn start_quit(&mut self) {
        self.quitting = true;
        self.analysis.cancel_job();
        self.data.cancel_job();
        self.cohort.cancel_job();
        self.status = "Finishing in-flight work before closing the workspace.".into();
    }
    fn request_quit(&mut self) {
        if self.clinical.unsaved_reviews() {
            self.confirm_exit = true;
        } else {
            self.start_quit();
        }
    }
    pub fn key(&mut self, key: KeyEvent) -> bool {
        if key.kind != KeyEventKind::Press {
            return false;
        }
        if self.quitting {
            return self.exit_ready();
        }
        if self.guide.key(
            key,
            self.load_input.is_some() || self.confirm_exit || self.editing(),
        ) {
            return false;
        }
        if self.confirm_exit {
            match key.code {
                KeyCode::Char('y' | 'Y') => {
                    self.confirm_exit = false;
                    self.start_quit();
                }
                KeyCode::Esc | KeyCode::Char('n' | 'N') => self.confirm_exit = false,
                _ => (),
            }
            return self.exit_ready();
        }
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
            self.request_quit();
            return self.exit_ready();
        }
        if let Some(input) = &mut self.load_input {
            match key.code {
                KeyCode::Esc => self.load_input = None,
                KeyCode::Backspace => {
                    input.pop();
                }
                KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    input.clear()
                }
                KeyCode::Char(c)
                    if !c.is_control()
                        && !key.modifiers.contains(KeyModifiers::CONTROL)
                        && input.len() + c.len_utf8() <= 4096 =>
                {
                    input.push(c)
                }
                KeyCode::Enter => {
                    let path = PathBuf::from(input.trim());
                    if path.as_os_str().is_empty() {
                        self.status = "Enter a file or folder path.".into();
                    } else {
                        match self.open(&path) {
                            Ok(()) => self.load_input = None,
                            Err(e) => self.status = e.to_string(),
                        }
                    }
                }
                _ => (),
            }
            return false;
        }
        match key.code {
            KeyCode::F(2) => self.select(Section::Analysis),
            KeyCode::F(3) => self.select(Section::Data),
            KeyCode::F(4) => self.select(Section::Clinical),
            KeyCode::F(5) => self.select(Section::Cohort),
            KeyCode::Esc if !self.editing() => (),
            KeyCode::Char('q') if !self.editing() => self.request_quit(),
            KeyCode::Char('l' | 'o') if !self.editing() => self.begin_load(),
            KeyCode::Char('m') if self.section == Section::Data && !self.editing() => {
                if let Err(e) = self.attach_comparison() {
                    self.status = e.to_string();
                }
            }
            _ => {
                let exit = match self.section {
                    Section::Analysis => self.analysis.key(key),
                    Section::Data => self.data.key(key),
                    Section::Clinical => self.clinical.key(key),
                    Section::Cohort => self.cohort.key(key),
                };
                // Esc closes editors, but never tears down a section or its state.
                if exit && key.code != KeyCode::Esc {
                    self.request_quit();
                }
            }
        }
        self.exit_ready()
    }
    pub fn paste(&mut self, text: &str) {
        if self.quitting || self.confirm_exit {
            return;
        }
        if self.guide.open {
            self.guide.paste(text);
            return;
        }
        if let Some(input) = &mut self.load_input {
            let text = text.trim().trim_matches('"').trim_matches('\'');
            if !text.chars().any(char::is_control) && input.len() + text.len() <= 4096 {
                input.push_str(text);
            } else {
                self.status = "Enter a single path of at most 4096 bytes.".into();
            }
        } else {
            match self.section {
                Section::Analysis => self.analysis.paste(text),
                Section::Data => self.data.paste(text.into()),
                Section::Clinical => self.clinical.paste(text),
                Section::Cohort => self.cohort.paste(text),
            }
        }
    }
    pub fn mouse(&mut self, event: MouseEvent) {
        if self.guide.open || self.load_input.is_some() || self.confirm_exit || self.quitting {
            return;
        }
        if event.kind == MouseEventKind::Down(MouseButton::Left)
            && let Some((_, section)) = self
                .navigation
                .iter()
                .find(|(r, _)| r.contains((event.column, event.row).into()))
        {
            if let Some(section) = *section {
                self.select(section);
            } else {
                self.begin_load();
            }
            return;
        }
        if self.section == Section::Analysis {
            if self.analysis.load_button(event) {
                self.begin_load();
            } else {
                self.analysis.mouse(event);
            }
        } else if self.section == Section::Cohort {
            self.cohort.mouse(event);
        }
    }
    pub fn draw(&mut self, frame: &mut Frame) {
        let [title, navigation, body, status] = Layout::vertical([
            Constraint::Length(1),
            Constraint::Length(3),
            Constraint::Min(1),
            Constraint::Length(2),
        ])
        .areas(frame.area());
        frame.render_widget(
            Block::default().style(Style::default().bg(ui::BG).fg(ui::TEXT)),
            frame.area(),
        );
        frame.render_widget(
            Paragraph::new("Jev Onco Statistical Hierarchy · Workspace")
                .style(Style::default().fg(ui::ACCENT).bold()),
            title,
        );
        let areas = Layout::horizontal([Constraint::Ratio(1, 5); 5]).split(navigation);
        self.navigation.clear();
        for ((rect, label), section) in areas
            .iter()
            .zip([
                "Load [l]",
                "Analysis [F2]",
                "Data [F3]",
                "Clinical [F4]",
                "Cohort [F5]",
            ])
            .zip([
                None,
                Some(Section::Analysis),
                Some(Section::Data),
                Some(Section::Clinical),
                Some(Section::Cohort),
            ])
        {
            frame.render_widget(
                Paragraph::new(label)
                    .style(Style::default().fg(if section == Some(self.section) {
                        ui::ACCENT
                    } else {
                        ui::MUTED
                    }))
                    .block(ui::panel("")),
                *rect,
            );
            self.navigation.push((*rect, section));
        }
        match self.section {
            Section::Analysis => self.analysis.draw_area(frame, body, true),
            Section::Data => self.data.draw_area(frame, body, true),
            Section::Clinical => self.clinical.draw_area(frame, body, true),
            Section::Cohort => self.cohort.draw_area(frame, body),
        }
        let jobs = if self.busy() { "Job running · " } else { "" };
        let dirty = if self.clinical.unsaved_reviews() {
            "Unsaved clinical reviews · "
        } else {
            ""
        };
        frame.render_widget(
            Paragraph::new(format!(
                "{jobs}{dirty}{}",
                workflows::display_text(&self.status)
            ))
            .wrap(Wrap { trim: false })
            .style(Style::default().fg(ui::GOLD)),
            status,
        );
        if self.load_input.is_some() || self.confirm_exit {
            let area = frame.area();
            let width = area.width.saturating_sub(2).min(100);
            let height = area.height.min(12);
            let rect = Rect::new(
                area.x + (area.width - width) / 2,
                area.y + (area.height - height) / 2,
                width,
                height,
            );
            let (title, text) = if let Some(input) = &self.load_input {
                (
                    "Load into this workspace",
                    format!(
                        "Molecular, clinical or cohort-study JSON, CSV/TSV, MAF/VCF, dataset/case bundle, or saved run. The file selects the section.\n\n{}\n\nEnter: load · Ctrl+u: clear · Esc: cancel\n\n{}",
                        workflows::display_text(input),
                        workflows::display_text(&self.status)
                    ),
                )
            } else {
                ("Unsaved clinical reviews", "Clinical reviews are unsaved, even if another section is visible.\n\ny: discard and quit · n/Esc: return\nUse F4, then 8 and s to save them.".into())
            };
            frame.render_widget(Clear, rect);
            frame.render_widget(
                Paragraph::new(text)
                    .wrap(Wrap { trim: false })
                    .block(ui::panel(title)),
                rect,
            );
        }
        self.guide.draw(frame);
    }
}

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

/// The only terminal initialization and event loop in the application.
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
                Event::Key(key) if app.key(key) => break,
                Event::Paste(text) => app.paste(&text),
                Event::Mouse(event) => app.mouse(event),
                _ => (),
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::{Terminal, backend::TestBackend};
    fn fixture(name: &str) -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join(name)
    }
    fn press(app: &mut App, key: KeyCode) -> bool {
        app.key(KeyEvent::new(key, KeyModifiers::NONE))
    }
    fn screen(app: &mut App, width: u16, height: u16) -> String {
        let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
        terminal.draw(|f| app.draw(f)).unwrap();
        terminal
            .backend()
            .buffer()
            .content
            .iter()
            .map(|c| c.symbol())
            .collect()
    }
    async fn settle(app: &mut App) {
        for _ in 0..1000 {
            app.poll().await;
            if !app.busy() {
                return;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        panic!("job did not complete");
    }
    #[tokio::test]
    async fn section_navigation_preserves_and_polls_one_analysis_even_when_hidden() {
        let mut app = App::new(None).unwrap();
        press(&mut app, KeyCode::Char('d'));
        assert!(app.busy());
        press(&mut app, KeyCode::F(3));
        assert_eq!(app.section, Section::Data);
        settle(&mut app).await;
        let original = app
            .analysis
            .archive
            .as_ref()
            .unwrap()
            .inference
            .request_sha256
            .clone();
        press(&mut app, KeyCode::F(4));
        press(&mut app, KeyCode::F(2));
        assert_eq!(
            app.analysis
                .archive
                .as_ref()
                .unwrap()
                .inference
                .request_sha256,
            original
        );
        assert!(screen(&mut app, 120, 40).contains("RANKED OUTCOMES"));
        assert!(!press(&mut app, KeyCode::Esc));
        assert!(!app.exit_ready());
        assert!(app.analysis.archive.is_some());
    }
    #[test]
    fn single_loader_routes_files_and_preserves_editors_and_shared_help() {
        let mut app =
            App::new(Some(fixture("fixtures/molecular/synthetic-features.json"))).unwrap();
        assert_eq!(app.section, Section::Analysis);
        let sample = app.analysis.features.sample_id.clone();
        app.open(&fixture("fixtures/clinical-case.json")).unwrap();
        assert_eq!(app.section, Section::Clinical);
        app.open(&fixture("fixtures/expression/synthetic-expression.tsv"))
            .unwrap();
        assert_eq!(app.section, Section::Data);
        assert!(app.data.editing());
        press(&mut app, KeyCode::F(2));
        assert_eq!(app.analysis.features.sample_id, sample);
        press(&mut app, KeyCode::F(3));
        assert!(app.data.editing());
        press(&mut app, KeyCode::F(1));
        assert!(app.guide.open);
        press(&mut app, KeyCode::Esc);
        assert!(app.data.editing());
        assert!(screen(&mut app, 200, 40).contains("synthetic-expression.tsv"));
        press(&mut app, KeyCode::Esc);
        app.open(&fixture("fixtures/basic-demo/molecular-complete.tsv"))
            .unwrap();
        assert_eq!(app.section, Section::Analysis);
        assert!(app.analysis.editing());
        press(&mut app, KeyCode::F(4));
        press(&mut app, KeyCode::F(2));
        assert!(app.analysis.editing());
        assert!(app.open(Path::new("/missing-file.json")).is_err());
        assert_eq!(app.analysis.features.sample_id, sample);
    }
    #[test]
    fn unsaved_clinical_reviews_are_protected_from_other_sections_and_global_load() {
        let mut app = App::new(None).unwrap();
        press(&mut app, KeyCode::F(4));
        press(&mut app, KeyCode::Char('7'));
        press(&mut app, KeyCode::Home);
        press(&mut app, KeyCode::Char('a'));
        app.paste("reviewer-1 | 2026-09-24 | acknowledged | Synthetic scope review only");
        press(&mut app, KeyCode::Enter);
        assert!(app.clinical.unsaved_reviews());
        press(&mut app, KeyCode::F(2));
        assert!(!press(&mut app, KeyCode::Char('q')));
        assert!(app.confirm_exit);
        assert!(screen(&mut app, 120, 40).contains("Unsaved clinical reviews"));
        press(&mut app, KeyCode::Char('n'));
        assert!(!app.exit_ready());
        assert!(app.open(&fixture("fixtures/clinical-case.json")).is_err());
        assert!(app.clinical.unsaved_reviews());
        press(&mut app, KeyCode::Char('q'));
        assert!(press(&mut app, KeyCode::Char('y')));
    }
    #[tokio::test]
    async fn saved_analysis_and_expression_bundles_open_in_the_same_session() {
        let temp = tempfile::tempdir().unwrap();
        let run = temp.path().join("run");
        let f = crate::molecular::example();
        let taxonomy = josh_core::molecular::onconpc_taxonomy();
        let r = josh_core::molecular::interpret(
            &f,
            &taxonomy,
            crate::molecular::demo_response(&josh_core::molecular::prepare(&f, &taxonomy).unwrap()),
            josh_core::Source::Mock,
        )
        .unwrap();
        crate::molecular::save_run(&run, &f, &r).unwrap();
        let mut app = App::new(Some(run)).unwrap();
        assert!(screen(&mut app, 120, 40).contains("RANKED OUTCOMES"));
        let data = josh_ingest::expression::import(
            include_bytes!("../../../fixtures/expression/synthetic-expression.tsv").to_vec(),
            &josh_ingest::expression::ImportOptions::default(),
            None,
        )
        .unwrap();
        let dataset = temp.path().join("dataset");
        josh_ingest::dataset::write(&dataset, &data).unwrap();
        app.open(&dataset).unwrap();
        assert_eq!(app.section, Section::Data);
        settle(&mut app).await;
        assert!(screen(&mut app, 120, 40).contains("Samples"));
        press(&mut app, KeyCode::F(2));
        assert!(screen(&mut app, 120, 40).contains("RANKED OUTCOMES"));
        assert!(app.attach_comparison().is_err()); // no accidental demo/sample substitution
        assert!(screen(&mut app, 120, 40).contains("RANKED OUTCOMES"));
    }
    #[test]
    fn navigation_mouse_and_small_terminals_use_the_shared_frame() {
        let mut app = App::new(None).unwrap();
        for (w, h) in [(140, 45), (80, 24), (60, 20), (20, 8), (1, 1)] {
            for section in [Section::Analysis, Section::Data, Section::Clinical] {
                app.select(section);
                screen(&mut app, w, h);
                app.begin_load();
                screen(&mut app, w, h);
                press(&mut app, KeyCode::Esc);
            }
        }
        screen(&mut app, 120, 40);
        app.mouse(MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: 40,
            row: 2,
            modifiers: KeyModifiers::NONE,
        });
        assert_eq!(app.section, Section::Analysis);
        app.mouse(MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: 5,
            row: 2,
            modifiers: KeyModifiers::NONE,
        });
        assert!(app.load_input.is_some());
        app.paste(fixture("fixtures/clinical-case.json").to_str().unwrap());
        press(&mut app, KeyCode::Enter);
        assert_eq!(app.section, Section::Clinical);
        assert!(app.load_input.is_none());
    }
}

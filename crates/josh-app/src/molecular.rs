//! Molecular CLI orchestration and bounded run archives, shared with the TUI.
use crate::workflows::{self, AppError};
use clap::{Subcommand, ValueEnum};
use josh_core::{
    Answer, DataClass, JevRequest, JevResponse, Question, Source, Usage, molecular::*,
};
use josh_explain::{Archive, Background, Config, Evaluator, Progress};
use std::{
    collections::BTreeMap,
    fs::{self, OpenOptions},
    io::{BufWriter, Write},
    path::{Path, PathBuf},
};

#[derive(Clone, Copy, ValueEnum)]
pub enum InputFormat {
    Csv,
    Tsv,
    Maf,
    Vcf,
}
#[derive(Clone, Copy, ValueEnum)]
pub enum Method {
    Exact,
    Permutation,
}
#[derive(Clone, Copy, ValueEnum)]
pub enum ExportFormat {
    Json,
    Csv,
    Svg,
}
#[derive(Subcommand)]
pub enum Command {
    /// Measure hosted full/baseline variation on a synthetic run; makes new billed calls.
    Repeatability {
        run_dir: PathBuf,
        #[arg(long)]
        out_dir: PathBuf,
        #[arg(long, default_value_t = 2)]
        replicates: usize,
        #[arg(long, default_value_t = 4)]
        max_evaluations: usize,
        #[arg(long, default_value_t = 50_000)]
        max_input_tokens: u64,
        #[arg(long, default_value_t = 120)]
        max_seconds: u64,
    },
    /// Compare ranking, signs and baseline sensitivity of two archived explanations offline.
    Stability {
        left: PathBuf,
        right: PathBuf,
        #[arg(long, default_value_t = 10)]
        top_k: usize,
    },
    /// Run/resume a bounded synthetic cohort with labels isolated from provider requests.
    Cohort {
        manifest: PathBuf,
        #[arg(long)]
        out_dir: PathBuf,
        #[arg(long, default_value_t = 100)]
        max_evaluations: usize,
        #[arg(long, default_value_t = 2_000_000)]
        max_input_tokens: u64,
        #[arg(long, default_value_t = 600)]
        max_seconds: u64,
    },
    /// Derive an experimental SBS96/NNLS fit from declared somatic SNVs and indexed FASTA.
    Signatures {
        features: PathBuf,
        #[arg(long)]
        fasta: PathBuf,
        #[arg(long)]
        fai: PathBuf,
        #[arg(long)]
        catalogue: PathBuf,
        #[arg(long)]
        opportunity_profile: String,
        #[arg(long, default_value_t = 50)]
        min_mutations: u64,
    },
    /// Attach a reproducible derived signature run; group it with its source variants.
    AttachSignatures {
        features: PathBuf,
        signatures: PathBuf,
    },
    /// Score frozen, labeled predictions locally; labels are never sent to Jev.
    Evaluate {
        records: PathBuf,
        #[arg(long)]
        taxonomy: Option<PathBuf>,
        #[arg(long, default_value = "test")]
        partition: String,
    },
    /// Print an invented molecular input (no prediction).
    Example,
    /// Show experimental detailed cancer classes and broad groups.
    Taxonomy,
    /// Import one sample; use --output to save the resulting feature set.
    Import {
        input: PathBuf,
        #[arg(long, value_enum)]
        input_format: InputFormat,
        #[arg(long)]
        sample: String,
        #[arg(long)]
        patient_group: String,
        #[arg(long)]
        source_id: String,
        #[arg(long)]
        assay: String,
        #[arg(long)]
        reference_build: Option<String>,
        #[arg(long)]
        synthetic: bool,
    },
    /// Join distinct modalities belonging to the same sample and patient group.
    Merge {
        #[arg(required=true,num_args=2..)]
        inputs: Vec<PathBuf>,
    },
    /// Attach a verified expression comparison as one inseparable feature vector.
    AttachExpression {
        features: PathBuf,
        evidence: PathBuf,
    },
    /// Validate features and preview the exact request without network access.
    Prepare {
        features: PathBuf,
        #[arg(long)]
        taxonomy: Option<PathBuf>,
    },
    /// Send one synthetic molecular sample to Jev and create a new run directory.
    Run {
        features: PathBuf,
        #[arg(long)]
        taxonomy: Option<PathBuf>,
        #[arg(long)]
        out_dir: PathBuf,
    },
    /// Produce a labeled analytical chart demonstration, never a Jev cancer prediction.
    Demo {
        #[arg(long)]
        out_dir: PathBuf,
    },
    /// Estimate explanation calls; no provider request.
    Plan {
        run_dir: PathBuf,
        #[command(flatten)]
        options: ExplainOptions,
    },
    /// Explain/resume an archived run; successful evaluations are checkpointed.
    Explain {
        run_dir: PathBuf,
        #[command(flatten)]
        options: ExplainOptions,
    },
    /// Verify and summarize a complete archive or a resumable checkpoint.
    Inspect { archive: PathBuf },
    /// Export validated explanation data or a standalone figure; use --output.
    Export {
        archive: PathBuf,
        #[arg(long, value_enum, default_value = "svg")]
        kind: ExportFormat,
    },
}
#[derive(clap::Args, Clone)]
pub struct ExplainOptions {
    /// Explicitly resend an interrupted/failed call; it may already have been billed.
    #[arg(long)]
    pub retry_uncertain: bool,
    #[arg(long, default_value = "NSCLC")]
    pub target: String,
    #[arg(long, value_enum, default_value = "permutation")]
    pub method: Method,
    #[arg(long, default_value_t = 32)]
    pub pairs: usize,
    #[arg(long, default_value_t = 42)]
    pub seed: u64,
    #[arg(long, default_value_t = 1024)]
    pub max_evaluations: usize,
    #[arg(long, default_value_t = 2_000_000)]
    pub max_input_tokens: u64,
    #[arg(long, default_value_t = 600)]
    pub max_seconds: u64,
    /// Compatible development-only background; omitted = masked-evidence Shapley.
    #[arg(long)]
    pub background: Option<PathBuf>,
}
impl ExplainOptions {
    pub fn config(&self) -> Config {
        Config {
            algorithm: match self.method {
                Method::Exact => josh_explain::Algorithm::Exact,
                Method::Permutation => josh_explain::Algorithm::Permutation,
            },
            baseline: if self.background.is_some() {
                josh_explain::Baseline::ReferenceBackground
            } else {
                josh_explain::Baseline::MaskedEvidence
            },
            target_class: self.target.clone(),
            permutation_pairs: self.pairs,
            seed: self.seed,
            max_evaluations: self.max_evaluations,
            max_input_tokens: self.max_input_tokens,
            max_seconds: self.max_seconds,
        }
    }
}
pub struct Output {
    pub value: serde_json::Value,
    pub text: String,
    pub raw: Option<String>,
}
pub fn load_features(path: &Path) -> Result<FeatureSet, AppError> {
    let f: FeatureSet = workflows::read_json(path, 16 * 1024 * 1024)?;
    f.validate()?;
    Ok(f)
}
pub fn load_archive(path: &Path) -> Result<Archive, AppError> {
    let a: Archive = workflows::read_json(path, 64 * 1024 * 1024)?;
    a.validate()?;
    Ok(a)
}
pub fn load_taxonomy(path: Option<&Path>) -> Result<TaxonomyDefinition, AppError> {
    let t = match path {
        Some(p) => workflows::read_json(p, 128 * 1024)?,
        None => onconpc_taxonomy(),
    };
    t.validate()?;
    Ok(t)
}
pub fn write_new(path: &Path, value: &impl serde::Serialize) -> Result<(), AppError> {
    workflows::save_new(path, &workflows::pretty(value)?)?;
    Ok(())
}
pub fn save_run(root: &Path, features: &FeatureSet, run: &InferenceRun) -> Result<(), AppError> {
    fs::create_dir(root)?;
    write_new(&root.join("features.json"), features)?;
    write_new(&root.join("inference.json"), run)?;
    Ok(())
}
fn checkpoint(root: &Path, archive: &Archive) -> Result<(), josh_explain::Error> {
    let write = || -> Result<(), AppError> {
        // Atomic replacement is restricted to the current run's explicitly owned checkpoint.
        let mut file = tempfile::NamedTempFile::new_in(root)?;
        {
            let mut writer = BufWriter::new(file.as_file_mut());
            serde_json::to_writer(&mut writer, archive)?;
            writer.flush()?;
        }
        file.as_file_mut().sync_all()?;
        file.persist(root.join("checkpoint.json"))
            .map_err(|e| e.error)?;
        Ok(())
    };
    write().map_err(|_| josh_explain::Error::Checkpoint)
}
pub fn initialize_archive(root: &Path, options: &ExplainOptions) -> Result<Archive, AppError> {
    let features = load_features(&root.join("features.json"))?;
    let inference: InferenceRun =
        workflows::read_json(&root.join("inference.json"), 2 * 1024 * 1024)?;
    let background: Option<Background> = options
        .background
        .as_ref()
        .map(|p| workflows::read_json(p, 16 * 1024 * 1024))
        .transpose()?;
    let config = options.config();
    if root.join("checkpoint.json").exists() {
        let mut a = load_archive(&root.join("checkpoint.json"))?;
        if hash(&a.features)? != hash(&features)?
            || hash(&a.inference)? != hash(&inference)?
            || hash(&a.background)? != hash(&background)?
        {
            return Err(
                josh_core::ValidationError("checkpoint inputs differ from selected run").into(),
            );
        }
        // Only budgets may change when resuming; a different mathematical run needs its own directory.
        let mut old = a.config.clone();
        old.max_evaluations = config.max_evaluations;
        old.max_input_tokens = config.max_input_tokens;
        old.max_seconds = config.max_seconds;
        if hash(&old)? != hash(&config)? {
            return Err(josh_core::ValidationError(
                "resume requires the same target, baseline, method and seed",
            )
            .into());
        }
        a.config = config;
        if options.retry_uncertain {
            a.in_flight = None;
        }
        a.validate()?;
        return Ok(a);
    }
    Ok(Archive::new(features, inference, config, background)?)
}

pub struct LiveEvaluator {
    client: josh_jev::Client,
    data_class: DataClass,
}
impl LiveEvaluator {
    pub fn new(data_class: DataClass) -> Result<Self, AppError> {
        if data_class != DataClass::Synthetic {
            return Err(josh_jev::Error::DataPolicy.into());
        }
        let key = std::env::var("TYPESAFE_API_KEY").map_err(|_| josh_jev::Error::MissingKey)?;
        Ok(Self {
            client: josh_jev::Client::new(&key)?,
            data_class,
        })
    }
}
impl Evaluator for LiveEvaluator {
    async fn evaluate(&mut self, request: &JevRequest) -> Result<JevResponse, josh_explain::Error> {
        self.client
            .evaluate(request, &self.data_class)
            .await
            .map_err(|e| match e {
                josh_jev::Error::Http(status) => josh_explain::Error::ProviderHttp(status),
                josh_jev::Error::Response | josh_jev::Error::Validation(_) => {
                    josh_explain::Error::ProviderResponse
                }
                _ => josh_explain::Error::Provider,
            })
    }
}

/// Analytical software fixture, restricted to the bundled feature set by callers.
/// Arbitrary coefficients illustrate positive/negative attributions; no learned cancer model.
pub struct AnalyticalDemo;
impl Evaluator for AnalyticalDemo {
    async fn evaluate(&mut self, request: &JevRequest) -> Result<JevResponse, josh_explain::Error> {
        Ok(demo_response(request))
    }
}
pub fn demo_response(request: &JevRequest) -> JevResponse {
    let mut signal = 0.0;
    if let Some(features) = request.state["features"].as_array() {
        for feature in features {
            if feature["status"] != "observed" {
                continue;
            }
            let v = &feature["value"];
            signal += match v["kind"].as_str().unwrap_or("") {
                "signature" => v["value"].as_f64().unwrap_or(0.0) * 0.04,
                "copy_number" => v["call"].as_f64().unwrap_or(0.0) * 0.025,
                "mutation" => 0.035,
                "age" => 0.015,
                "category" => 0.01,
                _ => 0.0,
            };
        }
    }
    let p = (0.12 + signal).clamp(0.01, 0.85);
    let criteria = match &request.questions["primary_site"] {
        Question::Choice { criteria, .. } => criteria,
        _ => unreachable!(),
    };
    let target = if criteria.contains_key("NSCLC") {
        "NSCLC"
    } else {
        criteria.keys().next().expect("validated taxonomy")
    };
    let probabilities: BTreeMap<_, _> = criteria
        .keys()
        .map(|id| {
            (
                id.clone(),
                if id == target {
                    p
                } else {
                    (1.0 - p) / (criteria.len() - 1) as f64
                },
            )
        })
        .collect();
    let choice = probabilities
        .iter()
        .max_by(|a, b| a.1.total_cmp(b.1))
        .expect("nonempty")
        .0
        .clone();
    JevResponse {
        model: request.model.clone(),
        answers: BTreeMap::from([
            (
                "primary_site".into(),
                Answer::Choice {
                    choice,
                    probabilities,
                    confidence: 0.0,
                },
            ),
            ("evidence_sufficient".into(), Answer::Noul { noul: 0.0 }),
            ("conflicting_evidence".into(), Answer::Noul { noul: 0.0 }),
        ]),
        usage: Usage {
            input_tokens: 0,
            output_tokens: 0,
        },
    }
}
pub fn example() -> FeatureSet {
    serde_json::from_str(include_str!(
        "../../../fixtures/molecular/synthetic-features.json"
    ))
    .expect("bundled molecular example")
}
pub fn expression_features(
    e: &josh_core::reference::EvidencePackage,
    patient_group: &str,
) -> Result<(FeatureSet, TaxonomyDefinition), AppError> {
    crate::reference::request(e)?;
    let values = e
        .similarities
        .iter()
        .map(|s| {
            Ok((
                s.class_id.clone(),
                s.pearson_r
                    .ok_or(josh_core::ValidationError("undefined similarity"))?,
            ))
        })
        .collect::<Result<_, josh_core::ValidationError>>()?;
    let f = FeatureSet {
        schema_version: 1,
        pipeline_version: PIPELINE.into(),
        sample_id: e.sample_id.clone(),
        patient_group_id: patient_group.into(),
        data_class: if e.synthetic {
            DataClass::Synthetic
        } else {
            DataClass::DeidentifiedResearch
        },
        features: vec![Feature {
            id: "expression-reference".into(),
            name: "Expression reference similarities".into(),
            modality: Modality::Expression,
            group: "expression-reference".into(),
            status: MeasurementStatus::Observed,
            value: Some(FeatureValue::Vector {
                values,
                units: "Pearson r; not probabilities".into(),
                reference_sha256: e.reference_sha256.clone(),
            }),
            assay: format!("reference:{}", e.reference_release_id),
            reference_build: None,
            coverage: format!("{}/{} reference genes", e.common_genes, e.reference_genes),
            source: FeatureSource {
                source_id: "expression-comparison".into(),
                sha256: e.measurement_sha256.clone(),
                record: 1,
            },
            depends_on: vec![],
        }],
    };
    let t = TaxonomyDefinition {
        version: "expression-reference-classes-v1".into(),
        classes: e
            .similarities
            .iter()
            .map(|s| CancerClass {
                id: s.class_id.clone(),
                name: s.cancer_type.clone(),
                parent: s.class_id.clone(),
            })
            .collect(),
        unknown_id: "unknown".into(),
        other_id: "other_origin".into(),
    };
    f.validate()?;
    t.validate()?;
    Ok((f, t))
}
pub async fn infer(
    features: &FeatureSet,
    taxonomy: &TaxonomyDefinition,
) -> Result<InferenceRun, AppError> {
    let request = prepare(features, taxonomy)?;
    let mut evaluator = LiveEvaluator::new(features.data_class.clone())?;
    let response = evaluator.evaluate(&request).await?;
    Ok(interpret(features, taxonomy, response, Source::Jev)?)
}
pub async fn infer_and_save(
    root: &Path,
    features: &FeatureSet,
    taxonomy: &TaxonomyDefinition,
) -> Result<InferenceRun, AppError> {
    let request = prepare(features, taxonomy)?;
    let mut evaluator = LiveEvaluator::new(features.data_class.clone())?;
    fs::create_dir(root)?;
    write_new(&root.join("features.json"), features)?;
    write_new(&root.join("request.json"), &request)?;
    // A failed or interrupted request leaves its exact input on disk. Rerunning
    // requires a new directory, so an ambiguous timeout cannot silently resubmit.
    let response = evaluator.evaluate(&request).await?;
    let run = interpret(features, taxonomy, response, Source::Jev)?;
    write_new(&root.join("inference.json"), &run)?;
    Ok(run)
}
pub(crate) struct RunLock(PathBuf);
impl RunLock {
    pub(crate) fn acquire(root: &Path, name: &str) -> Result<Self, AppError> {
        let path = root.join(name);
        OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)?;
        Ok(Self(path))
    }
}
impl Drop for RunLock {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
}
pub async fn explain(
    root: &Path,
    mut archive: Archive,
    progress: &Progress,
) -> Result<Archive, AppError> {
    // Reserve the final filename before making calls; never overwrite an export.
    if root.join("explanation.json").exists() {
        return Err(
            std::io::Error::new(std::io::ErrorKind::AlreadyExists, "explanation exists").into(),
        );
    }
    let _lock = RunLock::acquire(root, "explanation.lock")?;
    checkpoint(root, &archive)?;
    if archive.inference.source == Source::Mock {
        if hash(&archive.features)? != hash(&example())? || archive.background.is_some() {
            return Err(josh_core::ValidationError(
                "analytical demo accepts only the bundled fixture without background",
            )
            .into());
        }
        josh_explain::run(&mut archive, &mut AnalyticalDemo, progress, |a| {
            checkpoint(root, a)
        })
        .await?;
    } else if archive.inference.source == Source::Jev {
        let mut evaluator = LiveEvaluator::new(archive.features.data_class.clone())?;
        josh_explain::run(&mut archive, &mut evaluator, progress, |a| {
            checkpoint(root, a)
        })
        .await?;
    } else {
        return Err(josh_core::ValidationError(
            "unverified replay cannot start new provider evaluations",
        )
        .into());
    }
    write_new(&root.join("explanation.json"), &archive)?;
    Ok(archive)
}
pub fn summary(a: &Archive) -> String {
    match &a.result {
        Some(r) => format!(
            "{} | {} | {}\n{}\nRaw score: {:.4}; baseline: {:.4}; sum contributions: {:+.4}\nAdditivity residual: {:.3e}; evaluations: {}; input tokens: {}\nNo cancer calibration. {}",
            a.features.sample_id,
            match a.inference.source {
                Source::Mock => "ANALYTICAL DEMO / NO CANCER PREDICTION",
                Source::Replay => "UNVERIFIED REPLAY",
                Source::Jev => "JEV / RESEARCH",
            },
            r.target_class,
            r.method,
            r.full_probability,
            r.baseline_probability,
            r.attributions.iter().map(|x| x.contribution).sum::<f64>(),
            r.additivity_residual,
            r.evaluations,
            r.input_tokens,
            a.inference.status
        ),
        None => format!(
            "Incomplete explanation; {} completed evaluations. Resume with the same options.",
            a.evaluations.len()
        ),
    }
}
pub async fn execute(command: Command) -> Result<Output, AppError> {
    let mut raw = None;
    let mut text = String::new();
    let value = match command {
        Command::Repeatability {
            run_dir,
            out_dir,
            replicates,
            max_evaluations,
            max_input_tokens,
            max_seconds,
        } => {
            crate::molecular_research::repeatability(
                &run_dir,
                &out_dir,
                replicates,
                max_evaluations,
                max_input_tokens,
                max_seconds,
            )
            .await?
        }
        Command::Stability { left, right, top_k } => {
            serde_json::to_value(josh_explain::diagnostics::compare(
                &load_archive(&left)?,
                &load_archive(&right)?,
                top_k,
            )?)?
        }
        Command::Cohort {
            manifest,
            out_dir,
            max_evaluations,
            max_input_tokens,
            max_seconds,
        } => {
            crate::cohort::run(
                &manifest,
                &out_dir,
                max_evaluations,
                max_input_tokens,
                max_seconds,
            )
            .await?
        }
        Command::Signatures {
            features,
            fasta,
            fai,
            catalogue,
            opportunity_profile,
            min_mutations,
        } => serde_json::to_value(crate::signature_workflow::derive(
            &features,
            &fasta,
            &fai,
            &catalogue,
            &opportunity_profile,
            min_mutations,
        )?)?,
        Command::AttachSignatures {
            features,
            signatures,
        } => {
            let set = load_features(&features)?;
            let run = workflows::read_json(&signatures, 4 * 1024 * 1024)?;
            serde_json::to_value(crate::signature_workflow::attach(set, run)?)?
        }
        Command::Evaluate {
            records,
            taxonomy,
            partition,
        } => {
            let records: Vec<josh_features::evaluation::Record> =
                workflows::read_json(&records, 64 * 1024 * 1024)?;
            let taxonomy = load_taxonomy(taxonomy.as_deref())?;
            serde_json::to_value(josh_features::evaluation::evaluate(
                &records,
                &taxonomy.classes.iter().map(|c| c.id.clone()).collect(),
                &taxonomy.criteria().keys().cloned().collect(),
                &partition,
            )?)?
        }
        Command::Example => serde_json::to_value(example())?,
        Command::Taxonomy => serde_json::to_value(onconpc_taxonomy())?,
        Command::Import {
            input,
            input_format,
            sample,
            patient_group,
            source_id,
            assay,
            reference_build,
            synthetic,
        } => {
            let fmt = match input_format {
                InputFormat::Csv => josh_ingest::molecular::Format::Csv,
                InputFormat::Tsv => josh_ingest::molecular::Format::Tsv,
                InputFormat::Maf => josh_ingest::molecular::Format::Maf,
                InputFormat::Vcf => josh_ingest::molecular::Format::Vcf,
            };
            let report = josh_ingest::molecular::import(
                fs::File::open(input)?,
                fmt,
                &josh_ingest::molecular::Options {
                    sample_id: sample,
                    patient_group_id: patient_group,
                    data_class: if synthetic {
                        DataClass::Synthetic
                    } else {
                        DataClass::DeidentifiedResearch
                    },
                    source_id,
                    assay,
                    reference_build,
                },
            )?;
            text = format!(
                "Imported {} molecular observations; source {}. No provider request.",
                report.accepted_records, report.source_sha256
            );
            serde_json::to_value(report.features)?
        }
        Command::Merge { inputs } => {
            let mut sets = inputs
                .iter()
                .map(|p| load_features(p))
                .collect::<Result<Vec<_>, _>>()?;
            let mut f = sets.remove(0);
            for s in sets {
                if s.sample_id != f.sample_id
                    || s.patient_group_id != f.patient_group_id
                    || s.data_class != f.data_class
                {
                    return Err(josh_core::ValidationError(
                        "molecular sample or data-class mismatch",
                    )
                    .into());
                }
                f.features.extend(s.features);
            }
            f.validate()?;
            serde_json::to_value(f)?
        }
        Command::AttachExpression { features, evidence } => {
            let mut f = load_features(&features)?;
            let e: josh_core::reference::EvidencePackage =
                workflows::read_json(&evidence, 1024 * 1024)?;
            crate::reference::request(&e)?;
            if e.sample_id != f.sample_id || e.synthetic != (f.data_class == DataClass::Synthetic) {
                return Err(josh_core::ValidationError(
                    "expression evidence sample or data-class mismatch",
                )
                .into());
            }
            let values = e
                .similarities
                .iter()
                .map(|s| {
                    Ok((
                        s.class_id.clone(),
                        s.pearson_r
                            .ok_or(josh_core::ValidationError("undefined similarity"))?,
                    ))
                })
                .collect::<Result<_, josh_core::ValidationError>>()?;
            f.features.push(Feature {
                id: "expression-reference".into(),
                name: "Expression reference similarities".into(),
                modality: Modality::Expression,
                group: "expression-reference".into(),
                status: MeasurementStatus::Observed,
                value: Some(FeatureValue::Vector {
                    values,
                    units: "Pearson r; not probabilities".into(),
                    reference_sha256: e.reference_sha256,
                }),
                assay: format!("reference:{}", e.reference_release_id),
                reference_build: None,
                coverage: format!("{}/{} reference genes", e.common_genes, e.reference_genes),
                source: FeatureSource {
                    source_id: "expression-comparison".into(),
                    sha256: e.measurement_sha256,
                    record: 1,
                },
                depends_on: vec![],
            });
            f.validate()?;
            serde_json::to_value(f)?
        }
        Command::Prepare { features, taxonomy } => {
            let f = load_features(&features)?;
            let t = load_taxonomy(taxonomy.as_deref())?;
            let request = prepare(&f, &t)?;
            serde_json::json!({"sends_to_provider":false,"feature_sha256":hash(&f)?,"taxonomy":t,"prompt_version":PROMPT,"request_sha256":hash(&request)?,"request":request})
        }
        Command::Run {
            features,
            taxonomy,
            out_dir,
        } => {
            let f = load_features(&features)?;
            let t = load_taxonomy(taxonomy.as_deref())?;
            if out_dir.exists() {
                return Err(
                    std::io::Error::new(std::io::ErrorKind::AlreadyExists, "run exists").into(),
                );
            }
            let run = infer_and_save(&out_dir, &f, &t).await?;
            text = format!(
                "{} | Jev {} | {}\nRaw scores are uncalibrated. Archive: {}",
                f.sample_id,
                run.response.model,
                run.status,
                out_dir.display()
            );
            serde_json::to_value(run)?
        }
        Command::Demo { out_dir } => {
            let f = example();
            let t = onconpc_taxonomy();
            let run = interpret(&f, &t, demo_response(&prepare(&f, &t)?), Source::Mock)?;
            save_run(&out_dir, &f, &run)?;
            let a = Archive::new(
                f,
                run,
                Config {
                    algorithm: josh_explain::Algorithm::Exact,
                    ..Config::default()
                },
                None,
            )?;
            let a = explain(&out_dir, a, &Progress::default()).await?;
            text = summary(&a);
            serde_json::to_value(a)?
        }
        Command::Plan { run_dir, options } => {
            let a = initialize_archive(&run_dir, &options)?;
            serde_json::to_value(a.plan())?
        }
        Command::Explain { run_dir, options } => {
            let a = initialize_archive(&run_dir, &options)?;
            let a = explain(&run_dir, a, &Progress::default()).await?;
            text = summary(&a);
            serde_json::to_value(a)?
        }
        Command::Inspect { archive } => {
            let a = load_archive(&archive)?;
            text = summary(&a);
            serde_json::to_value(a)?
        }
        Command::Export { archive, kind } => {
            let a = load_archive(&archive)?;
            if a.result.is_none() {
                return Err(josh_core::ValidationError("explanation is incomplete").into());
            }
            raw = Some(match kind {
                ExportFormat::Json => workflows::pretty(&a)?,
                ExportFormat::Csv => crate::molecular_charts::csv(&a)?,
                ExportFormat::Svg => crate::molecular_charts::svg(&a)?,
            });
            serde_json::Value::Null
        }
    };
    if text.is_empty() {
        text = workflows::pretty(&value)?;
    }
    Ok(Output { value, text, raw })
}

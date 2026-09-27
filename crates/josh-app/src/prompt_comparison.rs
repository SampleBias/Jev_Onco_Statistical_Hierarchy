//! Paired version comparison. Labels stay local; failures remain in denominators.
use crate::{
    cohort::Manifest,
    molecular,
    workflows::{self, AppError},
};
use josh_core::{
    DataClass, JevRequest, Source, ValidationError,
    molecular::{self as core, FeatureSet},
};
use josh_explain::Evaluator;
use josh_features::evaluation::{self, Record};
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
    time::{Duration, Instant},
};

pub struct Comparison {
    versions: [String; 2],
    manifest: Manifest,
    features: Vec<FeatureSet>,
    requests: Vec<[JevRequest; 2]>,
    templates: Vec<Record>,
}

impl Comparison {
    pub fn load(path: &Path) -> Result<Self, AppError> {
        let manifest: Manifest = workflows::read_json(path, 2 * 1024 * 1024)?;
        let root = path.parent().unwrap_or(Path::new("."));
        let features = manifest
            .cases
            .iter()
            .map(|entry| molecular::load_features(&root.join(&entry.features)))
            .collect::<Result<Vec<_>, _>>()?;
        Self::new(manifest, features)
    }

    pub fn new(manifest: Manifest, features: Vec<FeatureSet>) -> Result<Self, AppError> {
        Self::with_versions(manifest, features, [core::PREVIOUS_PROMPT, core::V5_PROMPT])
    }

    pub fn with_versions(
        manifest: Manifest,
        features: Vec<FeatureSet>,
        versions: [&str; 2],
    ) -> Result<Self, AppError> {
        manifest.taxonomy.validate()?;
        if manifest.schema_version != 1
            || manifest.protocol_id.trim().is_empty()
            || manifest.protocol_id.len() > 128
            || manifest.cases.is_empty()
            || manifest.cases.len() > 500
            || manifest.cases.len() != features.len()
        {
            return Err(
                ValidationError("invalid paired comparison protocol; maximum 500 cases").into(),
            );
        }
        let templates = manifest
            .cases
            .iter()
            .zip(&features)
            .map(|(entry, f)| Record {
                sample_id: f.sample_id.clone(),
                patient_group_id: f.patient_group_id.clone(),
                partition: entry.partition.clone(),
                truth: entry.truth.clone(),
                probabilities: None,
                accepted: false,
                failure: Some("not_run".into()),
            })
            .collect::<Vec<_>>();
        let classes = manifest
            .taxonomy
            .classes
            .iter()
            .map(|c| c.id.clone())
            .collect();
        let outcomes = manifest.taxonomy.criteria().keys().cloned().collect();
        // Validate all partitions and patient groups before creating files or sending requests.
        evaluation::evaluate(&templates, &classes, &outcomes, &templates[0].partition)?;
        let requests = features
            .iter()
            .map(|f| {
                Ok([
                    core::prepare_versioned(f, &manifest.taxonomy, versions[0])?,
                    core::prepare_versioned(f, &manifest.taxonomy, versions[1])?,
                ])
            })
            .collect::<Result<Vec<_>, AppError>>()?;
        Ok(Self {
            versions: versions.map(str::to_owned),
            manifest,
            features,
            requests,
            templates,
        })
    }

    pub fn plan(&self) -> Result<Value, AppError> {
        let cases = self.features.iter().zip(&self.requests).map(|(f, r)| Ok(json!({
            "feature_sha256": core::hash(f)?,
            "request_sha256": [core::hash(&r[0])?, core::hash(&r[1])?],
            "request_bytes": [serde_json::to_vec(&r[0])?.len(), serde_json::to_vec(&r[1])?.len()],
        }))).collect::<Result<Vec<_>, AppError>>()?;
        let plan = json!({
            "schema_version":1, "manifest":self.manifest, "cases":cases,
            "model":josh_core::MODEL, "versions":self.versions,
            "planned_evaluations":2 * self.features.len(),
            "order":"paired, alternating which version is called first by case index",
            "labels_sent_to_provider":false, "automatic_retry":false,
            "all_synthetic":self.features.iter().all(|f| f.data_class == DataClass::Synthetic),
            "limitation":"Research comparison only. Synthetic scenario labels do not establish cancer accuracy. No calibration fitting or confidence intervals."
        });
        Ok(json!({"protocol_sha256":core::hash(&plan)?,"protocol":plan}))
    }

    fn prepare_directory(&self, root: &Path) -> Result<Value, AppError> {
        let plan = self.plan()?;
        std::fs::create_dir(root)?;
        molecular::write_new(&root.join("protocol.json"), &plan)?;
        for (i, (features, requests)) in self.features.iter().zip(&self.requests).enumerate() {
            for (arm, request) in requests.iter().enumerate() {
                let directory = root.join(format!("{i:04}-{}", ["baseline", "candidate"][arm]));
                std::fs::create_dir(&directory)?;
                molecular::write_new(&directory.join("features.json"), features)?;
                molecular::write_new(&directory.join("request.json"), request)?;
            }
        }
        Ok(plan)
    }

    pub fn prepare(&self, root: &Path) -> Result<Value, AppError> {
        let mut plan = self.prepare_directory(root)?;
        plan["execution"] = json!("prepared_only; no provider calls");
        Ok(plan)
    }

    pub async fn run(
        &self,
        root: &Path,
        evaluator: &mut impl Evaluator,
        max_evaluations: usize,
        max_input_tokens: u64,
        max_seconds: u64,
    ) -> Result<Value, AppError> {
        if self
            .features
            .iter()
            .any(|f| f.data_class != DataClass::Synthetic)
        {
            return Err(josh_jev::Error::DataPolicy.into());
        }
        if max_evaluations == 0
            || max_evaluations > 1000
            || max_input_tokens == 0
            || !(1..=86_400).contains(&max_seconds)
        {
            return Err(ValidationError("invalid comparison budgets").into());
        }
        let plan = self.prepare_directory(root)?;
        molecular::write_new(
            &root.join("execution.json"),
            &json!({
                "protocol_sha256":plan["protocol_sha256"],
                "max_evaluations":max_evaluations,"max_input_tokens":max_input_tokens,"max_seconds":max_seconds,
                "source":"jev", "automatic_retry":false
            }),
        )?;
        let mut records = [self.templates.clone(), self.templates.clone()];
        let mut attempts = 0;
        let mut tokens = 0_u64;
        let start = Instant::now();
        for (i, requests) in self.requests.iter().enumerate() {
            for arm in [i % 2, 1 - i % 2] {
                let request = &requests[arm];
                let reservation = serde_json::to_vec(request)?.len() as u64;
                let remaining = Duration::from_secs(max_seconds).checked_sub(start.elapsed());
                if attempts >= max_evaluations
                    || tokens.saturating_add(reservation) > max_input_tokens
                    || remaining.is_none_or(|d| d.is_zero())
                {
                    records[arm][i].failure = Some("budget_not_run".into());
                    continue;
                }
                let directory = root.join(format!("{i:04}-{}", ["baseline", "candidate"][arm]));
                molecular::write_new(
                    &directory.join("attempt.json"),
                    &json!({
                        "request_sha256":core::hash(request)?, "reserved_input_tokens":reservation,
                        "state":"started; completion uncertain until outcome.json exists", "automatic_retry":false
                    }),
                )?;
                attempts += 1;
                tokens = tokens.saturating_add(reservation);
                let result =
                    tokio::time::timeout(remaining.unwrap(), evaluator.evaluate(request)).await;
                let response = match result {
                    Ok(Ok(response)) => match josh_core::response::validate(request, &response) {
                        Ok(()) => {
                            tokens = tokens
                                .saturating_sub(reservation)
                                .saturating_add(response.usage.input_tokens);
                            Ok(response)
                        }
                        Err(mut d) => {
                            d.request_sha256 = Some(core::hash(request)?);
                            Err(josh_explain::Error::ProviderResponse(d))
                        }
                    },
                    Ok(Err(error)) => Err(error),
                    Err(_) => Err(josh_explain::Error::Timeout),
                };
                match response {
                    Ok(response) => {
                        let run = core::interpret_versioned(
                            &self.features[i],
                            &self.manifest.taxonomy,
                            response,
                            Source::Jev,
                            &self.versions[arm],
                        )?;
                        records[arm][i].probabilities =
                            Some(core::probabilities(&run.response)?.clone());
                        records[arm][i].accepted = run.status == "review_required";
                        records[arm][i].failure = None;
                        molecular::write_new(&directory.join("inference.json"), &run)?;
                    }
                    Err(error) => {
                        molecular::save_response_diagnostic(&directory, &error)?;
                        records[arm][i].failure = Some(error.to_string());
                    }
                }
                molecular::write_new(&directory.join("outcome.json"), &records[arm][i])?;
            }
        }
        let report = self.summarize(&records, &plan, attempts, tokens)?;
        molecular::write_new(&root.join("comparison.json"), &report)?;
        Ok(report)
    }

    fn summarize(
        &self,
        records: &[Vec<Record>; 2],
        plan: &Value,
        attempts: usize,
        tokens: u64,
    ) -> Result<Value, AppError> {
        let classes = self
            .manifest
            .taxonomy
            .classes
            .iter()
            .map(|c| c.id.clone())
            .collect();
        let outcomes = self.manifest.taxonomy.criteria().keys().cloned().collect();
        let mut partitions = BTreeMap::new();
        for partition in self
            .templates
            .iter()
            .map(|r| &r.partition)
            .collect::<BTreeSet<_>>()
        {
            let baseline = evaluation::evaluate(&records[0], &classes, &outcomes, partition)?;
            let candidate = evaluation::evaluate(&records[1], &classes, &outcomes, partition)?;
            let changes = records[0].iter().zip(&records[1]).filter(|(a,_)| &a.partition == partition).map(|(a,b)| {
                let top = |r: &Record| r.probabilities.as_ref().and_then(|p| p.iter().max_by(|a,b| a.1.total_cmp(b.1).then_with(|| b.0.cmp(a.0))).map(|(k,_)| k.clone()));
                json!({"sample_id":a.sample_id,"patient_group_id":a.patient_group_id,"truth":a.truth,
                    "baseline_top":top(a),"candidate_top":top(b),"baseline_failure":a.failure,"candidate_failure":b.failure,
                    "baseline_accepted":a.accepted,"candidate_accepted":b.accepted})
            }).collect::<Vec<_>>();
            partitions.insert(
                partition,
                json!({
                    "delta_candidate_minus_baseline":{
                        "top1_all_eligible":candidate.top1_all_eligible-baseline.top1_all_eligible,
                        "macro_f1":candidate.macro_f1-baseline.macro_f1,
                        "coverage":candidate.coverage-baseline.coverage
                    }, "baseline":baseline, "candidate":candidate, "paired_cases":changes
                }),
            );
        }
        Ok(
            json!({"schema_version":1,"protocol_sha256":plan["protocol_sha256"],
                "attempts":attempts,"input_tokens_or_reserved_failed_tokens":tokens,
                "records":{"baseline":records[0],"candidate":records[1]},"partitions":partitions,
                "limitation":"Synthetic-only execution. Descriptive paired results, not a cancer-validation study; no calibration fitted or uncertainty intervals estimated. Failures and budget omissions remain in eligible denominators."
            }),
        )
    }
}

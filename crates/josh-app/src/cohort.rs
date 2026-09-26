//! Bounded sequential cohort runs with patient-disjoint labels kept outside state.
use crate::{
    molecular,
    workflows::{self, AppError},
};
use josh_core::{
    Source, ValidationError,
    molecular::{self as core, FeatureSet, InferenceRun, TaxonomyDefinition},
};
use josh_features::evaluation::{self, Record};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

#[derive(Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Entry {
    pub features: PathBuf,
    pub partition: String,
    pub truth: String,
}
#[derive(Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    pub schema_version: u32,
    pub protocol_id: String,
    pub taxonomy: TaxonomyDefinition,
    pub cases: Vec<Entry>,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct CaseResult {
    feature_sha256: String,
    run: Option<InferenceRun>,
    failure: Option<String>,
    elapsed_ms: u64,
}

fn charged_tokens(result: &CaseResult, request: &josh_core::JevRequest) -> Result<u64, AppError> {
    Ok(match &result.run {
        Some(run) => run.response.usage.input_tokens,
        None => serde_json::to_vec(request)?.len() as u64,
    })
}

fn load_completed(
    out_dir: &Path,
    features: &[FeatureSet],
    requests: &[josh_core::JevRequest],
    taxonomy: &TaxonomyDefinition,
) -> Result<(Vec<Option<CaseResult>>, usize, u64), AppError> {
    let mut completed = Vec::with_capacity(features.len());
    let mut attempts = 0;
    let mut tokens = 0_u64;
    // Inspect every prior attempt before scheduling any new request. A later row
    // still consumes the budget when an earlier row has not run yet.
    for (i, request) in requests.iter().enumerate() {
        let result_path = out_dir.join(format!("{i:06}.json"));
        let attempt_path = out_dir.join(format!("attempt-{i:06}.json"));
        if attempt_path.exists() {
            let saved: josh_core::JevRequest = workflows::read_json(&attempt_path, 64 * 1024)?;
            if core::hash(&saved)? != core::hash(request)? {
                return Err(ValidationError("cohort attempted request changed").into());
            }
            attempts += 1;
            if !result_path.exists() {
                return Err(ValidationError("cohort contains an interrupted request with uncertain completion; do not silently resend").into());
            }
        } else if result_path.exists() {
            return Err(ValidationError("cohort result lacks an attempted request").into());
        } else {
            completed.push(None);
            continue;
        }
        let result: CaseResult = workflows::read_json(&result_path, 2 * 1024 * 1024)?;
        if result.feature_sha256 != core::hash(&features[i])?
            || result.run.is_some() == result.failure.is_some()
        {
            return Err(ValidationError("invalid cohort result archive").into());
        }
        if let Some(run) = &result.run {
            run.verify(&features[i])?;
            if run.source != Source::Jev || core::hash(&run.taxonomy)? != core::hash(taxonomy)? {
                return Err(ValidationError("cohort provider source or taxonomy changed").into());
            }
        }
        tokens = tokens.saturating_add(charged_tokens(&result, request)?);
        completed.push(Some(result));
    }
    Ok((completed, attempts, tokens))
}

pub async fn run(
    manifest_path: &Path,
    out_dir: &Path,
    max_evaluations: usize,
    max_input_tokens: u64,
    max_seconds: u64,
) -> Result<serde_json::Value, AppError> {
    let manifest: Manifest = workflows::read_json(manifest_path, 2 * 1024 * 1024)?;
    manifest.taxonomy.validate()?;
    if manifest.schema_version != 1
        || manifest.protocol_id.trim().is_empty()
        || manifest.protocol_id.len() > 128
        || manifest.cases.is_empty()
        || manifest.cases.len() > 1000
        || max_evaluations == 0
        || max_evaluations > 1000
        || max_input_tokens == 0
        || max_seconds == 0
        || max_seconds > 86_400
    {
        return Err(ValidationError("invalid cohort protocol or budgets").into());
    }
    let root = manifest_path.parent().unwrap_or(Path::new("."));
    let mut features: Vec<FeatureSet> = Vec::new();
    let mut templates = Vec::new();
    for entry in &manifest.cases {
        let f = molecular::load_features(&root.join(&entry.features))?;
        templates.push(Record {
            sample_id: f.sample_id.clone(),
            patient_group_id: f.patient_group_id.clone(),
            partition: entry.partition.clone(),
            truth: entry.truth.clone(),
            probabilities: None,
            accepted: false,
            failure: Some("pending".into()),
        });
        features.push(f);
    }
    let classes: BTreeSet<_> = manifest
        .taxonomy
        .classes
        .iter()
        .map(|c| c.id.clone())
        .collect();
    let outcomes = manifest.taxonomy.criteria().keys().cloned().collect();
    // Validates patient and sample disjointness across ALL partitions before a request.
    evaluation::evaluate(&templates, &classes, &outcomes, &templates[0].partition)?;
    let requests = features
        .iter()
        .map(|f| core::prepare(f, &manifest.taxonomy))
        .collect::<Result<Vec<_>, _>>()?;
    let fingerprint = serde_json::json!({"manifest":manifest,"features":features.iter().map(core::hash).collect::<Result<Vec<_>,_>>()?,"model":josh_core::MODEL,"prompt":core::PROMPT});
    let expected = core::hash(&fingerprint)?;
    if features
        .iter()
        .any(|f| f.data_class != josh_core::DataClass::Synthetic)
    {
        return Err(josh_jev::Error::DataPolicy.into());
    }
    if out_dir.exists() {
        let saved: serde_json::Value =
            workflows::read_json(&out_dir.join("manifest.json"), 4 * 1024 * 1024)?;
        if core::hash(&saved)? != expected {
            return Err(ValidationError("cohort resume manifest or features changed").into());
        }
    } else {
        std::fs::create_dir(out_dir)?;
        molecular::write_new(&out_dir.join("manifest.json"), &fingerprint)?;
    }
    let _lock = molecular::RunLock::acquire(out_dir, "cohort.lock")?;
    let (mut completed, mut attempts, mut tokens) =
        load_completed(out_dir, &features, &requests, &manifest.taxonomy)?;
    let mut evaluator = None;
    let mut timings = Vec::new();
    let start = Instant::now();
    for (i, request) in requests.iter().enumerate() {
        let result_path = out_dir.join(format!("{i:06}.json"));
        let attempt_path = out_dir.join(format!("attempt-{i:06}.json"));
        let saved = if let Some(result) = completed[i].take() {
            result
        } else {
            let reserve = serde_json::to_vec(request)?.len() as u64;
            if attempts >= max_evaluations
                || tokens.saturating_add(reserve) > max_input_tokens
                || start.elapsed() >= Duration::from_secs(max_seconds)
            {
                templates[i].failure = Some("budget_not_run".into());
                continue;
            }
            if evaluator.is_none() {
                evaluator = Some(molecular::LiveEvaluator::new(
                    features[i].data_class.clone(),
                )?);
            }
            molecular::write_new(&attempt_path, request)?;
            attempts += 1;
            let remaining = Duration::from_secs(max_seconds)
                .checked_sub(start.elapsed())
                .ok_or(ValidationError("cohort wall-time limit"))?;
            use josh_explain::Evaluator;
            let call_start = Instant::now();
            let response =
                tokio::time::timeout(remaining, evaluator.as_mut().unwrap().evaluate(request))
                    .await;
            let elapsed_ms = call_start.elapsed().as_millis().min(u64::MAX as u128) as u64;
            let result = match response {
                Ok(Ok(response)) => CaseResult {
                    feature_sha256: core::hash(&features[i])?,
                    run: Some(core::interpret(
                        &features[i],
                        &manifest.taxonomy,
                        response,
                        Source::Jev,
                    )?),
                    failure: None,
                    elapsed_ms,
                },
                Ok(Err(error)) => {
                    molecular::save_response_diagnostic(out_dir, &error)?;
                    CaseResult {
                        feature_sha256: core::hash(&features[i])?,
                        run: None,
                        failure: Some(error.to_string()),
                        elapsed_ms,
                    }
                }
                Err(_) => CaseResult {
                    feature_sha256: core::hash(&features[i])?,
                    run: None,
                    failure: Some("provider_error_or_timeout; no automatic retry".into()),
                    elapsed_ms,
                },
            };
            molecular::write_new(&result_path, &result)?;
            tokens = tokens.saturating_add(charged_tokens(&result, request)?);
            result
        };
        timings.push(
            serde_json::json!({"sample_id":features[i].sample_id,"elapsed_ms":saved.elapsed_ms}),
        );
        if let Some(run) = saved.run {
            templates[i].probabilities = Some(core::probabilities(&run.response)?.clone());
            templates[i].accepted = run.status == "review_required";
            templates[i].failure = None;
        } else {
            templates[i].failure = saved.failure;
        }
    }
    let mut reports = BTreeMap::new();
    for partition in templates
        .iter()
        .map(|r| r.partition.as_str())
        .collect::<BTreeSet<_>>()
    {
        reports.insert(
            partition.to_string(),
            evaluation::evaluate(&templates, &classes, &outcomes, partition)?,
        );
    }
    Ok(
        serde_json::json!({"protocol_sha256":expected,"attempts":attempts,"input_tokens_or_reserved_failed_tokens":tokens,"timings":timings,"records":templates,"reports":reports,"calibration":"not_fitted","confidence_intervals":"not_estimated"}),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resume_counts_later_failed_attempts_and_rejects_uncertain_calls() {
        let dir = tempfile::tempdir().unwrap();
        let f = molecular::example();
        let mut second = f.clone();
        second.sample_id = "second".into();
        second.patient_group_id = "second-patient".into();
        let features = vec![f, second];
        let taxonomy = core::onconpc_taxonomy();
        let requests = features
            .iter()
            .map(|f| core::prepare(f, &taxonomy).unwrap())
            .collect::<Vec<_>>();
        molecular::write_new(&dir.path().join("attempt-000001.json"), &requests[1]).unwrap();
        assert!(load_completed(dir.path(), &features, &requests, &taxonomy).is_err());
        molecular::write_new(
            &dir.path().join("000001.json"),
            &CaseResult {
                feature_sha256: core::hash(&features[1]).unwrap(),
                run: None,
                failure: Some("provider timeout".into()),
                elapsed_ms: 20000,
            },
        )
        .unwrap();
        let (saved, attempts, tokens) =
            load_completed(dir.path(), &features, &requests, &taxonomy).unwrap();
        assert!(saved[0].is_none());
        assert!(saved[1].is_some());
        assert_eq!(attempts, 1);
        assert_eq!(
            tokens,
            serde_json::to_vec(&requests[1]).unwrap().len() as u64
        );
    }
}

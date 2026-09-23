//! Small, explicitly budgeted synthetic repeatability experiment.
use crate::{
    molecular,
    workflows::{self, AppError},
};
use josh_core::{
    Source, ValidationError,
    molecular::{self as core, InferenceRun},
};
use josh_explain::Evaluator;
use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
    time::{Duration, Instant},
};

pub async fn repeatability(
    run_dir: &Path,
    out_dir: &Path,
    replicates: usize,
    max_evaluations: usize,
    max_input_tokens: u64,
    max_seconds: u64,
) -> Result<serde_json::Value, AppError> {
    if !(2..=5).contains(&replicates)
        || max_evaluations < 2 * replicates
        || max_evaluations > 10
        || max_input_tokens == 0
        || max_seconds == 0
        || max_seconds > 600
    {
        return Err(ValidationError(
            "repeatability needs 2..5 replicates and explicit compatible budgets",
        )
        .into());
    }
    let features = molecular::load_features(&run_dir.join("features.json"))?;
    let run: InferenceRun = workflows::read_json(&run_dir.join("inference.json"), 2 * 1024 * 1024)?;
    run.verify(&features)?;
    if run.source != Source::Jev {
        return Err(ValidationError("provider repeatability requires an original Jev run").into());
    }
    let mut evaluator = molecular::LiveEvaluator::new(features.data_class.clone())?;
    let baseline = core::prepare_masked(&features, &run.taxonomy, &BTreeSet::new())?;
    let requests = [(&run.request, "full"), (&baseline, "masked_baseline")];
    std::fs::create_dir(out_dir)?;
    molecular::write_new(
        &out_dir.join("protocol.json"),
        &serde_json::json!({
            "schema_version":1,"feature_sha256":run.feature_sha256,"request_sha256":run.request_sha256,
            "replicates":replicates,"max_evaluations":max_evaluations,"max_input_tokens":max_input_tokens,
            "max_seconds":max_seconds,"scope":"repeated full and masked-evidence baseline distributions; no attribution or clinical validation"
        }),
    )?;
    let start = Instant::now();
    let mut tokens = 0_u64;
    let mut completed = Vec::new();
    let mut distributions: BTreeMap<&str, Vec<BTreeMap<String, f64>>> = BTreeMap::new();
    for repeat in 0..replicates {
        for (request, condition) in requests {
            let reserve = serde_json::to_vec(request)?.len() as u64;
            if tokens.saturating_add(reserve) > max_input_tokens {
                return Err(josh_explain::Error::Budget.into());
            }
            let remaining = Duration::from_secs(max_seconds)
                .checked_sub(start.elapsed())
                .ok_or(josh_explain::Error::Timeout)?;
            let index = completed.len();
            molecular::write_new(&out_dir.join(format!("attempt-{index:02}.json")), request)?;
            let call_start = Instant::now();
            let result = tokio::time::timeout(remaining, evaluator.evaluate(request)).await;
            let response = match result {
                Ok(Ok(response)) => response,
                failure => {
                    let reason = match failure {
                        Ok(Err(e)) => e.to_string(),
                        _ => "provider wall-time limit".into(),
                    };
                    molecular::write_new(
                        &out_dir.join(format!("failure-{index:02}.json")),
                        &serde_json::json!({"condition":condition,"repeat":repeat,"reason":reason,"uncertain_input_token_reservation":reserve,"resubmitted":false}),
                    )?;
                    return Err(ValidationError("repeatability incomplete; successful calls and failed attempt retained; no automatic retry").into());
                }
            };
            tokens = tokens.saturating_add(response.usage.input_tokens);
            distributions
                .entry(condition)
                .or_default()
                .push(core::probabilities(&response)?.clone());
            let call = serde_json::json!({"condition":condition,"repeat":repeat,"request":request,"response":response,"elapsed_ms":call_start.elapsed().as_millis() as u64});
            molecular::write_new(&out_dir.join(format!("response-{index:02}.json")), &call)?;
            completed.push(call);
            if tokens > max_input_tokens {
                return Err(josh_explain::Error::Budget.into());
            }
        }
    }
    let mut summaries = BTreeMap::new();
    for (condition, values) in distributions {
        let mut classes = BTreeMap::new();
        for key in run.taxonomy.criteria().keys() {
            let mean = values.iter().map(|p| p[key]).sum::<f64>() / replicates as f64;
            let sd = (values.iter().map(|p| (p[key] - mean).powi(2)).sum::<f64>()
                / (replicates - 1) as f64)
                .sqrt();
            let min = values.iter().map(|p| p[key]).fold(1.0_f64, f64::min);
            let max = values.iter().map(|p| p[key]).fold(0.0_f64, f64::max);
            classes.insert(
                key.clone(),
                serde_json::json!({"mean":mean,"sample_standard_deviation":sd,"min":min,"max":max}),
            );
        }
        summaries.insert(condition, classes);
    }
    let report = serde_json::json!({"schema_version":1,"source":"jev","feature_sha256":run.feature_sha256,"model":run.response.model,"replicates_per_condition":replicates,"evaluations":completed.len(),"input_tokens":tokens,"conditions":summaries,"limitation":"A small synthetic diagnostic cannot establish future provider determinism, biological validity or clinical uncertainty."});
    molecular::write_new(&out_dir.join("report.json"), &report)?;
    Ok(report)
}

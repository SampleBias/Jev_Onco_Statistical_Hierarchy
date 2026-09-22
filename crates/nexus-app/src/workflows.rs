use nexus_core::errors::{ErrorCode, ErrorEnvelope};
use nexus_core::{Case, ResultRecord, Source, interpret, mock_response};
use std::{
    io::{Read, Write},
    path::Path,
};

pub type AppError = Box<dyn std::error::Error + Send + Sync>;

pub fn example_case() -> Case {
    serde_json::from_str(include_str!("../../../fixtures/synthetic-case.json"))
        .expect("bundled synthetic fixture must match the schema")
}

pub fn read_json<T: serde::de::DeserializeOwned>(path: &Path, max: usize) -> Result<T, AppError> {
    let mut bytes = Vec::new();
    if path == Path::new("-") {
        std::io::stdin()
            .lock()
            .take(max as u64 + 1)
            .read_to_end(&mut bytes)?;
    } else {
        std::fs::File::open(path)?
            .take(max as u64 + 1)
            .read_to_end(&mut bytes)?;
    }
    if bytes.len() > max {
        return Err(ErrorEnvelope::new(ErrorCode::InputTooLarge).into());
    }
    // Parser errors can contain submitted values; do not echo them.
    serde_json::from_slice(&bytes).map_err(|_| ErrorEnvelope::new(ErrorCode::InvalidJson).into())
}

pub fn load_case(path: &Path) -> Result<Case, AppError> {
    let case: Case = read_json(path, nexus_core::MAX_CASE_BYTES)?;
    case.validate()?;
    Ok(case)
}

pub fn import_cases(
    path: &Path,
    options: &nexus_ingest::Options,
    destination: &Path,
) -> Result<nexus_ingest::ImportReport, AppError> {
    let mut bundle = if path == Path::new("-") {
        nexus_ingest::import(std::io::stdin().lock(), options)?
    } else {
        nexus_ingest::import(std::fs::File::open(path)?, options)?
    };
    nexus_ingest::bundle::write(destination, &mut bundle)?;
    Ok(bundle.report)
}

pub fn import_report_text(report: &nexus_ingest::ImportReport) -> String {
    let mut lines = vec![
        format!(
            "Import: {:?} | Source: {}",
            report.status, report.source.source_id
        ),
        format!(
            "Cases: {} accepted / {} identifiable cases rejected",
            report.accepted_cases, report.rejected_cases
        ),
        format!(
            "Records: {} accepted / {} rejected / {} total",
            report.accepted_records, report.rejected_records, report.source.records
        ),
        format!("Source SHA-256: {}", report.source.sha256),
        format!("Observation statuses: {:?}", report.observation_counts),
        format!("Missing fields: {:?}", report.missingness),
        format!("Partitions: {:?}", report.split_counts),
        "Local import only. No provider request was sent.".into(),
    ];
    for (name, items) in [("Errors", &report.issues), ("Warnings", &report.warnings)] {
        lines.push(format!("\n{name}: {}", items.len()));
        for issue in items.iter().take(20) {
            lines.push(format!("  record {}: {:?}", issue.record, issue.code));
        }
        if items.len() > 20 {
            lines.push(
                "  First 20 shown; the complete list is in manifest.json or JSON output.".into(),
            );
        }
    }
    lines.join("\n")
}

pub fn demo(case: &Case) -> Result<ResultRecord, AppError> {
    Ok(interpret(case, mock_response(), Source::Mock)?)
}

pub async fn classify(case: &Case) -> Result<ResultRecord, AppError> {
    let key = std::env::var("TYPESAFE_API_KEY")
        .map_err(|_| ErrorEnvelope::new(ErrorCode::MissingApiKey))?;
    Ok(nexus_jev::classify(case, &key).await?)
}

pub fn key_configured() -> bool {
    std::env::var("TYPESAFE_API_KEY").is_ok_and(|key| !key.trim().is_empty())
}

/// Create a private new file; protect cases and previous results from overwrite.
pub fn save_new(path: &Path, content: &str) -> Result<(), AppError> {
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options.open(path)?;
    file.write_all(content.as_bytes())?;
    file.write_all(b"\n")?;
    Ok(())
}

pub fn display_text(value: &str) -> String {
    value
        .chars()
        .map(|c| if c.is_control() { ' ' } else { c })
        .collect()
}

pub fn result_text(result: &ResultRecord) -> String {
    let source = match result.source {
        Source::Jev => "JEV",
        Source::Mock => "MOCK / NO PREDICTION",
        Source::Replay => "REPLAY / UNVERIFIED",
    };
    let mut lines = vec![
        format!("Case: {} | Source: {source}", result.case_id),
        format!("Status: {} | Research only", result.status),
        format!(
            "Model: {} | Calibration: {}",
            result.model, result.calibration_status
        ),
        "Raw scores are not validated clinical probabilities.".into(),
        String::new(),
        format!("{:<26} {:>10}", "ORIGIN / OUTCOME", "RAW SCORE"),
    ];
    for item in &result.rankings {
        lines.push(format!(
            "{:<26} {:>9.2}%",
            item.origin,
            item.raw_probability * 100.0
        ));
    }
    lines.extend([
        String::new(),
        format!(
            "Review flags: {}",
            if result.reasons.is_empty() {
                "review required".into()
            } else {
                result.reasons.join(", ")
            }
        ),
        format!(
            "Evidence support: {:.2} | Conflict: {:.2} | Provider confidence: {:.2}",
            result.evidence_sufficient, result.conflicting_evidence, result.provider_confidence
        ),
        format!(
            "Tokens: {} input / {} output",
            result.usage.input_tokens, result.usage.output_tokens
        ),
        format!("Request SHA-256: {}", result.request_sha256),
        format!("Case revision SHA-256: {}", result.case_revision_sha256),
    ]);
    lines.join("\n")
}

pub fn pretty(value: &impl serde::Serialize) -> Result<String, AppError> {
    Ok(serde_json::to_string_pretty(value)?)
}

pub fn doctor() -> serde_json::Value {
    use std::io::IsTerminal;
    serde_json::json!({
        "application": "Jev Onco Nexus", "version": env!("CARGO_PKG_VERSION"),
        "model": nexus_core::MODEL, "offline_ready": true,
        "api_key_configured": key_configured(), "provider_check": "not_attempted",
        "tui_terminal_available": std::io::stdin().is_terminal() && std::io::stdout().is_terminal(),
        "live_data_policy": "synthetic_only", "calibration_status": "not_validated_for_cup"
    })
}

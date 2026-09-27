//! Deterministic Markdown from verified run data; no generated clinical narrative.
use crate::workflows::AppError;
use josh_core::{Answer, Source, molecular::*};
use josh_explain::Archive;
use std::fmt::Write;

/// Escape untrusted cells as plain text, including Markdown/HTML and terminal controls.
pub fn cell(value: &str) -> String {
    let mut out = String::new();
    for c in value.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '|' => out.push_str("&#124;"),
            '\\' | '`' | '*' | '_' | '[' | ']' | '#' | '!' => {
                out.push('\\');
                out.push(c);
            }
            c if c.is_control() => out.push(' '),
            _ => out.push(c),
        }
    }
    out
}

pub fn source_label(source: Source) -> &'static str {
    match source {
        Source::Jev => "Jev research inference",
        Source::Mock => "ANALYTICAL DEMO — NO CANCER PREDICTION",
        Source::Replay => "UNVERIFIED REPLAY",
    }
}

pub fn class_label<'a>(run: &'a InferenceRun, id: &'a str) -> &'a str {
    run.taxonomy
        .classes
        .iter()
        .find(|c| c.id == id)
        .map(|c| c.name.as_str())
        .unwrap_or_else(|| {
            if id == run.taxonomy.unknown_id {
                "Insufficient evidence"
            } else if id == run.taxonomy.other_id {
                "Other origin"
            } else {
                id
            }
        })
}

/// Parent of the leading class. Unresolved outcomes have none. This is a local lookup.
pub fn leading_parent<'a>(run: &'a InferenceRun) -> Result<Option<&'a str>, AppError> {
    let rows = rankings(run)?;
    let Some((id, _)) = rows.first() else {
        return Ok(None);
    };
    if *id == run.taxonomy.unknown_id || *id == run.taxonomy.other_id {
        return Ok(None);
    }
    Ok(run
        .taxonomy
        .classes
        .iter()
        .find(|class| class.id == *id)
        .map(|class| class.parent.as_str()))
}

pub fn rankings(run: &InferenceRun) -> Result<Vec<(&str, f64)>, AppError> {
    let mut rows: Vec<_> = probabilities(&run.response)?
        .iter()
        .map(|(id, p)| (id.as_str(), *p))
        .collect();
    rows.sort_by(|a, b| b.1.total_cmp(&a.1).then(a.0.cmp(b.0)));
    Ok(rows)
}

pub fn reason(value: &str) -> &str {
    match value {
        "offline_simulation_or_unverified_replay" => {
            "This is a demonstration or an unverified replay."
        }
        "unresolved_origin" => "The leading outcome is other origin or insufficient evidence.",
        "low_or_ambiguous_raw_score" => {
            "The leading score or separation between candidates is below the engineering threshold."
        }
        "insufficient_support" => {
            "Jev judged the supplied evidence insufficient under the engineering threshold."
        }
        "conflicting_evidence" => {
            "Jev identified conflicting evidence above the engineering threshold."
        }
        _ => value,
    }
}

pub fn markdown(
    features: &FeatureSet,
    run: &InferenceRun,
    archive: Option<&Archive>,
) -> Result<String, AppError> {
    run.verify(features)?;
    if let Some(a) = archive {
        a.validate()?;
        if hash(&a.features)? != hash(features)? || hash(&a.inference)? != hash(run)? {
            return Err("report explanation does not belong to this inference".into());
        }
    }
    let mut s = String::from("# JOSH analysis report\n\n");
    writeln!(s, "**{}**\n", source_label(run.source))?;
    writeln!(s, "| Run summary | Value |\n| --- | --- |")?;
    for (key, value) in [
        ("Sample", features.sample_id.as_str()),
        ("Patient group", features.patient_group_id.as_str()),
        (
            "Data class",
            if features.data_class == josh_core::DataClass::Synthetic {
                "Synthetic"
            } else {
                "Deidentified research"
            },
        ),
        (
            "Decision",
            if run.status == "abstained" {
                "Abstained — no origin assigned"
            } else {
                "Review required — no autonomous diagnosis"
            },
        ),
        ("Model", run.response.model.as_str()),
        ("Calibration", run.calibration_status.as_str()),
    ] {
        writeln!(s, "| {key} | {} |", cell(value))?;
    }
    s.push_str("\nRaw scores describe the model's distribution over the supplied options. They are not validated patient-level cancer probabilities. Decision thresholds are engineering defaults.\n");
    if !run.reasons.is_empty() {
        s.push_str("\n## Decision notes\n\n");
        for r in &run.reasons {
            writeln!(s, "- {}", cell(reason(r)))?;
        }
    }
    s.push_str("\n## Ranked outcomes\n\n| Rank | Outcome | Raw score |\n| ---: | --- | ---: |\n");
    for (i, (id, p)) in rankings(run)?.iter().enumerate() {
        writeln!(
            s,
            "| {} | {} | {:.2}% |",
            i + 1,
            cell(class_label(run, id)),
            p * 100.0
        )?;
    }
    if let Some(parent) = leading_parent(run)? {
        writeln!(
            s,
            "\nBroad group of the leading class: **{}**. This is the parent stored on that class, a local lookup, not a separate Jev judgment.",
            cell(parent)
        )?;
    }
    s.push_str("\n## Evidence checks\n\n| Check | Raw Jev output |\n| --- | ---: |\n");
    for (id, title) in [
        ("evidence_sufficient", "Evidence sufficient"),
        ("conflicting_evidence", "Conflicting evidence"),
    ] {
        if let Answer::Noul { noul } = run.response.answers[id] {
            writeln!(s, "| {title} | {:.2}% |", noul * 100.0)?;
        }
    }
    if let Answer::Choice { confidence, .. } = run.response.answers["primary_site"] {
        writeln!(
            s,
            "| Provider distribution concentration | {:.2}% |",
            confidence * 100.0
        )?;
    }
    let mut boundary = run
        .response
        .answers
        .iter()
        .filter(|(id, answer)| {
            !matches!(id.as_str(), "evidence_sufficient" | "conflicting_evidence")
                && matches!(answer, Answer::Noul { .. })
        })
        .collect::<Vec<_>>();
    boundary.sort_by(|a, b| a.0.cmp(b.0));
    if !boundary.is_empty() {
        s.push_str("\n## Boundary checks\n\nThese are informational. They do not change abstention or review.\n\n| Check | Raw Jev output |\n| --- | ---: |\n");
        for (id, answer) in boundary {
            if let Answer::Noul { noul } = answer {
                writeln!(s, "| {} | {:.2}% |", cell(id), noul * 100.0)?;
            }
        }
    }
    s.push_str("\nThese checks are separate judgments; they are not multiplied into an overall probability. Distribution concentration is not diagnostic confidence.\n");
    s.push_str("\n## Input evidence\n\nMissing and not-tested values remain unavailable; they are never treated as measured negatives or zero.\n\n| Feature | Modality | Status | Measurement | Assay | Coverage |\n| --- | --- | --- | --- | --- | --- |\n");
    for f in &features.features {
        writeln!(
            s,
            "| {} | {} | {} | {} | {} | {} |",
            cell(&f.name),
            f.modality.label(),
            status_label(&f.status),
            cell(
                &f.value
                    .as_ref()
                    .map(|v| v.display())
                    .unwrap_or_else(|| "Unavailable".into())
            ),
            cell(&f.assay),
            cell(&f.coverage)
        )?;
    }
    s.push_str("\n## Explanation\n\n");
    match archive {
        Some(a) if a.result.is_some() => {
            let r = a.result.as_ref().expect("checked result");
            writeln!(s, "Target: **{}**. Method: {}.\n", cell(class_label(run, &r.target_class)), cell(&r.method))?;
            writeln!(s, "Baseline {:.2}% + signed contributions {:+.2} percentage points = full-input score {:.2}%. Additivity residual: {:.3e}.\n", r.baseline_probability * 100.0, r.attributions.iter().map(|v| v.contribution).sum::<f64>() * 100.0, r.full_probability * 100.0, r.additivity_residual)?;
            s.push_str("| Evidence group | Contribution (percentage points) | Sampling standard error (percentage points) |\n| --- | ---: | ---: |\n");
            let mut attrs: Vec<_> = r.attributions.iter().collect();
            attrs.sort_by(|a,b| b.contribution.abs().total_cmp(&a.contribution.abs()).then(a.group.cmp(&b.group)));
            for v in attrs { writeln!(s, "| {} | {:+.4} | {} |", cell(&v.label), v.contribution * 100.0, v.sampling_standard_error.map(|e| format!("{:.4}", e * 100.0)).unwrap_or_else(|| "Not estimated".into()))?; }
            writeln!(s, "\n{} completed evaluations; {} attempts; {} reported input tokens across cached evaluations (including inference). Uncertain reserved tokens: {}.\n", r.evaluations, a.attempts, r.input_tokens, a.uncertain_input_tokens)?;
            for limitation in &r.limitations { writeln!(s, "- {}", cell(limitation))?; }
        }
        Some(a) => { writeln!(s, "Incomplete: {} cached evaluations; {} attempts. No complete attribution is available. The inference above is preserved. Resume from the checkpoint using the same options.", a.evaluations.len(), a.attempts)?; }
        None => s.push_str("Not requested. The inference above is complete. Optional explanations require additional evaluations and a separate budget.\n"),
    }
    s.push_str("\nContributions describe changes in model output, not causal effects or clinical confidence intervals.\n\n## Provenance\n\n| Field | Value |\n| --- | --- |\n");
    for (key, value) in [
        ("Feature SHA-256", run.feature_sha256.as_str()),
        ("Request SHA-256", run.request_sha256.as_str()),
        ("Prompt version", run.prompt_version.as_str()),
        ("Taxonomy version", run.taxonomy.version.as_str()),
        ("Processing version", features.pipeline_version.as_str()),
    ] {
        writeln!(s, "| {key} | {} |", cell(value))?;
    }
    writeln!(
        s,
        "| Inference input tokens | {} |\n| Inference output tokens | {} |",
        run.response.usage.input_tokens, run.response.usage.output_tokens
    )?;
    s.push_str("\n| Feature ID | Source | Source SHA-256 | Record | Reference build |\n| --- | --- | --- | ---: | --- |\n");
    for f in &features.features {
        writeln!(
            s,
            "| {} | {} | {} | {} | {} |",
            cell(&f.id),
            cell(&f.source.source_id),
            cell(&f.source.sha256),
            f.source.record,
            cell(f.reference_build.as_deref().unwrap_or("Unspecified"))
        )?;
    }
    s.push_str("\nHashes verify internal consistency, not provider authenticity or clinical validity. This report is generated locally from the saved run; exporting makes no provider request.\n");
    Ok(s)
}

pub fn status_label(status: &MeasurementStatus) -> &'static str {
    match status {
        MeasurementStatus::Observed => "Observed",
        MeasurementStatus::Unknown => "Unknown",
        MeasurementStatus::NotTested => "Not tested",
    }
}

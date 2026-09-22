use crate::*;

// Engineering defaults only; these are not validated clinical operating points.
const MIN_TOP_PROBABILITY: f64 = 0.75;
const MIN_MARGIN: f64 = 0.15;
const MIN_EVIDENCE_SUPPORT: f64 = 0.8;
const MAX_CONFLICT: f64 = 0.2;

fn probability(p: f64) -> bool {
    p.is_finite() && (0.0..=1.0).contains(&p)
}

pub fn interpret(
    case: &Case,
    response: JevResponse,
    source: Source,
) -> Result<ResultRecord, ValidationError> {
    let request = prepare(case)?;
    if response.model != request.model {
        return Err(ValidationError(
            "provider model does not match pinned model",
        ));
    }
    if response.answers.keys().ne(request.questions.keys()) {
        return Err(ValidationError(
            "provider answer keys do not match questions",
        ));
    }
    let Some(Answer::Choice {
        choice,
        probabilities,
        confidence,
    }) = response.answers.get("primary_site")
    else {
        return Err(ValidationError("missing primary_site choice answer"));
    };
    if probabilities.keys().ne(taxonomy().keys())
        || !probabilities.values().copied().all(probability)
        || !probability(*confidence)
    {
        return Err(ValidationError(
            "invalid provider probabilities, confidence, or taxonomy",
        ));
    }
    if (probabilities.values().sum::<f64>() - 1.0).abs() > 1e-6 {
        return Err(ValidationError("provider probabilities do not sum to one"));
    }
    let mut rankings: Vec<_> = probabilities
        .iter()
        .map(|(origin, p)| RankedOrigin {
            origin: origin.clone(),
            raw_probability: *p,
            calibrated_probability: None,
        })
        .collect();
    rankings.sort_by(|a, b| {
        b.raw_probability
            .total_cmp(&a.raw_probability)
            .then(a.origin.cmp(&b.origin))
    });
    if probabilities
        .get(choice)
        .is_none_or(|p| *p != rankings[0].raw_probability)
    {
        return Err(ValidationError(
            "provider choice is not a maximum-probability option",
        ));
    }
    let noul = |key| match response.answers.get(key) {
        Some(Answer::Noul { noul }) if probability(*noul) => Ok(*noul),
        _ => Err(ValidationError("missing or invalid Noul answer")),
    };
    let sufficient = noul("evidence_sufficient")?;
    let conflicting = noul("conflicting_evidence")?;
    let mut reasons = Vec::new();
    if source != Source::Jev {
        reasons.push("offline_simulation");
    }
    if case.findings.is_empty() {
        reasons.push("no_findings");
    } else if !case.has_observed_evidence() {
        reasons.push("no_observed_findings");
    }
    if choice == "insufficient_evidence" || choice == "other_origin" {
        reasons.push("unresolved_origin");
    }
    // Engineering defaults only. Clinical thresholds require held-out evaluation.
    if rankings[0].raw_probability < MIN_TOP_PROBABILITY {
        reasons.push("low_top_probability");
    }
    if rankings[0].raw_probability - rankings[1].raw_probability < MIN_MARGIN {
        reasons.push("ambiguous_ranking");
    }
    if sufficient < MIN_EVIDENCE_SUPPORT {
        reasons.push("insufficient_support");
    }
    if conflicting > MAX_CONFLICT {
        reasons.push("conflicting_findings");
    }
    let status = if reasons.is_empty() {
        "review_required"
    } else {
        "abstained"
    };
    Ok(ResultRecord {
        result_schema_version: RESULT_SCHEMA_VERSION,
        case_schema_version: case.schema_version,
        case_id: case.case_id.clone(),
        case_revision_sha256: provenance::case_revision(case)?,
        fingerprint_version: provenance::FINGERPRINT_VERSION,
        source,
        status,
        reasons,
        research_only: true,
        probability_kind: match source {
            Source::Jev => "raw_jev_choice_distribution",
            Source::Mock => "synthetic_contract_distribution",
            Source::Replay => "unverified_replay_distribution",
        },
        calibration_status: "not_validated_for_cup",
        model: response.model,
        prompt_version: prompt_version(case),
        taxonomy_version: TAXONOMY_VERSION,
        policy_version: POLICY_VERSION,
        request_sha256: provenance::request_sha256(&request)?,
        provider_confidence: *confidence,
        evidence_sufficient: sufficient,
        conflicting_evidence: conflicting,
        rankings,
        usage: response.usage,
    })
}

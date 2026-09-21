use nexus_core::*;

fn case() -> Case {
    serde_json::from_str(include_str!("../../../fixtures/synthetic-case.json")).unwrap()
}

fn strong_response() -> JevResponse {
    let mut response = mock_response();
    if let Answer::Choice {
        choice,
        probabilities,
        confidence,
    } = response.answers.get_mut("primary_site").unwrap()
    {
        *choice = "lung".into();
        *confidence = 0.7;
        probabilities.values_mut().for_each(|p| *p = 0.0);
        probabilities.insert("lung".into(), 0.85);
        probabilities.insert("colorectal".into(), 0.1);
        probabilities.insert("insufficient_evidence".into(), 0.05);
    }
    response
        .answers
        .insert("evidence_sufficient".into(), Answer::Noul { noul: 0.9 });
    response
}

#[test]
fn request_omits_local_identifiers_and_has_no_ground_truth() {
    let request = prepare(&case()).unwrap();
    assert!(request.state.get("case_id").is_none());
    assert!(request.state.get("data_class").is_none());
    assert_eq!(request.questions.len(), 3);
    let mut raw = serde_json::to_value(case()).unwrap();
    raw["known_primary"] = "lung".into();
    assert!(serde_json::from_value::<Case>(raw).is_err());
}

#[test]
fn invalid_cases_fail_before_request_creation() {
    let mut c = case();
    c.schema_version = 2;
    assert!(prepare(&c).is_err());
    c = case();
    c.age_years = Some(121);
    assert!(prepare(&c).is_err());
    c = case();
    c.findings[1].id = c.findings[0].id.clone();
    assert!(prepare(&c).is_err());
    c = case();
    c.findings[0].value = " ".into();
    assert!(prepare(&c).is_err());
    c = case();
    c.findings[0].value = "x".repeat(1025);
    assert!(prepare(&c).is_err());
}

#[test]
fn strong_answer_still_requires_review_and_is_not_clinically_calibrated() {
    let r = interpret(&case(), strong_response(), Source::Jev).unwrap();
    assert_eq!(r.status, "review_required");
    assert!(r.research_only);
    assert_eq!(r.rankings[0].origin, "lung");
    assert_eq!(r.rankings[0].raw_probability, 0.85);
    assert_eq!(r.provider_confidence, 0.7);
    assert!(
        r.rankings
            .iter()
            .all(|x| x.calibrated_probability.is_none())
    );
}

#[test]
fn empty_evidence_cannot_pass_even_with_high_model_probability() {
    let mut c = case();
    c.findings.clear();
    let r = interpret(&c, strong_response(), Source::Jev).unwrap();
    assert_eq!(r.status, "abstained");
    assert!(r.reasons.contains(&"no_findings"));
}

#[test]
fn mock_and_replay_cannot_be_mistaken_for_live_results() {
    for source in [Source::Mock, Source::Replay] {
        let r = interpret(&case(), strong_response(), source).unwrap();
        assert_eq!(r.source, source);
        assert_eq!(r.status, "abstained");
        assert!(r.reasons.contains(&"offline_simulation"));
    }
}

#[test]
fn low_support_and_conflicts_force_abstention() {
    for (key, value, reason) in [
        ("evidence_sufficient", 0.5, "insufficient_support"),
        ("conflicting_evidence", 0.8, "conflicting_findings"),
    ] {
        let mut response = strong_response();
        response
            .answers
            .insert(key.into(), Answer::Noul { noul: value });
        let r = interpret(&case(), response, Source::Jev).unwrap();
        assert_eq!(r.status, "abstained");
        assert!(r.reasons.contains(&reason));
    }
}

#[test]
fn rejects_invalid_distribution_values_instead_of_normalizing() {
    for value in [-0.1, 1.1, f64::NAN, f64::INFINITY, 0.5] {
        let mut response = strong_response();
        if let Answer::Choice { probabilities, .. } =
            response.answers.get_mut("primary_site").unwrap()
        {
            probabilities.insert("lung".into(), value);
        }
        assert!(interpret(&case(), response, Source::Jev).is_err());
    }
}

#[test]
fn rejects_missing_or_invented_classes() {
    for key in ["lung", "invented_cancer"] {
        let mut response = strong_response();
        if let Answer::Choice { probabilities, .. } =
            response.answers.get_mut("primary_site").unwrap()
        {
            if key == "lung" {
                probabilities.remove(key);
            } else {
                probabilities.insert(key.into(), 0.0);
            }
        }
        assert!(interpret(&case(), response, Source::Jev).is_err());
    }
}

#[test]
fn rejects_wrong_model_answer_type_and_missing_questions() {
    let mut response = strong_response();
    response.model = "jev-latest".into();
    assert!(interpret(&case(), response, Source::Jev).is_err());
    let mut response = strong_response();
    response.answers.remove("evidence_sufficient");
    assert!(interpret(&case(), response, Source::Jev).is_err());
    let mut response = strong_response();
    response
        .answers
        .insert("primary_site".into(), Answer::Noul { noul: 0.9 });
    assert!(interpret(&case(), response, Source::Jev).is_err());
}

#[test]
fn rejects_choice_that_disagrees_with_distribution() {
    let mut response = strong_response();
    if let Answer::Choice { choice, .. } = response.answers.get_mut("primary_site").unwrap() {
        *choice = "breast".into();
    }
    assert!(interpret(&case(), response, Source::Jev).is_err());
}

#[test]
fn request_hash_tracks_evidence_but_not_local_case_identifier() {
    let first = interpret(&case(), strong_response(), Source::Jev).unwrap();
    let mut c = case();
    c.case_id = "ANOTHER-ID".into();
    let second = interpret(&c, strong_response(), Source::Jev).unwrap();
    assert_eq!(first.request_sha256, second.request_sha256);
    c.findings[0].value = "different evidence".into();
    let third = interpret(&c, strong_response(), Source::Jev).unwrap();
    assert_ne!(first.request_sha256, third.request_sha256);
}

#[test]
fn no_match_and_ambiguous_answers_abstain() {
    for origin in ["other_origin", "insufficient_evidence"] {
        let mut response = strong_response();
        if let Answer::Choice {
            choice,
            probabilities,
            ..
        } = response.answers.get_mut("primary_site").unwrap()
        {
            *choice = origin.into();
            probabilities.values_mut().for_each(|p| *p = 0.0);
            probabilities.insert(origin.into(), 1.0);
        }
        let result = interpret(&case(), response, Source::Jev).unwrap();
        assert!(result.reasons.contains(&"unresolved_origin"));
    }
    let result = interpret(&case(), mock_response(), Source::Jev).unwrap();
    assert!(result.reasons.contains(&"ambiguous_ranking"));
}

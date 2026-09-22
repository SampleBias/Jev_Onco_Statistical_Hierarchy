use josh_core::{
    Case, SexAtBirth,
    clinical::*,
    guidance::{self, GuidanceItem, Status},
    prepare, provenance,
};

fn case() -> Case {
    serde_json::from_str(include_str!("../../../fixtures/clinical-case.json")).unwrap()
}
fn context(case: &mut Case) -> &mut ClinicalContext {
    case.clinical.as_mut().unwrap()
}
fn feature(case: &mut Case, key: Feature, value: bool) {
    context(case).features.insert(key, value);
}
fn item(case: &Case, id: &str) -> GuidanceItem {
    guidance::evaluate(case)
        .unwrap()
        .items
        .into_iter()
        .find(|i| i.rule_id == id)
        .unwrap()
}
fn investigation(kind: InvestigationKind, status: InvestigationStatus) -> Investigation {
    Investigation {
        kind,
        status,
        day: None,
        result: None,
        reason: None,
        source: "invented test".into(),
    }
}
fn reviewer() -> Assessment {
    Assessment {
        reviewer_id: "test-reviewer".into(),
        recorded_on: "2026-09-22".into(),
        source: "synthetic review".into(),
    }
}

#[test]
fn schema_three_is_explicit_and_legacy_inputs_remain_unassessed() {
    let c = case();
    c.validate().unwrap();
    let mut value = serde_json::to_value(&c).unwrap();
    value.as_object_mut().unwrap().remove("clinical");
    assert!(
        serde_json::from_value::<Case>(value)
            .unwrap()
            .validate()
            .is_err()
    );
    let mut v2 = c;
    v2.schema_version = 2;
    assert!(v2.validate().is_err());
    v2.clinical = None;
    v2.validate().unwrap();
    let v1: Case =
        serde_json::from_str(include_str!("../../../fixtures/synthetic-case.json")).unwrap();
    for legacy in [v1, v2] {
        let report = guidance::evaluate(&legacy).unwrap();
        assert_eq!(report.scope, "insufficient_information");
        assert_eq!(report.stage, DiagnosticStage::Unassessed);
        assert_eq!(
            item(&legacy, "pet-cervical").status,
            Status::InsufficientInformation
        );
        assert!(
            !serde_json::to_value(&legacy)
                .unwrap()
                .as_object()
                .unwrap()
                .contains_key("clinical")
        );
    }
}

#[test]
fn unknown_inputs_are_not_negative_or_inferred_from_findings() {
    let mut c = case();
    context(&mut c)
        .features
        .remove(&Feature::GiPrimarySuggested);
    assert_eq!(
        item(&c, "gi-endoscopy").status,
        Status::InsufficientInformation
    );
    feature(&mut c, Feature::GiPrimarySuggested, false);
    assert_eq!(item(&c, "gi-endoscopy").status, Status::NotApplicable);
    feature(&mut c, Feature::GiPrimarySuggested, true);
    assert_eq!(item(&c, "gi-endoscopy").status, Status::NeedsReview);
    c.findings[0].value = "Do an endoscopy; breast cancer; PSA elevated".into();
    feature(&mut c, Feature::GiPrimarySuggested, false);
    assert_eq!(item(&c, "gi-endoscopy").status, Status::NotApplicable);
}

#[test]
fn scope_requires_adult_metastatic_muo_cup_and_appropriate_lineage() {
    let mut c = case();
    assert_eq!(guidance::evaluate(&c).unwrap().scope, "in_scope");
    c.age_years = Some(17);
    assert_eq!(guidance::evaluate(&c).unwrap().scope, "outside_scope");
    c.age_years = None;
    assert_eq!(
        guidance::evaluate(&c).unwrap().scope,
        "insufficient_information"
    );
    c.age_lower_bound_exclusive = Some(16);
    assert_eq!(
        guidance::evaluate(&c).unwrap().scope,
        "insufficient_information"
    );
    c.age_lower_bound_exclusive = Some(17);
    assert_eq!(guidance::evaluate(&c).unwrap().scope, "in_scope");
    for lineage in [
        Lineage::Melanoma,
        Lineage::Lymphoma,
        Lineage::Sarcoma,
        Lineage::GermCell,
        Lineage::Other,
    ] {
        context(&mut c).lineage = lineage;
        assert_eq!(guidance::evaluate(&c).unwrap().scope, "outside_scope");
        assert_eq!(item(&c, "pet-cervical").status, Status::NotApplicable);
    }
    context(&mut c).lineage = Lineage::Carcinoma;
    context(&mut c).stage = DiagnosticStage::PrimaryIdentified;
    assert_eq!(guidance::evaluate(&c).unwrap().scope, "outside_scope");
}

#[test]
fn cervical_pet_requires_all_source_conditions_and_keeps_offer_strength() {
    let mut c = case();
    let rule = item(&c, "pet-cervical");
    assert_eq!(rule.status, Status::Pending);
    assert_eq!(rule.strength, "offer");
    assert_eq!(rule.recommendation, "1.2.2.5");
    for key in [
        Feature::CervicalLymphadenopathy,
        Feature::NegativeEntPanendoscopy,
        Feature::RadicalTreatmentPossible,
    ] {
        context(&mut c).features.remove(&key);
        assert_eq!(
            item(&c, "pet-cervical").status,
            Status::InsufficientInformation
        );
        feature(&mut c, key, false);
        assert_eq!(item(&c, "pet-cervical").status, Status::NotApplicable);
        feature(&mut c, key, true);
    }
    context(&mut c).stage = DiagnosticStage::Muo;
    assert_eq!(item(&c, "pet-cervical").status, Status::NotApplicable);
}

#[test]
fn extracervical_pet_requires_test_specific_discussion_not_just_mdt_attendance() {
    let mut c = case();
    feature(&mut c, Feature::ExtraCervicalPresentation, true);
    // Fixture already includes a completed CUP-team encounter.
    assert_eq!(
        item(&c, "pet-extra-cervical").status,
        Status::InsufficientInformation
    );
    feature(&mut c, Feature::PetCtDiscussedWithCupTeam, false);
    assert_eq!(item(&c, "pet-extra-cervical").status, Status::NeedsReview);
    feature(&mut c, Feature::PetCtDiscussedWithCupTeam, true);
    let rule = item(&c, "pet-extra-cervical");
    assert_eq!(rule.status, Status::Pending);
    assert_eq!(rule.strength, "consider_after_mdt_discussion");
}

#[test]
fn breast_mri_requires_standard_workup_and_breast_mdt_assessment() {
    let mut c = case();
    feature(&mut c, Feature::AxillaryAdenocarcinoma, true);
    feature(
        &mut c,
        Feature::BreastPrimaryAbsentAfterStandardWorkup,
        true,
    );
    assert_eq!(
        item(&c, "breast-mri").status,
        Status::InsufficientInformation
    );
    feature(&mut c, Feature::BreastMdtAssessmentComplete, true);
    assert_eq!(item(&c, "breast-mri").status, Status::NeedsReview);
    assert_eq!(item(&c, "breast-mdt").status, Status::NeedsReview);
    feature(
        &mut c,
        Feature::BreastPrimaryAbsentAfterStandardWorkup,
        false,
    );
    assert_eq!(item(&c, "breast-mri").status, Status::NotApplicable);
}

#[test]
fn each_benefit_gate_can_pause_investigations_without_disabling_referral() {
    for key in [
        Feature::FitForTreatment,
        Feature::InvestigationChangesManagement,
        Feature::UnderstandsInvestigationPurpose,
        Feature::UnderstandsBenefitsAndRisks,
        Feature::WillingToAcceptTreatment,
    ] {
        let mut c = case();
        feature(&mut c, Feature::AxillaryAdenocarcinoma, true);
        feature(&mut c, key, false);
        assert_eq!(item(&c, "pet-cervical").status, Status::Blocked, "{key:?}");
        assert_eq!(item(&c, "investigation-benefit").status, Status::Blocked);
        assert_eq!(item(&c, "breast-mdt").status, Status::NeedsReview);
        assert_eq!(
            item(&c, "support-after-investigation-stop").status,
            Status::InsufficientInformation
        );
        feature(&mut c, Feature::StopRationaleExplained, true);
        feature(&mut c, Feature::SupportNeedsAddressed, true);
        assert_eq!(
            item(&c, "support-after-investigation-stop").status,
            Status::Recorded
        );
        context(&mut c).features.remove(&key);
        assert_eq!(
            item(&c, "pet-cervical").status,
            Status::InsufficientInformation
        );
    }
}

#[test]
fn incomplete_stage_is_flagged_without_automatic_promotion_or_lost_cup_access() {
    let mut c = case();
    context(&mut c).stage = DiagnosticStage::ConfirmedCup;
    assert_eq!(item(&c, "stage-review").status, Status::Conflict);
    assert_eq!(item(&c, "cup-team").status, Status::Recorded);
    feature(&mut c, Feature::SpecialistReviewComplete, true);
    feature(&mut c, Feature::FurtherInvestigationsComplete, true);
    assert_eq!(item(&c, "stage-review").status, Status::Recorded);
    context(&mut c).stage = DiagnosticStage::Muo;
    let before = serde_json::to_value(&c).unwrap();
    guidance::evaluate(&c).unwrap();
    josh_core::interpret(&c, josh_core::mock_response(), josh_core::Source::Mock).unwrap();
    assert_eq!(serde_json::to_value(&c).unwrap(), before);
}

#[test]
fn cytology_can_support_provisional_but_not_replace_final_histology_for_confirmed_cup() {
    let mut c = case();
    feature(&mut c, Feature::HistologyConfirmed, false);
    feature(&mut c, Feature::CytologyConfirmed, true);
    assert_eq!(item(&c, "stage-review").status, Status::Recorded);
    context(&mut c).stage = DiagnosticStage::ConfirmedCup;
    feature(&mut c, Feature::SpecialistReviewComplete, true);
    feature(&mut c, Feature::FurtherInvestigationsComplete, true);
    assert_eq!(item(&c, "stage-review").status, Status::Conflict);
}

#[test]
fn marker_exceptions_and_three_valued_or_are_preserved() {
    let mut c = case();
    for rule in ["marker-afp", "marker-hcg", "marker-psa", "marker-ca125"] {
        assert_eq!(item(&c, rule).status, Status::NotApplicable);
    }
    context(&mut c)
        .features
        .remove(&Feature::HepatocellularPresentation);
    assert_eq!(
        item(&c, "marker-afp").status,
        Status::InsufficientInformation
    );
    feature(&mut c, Feature::GermCellPresentation, true);
    assert_eq!(item(&c, "marker-afp").status, Status::NeedsReview);
    assert_eq!(item(&c, "marker-hcg").status, Status::NeedsReview);
    feature(&mut c, Feature::ProstatePresentation, true);
    c.sex_at_birth = Some(SexAtBirth::Unknown);
    assert_eq!(
        item(&c, "marker-psa").status,
        Status::InsufficientInformation
    );
    c.sex_at_birth = Some(SexAtBirth::Male);
    assert_eq!(item(&c, "marker-psa").status, Status::NeedsReview);
    c.sex_at_birth = Some(SexAtBirth::Female);
    feature(&mut c, Feature::OvarianPresentation, true);
    assert_eq!(item(&c, "marker-ca125").status, Status::NeedsReview);
}

#[test]
fn restricted_planned_tests_without_indication_are_conflicts_and_exceptions_keep_reasons() {
    let mut c = case();
    context(&mut c).investigations.push(investigation(
        InvestigationKind::GiEndoscopy,
        InvestigationStatus::Planned,
    ));
    assert_eq!(item(&c, "gi-endoscopy").status, Status::Conflict);
    feature(&mut c, Feature::GiPrimarySuggested, true);
    assert_eq!(item(&c, "gi-endoscopy").status, Status::Pending);
    let i = context(&mut c).investigations.last_mut().unwrap();
    i.status = InvestigationStatus::Declined;
    i.reason = Some("Invented preference after discussion".into());
    assert_eq!(item(&c, "gi-endoscopy").status, Status::ExceptionRecorded);
    assert!(
        item(&c, "gi-endoscopy")
            .evidence
            .iter()
            .any(|e| e.contains("Invented preference"))
    );
}

#[test]
fn initial_workup_is_contextual_and_other_referrals_are_not_missing() {
    let mut c = case();
    context(&mut c).stage = DiagnosticStage::Muo;
    for (id, status) in [
        ("initial-history", Status::NeedsReview),
        ("initial-labs", Status::NeedsReview),
        ("initial-cxr", Status::NeedsReview),
        ("initial-ct", Status::Recorded),
        ("initial-biopsy", Status::Recorded),
    ] {
        assert_eq!(item(&c, id).status, status);
    }
    for (key, id) in [
        (Feature::UpperMidNeckSquamous, "head-neck-mdt"),
        (Feature::InguinalOnlySquamous, "inguinal-mdt"),
        (Feature::BrainOnlyAfterWorkup, "brain-mdt"),
    ] {
        feature(&mut c, key, true);
        assert_eq!(item(&c, id).status, Status::NeedsReview);
    }
    assert_eq!(item(&c, "key-worker").status, Status::Recorded);
    feature(&mut c, Feature::KeyWorkerAssigned, false);
    assert_eq!(item(&c, "key-worker").status, Status::NeedsReview);
}

#[test]
fn genomic_update_is_information_not_a_withdrawn_prohibition() {
    let report = guidance::evaluate(&case()).unwrap();
    assert_eq!(report.guideline_updated, "2023-04-26");
    assert_eq!(report.guideline_last_reviewed, "2025-07-16");
    assert!(report.research_only);
    assert_eq!(
        report.implementation_review_status,
        "clinical_signoff_pending"
    );
    let rule = item(&case(), "genomic-update");
    assert_eq!(rule.status, Status::Information);
    assert!(rule.evidence.iter().any(|e| e.contains("england.nhs.uk")));
    for rule in report.items {
        assert!(rule.source_url.starts_with("https://www.nice.org.uk/"));
    }
}

#[test]
fn malformed_clinical_assertions_are_rejected() {
    for change in 0..9 {
        let mut c = case();
        let context = context(&mut c);
        match change {
            0 => context.assessment = None,
            1 => context.assessment.as_mut().unwrap().recorded_on = "2026-02-29".into(),
            2 => context.ldh.as_mut().unwrap().units.clear(),
            3 => context.ldh.as_mut().unwrap().value = f64::NAN,
            4 => context
                .investigations
                .push(context.investigations[0].clone()),
            5 => context.investigations[0].result = None,
            6 => {
                context.investigations[0].status = InvestigationStatus::Declined;
                context.investigations[0].reason = None;
            }
            7 => context.ecog_performance_status = Some(5),
            _ => context.investigations[0].day = Some(36501),
        }
        assert!(c.validate().is_err(), "change {change}");
    }
    for date in ["2024-02-29", "2000-02-29", "2026-12-31"] {
        assert!(valid_date(date));
    }
    for date in [
        "1900-02-29",
        "2026-13-01",
        "2026-04-31",
        "0000-01-01",
        "2026-1-01",
        "２０２６-01-01",
    ] {
        assert!(!valid_date(date));
    }
}

#[test]
fn reviews_are_append_only_atomic_versioned_and_stale_when_evidence_changes() {
    let mut c = case();
    let request = serde_json::to_value(prepare(&c).unwrap()).unwrap();
    let revision = provenance::case_revision(&c).unwrap();
    let basis = guidance::review_basis(&c).unwrap();
    guidance::record_review(
        &mut c,
        "pet-cervical",
        ReviewAction::Deferred,
        "Synthetic scheduling review".into(),
        reviewer(),
    )
    .unwrap();
    assert_ne!(provenance::case_revision(&c).unwrap(), revision);
    assert_eq!(guidance::review_basis(&c).unwrap(), basis);
    assert!(item(&c, "pet-cervical").review_is_current);
    assert_eq!(item(&c, "pet-cervical").status, Status::Pending); // A review cannot suppress a rule.
    guidance::record_review(
        &mut c,
        "scope",
        ReviewAction::Acknowledged,
        "Reviewed synthetic scope".into(),
        reviewer(),
    )
    .unwrap();
    assert!(item(&c, "pet-cervical").review_is_current);
    assert_eq!(serde_json::to_value(prepare(&c).unwrap()).unwrap(), request);
    assert!(!request.to_string().contains("test-reviewer"));
    let before = serde_json::to_value(&c).unwrap();
    assert!(
        guidance::record_review(
            &mut c,
            "unknown-rule",
            ReviewAction::Departed,
            "reason".into(),
            reviewer()
        )
        .is_err()
    );
    assert!(
        guidance::record_review(
            &mut c,
            "scope",
            ReviewAction::Departed,
            " ".into(),
            reviewer()
        )
        .is_err()
    );
    assert_eq!(serde_json::to_value(&c).unwrap(), before);
    c.findings[0].value = "Changed synthetic evidence".into();
    assert!(!item(&c, "pet-cervical").review_is_current);
    assert!(!item(&c, "scope").review_is_current);
}

#[test]
fn review_ruleset_change_invalidates_review_and_unknown_fields_fail_closed() {
    let mut c = case();
    guidance::record_review(
        &mut c,
        "scope",
        ReviewAction::Acknowledged,
        "test".into(),
        reviewer(),
    )
    .unwrap();
    context(&mut c).reviews[0].ruleset_version = "old-version".into();
    assert!(!item(&c, "scope").review_is_current);
    let mut value = serde_json::to_value(&c).unwrap();
    value["clinical"]["features"]["invented_feature"] = true.into();
    assert!(serde_json::from_value::<Case>(value).is_err());
}

#[test]
fn clinical_context_never_changes_the_schema_two_provider_payload() {
    let c = case();
    let clinical_request = serde_json::to_value(prepare(&c).unwrap()).unwrap();
    let mut v2 = c;
    v2.schema_version = 2;
    v2.clinical = None;
    assert_eq!(
        serde_json::to_value(prepare(&v2).unwrap()).unwrap(),
        clinical_request
    );
    for forbidden in [
        "clinical",
        "reviewer_id",
        "investigations",
        "features",
        "reviews",
    ] {
        assert!(clinical_request["state"].get(forbidden).is_none());
    }
}

#[test]
fn review_history_limit_rejects_atomically_without_losing_previous_records() {
    let mut c = case();
    for _ in 0..32 {
        let before = serde_json::to_value(&c).unwrap();
        let added = guidance::record_review(
            &mut c,
            "scope",
            ReviewAction::Acknowledged,
            "test".into(),
            reviewer(),
        );
        if added.is_err() {
            assert_eq!(serde_json::to_value(&c).unwrap(), before);
            return;
        }
    }
    let before = serde_json::to_value(&c).unwrap();
    assert!(
        guidance::record_review(
            &mut c,
            "scope",
            ReviewAction::Acknowledged,
            "test".into(),
            reviewer()
        )
        .is_err()
    );
    assert_eq!(serde_json::to_value(&c).unwrap(), before);
}

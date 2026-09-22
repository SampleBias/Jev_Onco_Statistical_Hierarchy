//! Deterministic, local review prompts for a documented subset of NICE CG104.
//! No orders, diagnosis, probability adjustment or automatic clinical-stage assignment.
use crate::{Case, SexAtBirth, ValidationError, clinical::*, provenance};
use schemars::JsonSchema;
use serde::Serialize;

pub const RULESET_VERSION: &str = "cg104-review-v0.1";
pub const NICE_URL: &str = "https://www.nice.org.uk/guidance/cg104/chapter/Recommendations";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    NeedsReview,
    InsufficientInformation,
    NotApplicable,
    Recorded,
    Pending,
    ExceptionRecorded,
    Blocked,
    Conflict,
    Information,
}

#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct GuidanceItem {
    pub rule_id: String,
    pub recommendation: String,
    pub source_url: String,
    pub strength: String,
    pub status: Status,
    pub message: String,
    pub evidence: Vec<String>,
    pub review: Option<ReviewDecision>,
    pub review_is_current: bool,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct GuidanceReport {
    pub report_schema_version: u32,
    pub case_id: String,
    pub case_revision_sha256: String,
    pub review_basis_sha256: String,
    pub ruleset_version: String,
    pub guideline_updated: String,
    pub guideline_last_reviewed: String,
    pub implementation_review_status: String,
    pub research_only: bool,
    pub scope: String,
    pub stage: DiagnosticStage,
    pub lineage: Lineage,
    pub items: Vec<GuidanceItem>,
    pub clinical_context: Option<ClinicalContext>,
    pub limitations: Vec<String>,
}

fn all(values: &[Option<bool>]) -> Option<bool> {
    if values.contains(&Some(false)) {
        Some(false)
    } else if values.contains(&None) {
        None
    } else {
        Some(true)
    }
}
fn any(values: &[Option<bool>]) -> Option<bool> {
    if values.contains(&Some(true)) {
        Some(true)
    } else if values.contains(&None) {
        None
    } else {
        Some(false)
    }
}
fn known(value: Option<bool>) -> &'static str {
    match value {
        Some(true) => "yes",
        Some(false) => "no",
        None => "unknown",
    }
}

pub fn review_basis(case: &Case) -> Result<String, ValidationError> {
    let mut basis = case.clone();
    if let Some(c) = &mut basis.clinical {
        c.reviews.clear();
    }
    provenance::case_revision(&basis)
}

/// Append a local review without modifying clinical assertions or suppressing guidance.
pub fn record_review(
    case: &mut Case,
    rule_id: &str,
    action: ReviewAction,
    reason: String,
    assessment: Assessment,
) -> Result<(), ValidationError> {
    let report = evaluate(case)?;
    if !report.items.iter().any(|r| r.rule_id == rule_id) {
        return Err(ValidationError("unknown guidance rule"));
    }
    let mut next = case.clone();
    let context = next.clinical.as_mut().ok_or(ValidationError(
        "review recording requires schema 3 clinical context",
    ))?;
    context.reviews.push(ReviewDecision {
        rule_id: rule_id.into(),
        ruleset_version: RULESET_VERSION.into(),
        action,
        reason,
        assessment,
        basis_sha256: report.review_basis_sha256,
    });
    next.validate()?;
    *case = next;
    Ok(())
}

struct Rule {
    id: &'static str,
    rec: &'static str,
    strength: &'static str,
    message: &'static str,
    condition: Option<bool>,
    inputs: Vec<String>,
    target: Option<InvestigationKind>,
    investigation: bool,
    restricted: bool,
}

pub fn evaluate(case: &Case) -> Result<GuidanceReport, ValidationError> {
    case.validate()?;
    let basis = review_basis(case)?;
    let empty = ClinicalContext::default();
    let c = case.clinical.as_ref().unwrap_or(&empty);
    let f = |feature| c.feature(feature);
    let adult = case.age_years.map(|a| a >= 18).or_else(|| {
        case.age_lower_bound_exclusive
            .filter(|a| *a >= 17)
            .map(|_| true)
    });
    let in_lineage_scope = matches!(
        c.lineage,
        Lineage::Unknown | Lineage::Carcinoma | Lineage::Neuroendocrine
    );
    let stage_known = if c.stage == DiagnosticStage::Unassessed {
        None
    } else {
        Some(c.stage != DiagnosticStage::PrimaryIdentified)
    };
    let scope = all(&[
        adult,
        Some(in_lineage_scope),
        stage_known,
        f(Feature::MetastaticDisease),
    ]);
    let management = all(&[
        f(Feature::FitForTreatment),
        f(Feature::InvestigationChangesManagement),
        f(Feature::UnderstandsInvestigationPurpose),
        f(Feature::UnderstandsBenefitsAndRisks),
        f(Feature::WillingToAcceptTreatment),
    ]);
    let mut items = Vec::new();
    let mut push = |id: &str,
                    rec: &str,
                    strength: &str,
                    status: Status,
                    message: &str,
                    evidence: Vec<String>| {
        let review = c.reviews.iter().rev().find(|r| r.rule_id == id).cloned();
        let current = review
            .as_ref()
            .is_some_and(|r| r.basis_sha256 == basis && r.ruleset_version == RULESET_VERSION);
        items.push(GuidanceItem {
            rule_id: id.into(),
            recommendation: rec.into(),
            source_url: NICE_URL.into(),
            strength: strength.into(),
            status,
            message: message.into(),
            evidence,
            review,
            review_is_current: current,
        });
    };
    push(
        "scope",
        "Scope; terms used",
        "context",
        match scope {
            Some(true) => Status::Recorded,
            Some(false) => Status::NotApplicable,
            None => Status::InsufficientInformation,
        },
        "Adult MUO/CUP scope; assess alternative lineages separately.",
        vec![
            format!("adult: {}", known(adult)),
            format!("stage: {:?}; lineage: {:?}", c.stage, c.lineage),
            format!(
                "metastatic disease: {}",
                known(f(Feature::MetastaticDisease))
            ),
        ],
    );
    let advanced = matches!(
        c.stage,
        DiagnosticStage::ProvisionalCup | DiagnosticStage::ConfirmedCup
    );
    let mut prerequisites = vec![
        if c.stage == DiagnosticStage::ConfirmedCup {
            f(Feature::HistologyConfirmed)
        } else {
            any(&[
                f(Feature::HistologyConfirmed),
                f(Feature::CytologyConfirmed),
            ])
        },
        f(Feature::InitialWorkupComplete),
        if c.lineage == Lineage::Unknown {
            None
        } else {
            Some(matches!(
                c.lineage,
                Lineage::Carcinoma | Lineage::Neuroendocrine
            ))
        },
    ];
    if c.stage == DiagnosticStage::ConfirmedCup {
        prerequisites.extend([
            f(Feature::SpecialistReviewComplete),
            f(Feature::FurtherInvestigationsComplete),
        ]);
    }
    let stage_basis = if advanced {
        all(&prerequisites)
    } else {
        Some(true)
    };
    push(
        "stage-review",
        "Terms used: provisional / confirmed CUP",
        "context",
        if !advanced {
            Status::Information
        } else {
            match stage_basis {
                Some(true) => Status::Recorded,
                Some(false) => Status::Conflict,
                None => Status::InsufficientInformation,
            }
        },
        "Stage is clinician-recorded; check its supporting assessments.",
        vec![
            format!(
                "recorded stage: {:?}; prerequisites: {}",
                c.stage,
                known(stage_basis)
            ),
            format!("assessment: {:?}", c.assessment),
        ],
    );
    push(
        "investigation-benefit",
        "1.3.1.1–1.3.1.3",
        "do_not_offer / conditional",
        if scope == Some(false) {
            Status::NotApplicable
        } else if scope.is_none() {
            Status::InsufficientInformation
        } else {
            match management {
                Some(true) => Status::Recorded,
                Some(false) => Status::Blocked,
                None => Status::InsufficientInformation,
            }
        },
        "Review fitness, management benefit and informed treatment preferences before further investigation.",
        [
            Feature::FitForTreatment,
            Feature::InvestigationChangesManagement,
            Feature::UnderstandsInvestigationPurpose,
            Feature::UnderstandsBenefitsAndRisks,
            Feature::WillingToAcceptTreatment,
        ]
        .iter()
        .map(|v| format!("{v:?}: {}", known(f(*v))))
        .collect(),
    );
    let prognostic = c.ecog_performance_status.is_some()
        && c.ldh.is_some()
        && c.albumin.is_some()
        && f(Feature::LiverMetastases).is_some();
    push(
        "prognostic-context",
        "1.3.2.1–1.3.2.3",
        "take_account",
        if scope == Some(false) {
            Status::NotApplicable
        } else if prognostic && scope == Some(true) {
            Status::Recorded
        } else {
            Status::InsufficientInformation
        },
        "Document prognostic context for discussion; no survival estimate is calculated.",
        vec![format!(
            "ECOG: {:?}; LDH: {:?}; albumin: {:?}; liver metastases: {}",
            c.ecog_performance_status,
            c.ldh,
            c.albumin,
            known(f(Feature::LiverMetastases))
        )],
    );

    push(
        "support-after-investigation-stop",
        "1.3.1.3",
        "explain_and_support",
        if scope == Some(false) || management == Some(true) {
            Status::NotApplicable
        } else if scope.is_none() || management.is_none() {
            Status::InsufficientInformation
        } else {
            match all(&[
                f(Feature::StopRationaleExplained),
                f(Feature::SupportNeedsAddressed),
            ]) {
                Some(true) => Status::Recorded,
                Some(false) => Status::NeedsReview,
                None => Status::InsufficientInformation,
            }
        },
        "If further investigation is inappropriate, review explanation of the decision and physical and psychological support needs.",
        vec![format!(
            "stop rationale explained: {}; support needs addressed: {}",
            known(f(Feature::StopRationaleExplained)),
            known(f(Feature::SupportNeedsAddressed))
        )],
    );

    let evidence = |features: &[Feature]| {
        features
            .iter()
            .map(|v| format!("{v:?}: {}", known(f(*v))))
            .collect::<Vec<_>>()
    };
    let mut rules = Vec::new();
    let mut add =
        |id, rec, strength, message, condition, inputs, target, investigation, restricted| {
            rules.push(Rule {
                id,
                rec,
                strength,
                message,
                condition,
                inputs,
                target,
                investigation,
                restricted,
            });
        };
    use Feature::*;
    use InvestigationKind::*;
    let initial = match c.stage {
        DiagnosticStage::Muo => Some(true),
        DiagnosticStage::Unassessed => None,
        _ => Some(false),
    };
    for (id, target, message) in [
        (
            "initial-history",
            HistoryExamination,
            "Review history and examination coverage.",
        ),
        (
            "initial-labs",
            BaselineLaboratoryTests,
            "Review blood count, renal/liver tests, calcium, LDH and urinalysis in context.",
        ),
        (
            "initial-cxr",
            ChestXray,
            "Review chest radiography in the initial assessment.",
        ),
        (
            "initial-ct",
            CtChestAbdomenPelvis,
            "Review CT coverage of chest, abdomen and pelvis.",
        ),
        (
            "initial-biopsy",
            BiopsyHistology,
            "Review tissue diagnosis and distinction from alternative malignancies.",
        ),
    ] {
        add(
            id,
            "1.2.1.1",
            "offer_as_clinically_appropriate",
            message,
            initial,
            vec![format!(
                "stage: {:?}; individual appropriateness needs review",
                c.stage
            )],
            Some(target),
            true,
            false,
        );
    }
    add(
        "myeloma-screen",
        "1.2.1.1",
        "offer_as_clinically_appropriate",
        "Review myeloma screening for documented lytic bone lesions.",
        all(&[initial, f(LyticBoneLesions)]),
        evidence(&[LyticBoneLesions]),
        Some(MyelomaScreen),
        true,
        false,
    );
    add(
        "gi-endoscopy",
        "1.2.2.2",
        "only_if_indicated",
        "GI endoscopy requires a suggestive clinical, histological or radiological assessment.",
        f(GiPrimarySuggested),
        evidence(&[GiPrimarySuggested]),
        Some(GiEndoscopy),
        true,
        true,
    );
    add(
        "mammography",
        "1.2.2.3",
        "not_routine",
        "Review breast-compatible features before mammography.",
        f(BreastPrimarySuggested),
        evidence(&[BreastPrimarySuggested]),
        Some(Mammography),
        true,
        true,
    );
    add(
        "breast-mri",
        "1.2.2.4",
        "consider",
        "Review contrast-enhanced breast MRI for targeted biopsy after standard breast workup and breast MDT assessment.",
        all(&[
            f(AxillaryAdenocarcinoma),
            f(BreastPrimaryAbsentAfterStandardWorkup),
        ]),
        evidence(&[
            AxillaryAdenocarcinoma,
            BreastPrimaryAbsentAfterStandardWorkup,
            BreastMdtAssessmentComplete,
        ]),
        Some(BreastMri),
        true,
        false,
    );
    let provisional = match c.stage {
        DiagnosticStage::ProvisionalCup => Some(true),
        DiagnosticStage::Unassessed => None,
        _ => Some(false),
    };
    add(
        "pet-cervical",
        "1.2.2.5",
        "offer",
        "Review cervical PET-CT indication with negative ENT panendoscopy and radical-treatment option.",
        all(&[
            provisional,
            f(CervicalLymphadenopathy),
            f(NegativeEntPanendoscopy),
            f(RadicalTreatmentPossible),
        ]),
        evidence(&[
            CervicalLymphadenopathy,
            NegativeEntPanendoscopy,
            RadicalTreatmentPossible,
        ]),
        Some(PetCt),
        true,
        false,
    );
    add(
        "pet-extra-cervical",
        "1.2.2.6",
        "consider_after_mdt_discussion",
        "Before considering extra-cervical PET-CT, record discussion of this investigation with the CUP team or network MDT.",
        all(&[provisional, f(ExtraCervicalPresentation)]),
        evidence(&[ExtraCervicalPresentation, PetCtDiscussedWithCupTeam]),
        Some(PetCt),
        true,
        false,
    );
    let male = match case.sex_at_birth {
        Some(SexAtBirth::Male) => Some(true),
        Some(SexAtBirth::Female) => Some(false),
        _ => None,
    };
    let female = male.map(|v| !v);
    for (id, target, condition, features, message) in [
        (
            "marker-afp",
            Afp,
            any(&[f(GermCellPresentation), f(HepatocellularPresentation)]),
            vec![GermCellPresentation, HepatocellularPresentation],
            "AFP: review the documented germ-cell or hepatocellular presentation.",
        ),
        (
            "marker-hcg",
            Hcg,
            f(GermCellPresentation),
            vec![GermCellPresentation],
            "hCG: review the documented germ-cell presentation.",
        ),
        (
            "marker-psa",
            Psa,
            all(&[male, f(ProstatePresentation)]),
            vec![ProstatePresentation],
            "PSA: review the prostate-compatible presentation and applicability.",
        ),
        (
            "marker-ca125",
            Ca125,
            all(&[female, f(OvarianPresentation)]),
            vec![OvarianPresentation],
            "CA125: review ovarian-compatible presentation; specificity is limited.",
        ),
    ] {
        let mut inputs = evidence(&features);
        inputs.push(format!("recorded sex at birth: {:?}", case.sex_at_birth));
        add(
            id,
            "1.2.2.1",
            "only_listed_exceptions",
            message,
            condition,
            inputs,
            Some(target),
            true,
            true,
        );
    }
    add(
        "testicular-ultrasound",
        "1.2.1.1",
        "offer_as_clinically_appropriate",
        "Review ultrasound in a germ-cell-compatible presentation.",
        all(&[initial, male, f(GermCellPresentation)]),
        evidence(&[GermCellPresentation]),
        Some(TesticularUltrasound),
        true,
        false,
    );
    add(
        "ihc-panel",
        "1.2.2.7–1.2.2.8",
        "pathology_review",
        "Pathology review: CK7, CK20, TTF-1, PLAP and sex-specific ER/PSA in the source panel; select additional stains from results and clinical context. This checklist does not interpret stains or establish tissue origin.",
        f(Adenocarcinoma),
        evidence(&[Adenocarcinoma]),
        Some(IhcPanel),
        true,
        false,
    );
    add(
        "ascites-tissue",
        "1.2.3.3",
        "obtain_if_feasible",
        "Review histological tissue sampling for MUO with ascites.",
        all(&[initial, f(Ascites), f(TissueSamplingFeasible)]),
        evidence(&[Ascites, TissueSamplingFeasible]),
        Some(AscitesTissueSampling),
        true,
        false,
    );
    for (id, rec, target, feature, message) in [
        (
            "breast-mdt",
            "1.4.1.2",
            BreastMdt,
            AxillaryAdenocarcinoma,
            "Review referral to a breast MDT.",
        ),
        (
            "head-neck-mdt",
            "1.4.1.1",
            HeadNeckMdt,
            UpperMidNeckSquamous,
            "Review referral to a head and neck MDT.",
        ),
        (
            "inguinal-mdt",
            "1.4.1.3",
            SpecialistSurgicalMdt,
            InguinalOnlySquamous,
            "Review specialist surgical MDT assessment.",
        ),
        (
            "brain-mdt",
            "1.4.2.1",
            NeuroOncologyMdt,
            BrainOnlyAfterWorkup,
            "Review referral to a neuro-oncology MDT.",
        ),
    ] {
        add(
            id,
            rec,
            "refer",
            message,
            f(feature),
            evidence(&[feature]),
            Some(target),
            false,
            false,
        );
    }
    add(
        "cup-team",
        "1.1.1.1",
        "access_to_team",
        "Review CUP team involvement.",
        Some(true),
        vec![],
        Some(CupTeam),
        false,
        false,
    );
    push(
        "key-worker",
        "1.1.1.2",
        "record",
        match all(&[scope, f(KeyWorkerAssigned)]) {
            _ if scope == Some(false) => Status::NotApplicable,
            Some(true) => Status::Recorded,
            Some(false) if scope == Some(true) => Status::NeedsReview,
            _ => Status::InsufficientInformation,
        },
        "Record the designated key worker.",
        evidence(&[KeyWorkerAssigned]),
    );

    for rule in rules {
        let recorded = rule
            .target
            .and_then(|target| c.investigations.iter().find(|i| i.kind == target));
        let mut inputs = rule.inputs;
        if let Some(i) = recorded {
            inputs.push(format!("recorded investigation: {i:?}"));
        }
        let applies = all(&[scope, rule.condition]);
        // Attendance at an MDT is not evidence that this specific test was discussed.
        let discussion = match rule.id {
            "pet-extra-cervical" => f(PetCtDiscussedWithCupTeam),
            "breast-mri" => f(BreastMdtAssessmentComplete),
            _ => Some(true),
        };
        let status = match applies {
            Some(false) => {
                if scope == Some(true)
                    && rule.restricted
                    && recorded.is_some_and(|i| {
                        matches!(
                            i.status,
                            InvestigationStatus::Planned | InvestigationStatus::Pending
                        )
                    })
                {
                    Status::Conflict
                } else {
                    Status::NotApplicable
                }
            }
            None => Status::InsufficientInformation,
            Some(true) => {
                if (rule.investigation && advanced && stage_basis != Some(true))
                    || discussion.is_none()
                {
                    Status::InsufficientInformation
                } else if discussion == Some(false) {
                    Status::NeedsReview
                } else if recorded.is_some_and(|i| i.status == InvestigationStatus::Completed) {
                    Status::Recorded
                } else if recorded.is_some_and(|i| {
                    matches!(
                        i.status,
                        InvestigationStatus::Declined
                            | InvestigationStatus::NotIndicated
                            | InvestigationStatus::Contraindicated
                    )
                }) {
                    Status::ExceptionRecorded
                } else if rule.investigation && management == Some(false) {
                    Status::Blocked
                } else if rule.investigation && management.is_none() {
                    Status::InsufficientInformation
                } else if recorded.is_some() {
                    Status::Pending
                } else {
                    Status::NeedsReview
                }
            }
        };
        if rule.investigation {
            inputs.push(format!(
                "investigation-benefit prerequisites: {}",
                known(management)
            ));
        }
        push(
            rule.id,
            rule.rec,
            rule.strength,
            status,
            rule.message,
            inputs,
        );
    }
    push(
        "genomic-update",
        "1.2.2.9 / 1.3.2.4 withdrawn April 2023",
        "reference",
        Status::Information,
        "Use current genomic testing eligibility resources; the withdrawn prohibitions are not active rules.",
        vec![
            "https://www.nice.org.uk/guidance/cg104/chapter/Update-information".into(),
            "https://www.england.nhs.uk/publication/national-genomic-test-directories/".into(),
        ],
    );
    Ok(GuidanceReport {
        report_schema_version: 1, case_id: case.case_id.clone(), case_revision_sha256: provenance::case_revision(case)?,
        review_basis_sha256: basis, ruleset_version: RULESET_VERSION.into(), guideline_updated: "2023-04-26".into(),
        guideline_last_reviewed: "2025-07-16".into(), implementation_review_status: "clinical_signoff_pending".into(),
        research_only: true, scope: match scope { Some(true) => "in_scope", Some(false) => "outside_scope", None => "insufficient_information" }.into(),
        stage: c.stage, lineage: c.lineage, items, clinical_context: case.clinical.clone(),
        limitations: vec![
            "Selected CG104 diagnostic/review recommendations; not a complete guideline implementation or treatment protocol.".into(),
            "Recorded status does not establish clinical adequacy. Unknown inputs never imply negative findings.".into(),
            "Clinical signoff and outcome validation pending. Jev scores do not change rules or recorded stage.".into(),
            "Sex-specific source recommendations use recorded sex at birth; individual applicability requires clinician review.".into(),
            "The shared benefit gate conservatively pauses investigation prompts; it is not an order or an automated decision to stop care. Referrals and support are separate.".into(),
        ],
    })
}

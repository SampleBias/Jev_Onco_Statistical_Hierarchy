//! Local clinical assertions; never inferred from model scores or free text.
use crate::{ValidationError, valid_id, valid_sha256};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum DiagnosticStage {
    #[default]
    Unassessed,
    Muo,
    ProvisionalCup,
    ConfirmedCup,
    PrimaryIdentified,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Lineage {
    #[default]
    Unknown,
    Carcinoma,
    Neuroendocrine,
    Melanoma,
    Lymphoma,
    Sarcoma,
    GermCell,
    Other,
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum Feature {
    MetastaticDisease,
    HistologyConfirmed,
    CytologyConfirmed,
    InitialWorkupComplete,
    SpecialistReviewComplete,
    FurtherInvestigationsComplete,
    FitForTreatment,
    InvestigationChangesManagement,
    UnderstandsInvestigationPurpose,
    UnderstandsBenefitsAndRisks,
    WillingToAcceptTreatment,
    GiPrimarySuggested,
    BreastPrimarySuggested,
    GermCellPresentation,
    HepatocellularPresentation,
    ProstatePresentation,
    OvarianPresentation,
    LyticBoneLesions,
    CervicalLymphadenopathy,
    ExtraCervicalPresentation,
    PetCtDiscussedWithCupTeam,
    BreastMdtAssessmentComplete,
    NegativeEntPanendoscopy,
    RadicalTreatmentPossible,
    AxillaryAdenocarcinoma,
    UpperMidNeckSquamous,
    InguinalOnlySquamous,
    BrainOnlyAfterWorkup,
    BreastPrimaryAbsentAfterStandardWorkup,
    Ascites,
    TissueSamplingFeasible,
    Adenocarcinoma,
    LiverMetastases,
    KeyWorkerAssigned,
    StopRationaleExplained,
    SupportNeedsAddressed,
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum InvestigationKind {
    HistoryExamination,
    BaselineLaboratoryTests,
    ChestXray,
    CtChestAbdomenPelvis,
    BiopsyHistology,
    MyelomaScreen,
    GiEndoscopy,
    Mammography,
    BreastMri,
    PetCt,
    Afp,
    Hcg,
    Psa,
    Ca125,
    TesticularUltrasound,
    IhcPanel,
    AdditionalIhc,
    AscitesTissueSampling,
    CupTeam,
    BreastMdt,
    HeadNeckMdt,
    SpecialistSurgicalMdt,
    NeuroOncologyMdt,
    GenomicReview,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum InvestigationStatus {
    Planned,
    Pending,
    Completed,
    Declined,
    NotIndicated,
    Contraindicated,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Assessment {
    pub reviewer_id: String,
    /// ISO calendar date, YYYY-MM-DD; local only.
    pub recorded_on: String,
    pub source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Investigation {
    pub kind: InvestigationKind,
    pub status: InvestigationStatus,
    /// Days from one case-specific reference point; null means unknown.
    #[schemars(range(min = -36500, max = 36500))]
    pub day: Option<i32>,
    pub result: Option<String>,
    pub reason: Option<String>,
    pub source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Measurement {
    #[schemars(range(min = 0))]
    pub value: f64,
    pub units: String,
    #[schemars(range(min = -36500, max = 36500))]
    pub day: Option<i32>,
    pub source: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ReviewAction {
    Acknowledged,
    Deferred,
    NotApplicable,
    Departed,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ReviewDecision {
    pub rule_id: String,
    pub ruleset_version: String,
    pub action: ReviewAction,
    pub reason: String,
    pub assessment: Assessment,
    /// Case hash excluding review history, so changed evidence invalidates a review.
    pub basis_sha256: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
#[serde(default, deny_unknown_fields)]
pub struct ClinicalContext {
    pub assessment: Option<Assessment>,
    pub stage: DiagnosticStage,
    pub lineage: Lineage,
    /// An absent key is unknown, not false.
    pub features: BTreeMap<Feature, bool>,
    /// Latest episode record per kind; earlier events remain in prior case revisions.
    #[schemars(length(max = 32))]
    pub investigations: Vec<Investigation>,
    #[schemars(range(max = 4))]
    pub ecog_performance_status: Option<u8>,
    pub ldh: Option<Measurement>,
    pub albumin: Option<Measurement>,
    #[schemars(length(max = 32))]
    pub reviews: Vec<ReviewDecision>,
}

fn text_ok(s: &str, limit: usize) -> bool {
    !s.trim().is_empty() && s.len() <= limit && !s.chars().any(char::is_control)
}

pub fn valid_date(s: &str) -> bool {
    let b = s.as_bytes();
    if b.len() != 10
        || b[4] != b'-'
        || b[7] != b'-'
        || b.iter()
            .enumerate()
            .any(|(i, c)| i != 4 && i != 7 && !c.is_ascii_digit())
    {
        return false;
    }
    let year: u32 = s[..4].parse().unwrap_or(0);
    let month: usize = s[5..7].parse().unwrap_or(0);
    let day: u32 = s[8..].parse().unwrap_or(0);
    let leap = year.is_multiple_of(4) && (!year.is_multiple_of(100) || year.is_multiple_of(400));
    let days = [
        31,
        if leap { 29 } else { 28 },
        31,
        30,
        31,
        30,
        31,
        31,
        30,
        31,
        30,
        31,
    ];
    year > 0 && (1..=12).contains(&month) && day > 0 && day <= days[month.saturating_sub(1).min(11)]
}

impl Assessment {
    fn valid(&self) -> bool {
        valid_id(&self.reviewer_id) && valid_date(&self.recorded_on) && text_ok(&self.source, 256)
    }
}

impl ClinicalContext {
    pub fn feature(&self, feature: Feature) -> Option<bool> {
        self.features.get(&feature).copied()
    }

    pub fn validate(&self) -> Result<(), ValidationError> {
        let populated = self.stage != DiagnosticStage::Unassessed
            || self.lineage != Lineage::Unknown
            || !self.features.is_empty()
            || !self.investigations.is_empty()
            || self.ecog_performance_status.is_some()
            || self.ldh.is_some()
            || self.albumin.is_some();
        if self.assessment.as_ref().is_some_and(|a| !a.valid())
            || (populated && self.assessment.is_none())
        {
            return Err(ValidationError(
                "clinical assertions require a valid assessment, date and source",
            ));
        }
        if self.ecog_performance_status.is_some_and(|v| v > 4)
            || self.investigations.len() > 32
            || self.reviews.len() > 32
        {
            return Err(ValidationError("clinical context exceeds its limits"));
        }
        let mut kinds = BTreeSet::new();
        for i in &self.investigations {
            if !kinds.insert(i.kind)
                || !text_ok(&i.source, 256)
                || i.day.is_some_and(|d| !(-36500..=36500).contains(&d))
                || [&i.result, &i.reason]
                    .iter()
                    .any(|v| v.as_ref().is_some_and(|s| !text_ok(s, 512)))
                || (i.status == InvestigationStatus::Completed && i.result.is_none())
                || (matches!(
                    i.status,
                    InvestigationStatus::NotIndicated
                        | InvestigationStatus::Contraindicated
                        | InvestigationStatus::Declined
                ) && i.reason.is_none())
            {
                return Err(ValidationError(
                    "invalid investigation record; completion needs a result, exceptions need a reason",
                ));
            }
        }
        for m in [&self.ldh, &self.albumin].into_iter().flatten() {
            if !m.value.is_finite()
                || m.value < 0.0
                || !text_ok(&m.units, 32)
                || !text_ok(&m.source, 256)
                || m.day.is_some_and(|d| !(-36500..=36500).contains(&d))
            {
                return Err(ValidationError("invalid clinical measurement"));
            }
        }
        for r in &self.reviews {
            if !valid_id(&r.rule_id)
                || !text_ok(&r.ruleset_version, 64)
                || !text_ok(&r.reason, 512)
                || !r.assessment.valid()
                || !valid_sha256(&r.basis_sha256)
            {
                return Err(ValidationError("invalid clinical review record"));
            }
        }
        Ok(())
    }
}

use crate::MAX_CASE_BYTES;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Debug, thiserror::Error)]
#[error("{0}")]
pub struct ValidationError(pub &'static str);

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DataClass {
    Synthetic,
    DeidentifiedResearch,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceKind {
    Histology,
    Ihc,
    Molecular,
    Clinical,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Finding {
    #[schemars(regex(pattern = "^[A-Za-z0-9_-]{1,64}$"))]
    pub id: String,
    pub kind: EvidenceKind,
    /// Nonblank; limit is measured in UTF-8 bytes by the runtime validator.
    #[schemars(length(min = 1, max = 128), extend("x-maxUtf8Bytes" = 128))]
    pub name: String,
    /// Nonblank; limit is measured in UTF-8 bytes by the runtime validator.
    #[schemars(length(min = 1, max = 1024), extend("x-maxUtf8Bytes" = 1024))]
    pub value: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub observation: Option<Observation>,
    /// Local provenance. Excluded from the provider state.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<SourceReference>,
}

#[derive(
    Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq, PartialOrd, Ord,
)]
#[serde(rename_all = "snake_case")]
pub enum ObservationStatus {
    Observed,
    Positive,
    Negative,
    Equivocal,
    NotTested,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Observation {
    pub status: ObservationStatus,
    pub units: Option<String>,
    pub assay: Option<String>,
    /// Relative study timepoint; do not put direct identifiers here.
    pub timepoint: Option<String>,
    pub reference_build: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SourceReference {
    pub source_id: String,
    pub source_sha256: String,
    /// One-based input record; CSV/TSV header is record 1. Not a physical line number.
    pub record: u64,
    pub field: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CaseMetadata {
    /// Local group ID for cohort partitioning; never sent to Jev.
    pub patient_id: Option<String>,
    pub sample_id: Option<String>,
    pub institution_id: Option<String>,
    pub evidence_cutoff: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SexAtBirth {
    Female,
    Male,
    Intersex,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
#[schemars(extend("allOf" = [{
    "if": {"properties": {"schema_version": {"minimum": 2}}},
    "then": {"required": ["metadata"], "properties": {"metadata": {"type": "object"}, "findings": {"items": {"required": ["source", "observation"], "properties": {"source": {"type": "object"}, "observation": {"type": "object"}}}}}},
    "else": {"properties": {"metadata": {"type": "null"}, "age_lower_bound_exclusive": {"type": "null"}, "findings": {"items": {"properties": {"source": {"type": "null"}, "observation": {"type": "null"}}}}}}
}, {
    "if": {"properties": {"schema_version": {"const": 3}}},
    "then": {"required": ["clinical"], "properties": {"clinical": {"type": "object"}}},
    "else": {"properties": {"clinical": {"type": "null"}}}
}]))]
pub struct Case {
    #[schemars(range(min = 1, max = 3))]
    pub schema_version: u32,
    #[schemars(regex(pattern = "^[A-Za-z0-9_-]{1,64}$"))]
    pub case_id: String,
    pub data_class: DataClass,
    #[schemars(range(max = 120))]
    pub age_years: Option<u8>,
    pub sex_at_birth: Option<SexAtBirth>,
    /// Biopsy site, not necessarily the primary origin. Nonblank when supplied.
    #[schemars(length(min = 1, max = 128), extend("x-maxUtf8Bytes" = 128))]
    pub specimen_site: Option<String>,
    /// IDs must be unique. Runtime also limits serialized case size to 16384 bytes.
    #[schemars(length(max = 64))]
    pub findings: Vec<Finding>,
    /// Schema 2: preserve censored ages such as >89 without treating them as exact.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(range(max = 119))]
    pub age_lower_bound_exclusive: Option<u8>,
    /// Schema 2 local metadata; never included in the provider state.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<CaseMetadata>,
    /// Schema 3: clinician-recorded local context, excluded from Jev state.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub clinical: Option<crate::clinical::ClinicalContext>,
}

impl Case {
    pub fn has_observed_evidence(&self) -> bool {
        self.findings.iter().any(|finding| {
            finding.observation.as_ref().is_none_or(|o| {
                !matches!(
                    o.status,
                    ObservationStatus::Unknown | ObservationStatus::NotTested
                )
            })
        })
    }

    pub fn validate(&self) -> Result<(), ValidationError> {
        if !matches!(self.schema_version, 1..=3) {
            return Err(ValidationError("unsupported schema_version"));
        }
        if self.schema_version == 1
            && (self.metadata.is_some()
                || self.age_lower_bound_exclusive.is_some()
                || self
                    .findings
                    .iter()
                    .any(|f| f.source.is_some() || f.observation.is_some()))
        {
            return Err(ValidationError(
                "schema 1 cannot contain schema 2 evidence metadata",
            ));
        }
        if self.schema_version >= 2
            && (self.metadata.is_none()
                || self
                    .findings
                    .iter()
                    .any(|f| f.source.is_none() || f.observation.is_none()))
        {
            return Err(ValidationError(
                "schema 2/3 requires case metadata and finding source/observation",
            ));
        }
        if (self.schema_version == 3) != self.clinical.is_some() {
            return Err(ValidationError("clinical context requires schema 3"));
        }
        if let Some(context) = &self.clinical {
            context.validate()?;
        }
        if self.age_lower_bound_exclusive.is_some_and(|age| age >= 120)
            || (self.age_years.is_some() && self.age_lower_bound_exclusive.is_some())
        {
            return Err(ValidationError("invalid or conflicting censored age"));
        }
        if let Some(metadata) = &self.metadata {
            for value in [
                &metadata.patient_id,
                &metadata.sample_id,
                &metadata.institution_id,
            ]
            .into_iter()
            .flatten()
            {
                if !valid_id(value) {
                    return Err(ValidationError("invalid local metadata identifier"));
                }
            }
            if !optional_text(&metadata.evidence_cutoff, 128) {
                return Err(ValidationError("invalid evidence cutoff"));
            }
        }
        if !valid_id(&self.case_id) {
            return Err(ValidationError(
                "case_id must be 1-64 ASCII letters, digits, underscores or hyphens",
            ));
        }
        if self.age_years.is_some_and(|age| age > 120) {
            return Err(ValidationError("age_years exceeds 120"));
        }
        if self
            .specimen_site
            .as_ref()
            .is_some_and(|s| s.trim().is_empty() || s.len() > 128)
        {
            return Err(ValidationError("invalid specimen_site"));
        }
        if self.findings.len() > 64 {
            return Err(ValidationError("at most 64 findings allowed"));
        }
        let mut ids = BTreeSet::new();
        for finding in &self.findings {
            if let Some(source) = &finding.source
                && (!valid_id(&source.source_id)
                    || !valid_sha256(&source.source_sha256)
                    || source.record == 0
                    || source.field.trim().is_empty()
                    || source.field.len() > 128)
            {
                return Err(ValidationError("invalid finding source reference"));
            }
            if let Some(observation) = &finding.observation {
                if [
                    &observation.units,
                    &observation.assay,
                    &observation.timepoint,
                    &observation.reference_build,
                ]
                .into_iter()
                .any(|s| !optional_text(s, 128))
                {
                    return Err(ValidationError("invalid observation metadata"));
                }
                if let Some(recorded) = categorical_status(&finding.value)
                    && observation.status != recorded
                {
                    return Err(ValidationError(
                        "observation status contradicts the recorded value",
                    ));
                }
                if observation.status == ObservationStatus::NotTested
                    && categorical_status(&finding.value) != Some(ObservationStatus::NotTested)
                {
                    return Err(ValidationError(
                        "not-tested findings must have an explicit not-tested value",
                    ));
                }
            }
            if !valid_id(&finding.id) || !ids.insert(&finding.id) {
                return Err(ValidationError("finding IDs must be valid and unique"));
            }
            if finding.name.trim().is_empty()
                || finding.name.len() > 128
                || finding.value.trim().is_empty()
                || finding.value.len() > 1024
            {
                return Err(ValidationError(
                    "finding name or value is empty or too long",
                ));
            }
        }
        if serde_json::to_vec(self)
            .map_err(|_| ValidationError("invalid case"))?
            .len()
            > MAX_CASE_BYTES
        {
            return Err(ValidationError("case exceeds byte budget"));
        }
        Ok(())
    }
}

pub fn valid_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 64
        && id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
}

pub fn valid_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

fn optional_text(value: &Option<String>, max: usize) -> bool {
    value
        .as_ref()
        .is_none_or(|s| !s.trim().is_empty() && s.len() <= max)
}

/// Narrow lexical aliases only; no clinical inference from free text or intensity.
pub fn categorical_status(value: &str) -> Option<ObservationStatus> {
    match value.trim().to_ascii_lowercase().as_str() {
        "positive" | "pos" | "+" => Some(ObservationStatus::Positive),
        "negative" | "neg" | "-" => Some(ObservationStatus::Negative),
        "equivocal" => Some(ObservationStatus::Equivocal),
        "not tested" | "not_tested" | "not assayed" => Some(ObservationStatus::NotTested),
        "unknown" => Some(ObservationStatus::Unknown),
        _ => None,
    }
}

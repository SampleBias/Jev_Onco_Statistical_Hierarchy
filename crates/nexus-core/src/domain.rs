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

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
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
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum SexAtBirth {
    Female,
    Male,
    Intersex,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Case {
    #[schemars(range(min = 1, max = 1))]
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
}

impl Case {
    pub fn validate(&self) -> Result<(), ValidationError> {
        if self.schema_version != 1 {
            return Err(ValidationError("unsupported schema_version"));
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

fn valid_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 64
        && id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
}

use nexus_core::Case;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const IMPORTER_VERSION: &str = "structured-evidence-v1";
pub const MAX_SOURCE_BYTES: u64 = 64 * 1024 * 1024;
pub const MAX_RECORD_BYTES: usize = 16_384;
pub const MAX_RECORDS: usize = 50_000;
pub const MAX_CASES: usize = 2_000;
pub const MAX_MANIFEST_BYTES: usize = 16 * 1024 * 1024;
pub const MAX_REQUEST_BYTES: usize = 65_536;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum InputFormat {
    Json,
    Jsonl,
    Csv,
    Tsv,
}

#[derive(Debug, Clone)]
pub struct Options {
    pub format: InputFormat,
    pub source_id: String,
    pub split_seed: String,
    pub holdout_institution: Option<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("source read or bundle write failed")]
    Io(#[from] std::io::Error),
    #[error(
        "invalid import options; identifiers and seed must be 1-64 ASCII letters, digits, underscores or hyphens"
    )]
    Options,
    #[error("source exceeds the 64 MiB import limit")]
    SourceTooLarge,
    #[error("import exceeds the record or case count limit")]
    TooManyRecords,
    #[error("CSV/TSV headers are missing, duplicated or unsupported; see the import guide")]
    Columns,
    #[error("source has no records")]
    EmptySource,
    #[error("import bundle is incomplete, incompatible or has changed")]
    InvalidBundle,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum IssueCode {
    InvalidJson,
    InvalidRow,
    InvalidCase,
    RecordTooLarge,
    DuplicateCase,
    DuplicateFinding,
    ConflictingMetadata,
    ConflictingLabel,
    InvalidLabel,
    InvalidAge,
    InvalidStatus,
    ConflictingStatus,
    DuplicateSample,
    InvalidatedCase,
    CaseBudgetExceeded,
    RequestBudgetExceeded,
    MissingPatientGroup,
    MultipleSpecimens,
    UnknownObservation,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Issue {
    pub record: u64,
    pub code: IssueCode,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SourceManifest {
    pub source_id: String,
    pub sha256: String,
    pub format: InputFormat,
    pub bytes: u64,
    pub records: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ImportStatus {
    Complete,
    Partial,
    Rejected,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CaseEntry {
    pub case_id: String,
    pub case_revision_sha256: String,
    pub request_sha256: String,
    pub state_bytes: usize,
    pub request_bytes: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ImportReport {
    pub manifest_version: u32,
    pub importer_version: String,
    pub status: ImportStatus,
    pub source: SourceManifest,
    pub accepted_cases: usize,
    pub rejected_cases: usize,
    pub accepted_records: usize,
    pub rejected_records: usize,
    pub issues: Vec<Issue>,
    pub warnings: Vec<Issue>,
    pub observation_counts: BTreeMap<String, usize>,
    pub missingness: BTreeMap<String, usize>,
    pub label_counts: BTreeMap<String, usize>,
    pub split_counts: BTreeMap<String, usize>,
    /// Exact provider tokenization is not implemented; byte bounds are not token counts.
    pub token_budget_verified: bool,
    pub cases: Vec<CaseEntry>,
    pub artifacts_sha256: BTreeMap<String, String>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct LabelRecord {
    pub case_id: String,
    pub primary_origin: String,
    pub taxonomy_version: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Partition {
    Development,
    Calibration,
    Test,
    Unassigned,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SplitEntry {
    pub case_id: String,
    pub patient_id: Option<String>,
    pub partition: Partition,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SplitManifest {
    pub version: u32,
    pub algorithm: String,
    pub seed: String,
    pub holdout_institution: Option<String>,
    pub entries: Vec<SplitEntry>,
}

pub struct ImportBundle {
    pub report: ImportReport,
    pub cases: Vec<Case>,
    pub labels: Vec<LabelRecord>,
    pub splits: SplitManifest,
}

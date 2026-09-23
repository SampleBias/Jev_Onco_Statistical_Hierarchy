//! Versioned research reference and expression evidence contracts.
use crate::sample::{ExpressionUnit, GeneIdentity, Transform};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
pub const REFERENCE_PIPELINE: &str = "expression-reference-v1";
pub const MOLECULAR_PROMPT: &str = "molecular-evidence-v1";
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Compatibility {
    pub organism: String,
    pub units: ExpressionUnit,
    pub transform: Transform,
    pub platform: String,
    pub reference_genome: Option<String>,
    pub gene_map_sha256: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ReferenceMember {
    pub sample_id: String,
    pub patient_group_id: String,
    pub class_id: String,
    pub measurement_sha256: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ReferenceClass {
    pub class_id: String,
    pub cancer_type: String,
    pub samples: usize,
    pub mean_expression: Vec<f64>,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ReferenceRelease {
    pub schema_version: u32,
    pub pipeline_version: String,
    pub release_id: String,
    pub citation: String,
    pub created_at_unix_seconds: u64,
    pub synthetic: bool,
    pub source_dataset_id: String,
    pub source_manifest_sha256: String,
    pub source_input_sha256: String,
    pub source_labels_sha256: String,
    pub compatibility: Compatibility,
    pub minimum_genes: usize,
    pub minimum_overlap: f64,
    pub genes: Vec<GeneIdentity>,
    pub members: Vec<ReferenceMember>,
    pub classes: Vec<ReferenceClass>,
    pub limitations: Vec<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ReferenceSimilarity {
    pub class_id: String,
    pub cancer_type: String,
    pub reference_samples: usize,
    pub pearson_r: Option<f64>,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ComparisonStatus {
    Compared,
    Incompatible,
    InsufficientData,
    UndefinedSimilarity,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EvidencePackage {
    pub schema_version: u32,
    pub pipeline_version: String,
    pub dataset_id: String,
    pub sample_id: String,
    pub source_sha256: String,
    pub measurement_sha256: String,
    pub reference_release_id: String,
    pub reference_sha256: String,
    pub synthetic: bool,
    pub status: ComparisonStatus,
    pub reasons: Vec<String>,
    pub common_genes: usize,
    pub reference_genes: usize,
    pub overlap_fraction: f64,
    pub used_gene_ids: Vec<String>,
    pub similarities: Vec<ReferenceSimilarity>,
    pub missing_modalities: Vec<String>,
    pub conflict_assessment: String,
    pub ood_assessment: String,
    pub probability_kind: String,
    pub limitations: Vec<String>,
}

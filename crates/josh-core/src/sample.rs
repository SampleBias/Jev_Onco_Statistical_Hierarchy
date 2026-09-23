//! Molecular data contracts. Samples exist without patient or clinical context.
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub const SAMPLE_SCHEMA_VERSION: u32 = 1;
pub const EXPRESSION_PIPELINE: &str = "expression-import-v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExpressionUnit {
    Unknown,
    Counts,
    Tpm,
    Fpkm,
    Normalized,
    MicroarrayIntensity,
    Log2,
    ZScore,
}

impl ExpressionUnit {
    pub fn accepts(self, value: f64) -> bool {
        value.is_finite()
            && match self {
                Self::Counts => {
                    (0.0..=9_007_199_254_740_991.0).contains(&value) && value.fract() == 0.0
                }
                Self::Tpm | Self::Fpkm => value >= 0.0,
                _ => true,
            }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Transform {
    Identity,
    Log2OnePlus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum GeneNamespace {
    Auto,
    HgncId,
    Symbol,
    Ensembl,
    Entrez,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum MatrixLayout {
    Auto,
    Long,
    Wide,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Delimiter {
    Auto,
    Csv,
    Tsv,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Modality {
    Expression,
    Variants,
    Ihc,
    Histology,
    CopyNumber,
    Methylation,
    Fusions,
    StructuralVariants,
    Mirna,
    Proteomics,
    PathologyEmbedding,
    LegacyAnnotations,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Artifact {
    /// Relative to the immutable dataset bundle; never an arbitrary file locator.
    pub path: String,
    pub sha256: String,
    pub bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SourceAsset {
    pub original_name: String,
    pub artifact: Artifact,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct MappingAsset {
    pub release: String,
    pub artifact: Artifact,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ExpressionConfig {
    pub delimiter: Delimiter,
    pub layout: MatrixLayout,
    pub gene_column: String,
    pub value_column: String,
    pub sample_column: String,
    pub single_sample_id: Option<String>,
    pub gene_namespace: GeneNamespace,
    pub units: ExpressionUnit,
    pub transform: Transform,
    /// Human gene mapping is the only supported mapping authority in this version.
    pub organism: String,
    pub reference_genome: Option<String>,
    pub platform: Option<String>,
}

impl Default for ExpressionConfig {
    fn default() -> Self {
        Self {
            delimiter: Delimiter::Auto,
            layout: MatrixLayout::Auto,
            gene_column: "gene".into(),
            value_column: "expression".into(),
            sample_column: "sample_id".into(),
            single_sample_id: None,
            gene_namespace: GeneNamespace::Auto,
            units: ExpressionUnit::Unknown,
            transform: Transform::Identity,
            organism: "Homo sapiens".into(),
            reference_genome: None,
            platform: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Detection {
    pub delimiter: Delimiter,
    pub layout: MatrixLayout,
    pub modality: Modality,
    pub columns: Vec<String>,
    pub sample_columns: Vec<String>,
    pub notices: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct GeneIdentity {
    pub hgnc_id: String,
    pub symbol: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum MappingStatus {
    Exact,
    Alias,
    VersionStripped,
    Ambiguous,
    Unmapped,
    NotAttempted,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct GeneMapping {
    pub status: MappingStatus,
    pub gene: Option<GeneIdentity>,
    pub candidates: Vec<GeneIdentity>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ExpressionRecord {
    pub original_gene_id: String,
    pub original_value: String,
    /// CSV parser record ordinal including header; not physical line number.
    pub source_record: u64,
    /// One-based original column index.
    pub source_column: usize,
    pub raw_expression: Option<f64>,
    pub transformed_expression: Option<f64>,
    pub mapping: GeneMapping,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QcStatus {
    Pass,
    Warning,
    Blocked,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct QcIssue {
    pub code: String,
    pub source_record: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct QcReport {
    pub status: QcStatus,
    pub total_records: usize,
    pub measured_values: usize,
    pub missing_values: usize,
    pub invalid_values: usize,
    pub zero_values: usize,
    pub mapped_records: usize,
    pub ambiguous_records: usize,
    pub unmapped_records: usize,
    pub duplicate_gene_records: usize,
    /// Unambiguously mapped records / total records, including missing measurements.
    pub gene_id_mapping_rate: Option<f64>,
    pub minimum: Option<f64>,
    pub maximum: Option<f64>,
    pub issues: Vec<QcIssue>,
    /// This import milestone has no cancer reference or inference pipeline.
    pub reference_compatibility: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Assay {
    pub assay_id: String,
    pub modality: Modality,
    pub artifact: Artifact,
    pub qc: Option<QcReport>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Sample {
    pub sample_schema_version: u32,
    pub sample_id: String,
    pub data_class: crate::DataClass,
    pub patient_group_id: Option<String>,
    pub assays: Vec<Assay>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DictionaryField {
    pub name: String,
    pub meaning: String,
    pub original_column: Option<String>,
    pub units: Option<ExpressionUnit>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DatasetManifest {
    pub dataset_schema_version: u32,
    pub dataset_id: String,
    pub source_name: String,
    pub study: Option<String>,
    pub accession: Option<String>,
    pub citation: Option<String>,
    pub imported_at_unix_seconds: u64,
    pub pipeline_version: String,
    pub source: SourceAsset,
    pub gene_map: Option<MappingAsset>,
    pub expression_config: Option<ExpressionConfig>,
    pub detection: Option<Detection>,
    pub samples: Vec<Sample>,
    pub data_dictionary: Vec<DictionaryField>,
    pub notices: Vec<String>,
}

/// Source IDs can contain dots/spaces; they are never used as filesystem paths.
pub fn valid_label(value: &str) -> bool {
    !value.trim().is_empty() && value.len() <= 256 && !value.chars().any(char::is_control)
}

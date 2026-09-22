use crate::{MODEL, taxonomy};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Question {
    Choice {
        instructions: String,
        criteria: BTreeMap<String, String>,
    },
    Noul {
        instructions: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct JevRequest {
    pub model: String,
    pub state: serde_json::Value,
    pub questions: BTreeMap<String, Question>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Answer {
    Choice {
        choice: String,
        probabilities: BTreeMap<String, f64>,
        confidence: f64,
    },
    Noul {
        noul: f64,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Usage {
    pub input_tokens: u64,
    pub output_tokens: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct JevResponse {
    pub model: String,
    pub answers: BTreeMap<String, Answer>,
    pub usage: Usage,
}

#[derive(Debug, Clone, Copy, Serialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Source {
    Jev,
    Mock,
    Replay,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct RankedOrigin {
    pub origin: String,
    pub raw_probability: f64,
    pub calibrated_probability: Option<f64>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct ResultRecord {
    pub result_schema_version: u32,
    pub case_schema_version: u32,
    pub case_id: String,
    pub case_revision_sha256: String,
    pub fingerprint_version: &'static str,
    pub source: Source,
    pub status: &'static str,
    pub reasons: Vec<&'static str>,
    pub research_only: bool,
    pub probability_kind: &'static str,
    pub calibration_status: &'static str,
    pub model: String,
    pub prompt_version: &'static str,
    pub taxonomy_version: &'static str,
    pub policy_version: &'static str,
    pub request_sha256: String,
    pub provider_confidence: f64,
    pub evidence_sufficient: f64,
    pub conflicting_evidence: f64,
    pub rankings: Vec<RankedOrigin>,
    pub usage: Usage,
}

/// Deliberately uninformative simulation: never performs cancer classification.
pub fn mock_response() -> JevResponse {
    let options = taxonomy();
    let p = 1.0 / options.len() as f64;
    JevResponse {
        model: MODEL.into(),
        answers: BTreeMap::from([
            (
                "primary_site".into(),
                Answer::Choice {
                    choice: "insufficient_evidence".into(),
                    probabilities: options.keys().map(|k| (k.clone(), p)).collect(),
                    confidence: 0.0,
                },
            ),
            ("evidence_sufficient".into(), Answer::Noul { noul: 0.0 }),
            ("conflicting_evidence".into(), Answer::Noul { noul: 0.0 }),
        ]),
        usage: Usage {
            input_tokens: 0,
            output_tokens: 0,
        },
    }
}

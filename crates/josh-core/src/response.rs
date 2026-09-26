//! Bounded, value-free response diagnostics. Never retain provider text or credentials.
use crate::{Answer, JevRequest, JevResponse, Question};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResponseCode {
    BodyTooLarge,
    InvalidJson,
    JsonShape,
    ModelMismatch,
    QuestionMismatch,
    AnswerType,
    OptionMismatch,
    ProbabilityRange,
    ProbabilitySum,
    ConfidenceRange,
    ChoiceNotMaximum,
}
impl ResponseCode {
    pub fn message(self) -> &'static str {
        match self {
            Self::BodyTooLarge => "response exceeds the 64 KiB limit",
            Self::InvalidJson => "response is not complete valid JSON",
            Self::JsonShape => "response JSON has missing fields or incorrect field types",
            Self::ModelMismatch => "response model differs from the requested model",
            Self::QuestionMismatch => "response question IDs differ from the request",
            Self::AnswerType => "response answer type differs from the question type",
            Self::OptionMismatch => "response option IDs differ from the requested options",
            Self::ProbabilityRange => "response probability is non-finite or outside 0..1",
            Self::ProbabilitySum => "response probabilities do not sum to one within 0.000001",
            Self::ConfidenceRange => "response confidence is non-finite or outside 0..1",
            Self::ChoiceNotMaximum => {
                "response choice is missing or is not a highest-probability option"
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseDiagnostic {
    pub schema_version: u32,
    pub code: ResponseCode,
    pub request_sha256: Option<String>,
    pub response_sha256: Option<String>,
    pub response_bytes: Option<usize>,
    pub http_status: Option<u16>,
    /// Zero-based position in sorted request question IDs; no untrusted strings.
    pub question_index: Option<usize>,
    pub expected_count: Option<usize>,
    pub received_count: Option<usize>,
    pub probability_sum: Option<f64>,
    pub json_line: Option<usize>,
    pub json_column: Option<usize>,
    /// A fixed schema field name, never a provider-supplied value or map key.
    pub field: Option<String>,
}
impl ResponseDiagnostic {
    pub fn new(code: ResponseCode) -> Self {
        Self {
            schema_version: 1,
            code,
            request_sha256: None,
            response_sha256: None,
            response_bytes: None,
            http_status: None,
            question_index: None,
            expected_count: None,
            received_count: None,
            probability_sum: None,
            json_line: None,
            json_column: None,
            field: None,
        }
    }
    pub fn attach_body(mut self, request: &JevRequest, bytes: &[u8], status: u16) -> Self {
        self.request_sha256 = crate::molecular::hash(request).ok();
        self.response_sha256 = Some(crate::molecular::bytes_hash(bytes));
        self.response_bytes = Some(bytes.len());
        self.http_status = Some(status);
        self
    }
}
impl std::fmt::Display for ResponseDiagnostic {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Jev: {}", self.code.message())?;
        if let Some(field) = &self.field {
            write!(f, " (field: {field})")?;
        }
        if let Some(sum) = self.probability_sum {
            write!(f, " (received total {sum:.8})")?;
        }
        Ok(())
    }
}

pub fn validate(
    request: &JevRequest,
    response: &JevResponse,
) -> Result<(), Box<ResponseDiagnostic>> {
    if response.model != request.model {
        return Err(Box::new(ResponseDiagnostic::new(
            ResponseCode::ModelMismatch,
        )));
    }
    if response.answers.keys().ne(request.questions.keys()) {
        let mut d = ResponseDiagnostic::new(ResponseCode::QuestionMismatch);
        d.expected_count = Some(request.questions.len());
        d.received_count = Some(response.answers.len());
        return Err(Box::new(d));
    }
    let probability = |p: f64| p.is_finite() && (0.0..=1.0).contains(&p);
    for (index, (id, question)) in request.questions.iter().enumerate() {
        let mut d = ResponseDiagnostic::new(ResponseCode::AnswerType);
        d.question_index = Some(index);
        match (question, &response.answers[id]) {
            (
                Question::Choice { criteria, .. },
                Answer::Choice {
                    choice,
                    probabilities,
                    confidence,
                },
            ) => {
                d.expected_count = Some(criteria.len());
                d.received_count = Some(probabilities.len());
                let sum = probabilities.values().sum::<f64>();
                d.probability_sum = sum.is_finite().then_some(sum);
                d.code = if criteria.is_empty() || criteria.keys().ne(probabilities.keys()) {
                    ResponseCode::OptionMismatch
                } else if !probabilities.values().copied().all(probability) {
                    ResponseCode::ProbabilityRange
                } else if !probability(*confidence) {
                    ResponseCode::ConfidenceRange
                } else if (sum - 1.0).abs() > 1e-6 {
                    ResponseCode::ProbabilitySum
                } else if probabilities
                    .get(choice)
                    .is_none_or(|p| probabilities.values().any(|v| v > p))
                {
                    ResponseCode::ChoiceNotMaximum
                } else {
                    continue;
                };
            }
            (Question::Noul { .. }, Answer::Noul { noul }) => {
                if probability(*noul) {
                    continue;
                }
                d.code = ResponseCode::ProbabilityRange;
            }
            _ => (),
        }
        return Err(Box::new(d));
    }
    Ok(())
}

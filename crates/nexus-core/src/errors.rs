//! Stable public failures. Messages contain no submitted values or provider bodies.
use schemars::JsonSchema;
use serde::Serialize;

#[derive(Debug, Clone, Copy, Serialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ErrorCode {
    InvalidArguments,
    InvalidJson,
    InvalidCase,
    InputTooLarge,
    UnsupportedMediaType,
    NotFound,
    MethodNotAllowed,
    FileNotFound,
    OutputExists,
    IoError,
    MissingApiKey,
    DataPolicy,
    ProviderTransport,
    ProviderHttp,
    ProviderResponse,
    TerminalRequired,
    InternalError,
}

#[derive(Debug, Serialize, JsonSchema, thiserror::Error)]
#[error("{message}")]
pub struct ErrorDetail {
    pub code: ErrorCode,
    pub message: &'static str,
}

#[derive(Debug, Serialize, JsonSchema, thiserror::Error)]
#[error("{error}")]
pub struct ErrorEnvelope {
    pub error: ErrorDetail,
}

impl ErrorEnvelope {
    pub fn new(code: ErrorCode) -> Self {
        let message = match code {
            ErrorCode::InvalidArguments => "invalid arguments; run 'nexus --help' for usage",
            ErrorCode::InvalidJson => "input does not match the JSON schema",
            ErrorCode::InvalidCase => "case failed validation",
            ErrorCode::InputTooLarge => "input exceeds byte limit",
            ErrorCode::UnsupportedMediaType => "Content-Type must be application/json",
            ErrorCode::NotFound => "route not found",
            ErrorCode::MethodNotAllowed => "method not allowed",
            ErrorCode::FileNotFound => "input file not found",
            ErrorCode::OutputExists => "output file already exists; choose a new path",
            ErrorCode::IoError => "file, terminal or socket operation failed",
            ErrorCode::MissingApiKey => {
                "a nonempty TYPESAFE_API_KEY is required; use 'nexus demo' offline"
            }
            ErrorCode::DataPolicy => "live classification accepts declared synthetic cases only",
            ErrorCode::ProviderTransport => "Jev transport failed or timed out",
            ErrorCode::ProviderHttp => "Jev returned an unsuccessful HTTP status",
            ErrorCode::ProviderResponse => "Jev response failed contract validation",
            ErrorCode::TerminalRequired => {
                "TUI requires an interactive terminal; use 'nexus demo --format text' in scripts"
            }
            ErrorCode::InternalError => "internal operation failed",
        };
        Self {
            error: ErrorDetail { code, message },
        }
    }

    pub fn invalid_case(error: crate::ValidationError) -> Self {
        Self {
            error: ErrorDetail {
                code: ErrorCode::InvalidCase,
                message: error.0,
            },
        }
    }
}

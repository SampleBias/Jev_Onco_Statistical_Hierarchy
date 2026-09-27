use crate::workflows::AppError;
use josh_core::errors::{ErrorCode, ErrorEnvelope};

/// Response diagnostics contain only fixed messages and bounded numeric facts.
pub fn user_message(error: &AppError) -> String {
    if let Some(josh_explain::Error::ProviderResponse(d)) =
        error.downcast_ref::<josh_explain::Error>()
    {
        return d.to_string();
    }
    if let Some(josh_jev::Error::Response(d)) = error.downcast_ref::<josh_jev::Error>() {
        return d.to_string();
    }
    envelope(error).error.message.into()
}

pub fn envelope(error: &AppError) -> ErrorEnvelope {
    if let Some(e) = error.downcast_ref::<josh_explain::Error>() {
        return match e {
            josh_explain::Error::Invalid(e) => {
                ErrorEnvelope::invalid_case(josh_core::ValidationError(e.0))
            }
            josh_explain::Error::Budget | josh_explain::Error::Timeout => {
                ErrorEnvelope::new(ErrorCode::ExplanationBudget)
            }
            josh_explain::Error::Cancelled => ErrorEnvelope::new(ErrorCode::ExplanationCancelled),
            josh_explain::Error::Provider => ErrorEnvelope::new(ErrorCode::ProviderTransport),
            josh_explain::Error::ProviderHttp(_) => ErrorEnvelope::new(ErrorCode::ProviderHttp),
            josh_explain::Error::ProviderResponse(_) => {
                ErrorEnvelope::new(ErrorCode::ProviderResponse)
            }
            josh_explain::Error::Checkpoint => ErrorEnvelope::new(ErrorCode::ExplanationCheckpoint),
            josh_explain::Error::Uncertain => ErrorEnvelope::new(ErrorCode::ExplanationUncertain),
        };
    }
    if let Some(e) = error.downcast_ref::<josh_ingest::molecular::Error>() {
        return match e {
            josh_ingest::molecular::Error::Validation(e) => {
                ErrorEnvelope::invalid_case(josh_core::ValidationError(e.0))
            }
            josh_ingest::molecular::Error::Limit => ErrorEnvelope::new(ErrorCode::ImportLimit),
            josh_ingest::molecular::Error::Format => ErrorEnvelope::new(ErrorCode::MolecularFormat),
            josh_ingest::molecular::Error::Io => ErrorEnvelope::new(ErrorCode::IoError),
        };
    }
    if let Some(e) = error.downcast_ref::<ErrorEnvelope>() {
        return ErrorEnvelope {
            error: josh_core::errors::ErrorDetail {
                code: e.error.code,
                message: e.error.message,
            },
        };
    }
    if let Some(e) = error.downcast_ref::<josh_core::ValidationError>() {
        return ErrorEnvelope::invalid_case(josh_core::ValidationError(e.0));
    }
    if let Some(e) = error.downcast_ref::<josh_features::reference::ReferenceError>() {
        return ErrorEnvelope {
            error: josh_core::errors::ErrorDetail {
                code: ErrorCode::ReferenceConfiguration,
                message: e.0,
            },
        };
    }
    let code = if let Some(e) = error.downcast_ref::<std::io::Error>() {
        match e.kind() {
            std::io::ErrorKind::NotFound => ErrorCode::FileNotFound,
            std::io::ErrorKind::AlreadyExists => ErrorCode::OutputExists,
            _ => ErrorCode::IoError,
        }
    } else if let Some(e) = error.downcast_ref::<josh_jev::Error>() {
        match e {
            josh_jev::Error::Validation(_) => ErrorCode::InvalidCase,
            josh_jev::Error::DataPolicy => ErrorCode::DataPolicy,
            josh_jev::Error::MissingKey => ErrorCode::MissingApiKey,
            josh_jev::Error::Client | josh_jev::Error::Transport => ErrorCode::ProviderTransport,
            josh_jev::Error::Http { .. } => ErrorCode::ProviderHttp,
            josh_jev::Error::Response(_) => ErrorCode::ProviderResponse,
        }
    } else if let Some(e) = error.downcast_ref::<josh_ingest::dataset::DatasetError>() {
        use josh_ingest::dataset::DatasetError as D;
        match e {
            D::Io(e) => match e.kind() {
                std::io::ErrorKind::AlreadyExists => ErrorCode::OutputExists,
                std::io::ErrorKind::NotFound => ErrorCode::FileNotFound,
                _ => ErrorCode::IoError,
            },
            D::Limit => ErrorCode::ImportLimit,
            D::Detection | D::Columns | D::Format => ErrorCode::DatasetFormat,
            D::SampleId | D::Configuration => ErrorCode::DatasetConfiguration,
            D::Empty => ErrorCode::EmptyImport,
            D::GeneMap => ErrorCode::GeneMapping,
            D::InvalidBundle => ErrorCode::InvalidBundle,
            D::Serialization => ErrorCode::InternalError,
        }
    } else if let Some(e) = error.downcast_ref::<josh_ingest::Error>() {
        match e {
            josh_ingest::Error::Io(e) => match e.kind() {
                std::io::ErrorKind::AlreadyExists => ErrorCode::OutputExists,
                std::io::ErrorKind::NotFound => ErrorCode::FileNotFound,
                _ => ErrorCode::IoError,
            },
            josh_ingest::Error::Options => ErrorCode::InvalidArguments,
            josh_ingest::Error::SourceTooLarge | josh_ingest::Error::TooManyRecords => {
                ErrorCode::ImportLimit
            }
            josh_ingest::Error::Columns => ErrorCode::InvalidImportColumns,
            josh_ingest::Error::EmptySource => ErrorCode::EmptyImport,
            josh_ingest::Error::InvalidBundle => ErrorCode::InvalidBundle,
        }
    } else {
        ErrorCode::InternalError
    };
    ErrorEnvelope::new(code)
}

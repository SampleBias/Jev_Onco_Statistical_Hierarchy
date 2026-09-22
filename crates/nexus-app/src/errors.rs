use crate::workflows::AppError;
use nexus_core::errors::{ErrorCode, ErrorEnvelope};

pub fn envelope(error: &AppError) -> ErrorEnvelope {
    if let Some(e) = error.downcast_ref::<ErrorEnvelope>() {
        return ErrorEnvelope {
            error: nexus_core::errors::ErrorDetail {
                code: e.error.code,
                message: e.error.message,
            },
        };
    }
    if let Some(e) = error.downcast_ref::<nexus_core::ValidationError>() {
        return ErrorEnvelope::invalid_case(nexus_core::ValidationError(e.0));
    }
    let code = if let Some(e) = error.downcast_ref::<std::io::Error>() {
        match e.kind() {
            std::io::ErrorKind::NotFound => ErrorCode::FileNotFound,
            std::io::ErrorKind::AlreadyExists => ErrorCode::OutputExists,
            _ => ErrorCode::IoError,
        }
    } else if let Some(e) = error.downcast_ref::<nexus_jev::Error>() {
        match e {
            nexus_jev::Error::Validation(_) => ErrorCode::InvalidCase,
            nexus_jev::Error::DataPolicy => ErrorCode::DataPolicy,
            nexus_jev::Error::MissingKey => ErrorCode::MissingApiKey,
            nexus_jev::Error::Client | nexus_jev::Error::Transport => ErrorCode::ProviderTransport,
            nexus_jev::Error::Http(_) => ErrorCode::ProviderHttp,
            nexus_jev::Error::Response => ErrorCode::ProviderResponse,
        }
    } else if let Some(e) = error.downcast_ref::<nexus_ingest::Error>() {
        match e {
            nexus_ingest::Error::Io(e) => match e.kind() {
                std::io::ErrorKind::AlreadyExists => ErrorCode::OutputExists,
                std::io::ErrorKind::NotFound => ErrorCode::FileNotFound,
                _ => ErrorCode::IoError,
            },
            nexus_ingest::Error::Options => ErrorCode::InvalidArguments,
            nexus_ingest::Error::SourceTooLarge | nexus_ingest::Error::TooManyRecords => {
                ErrorCode::ImportLimit
            }
            nexus_ingest::Error::Columns => ErrorCode::InvalidImportColumns,
            nexus_ingest::Error::EmptySource => ErrorCode::EmptyImport,
            nexus_ingest::Error::InvalidBundle => ErrorCode::InvalidBundle,
        }
    } else {
        ErrorCode::InternalError
    };
    ErrorEnvelope::new(code)
}

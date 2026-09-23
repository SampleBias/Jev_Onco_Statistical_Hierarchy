use crate::workflows::AppError;
use josh_core::errors::{ErrorCode, ErrorEnvelope};

pub fn envelope(error: &AppError) -> ErrorEnvelope {
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
            josh_jev::Error::Http(_) => ErrorCode::ProviderHttp,
            josh_jev::Error::Response => ErrorCode::ProviderResponse,
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

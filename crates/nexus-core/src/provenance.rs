//! Versioned fingerprints of validated values; hashes do not authenticate a provider.
use crate::{Case, JevRequest, ValidationError};
use serde::Serialize;
use sha2::{Digest, Sha256};

pub const FINGERPRINT_VERSION: &str = "typed-json-sha256-v1";

fn digest(value: &impl Serialize) -> Result<String, ValidationError> {
    let bytes = serde_json::to_vec(value)
        .map_err(|_| ValidationError("cannot serialize fingerprint input"))?;
    Ok(format!("{:x}", Sha256::digest(bytes)))
}

/// Includes local identity and data class; whitespace/key order in source JSON is ignored.
pub fn case_revision(case: &Case) -> Result<String, ValidationError> {
    case.validate()?;
    digest(case)
}

/// Exact compact serialization used by reqwest's JSON body, without headers or credentials.
pub fn request_sha256(request: &JevRequest) -> Result<String, ValidationError> {
    digest(request)
}

//! Shared case, provider, policy and provenance contracts. No IO or credentials.
pub mod clinical;
mod domain;
pub mod errors;
pub mod guidance;
mod policy;
mod prompt;
pub mod provenance;
mod provider;
pub mod sample;
pub mod schema;
mod taxonomy;

pub use domain::*;
pub use policy::interpret;
pub use prompt::{prepare, prompt_version};
pub use provider::*;
pub use taxonomy::taxonomy;

pub const MODEL: &str = "jev-1.13.0";
pub const PROMPT_VERSION: &str = "cup-research-v0.1";
pub const EVIDENCE_PROMPT_VERSION: &str = "cup-research-v0.2";
pub const TAXONOMY_VERSION: &str = "demo-primary-sites-v0.1";
pub const POLICY_VERSION: &str = "unvalidated-research-gates-v0.2";
pub const MAX_CASE_BYTES: usize = 16_384;
pub const RESULT_SCHEMA_VERSION: u32 = 2;

//! Shared case, provider, policy and provenance contracts. No IO or credentials.
mod domain;
pub mod errors;
mod policy;
mod prompt;
pub mod provenance;
mod provider;
pub mod schema;
mod taxonomy;

pub use domain::*;
pub use policy::interpret;
pub use prompt::prepare;
pub use provider::*;
pub use taxonomy::taxonomy;

pub const MODEL: &str = "jev-1.13.0";
pub const PROMPT_VERSION: &str = "cup-research-v0.1";
pub const TAXONOMY_VERSION: &str = "demo-primary-sites-v0.1";
pub const POLICY_VERSION: &str = "unvalidated-research-gates-v0.1";
pub const MAX_CASE_BYTES: usize = 16_384;
pub const RESULT_SCHEMA_VERSION: u32 = 1;

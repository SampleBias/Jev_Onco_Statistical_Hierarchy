use nexus_core::{Case, DataClass, JevResponse, ResultRecord, Source, interpret, prepare};
use std::time::Duration;

const ENDPOINT: &str = "https://api.typesafe.ai/v1/systemone";
const MAX_RESPONSE_BYTES: usize = 65_536;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("{0}")]
    Validation(#[from] nexus_core::ValidationError),
    #[error("this initial live adapter accepts synthetic cases only")]
    DataPolicy,
    #[error("a nonempty TYPESAFE_API_KEY is required")]
    MissingKey,
    #[error("unable to configure Jev HTTP client")]
    Client,
    #[error("Jev transport failed or timed out")]
    Transport,
    #[error("Jev returned HTTP {0}")]
    Http(u16),
    #[error("invalid or oversized Jev response")]
    Response,
}

pub async fn classify(case: &Case, key: &str) -> Result<ResultRecord, Error> {
    let request = prepare(case)?;
    if case.data_class != DataClass::Synthetic {
        return Err(Error::DataPolicy);
    }
    if key.trim().is_empty() {
        return Err(Error::MissingKey);
    }
    if !case.has_observed_evidence() {
        return Err(nexus_core::ValidationError(
            "at least one observed finding is required for a live request",
        )
        .into());
    }
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(20))
        .connect_timeout(Duration::from_secs(5))
        .redirect(reqwest::redirect::Policy::none())
        .https_only(true)
        .build()
        .map_err(|_| Error::Client)?;
    // No automatic retries in v0.1: a timeout may already have incurred a charge.
    let mut response = client
        .post(ENDPOINT)
        .bearer_auth(key)
        .json(&request)
        .send()
        .await
        .map_err(|_| Error::Transport)?;
    if !response.status().is_success() {
        return Err(Error::Http(response.status().as_u16()));
    }
    if response
        .content_length()
        .is_some_and(|n| n > MAX_RESPONSE_BYTES as u64)
    {
        return Err(Error::Response);
    }
    let mut bytes = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(|_| Error::Transport)? {
        if bytes.len() + chunk.len() > MAX_RESPONSE_BYTES {
            return Err(Error::Response);
        }
        bytes.extend_from_slice(&chunk);
    }
    let response: JevResponse = serde_json::from_slice(&bytes).map_err(|_| Error::Response)?;
    interpret(case, response, Source::Jev).map_err(|_| Error::Response)
}

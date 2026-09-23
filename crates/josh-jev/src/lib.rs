use josh_core::{Case, DataClass, JevResponse, ResultRecord, Source, interpret, prepare};
use std::time::Duration;

const ENDPOINT: &str = "https://api.typesafe.ai/v1/systemone";
const MAX_RESPONSE_BYTES: usize = 65_536;

/// Reused transport. Credentials are deliberately not Debug/Serialize.
pub struct Client {
    http: reqwest::Client,
    key: String,
    endpoint: String,
}
impl Client {
    pub fn new(key: &str) -> Result<Self, Error> {
        if key.trim().is_empty() {
            return Err(Error::MissingKey);
        }
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(20))
            .connect_timeout(Duration::from_secs(5))
            .redirect(reqwest::redirect::Policy::none())
            .https_only(true)
            .build()
            .map_err(|_| Error::Client)?;
        Ok(Self {
            http,
            key: key.into(),
            endpoint: ENDPOINT.into(),
        })
    }
    /// One network attempt. Callers own budgets and resumption. Ambiguous timeouts
    /// are never automatically resent; rate-limit responses remain explicit errors.
    pub async fn evaluate(
        &self,
        request: &josh_core::JevRequest,
        data_class: &DataClass,
    ) -> Result<JevResponse, Error> {
        if *data_class != DataClass::Synthetic {
            return Err(Error::DataPolicy);
        }
        if request.model != josh_core::MODEL
            || serde_json::to_vec(request)
                .map_err(|_| Error::Response)?
                .len()
                > josh_core::molecular::MAX_REQUEST_BYTES
        {
            return Err(josh_core::ValidationError("invalid model or request byte limit").into());
        }
        let mut response = self
            .http
            .post(&self.endpoint)
            .bearer_auth(&self.key)
            .json(request)
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
        josh_core::molecular::validate_response(request, &response).map_err(|_| Error::Response)?;
        Ok(response)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("{0}")]
    Validation(#[from] josh_core::ValidationError),
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
        return Err(josh_core::ValidationError(
            "at least one observed finding is required for a live request",
        )
        .into());
    }
    let response = Client::new(key)?
        .evaluate(&request, &case.data_class)
        .await?;
    interpret(case, response, Source::Jev).map_err(|_| Error::Response)
}

#[cfg(test)]
mod transport_tests {
    use super::*;
    use std::io::{Read, Write};
    fn server(status: u16, body: String) -> (Client, std::thread::JoinHandle<()>) {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let task = std::thread::spawn(move || {
            let (mut socket, _) = listener.accept().unwrap();
            socket
                .set_read_timeout(Some(Duration::from_secs(3)))
                .unwrap();
            let mut request = Vec::new();
            let mut buf = [0; 8192];
            loop {
                let n = socket.read(&mut buf).unwrap();
                if n == 0 {
                    break;
                }
                request.extend_from_slice(&buf[..n]);
                if let Some(end) = request.windows(4).position(|w| w == b"\r\n\r\n") {
                    let headers = String::from_utf8_lossy(&request[..end]).to_ascii_lowercase();
                    let len = headers
                        .lines()
                        .find_map(|line| {
                            line.strip_prefix("content-length:")
                                .and_then(|v| v.trim().parse::<usize>().ok())
                        })
                        .unwrap_or(0);
                    if request.len() >= end + 4 + len {
                        break;
                    }
                }
            }
            let output = format!(
                "HTTP/1.1 {status} Test\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            );
            let _ = socket.write_all(output.as_bytes());
        });
        let client = Client {
            http: reqwest::Client::builder()
                .timeout(Duration::from_secs(3))
                .redirect(reqwest::redirect::Policy::none())
                .build()
                .unwrap(),
            key: "mock-credential".into(),
            endpoint: format!("http://{address}"),
        };
        (client, task)
    }
    fn request() -> josh_core::JevRequest {
        let case: Case =
            serde_json::from_str(include_str!("../../../fixtures/synthetic-case.json")).unwrap();
        prepare(&case).unwrap()
    }
    #[tokio::test]
    async fn validates_mock_http_response_and_does_not_retry_http_errors() {
        let (client, task) = server(
            200,
            serde_json::to_string(&josh_core::mock_response()).unwrap(),
        );
        client
            .evaluate(&request(), &DataClass::Synthetic)
            .await
            .unwrap();
        task.join().unwrap();
        for status in [401, 422, 429, 529, 302] {
            let (client, task) = server(status, "sensitive body never shown".into());
            let error = client
                .evaluate(&request(), &DataClass::Synthetic)
                .await
                .unwrap_err();
            assert!(matches!(error,Error::Http(s) if s==status));
            assert!(!error.to_string().contains("sensitive"));
            task.join().unwrap();
        }
    }
    #[tokio::test]
    async fn rejects_oversized_malformed_and_changed_model_responses() {
        let mut drift = josh_core::mock_response();
        drift.model = "different-model".into();
        for body in [
            "{".into(),
            "x".repeat(MAX_RESPONSE_BYTES + 1),
            serde_json::to_string(&drift).unwrap(),
        ] {
            let (client, task) = server(200, body);
            assert!(matches!(
                client.evaluate(&request(), &DataClass::Synthetic).await,
                Err(Error::Response)
            ));
            task.join().unwrap();
        }
    }
}

use josh_core::response::{ResponseCode, ResponseDiagnostic};
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
    /// One accepted call. Timeouts and other errors are not resent. HTTP 429 and 529
    /// retry at most twice, because those statuses mean the call was not accepted.
    pub async fn evaluate(
        &self,
        request: &josh_core::JevRequest,
        data_class: &DataClass,
    ) -> Result<JevResponse, Error> {
        if *data_class != DataClass::Synthetic {
            return Err(Error::DataPolicy);
        }
        if request.model != josh_core::MODEL {
            return Err(josh_core::ValidationError("invalid model").into());
        }
        josh_core::molecular::within_model_context(request)?;
        let mut attempts = 1u32;
        let mut response = loop {
            let response = self
                .http
                .post(&self.endpoint)
                .bearer_auth(&self.key)
                .json(request)
                .send()
                .await
                .map_err(|_| Error::Transport)?;
            let status = response.status().as_u16();
            if matches!(status, 429 | 529) && attempts <= 2 {
                let wait = retry_after(&response).min(Duration::from_secs(2));
                attempts += 1;
                tokio::time::sleep(wait).await;
                continue;
            }
            if !response.status().is_success() {
                return Err(Error::Http { status, attempts });
            }
            break response;
        };
        let status = response.status().as_u16();
        if response
            .content_length()
            .is_some_and(|n| n > MAX_RESPONSE_BYTES as u64)
        {
            let mut d = ResponseDiagnostic::new(ResponseCode::BodyTooLarge);
            d.request_sha256 = josh_core::molecular::hash(request).ok();
            d.http_status = Some(status);
            return Err(Error::Response(Box::new(d)));
        }
        let mut bytes = Vec::new();
        while let Some(chunk) = response.chunk().await.map_err(|_| Error::Transport)? {
            if bytes.len() + chunk.len() > MAX_RESPONSE_BYTES {
                let mut d = ResponseDiagnostic::new(ResponseCode::BodyTooLarge);
                d.request_sha256 = josh_core::molecular::hash(request).ok();
                d.http_status = Some(status);
                return Err(Error::Response(Box::new(d)));
            }
            bytes.extend_from_slice(&chunk);
        }
        decode_response(request, &bytes, status)
    }
}

/// Can also inspect a locally captured response without making a network request.
pub fn decode_response(
    request: &josh_core::JevRequest,
    bytes: &[u8],
    status: u16,
) -> Result<JevResponse, Error> {
    if bytes.len() > MAX_RESPONSE_BYTES {
        let mut d = ResponseDiagnostic::new(ResponseCode::BodyTooLarge);
        d.request_sha256 = josh_core::molecular::hash(request).ok();
        d.response_bytes = Some(bytes.len());
        d.http_status = Some(status);
        return Err(Error::Response(Box::new(d)));
    }
    let response: JevResponse = serde_json::from_slice(bytes).map_err(|e| {
        let code = if e.is_data() {
            ResponseCode::JsonShape
        } else {
            ResponseCode::InvalidJson
        };
        let mut d = ResponseDiagnostic::new(code).attach_body(request, bytes, status);
        if e.is_data()
            && let Ok(value) = serde_json::from_slice::<serde_json::Value>(bytes)
        {
            let (field, index) = invalid_field(request, &value);
            d.field = field.map(str::to_owned);
            d.question_index = index;
        }
        // serde error messages may contain values from the provider; retain only coordinates.
        d.json_line = Some(e.line());
        d.json_column = Some(e.column());
        Error::Response(Box::new(d))
    })?;
    josh_core::response::validate(request, &response)
        .map_err(|d| Error::Response(Box::new((*d).attach_body(request, bytes, status))))?;
    Ok(response)
}

fn invalid_field(
    request: &josh_core::JevRequest,
    value: &serde_json::Value,
) -> (Option<&'static str>, Option<usize>) {
    if !value["model"].is_string() {
        return (Some("model"), None);
    }
    if !value["answers"].is_object() {
        return (Some("answers"), None);
    }
    if !value["usage"].is_object() {
        return (Some("usage"), None);
    }
    for field in ["input_tokens", "output_tokens"] {
        if value["usage"][field].as_u64().is_none() {
            return (Some(field), None);
        }
    }
    for (index, id) in request.questions.keys().enumerate() {
        let Some(answer) = value["answers"].get(id) else {
            continue;
        };
        match answer["type"].as_str() {
            Some("choice") => {
                if !answer["choice"].is_string() {
                    return (Some("choice"), Some(index));
                }
                if answer["confidence"].as_f64().is_none() {
                    return (Some("confidence"), Some(index));
                }
                if answer["probabilities"]
                    .as_object()
                    .is_none_or(|p| p.values().any(|v| v.as_f64().is_none()))
                {
                    return (Some("probabilities"), Some(index));
                }
            }
            Some("noul") => {
                if answer["noul"].as_f64().is_none() {
                    return (Some("noul"), Some(index));
                }
            }
            _ => return (Some("type"), Some(index)),
        }
    }
    (None, None)
}

fn retry_after(response: &reqwest::Response) -> Duration {
    response
        .headers()
        .get(reqwest::header::RETRY_AFTER)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.trim().parse::<u64>().ok())
        .map(Duration::from_secs)
        .unwrap_or(Duration::from_millis(200))
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
    #[error("Jev returned HTTP {status} after {attempts} attempt(s)")]
    Http { status: u16, attempts: u32 },
    #[error("{0}")]
    Response(Box<ResponseDiagnostic>),
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
    interpret(case, response, Source::Jev).map_err(Error::Validation)
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
    #[test]
    fn diagnostics_identify_failures_without_echoing_provider_content() {
        use serde_json::json;
        let request = request();
        let valid = serde_json::to_value(josh_core::mock_response()).unwrap();
        let changes = [
            (
                "/model",
                json!("SECRET-PROVIDER-VALUE"),
                ResponseCode::ModelMismatch,
            ),
            (
                "/answers/primary_site/type",
                json!("SECRET-PROVIDER-VALUE"),
                ResponseCode::JsonShape,
            ),
            (
                "/usage/input_tokens",
                json!("SECRET-PROVIDER-VALUE"),
                ResponseCode::JsonShape,
            ),
            (
                "/answers/primary_site/confidence",
                json!(1.5),
                ResponseCode::ConfidenceRange,
            ),
            (
                "/answers/primary_site/probabilities",
                json!({"SECRET-PROVIDER-VALUE":1.0}),
                ResponseCode::OptionMismatch,
            ),
            (
                "/answers/primary_site/choice",
                json!("SECRET-PROVIDER-VALUE"),
                ResponseCode::ChoiceNotMaximum,
            ),
            (
                "/answers/evidence_sufficient/noul",
                json!(-0.1),
                ResponseCode::ProbabilityRange,
            ),
            (
                "/answers/evidence_sufficient",
                json!({"type":"choice","choice":"a","probabilities":{"a":1.0},"confidence":1.0}),
                ResponseCode::AnswerType,
            ),
        ];
        for (pointer, change, expected) in changes {
            let mut body = valid.clone();
            *body.pointer_mut(pointer).unwrap() = change;
            let bytes = serde_json::to_vec(&body).unwrap();
            let Error::Response(d) = decode_response(&request, &bytes, 200).unwrap_err() else {
                panic!("expected diagnostic");
            };
            assert_eq!(d.code, expected);
            assert_eq!(d.http_status, Some(200));
            assert_eq!(d.response_bytes, Some(bytes.len()));
            assert_eq!(
                d.request_sha256.as_deref(),
                Some(josh_core::molecular::hash(&request).unwrap().as_str())
            );
            assert_eq!(
                d.response_sha256.as_deref(),
                Some(josh_core::molecular::bytes_hash(&bytes).as_str())
            );
            assert!(
                !serde_json::to_string(&d)
                    .unwrap()
                    .contains("SECRET-PROVIDER-VALUE")
            );
            assert!(!d.to_string().contains("SECRET-PROVIDER-VALUE"));
        }
        for (bytes, expected) in [
            (
                b"{\"SECRET-PROVIDER-VALUE\":".to_vec(),
                ResponseCode::InvalidJson,
            ),
            (
                vec![b'x'; MAX_RESPONSE_BYTES + 1],
                ResponseCode::BodyTooLarge,
            ),
        ] {
            let Error::Response(d) = decode_response(&request, &bytes, 200).unwrap_err() else {
                panic!();
            };
            assert_eq!(d.code, expected);
        }
        let mut missing = valid.clone();
        missing["answers"]["primary_site"]
            .as_object_mut()
            .unwrap()
            .remove("confidence");
        let Error::Response(d) =
            decode_response(&request, &serde_json::to_vec(&missing).unwrap(), 200).unwrap_err()
        else {
            panic!();
        };
        assert_eq!(d.field.as_deref(), Some("confidence"));
        missing = valid.clone();
        missing["answers"]
            .as_object_mut()
            .unwrap()
            .remove("evidence_sufficient");
        let Error::Response(d) =
            decode_response(&request, &serde_json::to_vec(&missing).unwrap(), 200).unwrap_err()
        else {
            panic!();
        };
        assert_eq!(d.code, ResponseCode::QuestionMismatch);
        assert_eq!(d.received_count, Some(2));
    }

    #[test]
    fn probability_sum_failure_retains_total_and_never_normalizes() {
        let request = request();
        let mut response = josh_core::mock_response();
        if let josh_core::Answer::Choice { probabilities, .. } =
            response.answers.get_mut("primary_site").unwrap()
        {
            for p in probabilities.values_mut() {
                *p *= 0.99;
            }
        }
        let bytes = serde_json::to_vec(&response).unwrap();
        let Error::Response(d) = decode_response(&request, &bytes, 200).unwrap_err() else {
            panic!();
        };
        assert_eq!(d.code, ResponseCode::ProbabilitySum);
        assert!((d.probability_sum.unwrap() - 0.99).abs() < 1e-12);
        assert!(d.to_string().contains("0.99000000"));
        let response = josh_core::mock_response();
        let bytes = serde_json::to_vec(&response).unwrap();
        assert_eq!(
            serde_json::to_vec(&decode_response(&request, &bytes, 200).unwrap()).unwrap(),
            bytes
        );
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
        for status in [401, 422, 500, 302] {
            let (client, task) = server(status, "sensitive body never shown".into());
            let error = client
                .evaluate(&request(), &DataClass::Synthetic)
                .await
                .unwrap_err();
            assert!(matches!(error, Error::Http { status: s, attempts: 1 } if s == status));
            assert!(!error.to_string().contains("sensitive"));
            task.join().unwrap();
        }
    }
    fn scripted(steps: Vec<(u16, String, String)>) -> (Client, std::thread::JoinHandle<()>) {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let task = std::thread::spawn(move || {
            for (status, headers, body) in steps {
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
                        let head = String::from_utf8_lossy(&request[..end]).to_ascii_lowercase();
                        let len = head
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
                    "HTTP/1.1 {status} Test\r\nContent-Type: application/json\r\n{headers}Content-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                );
                let _ = socket.write_all(output.as_bytes());
            }
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
    #[tokio::test]
    async fn retries_rate_limit_and_overload_then_stops() {
        let body = serde_json::to_string(&josh_core::mock_response()).unwrap();
        let (client, task) = scripted(vec![
            (429, "Retry-After: 0\r\n".into(), "no".into()),
            (200, String::new(), body),
        ]);
        client
            .evaluate(&request(), &DataClass::Synthetic)
            .await
            .unwrap();
        task.join().unwrap();
        let (client, task) = scripted(vec![
            (529, String::new(), "no".into()),
            (529, String::new(), "no".into()),
            (529, String::new(), "no".into()),
        ]);
        let error = client
            .evaluate(&request(), &DataClass::Synthetic)
            .await
            .unwrap_err();
        assert!(matches!(
            error,
            Error::Http {
                status: 529,
                attempts: 3
            }
        ));
        assert!(error.to_string().contains("3 attempt"));
        task.join().unwrap();
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
                Err(Error::Response(_))
            ));
            task.join().unwrap();
        }
    }
}

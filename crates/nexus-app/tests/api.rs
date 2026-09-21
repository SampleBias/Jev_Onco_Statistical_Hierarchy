use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use http_body_util::BodyExt;
use serde_json::{Value, json};
use tower::ServiceExt;

async fn post(path: &str, body: Value) -> (StatusCode, Vec<u8>) {
    let response = nexus_app::router()
        .oneshot(
            Request::post(path)
                .header("content-type", "application/json")
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    let status = response.status();
    (
        status,
        response
            .into_body()
            .collect()
            .await
            .unwrap()
            .to_bytes()
            .to_vec(),
    )
}

fn case() -> Value {
    serde_json::from_str(include_str!("../../../fixtures/synthetic-case.json")).unwrap()
}

#[tokio::test]
async fn offline_demo_returns_explicit_abstention() {
    let (status, bytes) = post("/v1/demo", case()).await;
    assert_eq!(status, StatusCode::OK);
    let result: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(result["source"], "mock");
    assert_eq!(result["status"], "abstained");
    assert_eq!(result["calibration_status"], "not_validated_for_cup");
}

#[tokio::test]
async fn prepared_request_excludes_case_identifier() {
    let (status, bytes) = post("/v1/prepare", case()).await;
    assert_eq!(status, StatusCode::OK);
    let result: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(result["sends_to_provider"], false);
    assert!(result["request"]["state"].get("case_id").is_none());
}

#[tokio::test]
async fn invalid_and_oversize_inputs_are_rejected() {
    let mut invalid = case();
    invalid["schema_version"] = json!(42);
    assert_eq!(
        post("/v1/demo", invalid).await.0,
        StatusCode::UNPROCESSABLE_ENTITY
    );
    let mut oversized = case();
    oversized["findings"][0]["value"] = json!("x".repeat(20_000));
    assert_eq!(
        post("/v1/demo", oversized).await.0,
        StatusCode::PAYLOAD_TOO_LARGE
    );
}

#[tokio::test]
async fn http_has_no_live_classification_route() {
    assert_eq!(post("/v1/classify", case()).await.0, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn all_http_failure_paths_return_sanitized_json_envelopes() {
    let original = case().to_string();
    for (path, method, content_type, body, status, code) in [
        (
            "/v1/demo",
            "POST",
            "application/json",
            "{broken-private-value".into(),
            400,
            "invalid_json",
        ),
        (
            "/v1/demo",
            "POST",
            "application/json",
            format!("{original} {{}}"),
            400,
            "invalid_json",
        ),
        (
            "/v1/demo",
            "POST",
            "application/json",
            "x".repeat(nexus_core::MAX_CASE_BYTES + 1),
            413,
            "input_too_large",
        ),
        (
            "/v1/demo",
            "POST",
            "text/plain",
            original.clone(),
            415,
            "unsupported_media_type",
        ),
        (
            "/v1/demo",
            "POST",
            "application/json",
            r#"{"private":"do-not-echo"}"#.into(),
            422,
            "invalid_json",
        ),
        (
            "/unknown",
            "POST",
            "application/json",
            original.clone(),
            404,
            "not_found",
        ),
        (
            "/v1/demo",
            "PUT",
            "application/json",
            original,
            405,
            "method_not_allowed",
        ),
    ] {
        let response = nexus_app::router()
            .oneshot(
                Request::builder()
                    .method(method)
                    .uri(path)
                    .header("content-type", content_type)
                    .body(Body::from(body))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status().as_u16(), status, "{code}");
        assert_eq!(response.headers()["content-type"], "application/json");
        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        let value: Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(value["error"]["code"], code);
        assert!(!String::from_utf8_lossy(&bytes).contains("private"));
        assert!(!String::from_utf8_lossy(&bytes).contains("do-not-echo"));
    }
}

#[tokio::test]
async fn http_accepts_exact_body_budget_and_uses_shared_domain_errors() {
    let original = case().to_string();
    let padded = format!(
        "{original}{}",
        " ".repeat(nexus_core::MAX_CASE_BYTES - original.len())
    );
    let response = nexus_app::router()
        .oneshot(
            Request::post("/v1/prepare")
                .header("content-type", "application/json")
                .body(Body::from(padded))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let mut invalid = case();
    invalid["age_years"] = json!(121);
    let (status, body) = post("/v1/prepare", invalid).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    let error: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(error["error"]["code"], "invalid_case");
}

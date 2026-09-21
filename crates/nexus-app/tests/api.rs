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

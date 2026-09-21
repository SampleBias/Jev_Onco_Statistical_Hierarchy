use axum::{
    Json, Router,
    extract::DefaultBodyLimit,
    http::StatusCode,
    routing::{get, post},
};
use nexus_core::{Case, MAX_CASE_BYTES, Source, interpret, mock_response, prepare};
use serde_json::{Value, json};

pub mod tui;
pub mod workflows;

type ApiError = (StatusCode, Json<Value>);
fn invalid(message: &str) -> ApiError {
    (
        StatusCode::UNPROCESSABLE_ENTITY,
        Json(json!({"error": message})),
    )
}

pub fn router() -> Router {
    Router::new()
        .route("/health", get(|| async { Json(json!({"status": "ok", "mode": "offline_research", "live_classification": false})) }))
        .route("/v1/prepare", post(prepare_case))
        .route("/v1/demo", post(demo_case))
        .layer(DefaultBodyLimit::max(MAX_CASE_BYTES))
}

async fn prepare_case(Json(case): Json<Case>) -> Result<Json<Value>, ApiError> {
    let request = prepare(&case).map_err(|e| invalid(&e.to_string()))?;
    Ok(Json(
        json!({"research_only": true, "sends_to_provider": false, "request": request}),
    ))
}

async fn demo_case(Json(case): Json<Case>) -> Result<Json<nexus_core::ResultRecord>, ApiError> {
    interpret(&case, mock_response(), Source::Mock)
        .map(Json)
        .map_err(|e| invalid(&e.to_string()))
}

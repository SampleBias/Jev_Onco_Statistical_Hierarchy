use axum::{
    Json, Router,
    extract::{DefaultBodyLimit, rejection::JsonRejection},
    http::StatusCode,
    routing::{get, post},
};
use josh_core::errors::{ErrorCode, ErrorEnvelope};
use josh_core::{Case, MAX_CASE_BYTES, Source, interpret, mock_response, prepare};
use schemars::JsonSchema;
use serde::Serialize;

pub mod contracts;
pub mod data;
pub mod errors;
pub mod guide;
pub mod reference;
pub mod tui;
pub mod ui;
pub mod workbench;
pub mod workflows;

#[derive(Serialize, JsonSchema)]
pub struct HealthResponse {
    pub status: &'static str,
    pub mode: &'static str,
    pub live_classification: bool,
}

#[derive(Serialize, JsonSchema)]
pub struct PreparedResponse {
    pub research_only: bool,
    pub sends_to_provider: bool,
    pub request: josh_core::JevRequest,
}

type ApiError = (StatusCode, Json<ErrorEnvelope>);

fn decode(case: Result<Json<Case>, JsonRejection>) -> Result<Case, ApiError> {
    case.map(|Json(case)| case).map_err(|e| {
        let (status, code) = match e.status() {
            StatusCode::PAYLOAD_TOO_LARGE => {
                (StatusCode::PAYLOAD_TOO_LARGE, ErrorCode::InputTooLarge)
            }
            StatusCode::UNSUPPORTED_MEDIA_TYPE => (
                StatusCode::UNSUPPORTED_MEDIA_TYPE,
                ErrorCode::UnsupportedMediaType,
            ),
            StatusCode::UNPROCESSABLE_ENTITY => {
                (StatusCode::UNPROCESSABLE_ENTITY, ErrorCode::InvalidJson)
            }
            _ => (StatusCode::BAD_REQUEST, ErrorCode::InvalidJson),
        };
        (status, Json(ErrorEnvelope::new(code)))
    })
}

fn invalid(error: josh_core::ValidationError) -> ApiError {
    (
        StatusCode::UNPROCESSABLE_ENTITY,
        Json(ErrorEnvelope::invalid_case(error)),
    )
}

pub fn router() -> Router {
    Router::new()
        .route(
            "/health",
            get(|| async {
                Json(HealthResponse {
                    status: "ok",
                    mode: "offline_research",
                    live_classification: false,
                })
            }),
        )
        .route("/v1/prepare", post(prepare_case))
        .route("/v1/demo", post(demo_case))
        .route("/v1/guidance", post(guidance_case))
        .fallback(|| async {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorEnvelope::new(ErrorCode::NotFound)),
            )
        })
        .method_not_allowed_fallback(|| async {
            (
                StatusCode::METHOD_NOT_ALLOWED,
                Json(ErrorEnvelope::new(ErrorCode::MethodNotAllowed)),
            )
        })
        .layer(DefaultBodyLimit::max(MAX_CASE_BYTES))
}

async fn prepare_case(
    input: Result<Json<Case>, JsonRejection>,
) -> Result<Json<PreparedResponse>, ApiError> {
    let request = prepare(&decode(input)?).map_err(invalid)?;
    Ok(Json(PreparedResponse {
        research_only: true,
        sends_to_provider: false,
        request,
    }))
}

async fn guidance_case(
    input: Result<Json<Case>, JsonRejection>,
) -> Result<Json<josh_core::guidance::GuidanceReport>, ApiError> {
    josh_core::guidance::evaluate(&decode(input)?)
        .map(Json)
        .map_err(invalid)
}

async fn demo_case(
    input: Result<Json<Case>, JsonRejection>,
) -> Result<Json<josh_core::ResultRecord>, ApiError> {
    interpret(&decode(input)?, mock_response(), Source::Mock)
        .map(Json)
        .map_err(invalid)
}

use axum::{
    Json,
    extract::State,
    http::HeaderMap,
};
use serde::Deserialize;
use serde_json::{Value, json};

use crate::auth;
use crate::errors::AppError;
use crate::repositories::traces;
use crate::state::AppState;

const ALLOWED_AGENTS: [&str; 8] = [
    "scout",
    "analyst",
    "evidence_checker",
    "chief",
    "compliance",
    "presenter",
    "scheduler",
    "dispatcher",
];

#[derive(Debug, Deserialize)]
pub struct IngestTrace {
    pub agent: String,
    pub action: String,
    pub outcome: Option<String>,
    pub ticker: Option<String>,
    pub details: Option<Value>,
}

pub async fn post_trace(
    headers: HeaderMap,
    State(state): State<AppState>,
    Json(body): Json<IngestTrace>,
) -> Result<Json<Value>, AppError> {
    if let Some(token) = state.config.internal_api_token.as_deref() {
        if !token.is_empty() && !auth::require_internal_token(&headers, &state.config) {
            return Err(AppError::Unauthorized);
        }
    }
    let agent = body.agent.trim().to_lowercase();
    if !ALLOWED_AGENTS.contains(&agent.as_str()) {
        return Err(AppError::BadRequest(format!(
            "agent '{}' tidak dikenal",
            body.agent
        )));
    }
    if body.action.trim().is_empty() {
        return Err(AppError::BadRequest("action wajib diisi".into()));
    }
    let id = traces::trace(
        &state.pool,
        &agent,
        body.action.trim(),
        body.outcome.as_deref(),
        body.ticker.as_deref(),
        body.details,
    )
    .await?;
    Ok(Json(json!({ "trace_id": id, "status": "ok" })))
}

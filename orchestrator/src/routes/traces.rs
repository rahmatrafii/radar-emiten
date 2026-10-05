use axum::{Json, extract::State};
use serde::Deserialize;
use serde_json::{Value, json};

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
    State(state): State<AppState>,
    Json(body): Json<IngestTrace>,
) -> Result<Json<Value>, AppError> {
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

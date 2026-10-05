use axum::{
    Json,
    extract::{Query, State},
    http::HeaderMap,
};
use serde::Deserialize;
use serde_json::{Value, json};

use crate::auth;
use crate::errors::AppError;
use crate::models::finding::Finding;
use crate::repositories::snapshots;
use crate::state::AppState;

#[derive(Debug, Deserialize)]
pub struct PreviousQuery {
    pub ticker: Option<String>,
    pub metric_name: Option<String>,
}

/// GET /internal/snapshots/previous — pembanding historis untuk MCP server.
pub async fn internal_previous_snapshot(
    headers: HeaderMap,
    State(state): State<AppState>,
    Query(q): Query<PreviousQuery>,
) -> Result<Json<Value>, AppError> {
    if !auth::require_internal_token(&headers, &state.config) {
        return Err(AppError::Unauthorized);
    }
    let (Some(ticker), Some(metric_name)) = (q.ticker, q.metric_name) else {
        return Err(AppError::BadRequest("ticker dan metric_name wajib".into()));
    };
    let (_, previous) = snapshots::latest_and_previous(&state.pool, &ticker, &metric_name).await?;
    Ok(Json(json!({ "previous": previous })))
}

/// POST /internal/findings — Finding harus melewati validasi service yang sama.
pub async fn internal_record_finding(
    headers: HeaderMap,
    State(state): State<AppState>,
    Json(finding): Json<Finding>,
) -> Result<Json<Value>, AppError> {
    if !auth::require_internal_token(&headers, &state.config) {
        return Err(AppError::Unauthorized);
    }
    crate::routes::findings::accept_finding(&state, finding).await
}

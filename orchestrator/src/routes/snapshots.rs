use axum::{
    Json,
    extract::{Query, State},
    http::HeaderMap,
};
use serde::Deserialize;
use serde_json::{Value, json};

use crate::auth;
use crate::errors::AppError;
use crate::metrics;
use crate::repositories::snapshots;
use crate::state::AppState;

#[derive(Debug, Deserialize)]
pub struct IngestSnapshot {
    pub ticker: String,
    pub subsector: Option<String>,
    pub metric_name: String,
    pub value: f64,
    pub period: String,
    pub source: String,
    pub observed_at: String,
    pub raw_data: Option<Value>,
}

pub async fn post_snapshot(
    headers: HeaderMap,
    State(state): State<AppState>,
    Json(body): Json<IngestSnapshot>,
) -> Result<Json<Value>, AppError> {
    if let Some(token) = state.config.internal_api_token.as_deref() {
        if !token.is_empty() && !auth::require_internal_token(&headers, &state.config) {
            return Err(AppError::Unauthorized);
        }
    }
    if body.ticker.len() > 10
        || !body
            .ticker
            .chars()
            .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit())
    {
        return Err(AppError::BadRequest("ticker tidak valid".into()));
    }
    if !metrics::is_allowed(&body.metric_name) {
        return Err(AppError::BadRequest(format!(
            "metric_name '{}' tidak ada di registry",
            body.metric_name
        )));
    }
    if body.period.trim().is_empty() || body.source.trim().is_empty() {
        return Err(AppError::BadRequest("period/source wajib diisi".into()));
    }
    if !body.value.is_finite() {
        return Err(AppError::BadRequest("value harus angka finite".into()));
    }
    let observed_at = chrono::DateTime::parse_from_rfc3339(&body.observed_at)
        .map_err(|_| AppError::BadRequest("observed_at harus RFC3339".into()))?
        .with_timezone(&chrono::Utc);

    let id = snapshots::upsert_observed(
        &state.pool,
        &body.ticker,
        &body.metric_name,
        &body.period,
        body.value,
        &body.source,
        body.subsector.as_deref(),
        observed_at,
    )
    .await?;
    // raw_data diterima di DTO; skema snapshot_data saat ini belum punya kolom tersebut.
    let _ = body.raw_data;

    Ok(Json(json!({ "snapshot_id": id, "status": "ok" })))
}

#[derive(Debug, Deserialize)]
pub struct ListQuery {
    pub ticker: Option<String>,
    pub metric_name: Option<String>,
    pub limit: Option<i64>,
}

pub async fn get_snapshots(
    State(state): State<AppState>,
    Query(q): Query<ListQuery>,
) -> Result<Json<Value>, AppError> {
    let ticker = q
        .ticker
        .ok_or_else(|| AppError::BadRequest("parameter ticker wajib".into()))?;
    let metric_name = q
        .metric_name
        .ok_or_else(|| AppError::BadRequest("parameter metric_name wajib".into()))?;
    let limit = q.limit.unwrap_or(50).clamp(1, 200);

    let rows = snapshots::list(&state.pool, &ticker, &metric_name, limit).await?;
    Ok(Json(json!({ "snapshots": rows })))
}

use axum::{Json, extract::State};
use serde::Deserialize;
use serde_json::{Value, json};

use crate::errors::AppError;
use crate::repositories::credits;
use crate::state::AppState;

#[derive(Debug, Deserialize)]
pub struct IngestCredit {
    pub endpoint: String,
    pub credits: i32,
    pub from_cache: Option<bool>,
    pub call_reference: Option<String>,
}

pub async fn post_credit(
    State(state): State<AppState>,
    Json(body): Json<IngestCredit>,
) -> Result<Json<Value>, AppError> {
    if body.endpoint.trim().is_empty() {
        return Err(AppError::BadRequest("endpoint wajib diisi".into()));
    }
    if body.credits < 0 {
        return Err(AppError::BadRequest("credits tidak boleh negatif".into()));
    }
    let id = credits::record(
        &state.pool,
        body.endpoint.trim(),
        body.credits,
        body.from_cache.unwrap_or(false),
        body.call_reference.as_deref(),
    )
    .await?;
    Ok(Json(json!({ "credit_id": id, "status": "ok" })))
}

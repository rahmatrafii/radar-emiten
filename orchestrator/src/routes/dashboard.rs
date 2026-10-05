use axum::{
    Json,
    extract::{Query, State},
};
use serde::Deserialize;
use serde_json::{Value, json};

use crate::errors::AppError;
use crate::repositories::{credits, findings, theses, traces};
use crate::state::AppState;

#[derive(Debug, Deserialize)]
pub struct PageQuery {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

fn page(q: &PageQuery) -> (i64, i64) {
    (
        q.limit.unwrap_or(50).clamp(1, 200),
        q.offset.unwrap_or(0).max(0),
    )
}

/// GET /api/tesis — daftar tesis; nomor pengguna disamarkan.
pub async fn api_tesis(
    State(state): State<AppState>,
    Query(q): Query<PageQuery>,
) -> Result<Json<Value>, AppError> {
    let (limit, offset) = page(&q);
    let rows = theses::list_recent(&state.pool, limit, offset).await?;
    Ok(Json(json!({ "tesis": rows })))
}

/// GET /api/findings — Finding yang diterima (lolos).
pub async fn api_findings(
    State(state): State<AppState>,
    Query(q): Query<PageQuery>,
) -> Result<Json<Value>, AppError> {
    let (limit, offset) = page(&q);
    let rows = findings::list(&state.pool, Some("lolos"), limit, offset).await?;
    Ok(Json(json!({ "findings": rows })))
}

/// GET /api/ditolak — Finding/payload yang ditolak + alasan.
pub async fn api_rejected(
    State(state): State<AppState>,
    Query(q): Query<PageQuery>,
) -> Result<Json<Value>, AppError> {
    let (limit, offset) = page(&q);
    let rows = findings::list(&state.pool, Some("ditolak"), limit, offset).await?;
    Ok(Json(json!({ "ditolak": rows })))
}

/// GET /api/jejak — urutan kerja agent dari database.
pub async fn api_traces(
    State(state): State<AppState>,
    Query(q): Query<PageQuery>,
) -> Result<Json<Value>, AppError> {
    let (limit, offset) = page(&q);
    let rows = traces::list_recent(&state.pool, limit, offset).await?;
    Ok(Json(json!({ "jejak": rows })))
}

/// GET /api/kredit — budget konfigurasi, terpakai, sisa, waktu update.
pub async fn api_credits(State(state): State<AppState>) -> Result<Json<Value>, AppError> {
    let total = credits::total_used(&state.pool).await?;
    let latest = credits::latest_recorded_at(&state.pool).await?;
    let budget = state.config.sectors_credit_budget;
    let used = total as f64;
    Ok(Json(json!({
        "budget_configured": budget,
        "total_used": used,
        "remaining": (budget - used).max(0.0),
        "last_updated_at": latest,
        "note": "budget adalah konfigurasi, bukan saldo akun Sectors yang terverifikasi",
    })))
}

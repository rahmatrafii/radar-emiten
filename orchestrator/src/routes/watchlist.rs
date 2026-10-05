use axum::{Json, extract::State};
use serde_json::{Value, json};

use crate::errors::AppError;
use crate::repositories::theses;
use crate::state::AppState;

/// GET /pantauan — ticker + metric dari tesis aktif saja (tanpa nomor WhatsApp).
pub async fn get_watchlist(State(state): State<AppState>) -> Result<Json<Value>, AppError> {
    let rows = theses::active_watchlist(&state.pool).await?;
    let mut grouped: std::collections::BTreeMap<i64, Value> = std::collections::BTreeMap::new();
    for row in rows {
        let entry = grouped.entry(row.tesis_id).or_insert_with(|| {
            json!({ "tesis_id": row.tesis_id, "ticker": row.ticker, "status": row.status, "metric": [] })
        });
        entry["metric"].as_array_mut().unwrap().push(json!({
            "name": row.metric_name,
            "desired_direction": row.desired_direction,
        }));
    }
    let list: Vec<Value> = grouped.into_values().collect();
    Ok(Json(json!({ "pantauan": list })))
}

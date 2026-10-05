use axum::{Json, extract::State};
use serde_json::{Value, json};

use crate::errors::AppError;
use crate::models::finding::Finding;
use crate::repositories::{compliance, findings, snapshots, traces};
use crate::state::AppState;
use crate::validation;

/// Terapkan validasi Finding resmi + cross-check database + audit, lalu simpan.
/// Dipakai oleh POST /findings dan POST /internal/findings.
pub async fn accept_finding(state: &AppState, finding: Finding) -> Result<Json<Value>, AppError> {
    let payload = serde_json::to_value(&finding)
        .map_err(|_| AppError::BadRequest("payload Finding tidak dapat diserialisasi".into()))?;

    // Gate 1: validasi format/kontrak deterministik.
    if let Err(reason) = validation::validate_finding(&finding) {
        let finding_id = findings::insert(
            &state.pool,
            &finding.ticker,
            Some(&finding.subsector),
            &finding.metric_name,
            validation::to_f64(&finding.current_value),
            validation::to_f64(&finding.previous_value),
            Some(&finding.period),
            Some(&finding.source),
            chrono::DateTime::parse_from_rfc3339(&finding.observed_at)
                .ok()
                .map(|t| t.with_timezone(&chrono::Utc)),
            Some(finding.confidence_score),
            Some(&finding.finding_summary),
            payload,
            "ditolak",
        )
        .await
        .unwrap_or(0);
        let _ = compliance::log(
            &state.pool,
            if finding_id == 0 {
                None
            } else {
                Some(finding_id)
            },
            false,
            false,
            json!([]),
            Some(&reason),
            "finding",
            None,
        )
        .await;
        return Ok(Json(json!({
            "finding_id": if finding_id == 0 { Value::Null } else { json!(finding_id) },
            "status": "rejected",
            "rejection_reason": reason,
        })));
    }

    let current = validation::to_f64(&finding.current_value).unwrap();
    let previous = validation::to_f64(&finding.previous_value).unwrap();
    let observed_at = chrono::DateTime::parse_from_rfc3339(&finding.observed_at)
        .unwrap()
        .with_timezone(&chrono::Utc);

    // Gate 2: cross-check dengan snapshot tersimpan.
    let current_ok = snapshots::value_exists(
        &state.pool,
        &finding.ticker,
        &finding.metric_name,
        &finding.period,
        &finding.source,
        current,
    )
    .await?;
    let previous_ok = snapshots::previous_exists(
        &state.pool,
        &finding.ticker,
        &finding.metric_name,
        previous,
        &finding.period,
    )
    .await?;

    if !current_ok || !previous_ok {
        let reason = "current_value/previous_value tidak cocok dengan snapshot tersimpan";
        let finding_id = findings::insert(
            &state.pool,
            &finding.ticker,
            Some(&finding.subsector),
            &finding.metric_name,
            Some(current),
            Some(previous),
            Some(&finding.period),
            Some(&finding.source),
            Some(observed_at),
            Some(finding.confidence_score),
            Some(&finding.finding_summary),
            payload,
            "ditolak",
        )
        .await?;
        compliance::log(
            &state.pool,
            Some(finding_id),
            false,
            false,
            json!([]),
            Some(reason),
            "finding",
            None,
        )
        .await?;
        return Ok(Json(json!({
            "finding_id": finding_id,
            "status": "rejected",
            "rejection_reason": reason,
        })));
    }

    // Gate 3: duplikasi lolos (unique partial index).
    let inserted = findings::insert(
        &state.pool,
        &finding.ticker,
        Some(&finding.subsector),
        &finding.metric_name,
        Some(current),
        Some(previous),
        Some(&finding.period),
        Some(&finding.source),
        Some(observed_at),
        Some(finding.confidence_score),
        Some(&finding.finding_summary),
        payload,
        "lolos",
    )
    .await;

    match inserted {
        Ok(finding_id) => {
            compliance::log(
                &state.pool,
                Some(finding_id),
                true,
                true,
                json!([]),
                None,
                "finding",
                None,
            )
            .await?;
            traces::trace(
                &state.pool,
                "evidence_checker",
                "finding_accepted",
                Some("accepted"),
                Some(&finding.ticker),
                None,
            )
            .await?;
            Ok(Json(json!({
                "finding_id": finding_id,
                "status": "accepted",
                "rejection_reason": Value::Null,
            })))
        }
        Err(sqlx::Error::Database(db_err)) if db_err.is_unique_violation() => {
            let reason = "finding duplikat: ticker+metric+period sudah ada yang lolos";
            compliance::log(
                &state.pool,
                None,
                false,
                true,
                json!([]),
                Some(reason),
                "finding",
                None,
            )
            .await?;
            Ok(Json(json!({
                "finding_id": Value::Null,
                "status": "rejected",
                "rejection_reason": reason,
            })))
        }
        Err(e) => Err(e.into()),
    }
}

pub async fn post_finding(
    State(state): State<AppState>,
    Json(finding): Json<Finding>,
) -> Result<Json<Value>, AppError> {
    accept_finding(&state, finding).await
}

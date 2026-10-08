use axum::{
    Json,
    extract::State,
    http::HeaderMap,
};
use serde_json::{Value, json};

use crate::auth;
use crate::errors::AppError;
use crate::models::finding::Finding;
use crate::repositories::{compliance, findings, traces};
use crate::services::{compliance_service, evidence_verifier};
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

    // Gate 1.5: compliance check kata terlarang (D-02, milik Dian).
    // Sumber daftar: docs/PROMPTS.md bagian 3, via compliance_service.
    let compliance_result = compliance_service::check_compliance(&finding.finding_summary);
    if !compliance_result.is_compliant {
        let reason = format!(
            "kata terlarang terdeteksi: {}",
            compliance_result.prohibited_words_detected.join(", ")
        );
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
            serde_json::to_value(&compliance_result.prohibited_words_detected).unwrap_or(json!([])),
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

    // Gate 2: evidence verification 1:1 via service (D-03, milik Dian).
    // Logic sama seperti sebelumnya, kini terpusat di evidence_verifier.
    let verification = evidence_verifier::verify_evidence(&state.pool, &finding).await?;

    if !verification.evidence_verified {
        let reason = verification
            .rejection_reason
            .unwrap_or_else(|| "evidence tidak terverifikasi".to_string());
        let current = validation::to_f64(&finding.current_value).unwrap_or(0.0);
        let previous = validation::to_f64(&finding.previous_value).unwrap_or(0.0);
        let observed_at = chrono::DateTime::parse_from_rfc3339(&finding.observed_at)
            .map(|t| t.with_timezone(&chrono::Utc))
            .unwrap_or_else(|_| chrono::Utc::now());
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
            Some(&reason),
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

    // Gate 1 + Gate 2 lolos → nilai numerik dan timestamp valid.
    let current = validation::to_f64(&finding.current_value).unwrap();
    let previous = validation::to_f64(&finding.previous_value).unwrap();
    let observed_at = chrono::DateTime::parse_from_rfc3339(&finding.observed_at)
        .unwrap()
        .with_timezone(&chrono::Utc);

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
    headers: HeaderMap,
    State(state): State<AppState>,
    Json(finding): Json<Finding>,
) -> Result<Json<Value>, AppError> {
    if let Some(token) = state.config.internal_api_token.as_deref() {
        if !token.is_empty() && !auth::require_internal_token(&headers, &state.config) {
            return Err(AppError::Unauthorized);
        }
    }
    accept_finding(&state, finding).await
}

use sqlx::PgPool;

pub async fn log(
    pool: &PgPool,
    finding_id: Option<i64>,
    is_compliant: bool,
    evidence_verified: bool,
    prohibited_words_detected: serde_json::Value,
    rejection_reason: Option<&str>,
    stage: &str,
    details: Option<serde_json::Value>,
) -> sqlx::Result<i64> {
    let row: (i64,) = sqlx::query_as(
        "INSERT INTO compliance_audit_logs \
         (finding_id, is_compliant, evidence_verified, prohibited_words_detected, rejection_reason, stage, details) \
         VALUES ($1,$2,$3,$4,$5,$6,$7) RETURNING id",
    )
    .bind(finding_id)
    .bind(is_compliant)
    .bind(evidence_verified)
    .bind(prohibited_words_detected)
    .bind(rejection_reason)
    .bind(stage)
    .bind(details)
    .fetch_one(pool)
    .await?;
    Ok(row.0)
}

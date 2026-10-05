use sqlx::PgPool;

pub async fn insert(
    pool: &PgPool,
    ticker: &str,
    subsector: Option<&str>,
    metric_name: &str,
    current_value: Option<f64>,
    previous_value: Option<f64>,
    period: Option<&str>,
    source: Option<&str>,
    observed_at: Option<chrono::DateTime<chrono::Utc>>,
    confidence_score: Option<f64>,
    finding_summary: Option<&str>,
    payload: serde_json::Value,
    status: &str,
) -> sqlx::Result<i64> {
    let row: (i64,) = sqlx::query_as(
        "INSERT INTO findings (ticker, subsector, metric_name, current_value, previous_value, \
         period, source, observed_at, confidence_score, finding_summary, payload, status) \
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12) RETURNING id",
    )
    .bind(ticker)
    .bind(subsector)
    .bind(metric_name)
    .bind(current_value)
    .bind(previous_value)
    .bind(period)
    .bind(source)
    .bind(observed_at)
    .bind(confidence_score)
    .bind(finding_summary)
    .bind(payload)
    .bind(status)
    .fetch_one(pool)
    .await?;
    Ok(row.0)
}

/// Finding terbaru yang lolos untuk ticker tesis aktif milik pengguna.
pub async fn latest_accepted_for_user(
    pool: &PgPool,
    pengguna_id: i64,
) -> sqlx::Result<Option<crate::models::finding::FindingRecord>> {
    sqlx::query_as(
        "SELECT f.id, f.ticker, f.subsector, f.metric_name, f.current_value, f.previous_value, \
         f.period, f.source, f.observed_at, f.confidence_score, f.finding_summary, f.status, \
         f.rejection_reason, f.dibuat_pada \
         FROM findings f WHERE f.status='lolos' AND f.ticker IN \
         (SELECT ticker FROM tesis WHERE pengguna_id=$1 AND aktif=true) \
         ORDER BY f.dibuat_pada DESC LIMIT 1",
    )
    .bind(pengguna_id)
    .fetch_optional(pool)
    .await
}

pub async fn list(
    pool: &PgPool,
    status: Option<&str>,
    limit: i64,
    offset: i64,
) -> sqlx::Result<Vec<crate::models::finding::FindingRecord>> {
    sqlx::query_as(
        "SELECT id, ticker, subsector, metric_name, current_value, previous_value, period, \
         source, observed_at, confidence_score, finding_summary, status, rejection_reason, dibuat_pada \
         FROM findings WHERE ($1::TEXT IS NULL OR status = $1) \
         ORDER BY dibuat_pada DESC LIMIT $2 OFFSET $3",
    )
    .bind(status)
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await
}

pub async fn set_status(
    pool: &PgPool,
    finding_id: i64,
    status: &str,
    is_compliant: Option<bool>,
    evidence_verified: Option<bool>,
    prohibited_words: Vec<String>,
    rejection_reason: Option<&str>,
) -> sqlx::Result<()> {
    sqlx::query(
        "UPDATE findings SET status=$2, is_compliant=$3, evidence_verified=$4, \
         prohibited_words=$5, rejection_reason=$6 WHERE id=$1",
    )
    .bind(finding_id)
    .bind(status)
    .bind(is_compliant)
    .bind(evidence_verified)
    .bind(&prohibited_words)
    .bind(rejection_reason)
    .execute(pool)
    .await?;
    Ok(())
}

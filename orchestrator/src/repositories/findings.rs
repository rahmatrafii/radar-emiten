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

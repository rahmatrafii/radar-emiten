use sqlx::PgPool;

pub async fn trace(
    pool: &PgPool,
    agent: &str,
    aksi: &str,
    outcome: Option<&str>,
    ticker: Option<&str>,
    details: Option<serde_json::Value>,
) -> sqlx::Result<i64> {
    let row: (i64,) = sqlx::query_as(
        "INSERT INTO jejak_agent (agent, aksi, outcome, ticker, details) \
         VALUES ($1,$2,$3,$4,$5) RETURNING id",
    )
    .bind(agent)
    .bind(aksi)
    .bind(outcome)
    .bind(ticker)
    .bind(details)
    .fetch_one(pool)
    .await?;
    Ok(row.0)
}

#[derive(Debug, Clone, serde::Serialize, sqlx::FromRow)]
pub struct TraceRecord {
    pub id: i64,
    pub agent: String,
    pub aksi: String,
    pub outcome: Option<String>,
    pub ticker: Option<String>,
    pub details: Option<serde_json::Value>,
    pub dicatat_pada: chrono::DateTime<chrono::Utc>,
}

/// Jejak agent terbaru untuk /api/jejak.
pub async fn list_recent(pool: &PgPool, limit: i64, offset: i64) -> sqlx::Result<Vec<TraceRecord>> {
    sqlx::query_as(
        "SELECT id, agent, aksi, outcome, ticker, details, dicatat_pada \
         FROM jejak_agent ORDER BY dicatat_pada DESC, id DESC LIMIT $1 OFFSET $2",
    )
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await
}

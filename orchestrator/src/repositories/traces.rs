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

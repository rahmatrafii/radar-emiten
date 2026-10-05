use sqlx::PgPool;

pub async fn record(
    pool: &PgPool,
    endpoint: &str,
    kredit: i32,
    dari_cache: bool,
    call_reference: Option<&str>,
) -> sqlx::Result<i64> {
    let row: (i64,) = sqlx::query_as(
        "INSERT INTO penggunaan_kredit (endpoint, kredit, dari_cache, call_reference) \
         VALUES ($1,$2,$3,$4) RETURNING id",
    )
    .bind(endpoint)
    .bind(kredit)
    .bind(dari_cache)
    .bind(call_reference)
    .fetch_one(pool)
    .await?;
    Ok(row.0)
}

pub async fn total_used(pool: &PgPool) -> sqlx::Result<i64> {
    let row: (i64,) = sqlx::query_as("SELECT COALESCE(SUM(kredit),0) FROM penggunaan_kredit")
        .fetch_one(pool)
        .await?;
    Ok(row.0)
}

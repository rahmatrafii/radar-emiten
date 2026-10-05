use sqlx::PgPool;

/// Ambil snapshot paling baru untuk ticker+metric, dan snapshot pembanding
/// (periode sebelumnya berdasarkan waktu observasi, bukan urutan alfabetis).
pub async fn latest_and_previous(
    pool: &PgPool,
    ticker: &str,
    metric_name: &str,
) -> sqlx::Result<(Option<crate::models::snapshot::Snapshot>, Option<crate::models::snapshot::Snapshot>)> {
    let rows: Vec<crate::models::snapshot::Snapshot> = sqlx::query_as(
        "SELECT id, ticker, metric_name, period, value, source, diambil_pada, subsector \
         FROM snapshot_data WHERE ticker = $1 AND metric_name = $2 \
         ORDER BY diambil_pada DESC LIMIT 2",
    )
    .bind(ticker)
    .bind(metric_name)
    .fetch_all(pool)
    .await?;

    let mut it = rows.into_iter();
    let current = it.next();
    let previous = it.next();
    Ok((current, previous))
}

/// Upsert snapshot; duplikat (ticker, metric_name, period, source) tidak membuat baris baru.
pub async fn upsert(
    pool: &PgPool,
    ticker: &str,
    metric_name: &str,
    period: &str,
    value: f64,
    source: &str,
    subsector: Option<&str>,
) -> sqlx::Result<i64> {
    let row: (i64,) = sqlx::query_as(
        "INSERT INTO snapshot_data (ticker, metric_name, period, value, source, subsector) \
         VALUES ($1, $2, $3, $4, $5, $6) \
         ON CONFLICT (ticker, metric_name, period, source) \
         DO UPDATE SET value = EXCLUDED.value \
         RETURNING id",
    )
    .bind(ticker)
    .bind(metric_name)
    .bind(period)
    .bind(value)
    .bind(source)
    .bind(subsector)
    .fetch_one(pool)
    .await?;
    Ok(row.0)
}

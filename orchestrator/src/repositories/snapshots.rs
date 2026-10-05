use sqlx::PgPool;

/// Ambil snapshot paling baru untuk ticker+metric, dan snapshot pembanding
/// (periode sebelumnya berdasarkan waktu observasi, bukan urutan alfabetis).
pub async fn latest_and_previous(
    pool: &PgPool,
    ticker: &str,
    metric_name: &str,
) -> sqlx::Result<(
    Option<crate::models::snapshot::Snapshot>,
    Option<crate::models::snapshot::Snapshot>,
)> {
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

/// Upsert snapshot dengan timestamp observasi eksplisit dari sumber.
pub async fn upsert_observed(
    pool: &PgPool,
    ticker: &str,
    metric_name: &str,
    period: &str,
    value: f64,
    source: &str,
    subsector: Option<&str>,
    observed_at: chrono::DateTime<chrono::Utc>,
) -> sqlx::Result<i64> {
    let row: (i64,) = sqlx::query_as(
        "INSERT INTO snapshot_data (ticker, metric_name, period, value, source, subsector, diambil_pada) \
         VALUES ($1, $2, $3, $4, $5, $6, $7) \
         ON CONFLICT (ticker, metric_name, period, source) \
         DO UPDATE SET value = EXCLUDED.value, diambil_pada = EXCLUDED.diambil_pada \
         RETURNING id",
    )
    .bind(ticker)
    .bind(metric_name)
    .bind(period)
    .bind(value)
    .bind(source)
    .bind(subsector)
    .bind(observed_at)
    .fetch_one(pool)
    .await?;
    Ok(row.0)
}

/// Riwayat observasi terbaru untuk ticker+metric (bukan dari Sectors API).
pub async fn list(
    pool: &PgPool,
    ticker: &str,
    metric_name: &str,
    limit: i64,
) -> sqlx::Result<Vec<crate::models::snapshot::Snapshot>> {
    sqlx::query_as(
        "SELECT id, ticker, metric_name, period, value, source, diambil_pada, subsector \
         FROM snapshot_data WHERE ticker = $1 AND metric_name = $2 \
         ORDER BY diambil_pada DESC LIMIT $3",
    )
    .bind(ticker)
    .bind(metric_name)
    .bind(limit)
    .fetch_all(pool)
    .await
}

/// Cek konsistensi evidensi: value sekarang harus cocok dengan snapshot tersimpan.
pub async fn value_exists(
    pool: &PgPool,
    ticker: &str,
    metric_name: &str,
    period: &str,
    source: &str,
    value: f64,
) -> sqlx::Result<bool> {
    let row: (bool,) = sqlx::query_as(
        "SELECT EXISTS(SELECT 1 FROM snapshot_data \
         WHERE ticker=$1 AND metric_name=$2 AND period=$3 AND source=$4 AND value=$5)",
    )
    .bind(ticker)
    .bind(metric_name)
    .bind(period)
    .bind(source)
    .bind(value)
    .fetch_one(pool)
    .await?;
    Ok(row.0)
}

/// Cek bahwa nilai pembanding memang snapshot lain (periode berbeda).
pub async fn previous_exists(
    pool: &PgPool,
    ticker: &str,
    metric_name: &str,
    value: f64,
    current_period: &str,
) -> sqlx::Result<bool> {
    let row: (bool,) = sqlx::query_as(
        "SELECT EXISTS(SELECT 1 FROM snapshot_data \
         WHERE ticker=$1 AND metric_name=$2 AND value=$3 AND period <> $4)",
    )
    .bind(ticker)
    .bind(metric_name)
    .bind(value)
    .bind(current_period)
    .fetch_one(pool)
    .await?;
    Ok(row.0)
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

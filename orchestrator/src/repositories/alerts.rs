use sqlx::PgPool;

pub async fn insert(
    pool: &PgPool,
    pengguna_id: i64,
    tesis_id: i64,
    finding_id: i64,
    metric_name: &str,
    period: &str,
    header: &str,
    isi_pesan: &str,
) -> sqlx::Result<i64> {
    let row: (i64,) = sqlx::query_as(
        "INSERT INTO alerts (pengguna_id, tesis_id, finding_id, header, isi_pesan, metric_name, period) \
         VALUES ($1,$2,$3,$4,$5,$6,$7) RETURNING id",
    )
    .bind(pengguna_id)
    .bind(tesis_id)
    .bind(finding_id)
    .bind(header)
    .bind(isi_pesan)
    .bind(metric_name)
    .bind(period)
    .fetch_one(pool)
    .await?;
    Ok(row.0)
}

/// Jumlah alert yang benar-benar terkirim untuk user pada hari ini (WIB).
pub async fn count_sent_today(pool: &PgPool, pengguna_id: i64) -> sqlx::Result<i64> {
    let row: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM alerts WHERE pengguna_id=$1 AND status_kirim='terkirim' \
         AND dikirim_pada >= date_trunc('day', NOW() AT TIME ZONE 'Asia/Jakarta') AT TIME ZONE 'Asia/Jakarta'",
    )
    .bind(pengguna_id)
    .fetch_one(pool)
    .await?;
    Ok(row.0)
}

pub async fn mark_sent(pool: &PgPool, alert_id: i64) -> sqlx::Result<()> {
    sqlx::query("UPDATE alerts SET status_kirim='terkirim', dikirim_pada=NOW() WHERE id=$1")
        .bind(alert_id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn mark_status(pool: &PgPool, alert_id: i64, status: &str) -> sqlx::Result<()> {
    sqlx::query("UPDATE alerts SET status_kirim=$2 WHERE id=$1")
        .bind(alert_id)
        .bind(status)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn find_existing(
    pool: &PgPool,
    pengguna_id: i64,
    tesis_id: i64,
    metric_name: &str,
    period: &str,
) -> sqlx::Result<Option<(i64, String)>> {
    let row: Option<(i64, String)> = sqlx::query_as(
        "SELECT id, status_kirim FROM alerts \
         WHERE pengguna_id=$1 AND tesis_id=$2 AND metric_name=$3 AND period=$4",
    )
    .bind(pengguna_id)
    .bind(tesis_id)
    .bind(metric_name)
    .bind(period)
    .fetch_optional(pool)
    .await?;
    Ok(row)
}

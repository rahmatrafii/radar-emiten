use sqlx::PgPool;

pub async fn insert_pending(
    pool: &PgPool,
    pengguna_id: i64,
    teks_asli: &str,
    ticker: &str,
) -> sqlx::Result<i64> {
    let row: (i64,) = sqlx::query_as(
        "INSERT INTO tesis (pengguna_id, teks_asli, ticker, status, aktif) \
         VALUES ($1,$2,$3,'PENDING_CONFIRMATION',false) RETURNING id",
    )
    .bind(pengguna_id)
    .bind(teks_asli)
    .bind(ticker)
    .fetch_one(pool)
    .await?;
    Ok(row.0)
}

pub async fn add_indicator(
    pool: &PgPool,
    tesis_id: i64,
    metric_name: &str,
    desired_direction: &str,
) -> sqlx::Result<()> {
    sqlx::query(
        "INSERT INTO indikator_tesis (tesis_id, metric_name, desired_direction) VALUES ($1,$2,$3) \
         ON CONFLICT (tesis_id, metric_name) DO NOTHING",
    )
    .bind(tesis_id)
    .bind(metric_name)
    .bind(desired_direction)
    .execute(pool)
    .await?;
    Ok(())
}

/// Tesis terakhir milik pengguna yang menunggu konfirmasi (id, ticker).
pub async fn latest_pending(
    pool: &PgPool,
    pengguna_id: i64,
) -> sqlx::Result<Option<(i64, String)>> {
    let row: Option<(i64, String)> = sqlx::query_as(
        "SELECT id, ticker FROM tesis WHERE pengguna_id=$1 AND status='PENDING_CONFIRMATION' \
         ORDER BY id DESC LIMIT 1",
    )
    .bind(pengguna_id)
    .fetch_optional(pool)
    .await?;
    Ok(row)
}

/// Batalkan (hapus) tesis pending terakhir pengguna; kembalikan true bila ada yang dihapus.
pub async fn cancel_pending(pool: &PgPool, pengguna_id: i64) -> sqlx::Result<bool> {
    let res = sqlx::query(
        "DELETE FROM tesis WHERE id IN (SELECT id FROM tesis WHERE pengguna_id=$1 \
         AND status='PENDING_CONFIRMATION' ORDER BY id DESC LIMIT 1)",
    )
    .bind(pengguna_id)
    .execute(pool)
    .await?;
    Ok(res.rows_affected() > 0)
}

/// Daftar tesis aktif pengguna: (id, ticker, status).
pub async fn list_active(
    pool: &PgPool,
    pengguna_id: i64,
) -> sqlx::Result<Vec<(i64, String, String)>> {
    sqlx::query_as(
        "SELECT id, ticker, status FROM tesis WHERE pengguna_id=$1 AND aktif=true ORDER BY id",
    )
    .bind(pengguna_id)
    .fetch_all(pool)
    .await
}

/// Hapus tesis tertentu milik pengguna (workflow /hapus dengan konfirmasi id).
pub async fn delete_by_id(pool: &PgPool, pengguna_id: i64, tesis_id: i64) -> sqlx::Result<bool> {
    let res = sqlx::query("DELETE FROM tesis WHERE id=$1 AND pengguna_id=$2")
        .bind(tesis_id)
        .bind(pengguna_id)
        .execute(pool)
        .await?;
    Ok(res.rows_affected() > 0)
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ThesisRow {
    pub id: i64,
    pub ticker: String,
    pub status: String,
    pub aktif: bool,
    pub dibuat_pada: chrono::DateTime<chrono::Utc>,
    pub nomor_masked: Option<String>,
}

/// Daftar tesis dengan nomor pengguna yang sudah disamarkan (siap untuk /api/tesis).
#[derive(sqlx::FromRow)]
struct ThesisJoin {
    id: i64,
    ticker: String,
    status: String,
    aktif: bool,
    dibuat_pada: chrono::DateTime<chrono::Utc>,
    nomor: Option<String>,
}

pub async fn list_recent(pool: &PgPool, limit: i64, offset: i64) -> sqlx::Result<Vec<ThesisRow>> {
    let rows: Vec<ThesisJoin> = sqlx::query_as(
        "SELECT t.id, t.ticker, t.status, t.aktif, t.dibuat_pada, p.nomor_wa AS nomor \
         FROM tesis t LEFT JOIN pengguna p ON p.id = t.pengguna_id \
         ORDER BY t.dibuat_pada DESC LIMIT $1 OFFSET $2",
    )
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await?;
    Ok(rows
        .into_iter()
        .map(|r| ThesisRow {
            id: r.id,
            ticker: r.ticker,
            status: r.status,
            aktif: r.aktif,
            dibuat_pada: r.dibuat_pada,
            nomor_masked: r.nomor.map(|n| crate::masking::mask_wa(&n)),
        })
        .collect())
}

#[derive(Debug, Clone, serde::Serialize, sqlx::FromRow)]
pub struct WatchRow {
    pub tesis_id: i64,
    pub ticker: String,
    pub status: String,
    pub metric_name: String,
    pub desired_direction: String,
}

/// Ticker + metric dari tesis aktif saja (untuk /pantauan).
pub async fn active_watchlist(pool: &PgPool) -> sqlx::Result<Vec<WatchRow>> {
    sqlx::query_as(
        "SELECT t.id AS tesis_id, t.ticker, t.status, i.metric_name, i.desired_direction \
         FROM tesis t JOIN indikator_tesis i ON i.tesis_id = t.id \
         WHERE t.aktif = true ORDER BY t.ticker, i.metric_name",
    )
    .fetch_all(pool)
    .await
}

pub async fn count_active(pool: &PgPool, pengguna_id: i64) -> sqlx::Result<i64> {
    let row: (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM tesis WHERE pengguna_id=$1 AND aktif=true")
            .bind(pengguna_id)
            .fetch_one(pool)
            .await?;
    Ok(row.0)
}

pub async fn activate(pool: &PgPool, tesis_id: i64) -> sqlx::Result<()> {
    sqlx::query("UPDATE tesis SET aktif=true, status='BELUM_CUKUP_DATA' WHERE id=$1")
        .bind(tesis_id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn update_status(pool: &PgPool, tesis_id: i64, status: &str) -> sqlx::Result<()> {
    sqlx::query("UPDATE tesis SET status=$2 WHERE id=$1")
        .bind(tesis_id)
        .bind(status)
        .execute(pool)
        .await?;
    Ok(())
}

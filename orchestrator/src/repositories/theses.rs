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

pub async fn count_active(pool: &PgPool, pengguna_id: i64) -> sqlx::Result<i64> {
    let row: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM tesis WHERE pengguna_id=$1 AND aktif=true",
    )
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

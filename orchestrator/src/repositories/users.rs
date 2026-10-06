use sqlx::PgPool;

pub async fn upsert_by_wa(pool: &PgPool, nomor_wa: &str) -> sqlx::Result<i64> {
    let row: (i64,) = sqlx::query_as(
        "INSERT INTO pengguna (nomor_wa) VALUES ($1) \
         ON CONFLICT (nomor_wa) DO UPDATE SET nomor_wa = EXCLUDED.nomor_wa \
         RETURNING id",
    )
    .bind(nomor_wa)
    .fetch_one(pool)
    .await?;
    Ok(row.0)
}

pub async fn set_opt_in(pool: &PgPool, user_id: i64, opt_in: bool) -> sqlx::Result<()> {
    sqlx::query("UPDATE pengguna SET opt_in=$2 WHERE id=$1")
        .bind(user_id)
        .bind(opt_in)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn is_muted(pool: &PgPool, user_id: i64) -> sqlx::Result<bool> {
    let row: (bool,) = sqlx::query_as(
        "SELECT jeda_sampai IS NOT NULL AND jeda_sampai > NOW() FROM pengguna WHERE id=$1",
    )
    .bind(user_id)
    .fetch_one(pool)
    .await?;
    Ok(row.0)
}

pub async fn set_muted_until(
    pool: &PgPool,
    user_id: i64,
    until: Option<chrono::DateTime<chrono::Utc>>,
) -> sqlx::Result<()> {
    sqlx::query("UPDATE pengguna SET jeda_sampai=$2 WHERE id=$1")
        .bind(user_id)
        .bind(until)
        .execute(pool)
        .await?;
    Ok(())
}

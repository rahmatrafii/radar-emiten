use sqlx::PgPool;

/// Simpan pesan masuk; kembalikan true jika pesan baru (bukan duplikat `id_pesan_wa`).
pub async fn insert_if_new(
    pool: &PgPool,
    id_pesan_wa: &str,
    pengguna_id: Option<i64>,
    nomor_pengirim: &str,
    isi: Option<&str>,
) -> sqlx::Result<bool> {
    let res = sqlx::query(
        "INSERT INTO pesan_masuk (id_pesan_wa, pengguna_id, nomor_pengirim, isi) \
         VALUES ($1,$2,$3,$4) ON CONFLICT (id_pesan_wa) DO NOTHING",
    )
    .bind(id_pesan_wa)
    .bind(pengguna_id)
    .bind(nomor_pengirim)
    .bind(isi)
    .execute(pool)
    .await?;
    Ok(res.rows_affected() > 0)
}

pub async fn set_status(pool: &PgPool, id_pesan_wa: &str, status: &str) -> sqlx::Result<()> {
    sqlx::query(
        "UPDATE pesan_masuk SET processing_status=$2, \
         attempt_count = attempt_count + 1, \
         diproses_pada = CASE WHEN $2 IN ('processed','failed','ignored') THEN NOW() ELSE diproses_pada END \
         WHERE id_pesan_wa=$1",
    )
    .bind(id_pesan_wa)
    .bind(status)
    .execute(pool)
    .await?;
    Ok(())
}

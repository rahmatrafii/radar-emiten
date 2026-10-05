//! Test repository — membutuhkan PostgreSQL lokal (DATABASE_URL).
//! Dijalankan lewat: `cargo test -- --ignored` jika tidak set DB.
use sqlx::PgPool;

async fn pool() -> Option<PgPool> {
    let url = std::env::var("DATABASE_URL").ok()?;
    PgPool::connect(&url).await.ok()
}

#[tokio::test]
async fn snapshots_upsert_deduplikasi() {
    let Some(pool) = pool().await else { return };
    // Bersihkan data tes
    sqlx::query("DELETE FROM snapshot_data WHERE ticker='TEST'").execute(&pool).await.unwrap();

    let id1 = repositories::snapshots::upsert(&pool, "TEST", "roe", "Q1 2026", 10.0, "unit-test", Some("banks")).await.unwrap();
    let id2 = repositories::snapshots::upsert(&pool, "TEST", "roe", "Q1 2026", 11.0, "unit-test", Some("banks")).await.unwrap();
    assert_eq!(id1, id2, "upsert harus mengembalikan id yang sama untuk observasi duplikat");

    sqlx::query("DELETE FROM snapshot_data WHERE ticker='TEST'").execute(&pool).await.unwrap();
}

#[tokio::test]
async fn findings_insert_dan_audit() {
    let Some(pool) = pool().await else { return };
    sqlx::query("DELETE FROM compliance_audit_logs WHERE finding_id IN (SELECT id FROM findings WHERE ticker='TEST')").execute(&pool).await.unwrap();
    sqlx::query("DELETE FROM findings WHERE ticker='TEST'").execute(&pool).await.unwrap();

    let fid = repositories::findings::insert(
        &pool, "TEST", Some("banks"), "roe", Some(12.0), Some(10.0), Some("Q1 2026"),
        Some("unit-test"), None, Some(0.9), Some("test finding"), serde_json::json!({}), "menunggu",
    ).await.unwrap();

    repositories::compliance::log(&pool, Some(fid), true, true, serde_json::json!([]), None, "finding", None).await.unwrap();

    let row: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM compliance_audit_logs WHERE finding_id=$1")
        .bind(fid).fetch_one(&pool).await.unwrap();
    assert_eq!(row.0, 1);

    sqlx::query("DELETE FROM compliance_audit_logs WHERE finding_id=$1").bind(fid).execute(&pool).await.unwrap();
    sqlx::query("DELETE FROM findings WHERE id=$1").bind(fid).execute(&pool).await.unwrap();
}

#[tokio::test]
async fn inbound_deduplikasi_message_id() {
    let Some(pool) = pool().await else { return };
    sqlx::query("DELETE FROM pesan_masuk WHERE id_pesan_wa LIKE 'wamid.TEST%'").execute(&pool).await.unwrap();

    let first = repositories::inbound::insert_if_new(&pool, "wamid.TEST1", None, "628111", Some("halo")).await.unwrap();
    let dupe = repositories::inbound::insert_if_new(&pool, "wamid.TEST1", None, "628111", Some("halo")).await.unwrap();
    assert!(first);
    assert!(!dupe);

    sqlx::query("DELETE FROM pesan_masuk WHERE id_pesan_wa LIKE 'wamid.TEST%'").execute(&pool).await.unwrap();
}

#[tokio::test]
async fn alerts_dedupe_periode() {
    let Some(pool) = pool().await else { return };
    // butuh user + tesis + finding yang valid; pakai insert manual
    let uid: (i64,) = sqlx::query_as("INSERT INTO pengguna (nomor_wa) VALUES ('6280000000001') RETURNING id").fetch_one(&pool).await.unwrap();
    let tid: (i64,) = sqlx::query_as("INSERT INTO tesis (pengguna_id, teks_asli, ticker) VALUES ($1,'t','TEST') RETURNING id").bind(uid.0).fetch_one(&pool).await.unwrap();
    let fid: (i64,) = sqlx::query_as("INSERT INTO findings (payload) VALUES ('{}') RETURNING id").fetch_one(&pool).await.unwrap();

    let a1 = repositories::alerts::insert(&pool, uid.0, tid.0, fid.0, "roe", "Q1 2026", "h", "isi").await;
    let a2 = repositories::alerts::insert(&pool, uid.0, tid.0, fid.0, "roe", "Q1 2026", "h", "isi").await;
    assert!(a1.is_ok());
    assert!(a2.is_err(), "alert duplikat harus ditolak constraint unik");

    sqlx::query("DELETE FROM alerts WHERE id=$1").bind(a1.unwrap()).execute(&pool).await.unwrap();
    sqlx::query("DELETE FROM findings WHERE id=$1").bind(fid.0).execute(&pool).await.unwrap();
    sqlx::query("DELETE FROM tesis WHERE id=$1").bind(tid.0).execute(&pool).await.unwrap();
    sqlx::query("DELETE FROM pengguna WHERE id=$1").bind(uid.0).execute(&pool).await.unwrap();
}

use orchestrator::repositories;

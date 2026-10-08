//! Test pipeline Tahap 8 — butuh PostgreSQL lokal (DATABASE_URL).
//! Tanpa DATABASE_URL, test dilewati (pola sama seperti repository_tests).

use orchestrator::config::{AppMode, Config};
use orchestrator::integrations::mcp_gateway::MockMcpGateway;
use orchestrator::orchestrator::agent_pipeline::AgentPipeline;
use orchestrator::repositories::{alerts, snapshots, theses, users};
use orchestrator::state::AppState;
use sqlx::PgPool;

const SRC: &str = "Sectors API v2 /v2/company/report/BBCA";

async fn pool() -> Option<PgPool> {
    let url = std::env::var("DATABASE_URL").ok()?;
    PgPool::connect(&url).await.ok()
}

fn mock_config() -> Config {
    Config {
        app_mode: AppMode::Mock,
        policy_disclaimer_conflict_acknowledged: true,
        ..Default::default()
    }
}

async fn bersihkan(pool: &PgPool, nomor: &str) {
    sqlx::query("DELETE FROM finding_tesis WHERE tesis_id IN (SELECT id FROM tesis WHERE pengguna_id IN (SELECT id FROM pengguna WHERE nomor_wa=$1))")
        .bind(nomor).execute(pool).await.unwrap();
    sqlx::query(
        "DELETE FROM alerts WHERE pengguna_id IN (SELECT id FROM pengguna WHERE nomor_wa=$1)",
    )
    .bind(nomor)
    .execute(pool)
    .await
    .unwrap();
    sqlx::query(
        "DELETE FROM tesis WHERE pengguna_id IN (SELECT id FROM pengguna WHERE nomor_wa=$1)",
    )
    .bind(nomor)
    .execute(pool)
    .await
    .unwrap();
    sqlx::query("DELETE FROM pengguna WHERE nomor_wa=$1")
        .bind(nomor)
        .execute(pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM compliance_audit_logs WHERE finding_id IN (SELECT id FROM findings WHERE ticker='BBCA')")
        .execute(pool).await.unwrap();
    sqlx::query("DELETE FROM findings WHERE ticker='BBCA'")
        .execute(pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM snapshot_data WHERE ticker='BBCA'")
        .execute(pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM jejak_agent WHERE ticker='BBCA'")
        .execute(pool)
        .await
        .unwrap();
}

async fn set_up_tetis(pool: &PgPool, nomor: &str) -> (i64, i64) {
    let pengguna_id = users::upsert_by_wa(pool, nomor).await.unwrap();
    let tesis_id = theses::insert_pending(pool, pengguna_id, "BBCA net_profit_margin naik", "BBCA")
        .await
        .unwrap();
    theses::add_indicator(pool, tesis_id, "net_profit_margin", "increase")
        .await
        .unwrap();
    theses::activate(pool, tesis_id).await.unwrap();
    (pengguna_id, tesis_id)
}

async fn seed_sebelumnya(pool: &PgPool) {
    snapshots::upsert_observed(
        pool,
        "BBCA",
        "net_profit_margin",
        "Q2 2024",
        43.12,
        SRC,
        Some("banks"),
        chrono::DateTime::parse_from_rfc3339("2026-07-02T10:00:00Z")
            .unwrap()
            .with_timezone(&chrono::Utc),
    )
    .await
    .unwrap();
}

#[tokio::test]
async fn pipeline_evidence_valid_lalu_duplikat() {
    let Some(pool) = pool().await else { return };
    let nomor = "6281111111111";
    bersihkan(&pool, nomor).await;
    set_up_tetis(&pool, nomor).await;
    seed_sebelumnya(&pool).await;

    let state = AppState::new(pool.clone(), mock_config());
    let pipeline = AgentPipeline::new(state);

    let r1 = pipeline.run_cycle().await;
    assert_eq!(r1.snapshots_stored, 1);
    assert_eq!(r1.findings_accepted, 1);
    assert_eq!(
        r1.alerts_sent, 1,
        "mock mode harus mock_sent, count alert terkirim: {r1:?}",
    );

    // Status tesis MENGUAT karena delta +8.65% > threshold 2% dan arah increase.
    let status: String = sqlx::query_scalar("SELECT status FROM tesis WHERE ticker='BBCA'")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(status, "MENGUAT");

    // Cycle kedua: Finding/periode/metric sama -> ditolak duplikat, tidak kirim ulang alert.
    let r2 = pipeline.run_cycle().await;
    assert_eq!(r2.findings_accepted, 0);
    assert_eq!(r2.findings_rejected, 1);
    assert_eq!(r2.alerts_sent, 0);
}

#[tokio::test]
async fn pipeline_baseline_tanpa_pembanding() {
    let Some(pool) = pool().await else { return };
    let nomor = "6282222222222";
    bersihkan(&pool, nomor).await;
    set_up_tetis(&pool, nomor).await;
    // Tidak seed snapshot sebelumnya: snapshot pertama jadi baseline.

    let state = AppState::new(pool.clone(), mock_config());
    let pipeline = AgentPipeline::new(state);
    let r = pipeline.run_cycle().await;
    assert_eq!(r.baselines_created, 1);
    assert_eq!(r.findings_accepted, 0);
    assert_eq!(r.alerts_sent, 0);
}

#[tokio::test]
async fn pipeline_mcp_410_tidak_ada_alert() {
    let Some(pool) = pool().await else { return };
    let nomor = "6283333333333";
    bersihkan(&pool, nomor).await;
    set_up_tetis(&pool, nomor).await;

    let mut state = AppState::new(pool.clone(), mock_config());
    state.mcp = std::sync::Arc::new(MockMcpGateway::deprecated());
    let pipeline = AgentPipeline::new(state);
    let r = pipeline.run_cycle().await;
    assert!(!r.errors.is_empty());
    assert_eq!(r.alerts_sent, 0);
    assert_eq!(r.findings_accepted, 0);
}

#[tokio::test]
async fn pipeline_muted_tidak_kirim_alert() {
    let Some(pool) = pool().await else { return };
    let nomor = "6284444444444";
    bersihkan(&pool, nomor).await;
    let (pengguna_id, _) = set_up_tetis(&pool, nomor).await;
    seed_sebelumnya(&pool).await;
    users::set_muted_until(
        &pool,
        pengguna_id,
        Some(chrono::Utc::now() + chrono::Duration::hours(1)),
    )
    .await
    .unwrap();

    let state = AppState::new(pool.clone(), mock_config());
    let pipeline = AgentPipeline::new(state);
    let r = pipeline.run_cycle().await;
    assert_eq!(r.findings_accepted, 1);
    assert_eq!(r.alerts_sent, 0);
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM alerts")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(count, 0);
}

#[tokio::test]
async fn pipeline_alert_keempat_ditunda_harian() {
    let Some(pool) = pool().await else { return };
    let nomor = "6285555555555";
    bersihkan(&pool, nomor).await;
    let (pengguna_id, tesis_id) = set_up_tetis(&pool, nomor).await;
    seed_sebelumnya(&pool).await;

    // Isi 3 alert terkirim hari ini agar kuota harian habis.
    sqlx::query("INSERT INTO findings(id, ticker, metric_name, period, status, payload) VALUES (900001,'BBCA','roe','Q1 2024','lolos','{}'::jsonb),(900002,'BBCA','roe','Q2 2024','lolos','{}'::jsonb),(900003,'BBCA','roe','FY 2024','lolos','{}'::jsonb) ON CONFLICT DO NOTHING")
        .execute(&pool).await.unwrap();
    for (i, f) in [900001_i64, 900002, 900003].iter().enumerate() {
        let id = alerts::insert(
            &pool,
            pengguna_id,
            tesis_id,
            *f,
            "roe",
            &format!("P{i}"),
            "h",
            "b",
        )
        .await
        .unwrap();
        alerts::mark_sent(&pool, id).await.unwrap();
    }

    let state = AppState::new(pool.clone(), mock_config());
    let pipeline = AgentPipeline::new(state);
    let r = pipeline.run_cycle().await;
    assert_eq!(r.findings_accepted, 1);
    assert_eq!(r.alerts_sent, 0);
    assert_eq!(r.alerts_deferred, 1);

    sqlx::query("DELETE FROM alerts WHERE pengguna_id=$1")
        .bind(pengguna_id)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM findings WHERE id >= 900001")
        .execute(&pool)
        .await
        .unwrap();
}

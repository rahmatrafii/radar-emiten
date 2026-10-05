//! Tes workflow tesis & command router — butuh DATABASE_URL.
use orchestrator::config::{AppMode, Config};
use orchestrator::services::command_router;
use orchestrator::state::AppState;

async fn test_state() -> Option<AppState> {
    let url = std::env::var("DATABASE_URL").ok()?;
    let pool = sqlx::PgPool::connect(&url).await.ok()?;
    sqlx::migrate!("./migrations").run(&pool).await.ok()?;
    Some(AppState::new(
        pool,
        Config {
            app_mode: AppMode::Mock,
            host: "127.0.0.1".into(),
            port: 0,
            database_url: url,
            gemini_api_key: None,
            gemini_model: None,
            whatsapp_access_token: None,
            whatsapp_phone_number_id: None,
            whatsapp_app_secret: None,
            whatsapp_verify_token: None,
            whatsapp_graph_api_version: None,
            whatsapp_template_name: None,
            whatsapp_template_language: None,
            admin_phone: Some("6281112223333".into()),
            internal_api_token: Some("t".into()),
            sectors_credit_budget: 1000.0,
            max_alerts_per_day: 3,
            neutral_relative_threshold: 0.02,
            scheduler_interval_seconds: 86400,
            mcp_transport: None,
            mcp_server_command: None,
            mcp_server_args: None,
            mcp_callback_base_url: None,
            policy_disclaimer_conflict_acknowledged: false,
        },
    ))
}

async fn cleanup(pool: &sqlx::PgPool) {
    sqlx::query(
        "DELETE FROM indikator_tesis WHERE tesis_id IN (SELECT id FROM tesis WHERE ticker='TEST')",
    )
    .execute(pool)
    .await
    .ok();
    sqlx::query("DELETE FROM tesis WHERE ticker='TEST'")
        .execute(pool)
        .await
        .ok();
    sqlx::query("DELETE FROM pengguna WHERE nomor_wa='6281112223333'")
        .execute(pool)
        .await
        .ok();
}

#[tokio::test]
async fn alur_tesis_pending_lalu_ya() {
    let Some(state) = test_state().await else {
        return;
    };
    cleanup(&state.pool).await;
    let uid = orchestrator::repositories::users::upsert_by_wa(&state.pool, "6281112223333")
        .await
        .unwrap();

    let reply = command_router::route(&state, uid, "6281112223333", "TEST net_profit_margin naik")
        .await
        .unwrap();
    assert!(reply.contains("Balas YA"), "{reply}");

    let pending = orchestrator::repositories::theses::latest_pending(&state.pool, uid)
        .await
        .unwrap();
    assert!(pending.is_some());

    let reply = command_router::route(&state, uid, "6281112223333", "YA")
        .await
        .unwrap();
    assert!(reply.contains("diaktifkan"), "{reply}");

    let active = orchestrator::repositories::theses::list_active(&state.pool, uid)
        .await
        .unwrap();
    assert_eq!(active.len(), 1);

    cleanup(&state.pool).await;
}

#[tokio::test]
async fn metric_asing_ditolak() {
    let Some(state) = test_state().await else {
        return;
    };
    cleanup(&state.pool).await;
    let uid = orchestrator::repositories::users::upsert_by_wa(&state.pool, "6281112223333")
        .await
        .unwrap();

    let reply = command_router::route(&state, uid, "6281112223333", "TEST lunar_magic naik")
        .await
        .unwrap();
    assert!(reply.contains("belum memahami"), "{reply}");
    let pending = orchestrator::repositories::theses::latest_pending(&state.pool, uid)
        .await
        .unwrap();
    assert!(pending.is_none());

    cleanup(&state.pool).await;
}

#[tokio::test]
async fn batas_tiga_tesis_aktif() {
    let Some(state) = test_state().await else {
        return;
    };
    cleanup(&state.pool).await;
    let uid = orchestrator::repositories::users::upsert_by_wa(&state.pool, "6281112223333")
        .await
        .unwrap();

    // 3 tesis aktif langsung di DB.
    for _ in 0..3 {
        let tid = orchestrator::repositories::theses::insert_pending(&state.pool, uid, "t", "TEST")
            .await
            .unwrap();
        orchestrator::repositories::theses::activate(&state.pool, tid)
            .await
            .unwrap();
    }
    // Tesis ke-4 menunggu konfirmasi.
    orchestrator::repositories::theses::insert_pending(&state.pool, uid, "keempat", "TEST")
        .await
        .unwrap();

    let reply = command_router::route(&state, uid, "6281112223333", "YA")
        .await
        .unwrap();
    assert!(reply.contains("Batas maksimal 3"), "{reply}");

    cleanup(&state.pool).await;
}

#[tokio::test]
async fn diam_lalu_lanjut() {
    let Some(state) = test_state().await else {
        return;
    };
    cleanup(&state.pool).await;
    let uid = orchestrator::repositories::users::upsert_by_wa(&state.pool, "6281112223333")
        .await
        .unwrap();

    let reply = command_router::route(&state, uid, "6281112223333", "/diam")
        .await
        .unwrap();
    assert!(reply.contains("dihentikan"), "{reply}");
    let muted: (Option<chrono::DateTime<chrono::Utc>>,) =
        sqlx::query_as("SELECT jeda_sampai FROM pengguna WHERE id=$1")
            .bind(uid)
            .fetch_one(&state.pool)
            .await
            .unwrap();
    assert!(muted.0.is_some());

    let reply = command_router::route(&state, uid, "6281112223333", "/lanjut")
        .await
        .unwrap();
    assert!(reply.contains("diaktifkan"), "{reply}");
    let muted: (Option<chrono::DateTime<chrono::Utc>>,) =
        sqlx::query_as("SELECT jeda_sampai FROM pengguna WHERE id=$1")
            .bind(uid)
            .fetch_one(&state.pool)
            .await
            .unwrap();
    assert!(muted.0.is_none());

    cleanup(&state.pool).await;
}

#[tokio::test]
async fn kredit_hanya_admin() {
    let Some(state) = test_state().await else {
        return;
    };
    cleanup(&state.pool).await;
    let uid = orchestrator::repositories::users::upsert_by_wa(&state.pool, "6281112223333")
        .await
        .unwrap();

    let reply = command_router::route(&state, uid, "6289990001111", "/kredit")
        .await
        .unwrap();
    assert!(reply.contains("hanya untuk admin"), "{reply}");

    let reply = command_router::route(&state, uid, "6281112223333", "/kredit")
        .await
        .unwrap();
    assert!(reply.contains("Meter kredit"), "{reply}");

    cleanup(&state.pool).await;
}

#[tokio::test]
async fn empat_metric_ditolak() {
    let Some(state) = test_state().await else {
        return;
    };
    cleanup(&state.pool).await;
    let uid = orchestrator::repositories::users::upsert_by_wa(&state.pool, "6281112223333")
        .await
        .unwrap();

    let reply = command_router::route(
        &state,
        uid,
        "6281112223333",
        "TEST roe roa eps net_income naik",
    )
    .await
    .unwrap();
    assert!(reply.contains("belum memahami"), "{reply}");
    assert!(reply.contains("maksimal 3"), "{reply}");

    cleanup(&state.pool).await;
}

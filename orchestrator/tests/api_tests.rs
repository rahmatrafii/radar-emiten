//! Test API Tahap 3 — membutuhkan PostgreSQL lokal (DATABASE_URL).
//! Tanpa DATABASE_URL test dilewati (skip) agar `cargo test` tetap hijau.
use axum::body::Body;
use axum::http::{Request, StatusCode};
use tower::ServiceExt;

use orchestrator::config::{AppMode, Config};
use orchestrator::routes;
use orchestrator::state::AppState;

async fn test_state() -> Option<AppState> {
    let url = std::env::var("DATABASE_URL").ok()?;
    let pool = sqlx::PgPool::connect(&url).await.ok()?;
    // Pastikan semua migrasi sudah jalan (idempotent).
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
            admin_phone: None,
            internal_api_token: Some("test-token".into()),
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

async fn call(app: axum::Router, req: Request<Body>) -> (StatusCode, String) {
    let res = app.oneshot(req).await.unwrap();
    let status = res.status();
    let bytes = axum::body::to_bytes(res.into_body(), 1 << 20)
        .await
        .unwrap();
    (status, String::from_utf8(bytes.to_vec()).unwrap())
}

fn json_req(method: &str, uri: &str, body: serde_json::Value) -> Request<Body> {
    Request::builder()
        .method(method)
        .uri(uri)
        .header("content-type", "application/json")
        .header("authorization", "Bearer test-token")
        .body(Body::from(body.to_string()))
        .unwrap()
}

async fn cleanup(pool: &sqlx::PgPool) {
    sqlx::query("DELETE FROM compliance_audit_logs WHERE finding_id IN (SELECT id FROM findings WHERE ticker='TEST')").execute(pool).await.ok();
    sqlx::query("DELETE FROM findings WHERE ticker='TEST'")
        .execute(pool)
        .await
        .ok();
    sqlx::query("DELETE FROM snapshot_data WHERE ticker='TEST'")
        .execute(pool)
        .await
        .ok();
    sqlx::query("DELETE FROM jejak_agent WHERE ticker='TEST'")
        .execute(pool)
        .await
        .ok();
    sqlx::query("DELETE FROM penggunaan_kredit WHERE endpoint='test-endpoint'")
        .execute(pool)
        .await
        .ok();
}

#[tokio::test]
async fn snapshot_roundtrip_via_api() {
    let Some(state) = test_state().await else {
        return;
    };
    cleanup(&state.pool).await;
    let app = routes::routes().with_state(state.clone());

    let (status, _) = call(
        app.clone(),
        json_req(
            "POST",
            "/snapshots",
            serde_json::json!({
                "ticker": "TEST", "subsector": "banks", "metric_name": "roe",
                "value": 12.5, "period": "Q1 2026", "source": "unit-test",
                "observed_at": "2026-04-01T00:00:00Z"
            }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let (status, body) = call(
        app.clone(),
        Request::builder()
            .uri("/snapshots?ticker=TEST&metric_name=roe")
            .body(Body::empty())
            .unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(
        body.contains("12.5"),
        "respons harus memuat nilai snapshot: {body}"
    );

    cleanup(&state.pool).await;
}

#[tokio::test]
async fn finding_valid_diterima_invalid_ditolak() {
    let Some(state) = test_state().await else {
        return;
    };
    cleanup(&state.pool).await;

    // Siapkan snapshot pembanding.
    orchestrator::repositories::snapshots::upsert_observed(
        &state.pool,
        "TEST",
        "roe",
        "Q1 2026",
        12.5,
        "unit-test",
        Some("banks"),
        chrono::Utc::now(),
    )
    .await
    .unwrap();
    orchestrator::repositories::snapshots::upsert_observed(
        &state.pool,
        "TEST",
        "roe",
        "Q4 2025",
        10.0,
        "unit-test",
        Some("banks"),
        chrono::Utc::now(),
    )
    .await
    .unwrap();

    let app = routes::routes().with_state(state.clone());
    let valid = serde_json::json!({
        "ticker": "TEST", "subsector": "banks", "metric_name": "roe",
        "current_value": 12.5, "previous_value": 10.0, "period": "Q1 2026",
        "source": "unit-test", "observed_at": "2026-04-01T00:00:00Z",
        "confidence_score": 0.9, "finding_summary": "ROE naik pada Q1 2026."
    });
    let (status, body) = call(app.clone(), json_req("POST", "/findings", valid)).await;
    assert_eq!(status, StatusCode::OK);
    assert!(body.contains("\"accepted\""), "{body}");

    // current_value tidak cocok dengan snapshot -> rejected + audit.
    let mismatch = serde_json::json!({
        "ticker": "TEST", "subsector": "banks", "metric_name": "roe",
        "current_value": 99.9, "previous_value": 10.0, "period": "Q1 2026",
        "source": "unit-test", "observed_at": "2026-04-01T00:00:00Z",
        "confidence_score": 0.9, "finding_summary": "ROE naik pada Q1 2026."
    });
    let (_, body) = call(app.clone(), json_req("POST", "/findings", mismatch)).await;
    assert!(body.contains("\"rejected\""), "{body}");

    // Field tambahan ditolak Serde.
    let extra = serde_json::json!({
        "ticker": "TEST", "subsector": "banks", "metric_name": "roe",
        "current_value": 12.5, "previous_value": 10.0, "period": "Q1 2026",
        "source": "unit-test", "observed_at": "2026-04-01T00:00:00Z",
        "confidence_score": 0.9, "finding_summary": "x", "thesis_id": 1
    });
    let (status, _) = call(app.clone(), json_req("POST", "/findings", extra)).await;
    assert!(
        status.is_client_error(),
        "field tambahan harus ditolak, dapat {status}"
    );

    cleanup(&state.pool).await;
}

#[tokio::test]
async fn internal_endpoint_menolak_tanpa_token() {
    let Some(state) = test_state().await else {
        return;
    };
    let app = routes::routes().with_state(state.clone());

    let req = Request::builder()
        .uri("/internal/snapshots/previous?ticker=TEST&metric_name=roe")
        .body(Body::empty())
        .unwrap();
    let (status, _) = call(app.clone(), req).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);

    let req = Request::builder()
        .uri("/internal/snapshots/previous?ticker=TEST&metric_name=roe")
        .header("authorization", "Bearer salah")
        .body(Body::empty())
        .unwrap();
    let (status, _) = call(app, req).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn jejak_dan_kredit_roundtrip() {
    let Some(state) = test_state().await else {
        return;
    };
    cleanup(&state.pool).await;
    let app = routes::routes().with_state(state.clone());

    let (status, _) = call(
        app.clone(),
        json_req("POST", "/jejak", serde_json::json!({"agent": "scout", "action": "evidence_fetched", "outcome": "ok", "ticker": "TEST"})),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let (status, body) = call(
        app.clone(),
        Request::builder()
            .uri("/api/jejak")
            .body(Body::empty())
            .unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(body.contains("evidence_fetched"), "{body}");

    let (status, _) = call(
        app.clone(),
        json_req(
            "POST",
            "/kredit",
            serde_json::json!({"endpoint": "test-endpoint", "credits": 5, "from_cache": true}),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let (status, body) = call(
        app,
        Request::builder()
            .uri("/api/kredit")
            .body(Body::empty())
            .unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(body.contains("\"budget_configured\""), "{body}");

    cleanup(&state.pool).await;
}

#[tokio::test]
async fn pantauan_hanya_tesis_aktif() {
    let Some(state) = test_state().await else {
        return;
    };
    sqlx::query(
        "DELETE FROM indikator_tesis WHERE tesis_id IN (SELECT id FROM tesis WHERE ticker='TEST')",
    )
    .execute(&state.pool)
    .await
    .ok();
    sqlx::query("DELETE FROM tesis WHERE ticker='TEST'")
        .execute(&state.pool)
        .await
        .ok();

    let uid: (i64,) =
        sqlx::query_as("INSERT INTO pengguna (nomor_wa) VALUES ('6289998887771') RETURNING id")
            .fetch_one(&state.pool)
            .await
            .unwrap();
    let tid: (i64,) = sqlx::query_as("INSERT INTO tesis (pengguna_id, teks_asli, ticker, aktif, status) VALUES ($1,'uji','TEST',true,'MENGUAT') RETURNING id").bind(uid.0).fetch_one(&state.pool).await.unwrap();
    sqlx::query("INSERT INTO indikator_tesis (tesis_id, metric_name, desired_direction) VALUES ($1,'roe','increase')").bind(tid.0).execute(&state.pool).await.unwrap();

    let app = routes::routes().with_state(state.clone());
    let (status, body) = call(
        app,
        Request::builder()
            .uri("/pantauan")
            .body(Body::empty())
            .unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(body.contains("roe"), "{body}");
    assert!(
        !body.contains("6289998887771"),
        "nomor mentah tidak boleh bocor: {body}"
    );

    sqlx::query("DELETE FROM indikator_tesis WHERE tesis_id=$1")
        .bind(tid.0)
        .execute(&state.pool)
        .await
        .ok();
    sqlx::query("DELETE FROM tesis WHERE id=$1")
        .bind(tid.0)
        .execute(&state.pool)
        .await
        .ok();
    sqlx::query("DELETE FROM pengguna WHERE id=$1")
        .bind(uid.0)
        .execute(&state.pool)
        .await
        .ok();
}

//! Tes webhook WhatsApp — butuh PostgreSQL lokal (DATABASE_URL).
use axum::body::Body;
use axum::http::{Request, StatusCode};
use hmac::{Hmac, Mac};
use tower::ServiceExt;

use orchestrator::config::{AppMode, Config};
use orchestrator::routes;
use orchestrator::state::AppState;

async fn test_state() -> Option<AppState> {
    let url = std::env::var("DATABASE_URL").ok()?;
    let pool = sqlx::PgPool::connect(&url).await.ok()?;
    sqlx::migrate!("./migrations").run(&pool).await.ok()?;
    Some(AppState::new(
        pool,
        Config {
            app_mode: AppMode::Mock,
            port: 0,
            database_url: url,
            whatsapp_app_secret: Some("test-secret".into()),
            whatsapp_verify_token: Some("test-verify".into()),
            internal_api_token: Some("test-token".into()),
            policy_disclaimer_conflict_acknowledged: false,
            ..Default::default()
        },
    ))
}

fn sign(secret: &str, body: &[u8]) -> String {
    let mut mac = <Hmac<sha2::Sha256> as Mac>::new_from_slice(secret.as_bytes()).unwrap();
    mac.update(body);
    format!("sha256={}", hex::encode(mac.finalize().into_bytes()))
}

async fn call(app: axum::Router, req: Request<Body>) -> (StatusCode, String) {
    let res = app.oneshot(req).await.unwrap();
    let status = res.status();
    let bytes = axum::body::to_bytes(res.into_body(), 1 << 20)
        .await
        .unwrap();
    (status, String::from_utf8(bytes.to_vec()).unwrap())
}

fn webhook_req(body: &str, signature: Option<&str>) -> Request<Body> {
    let mut b = Request::builder()
        .method("POST")
        .uri("/webhook/whatsapp")
        .header("content-type", "application/json");
    if let Some(sig) = signature {
        b = b.header("x-hub-signature-256", sig);
    }
    b.body(Body::from(body.to_string())).unwrap()
}

const TEXT_PAYLOAD: &str = r#"{
  "object": "whatsapp_business_account",
  "entry": [{"id": "1", "changes": [{"field": "messages", "value": {
    "messaging_product": "whatsapp",
    "metadata": {"phone_number_id": "123"},
    "messages": [{"id": "wamid.TESTWEB1", "from": "6281112223333", "timestamp": "1762000000", "type": "text", "text": {"body": "halo"}}]
  }}]}]
}"#;

#[tokio::test]
async fn challenge_token_benar_mengembalikan_challenge() {
    let Some(state) = test_state().await else {
        return;
    };
    let app = routes::routes().with_state(state);
    let (status, body) = call(
        app,
        Request::builder()
            .uri("/webhook/whatsapp?hub.mode=subscribe&hub.verify_token=test-verify&hub.challenge=abc123")
            .body(Body::empty())
            .unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, "abc123");
}

#[tokio::test]
async fn challenge_token_salah_ditolak() {
    let Some(state) = test_state().await else {
        return;
    };
    let app = routes::routes().with_state(state);
    let (status, _) = call(
        app,
        Request::builder()
            .uri("/webhook/whatsapp?hub.mode=subscribe&hub.verify_token=salah&hub.challenge=abc123")
            .body(Body::empty())
            .unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn pesan_text_valid_disimpan_sekali() {
    let Some(state) = test_state().await else {
        return;
    };
    sqlx::query("DELETE FROM pesan_masuk WHERE id_pesan_wa='wamid.TESTWEB1'")
        .execute(&state.pool)
        .await
        .ok();
    let app = routes::routes().with_state(state.clone());

    let sig = sign("test-secret", TEXT_PAYLOAD.as_bytes());
    let (status, body) = call(app.clone(), webhook_req(TEXT_PAYLOAD, Some(&sig))).await;
    assert_eq!(status, StatusCode::OK);
    assert!(body.contains("\"processed\": 1"), "{body}");

    let row: (String,) = sqlx::query_as(
        "SELECT processing_status FROM pesan_masuk WHERE id_pesan_wa='wamid.TESTWEB1'",
    )
    .fetch_one(&state.pool)
    .await
    .unwrap();
    assert_eq!(row.0, "processed");

    // replay: payload & signature sama, tidak boleh disimpan ulang
    let sig2 = sign("test-secret", TEXT_PAYLOAD.as_bytes());
    let (status, body) = call(app, webhook_req(TEXT_PAYLOAD, Some(&sig2))).await;
    assert_eq!(status, StatusCode::OK);
    assert!(body.contains("\"duplicates\": 1"), "{body}");

    let count: (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM pesan_masuk WHERE id_pesan_wa='wamid.TESTWEB1'")
            .fetch_one(&state.pool)
            .await
            .unwrap();
    assert_eq!(count.0, 1);

    sqlx::query("DELETE FROM pesan_masuk WHERE id_pesan_wa='wamid.TESTWEB1'")
        .execute(&state.pool)
        .await
        .ok();
    sqlx::query("DELETE FROM jejak_agent WHERE details->>'wamid'='wamid.TESTWEB1'")
        .execute(&state.pool)
        .await
        .ok();
    sqlx::query("DELETE FROM pengguna WHERE nomor_wa='6281112223333'")
        .execute(&state.pool)
        .await
        .ok();
}

#[tokio::test]
async fn signature_salah_tidak_menyimpan() {
    let Some(state) = test_state().await else {
        return;
    };
    sqlx::query("DELETE FROM pesan_masuk WHERE id_pesan_wa='wamid.TESTWEB1'")
        .execute(&state.pool)
        .await
        .ok();
    let app = routes::routes().with_state(state.clone());

    let (status, _) = call(app, webhook_req(TEXT_PAYLOAD, Some("sha256=deadbeef"))).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);

    let count: (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM pesan_masuk WHERE id_pesan_wa='wamid.TESTWEB1'")
            .fetch_one(&state.pool)
            .await
            .unwrap();
    assert_eq!(count.0, 0);
}

#[tokio::test]
async fn status_update_tidak_panic() {
    let Some(state) = test_state().await else {
        return;
    };
    let app = routes::routes().with_state(state);
    let payload = r#"{"entry":[{"changes":[{"value":{"statuses":[{"id":"wamid.x","status":"delivered"}]}}]}]}"#;
    let sig = sign("test-secret", payload.as_bytes());
    let (status, body) = call(app, webhook_req(payload, Some(&sig))).await;
    assert_eq!(status, StatusCode::OK);
    assert!(body.contains("\"processed\": 0"), "{body}");
}

#[tokio::test]
async fn pesan_media_ditandai_ignored() {
    let Some(state) = test_state().await else {
        return;
    };
    sqlx::query("DELETE FROM pesan_masuk WHERE id_pesan_wa='wamid.TESTIMG'")
        .execute(&state.pool)
        .await
        .ok();
    let app = routes::routes().with_state(state.clone());
    let payload = r#"{"entry":[{"changes":[{"value":{"messages":[{"id":"wamid.TESTIMG","from":"6281112223333","type":"image"}]}}]}]}"#;
    let sig = sign("test-secret", payload.as_bytes());
    let (status, body) = call(app, webhook_req(payload, Some(&sig))).await;
    assert_eq!(status, StatusCode::OK);
    assert!(body.contains("\"ignored\": 1"), "{body}");

    let row: (String,) = sqlx::query_as(
        "SELECT processing_status FROM pesan_masuk WHERE id_pesan_wa='wamid.TESTIMG'",
    )
    .fetch_one(&state.pool)
    .await
    .unwrap();
    assert_eq!(row.0, "ignored");

    sqlx::query("DELETE FROM pesan_masuk WHERE id_pesan_wa='wamid.TESTIMG'")
        .execute(&state.pool)
        .await
        .ok();
    sqlx::query("DELETE FROM jejak_agent WHERE details->>'wamid'='wamid.TESTIMG'")
        .execute(&state.pool)
        .await
        .ok();
    sqlx::query("DELETE FROM pengguna WHERE nomor_wa='6281112223333'")
        .execute(&state.pool)
        .await
        .ok();
}

//! Tes klien WhatsApp (mock mode). Tidak memerlukan DB maupun kredensial.
use orchestrator::config::{AppMode, Config};
use orchestrator::models::whatsapp::{DISCLAIMER_RESMI, WhatsAppAlertPayload};
use orchestrator::services::whatsapp_client::WhatsAppClient;

fn cfg(mode: AppMode, ack_conflict: bool) -> Config {
    Config {
        app_mode: mode,
        port: 0,
        database_url: "postgres://unused".into(),
        internal_api_token: Some("t".into()),
        policy_disclaimer_conflict_acknowledged: ack_conflict,
        ..Default::default()
    }
}

fn payload() -> WhatsAppAlertPayload {
    WhatsAppAlertPayload {
        recipient_number: "6281234567890".into(),
        header: "BBCA · Net Profit Margin".into(),
        body: "Nilai terbaru: 46.85.".into(),
        disclaimer: DISCLAIMER_RESMI.into(),
    }
}

#[tokio::test]
async fn mock_send_alert_ditandai_mock_sent() {
    let client = WhatsAppClient::new(&cfg(AppMode::Mock, true));
    let outcome = client.send_alert(&payload()).await.unwrap();
    assert_eq!(outcome.status, "mock_sent");
    assert!(outcome.message_id.unwrap().starts_with("mock-"));
    assert_eq!(client.mock_records().len(), 1);
}

#[tokio::test]
async fn konflik_disclaimer_memblokir_sebelum_kirim() {
    let client = WhatsAppClient::new(&cfg(AppMode::Mock, false));
    let outcome = client.send_alert(&payload()).await.unwrap();
    assert_eq!(outcome.status, "blocked_by_policy_conflict");
    assert_eq!(
        outcome.detail.as_deref(),
        Some("DISCLAIMER_POLICY_CONFLICT")
    );
    assert!(
        client.mock_records().is_empty(),
        "tidak boleh ada request terkirim saat diblokir"
    );
}

#[tokio::test]
async fn payload_invalid_ditolak() {
    let client = WhatsAppClient::new(&cfg(AppMode::Mock, true));
    let mut p = payload();
    p.disclaimer = "Disclaimer karangan.".into();
    let err = client.send_alert(&p).await.unwrap_err();
    assert!(err.to_string().contains("disclaimer"));
}

#[tokio::test]
async fn real_mode_tanpa_token_gagal_terang() {
    let client = WhatsAppClient::new(&cfg(AppMode::Real, true));
    let err = client.send_alert(&payload()).await.unwrap_err();
    assert!(err.to_string().contains("WHATSAPP_ACCESS_TOKEN"));
}

#[tokio::test]
async fn real_mode_tetap_diblokir_konflik_disclaimer() {
    let client = WhatsAppClient::new(&cfg(AppMode::Real, false));
    let outcome = client.send_alert(&payload()).await.unwrap();
    assert_eq!(outcome.status, "blocked_by_policy_conflict");
}

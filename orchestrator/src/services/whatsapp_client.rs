//! Klien WhatsApp Cloud API.
//! - `APP_MODE=mock`: `MockWhatsAppClient` mencatat request, hasil `mock_sent`.
//! - `APP_MODE=real`: HTTP nyata ke Graph API, cek status HTTP + message_id.
//! Akses token tidak pernah dicatat ke log/respons.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use serde_json::{Value, json};

use crate::config::{AppMode, Config};
use crate::errors::AppError;
use crate::models::whatsapp::{SendOutcome, WhatsAppAlertPayload};

#[derive(Debug, Clone)]
pub struct MockRecord {
    pub kind: String,
    pub recipient: String,
    pub preview: String,
}

#[derive(Clone)]
pub struct WhatsAppClient {
    config: Config,
    http: reqwest::Client,
    mock_sent: Arc<Mutex<Vec<MockRecord>>>,
    mock_counter: Arc<Mutex<u64>>,
}

impl WhatsAppClient {
    pub fn new(config: &Config) -> Self {
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(15))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());
        Self {
            config: config.clone(),
            http,
            mock_sent: Arc::new(Mutex::new(Vec::new())),
            mock_counter: Arc::new(Mutex::new(0)),
        }
    }

    /// Catatan request mock (untuk tes & debugging mock mode).
    pub fn mock_records(&self) -> Vec<MockRecord> {
        self.mock_sent.lock().unwrap().clone()
    }

    /// Teks alert digabung dari header + body + disclaimer (angka dari template Rust, bukan LLM).
    fn render_alert_text(payload: &WhatsAppAlertPayload) -> String {
        format!(
            "{}\n\n{}\n\n{}",
            payload.header, payload.body, payload.disclaimer
        )
    }

    /// Kirim pesan text (jendela layanan 24 jam).
    pub async fn send_text(&self, recipient: &str, text: &str) -> Result<SendOutcome, AppError> {
        self.dispatch("text", recipient, json!({ "messaging_product": "whatsapp", "to": recipient, "type": "text", "text": { "body": text } }), text)
            .await
    }

    /// Kirim template yang sudah disetujui Meta.
    pub async fn send_template(
        &self,
        recipient: &str,
        template_name: &str,
        language: &str,
        parameters: Vec<String>,
    ) -> Result<SendOutcome, AppError> {
        let body = json!({
            "messaging_product": "whatsapp",
            "to": recipient,
            "type": "template",
            "template": {
                "name": template_name,
                "language": { "code": language },
                "components": [{
                    "type": "body",
                    "parameters": parameters.iter().map(|t| json!({ "type": "text", "text": t })).collect::<Vec<_>>(),
                }],
            }
        });
        self.dispatch("template", recipient, body, template_name)
            .await
    }

    /// Validasi payload internal lalu kirim. Hard gate konflik disclaimer.
    pub async fn send_alert(
        &self,
        payload: &WhatsAppAlertPayload,
    ) -> Result<SendOutcome, AppError> {
        payload.validate().map_err(AppError::BadRequest)?;
        if !self.config.policy_disclaimer_conflict_acknowledged {
            return Ok(SendOutcome {
                status: "blocked_by_policy_conflict".into(),
                message_id: None,
                detail: Some("DISCLAIMER_POLICY_CONFLICT".into()),
            });
        }
        let text = Self::render_alert_text(payload);
        self.dispatch("alert", &payload.recipient_number, json!({ "messaging_product": "whatsapp", "to": payload.recipient_number, "type": "text", "text": { "body": text } }), &text)
            .await
    }

    async fn dispatch(
        &self,
        kind: &str,
        recipient: &str,
        body: Value,
        preview: &str,
    ) -> Result<SendOutcome, AppError> {
        match self.config.app_mode {
            AppMode::Mock => {
                let mut n = self.mock_counter.lock().unwrap();
                *n += 1;
                let id = format!("mock-{n}");
                drop(n);
                self.mock_sent.lock().unwrap().push(MockRecord {
                    kind: kind.into(),
                    recipient: recipient.into(),
                    preview: preview.chars().take(80).collect(),
                });
                tracing::info!("mock whatsapp {} -> {} ({})", kind, recipient, id);
                Ok(SendOutcome {
                    status: "mock_sent".into(),
                    message_id: Some(id),
                    detail: None,
                })
            }
            AppMode::Real => self.dispatch_real(body).await,
        }
    }

    async fn dispatch_real(&self, body: Value) -> Result<SendOutcome, AppError> {
        let token = self
            .config
            .whatsapp_access_token
            .as_deref()
            .ok_or_else(|| AppError::BadRequest("WHATSAPP_ACCESS_TOKEN belum diset".into()))?;
        let phone_id = self
            .config
            .whatsapp_phone_number_id
            .as_deref()
            .ok_or_else(|| AppError::BadRequest("WHATSAPP_PHONE_NUMBER_ID belum diset".into()))?;
        let version = self
            .config
            .whatsapp_graph_api_version
            .as_deref()
            .ok_or_else(|| AppError::BadRequest("WHATSAPP_GRAPH_API_VERSION belum diset".into()))?;

        let url = format!("https://graph.facebook.com/{version}/{phone_id}/messages");
        let res = self
            .http
            .post(&url)
            .bearer_auth(token)
            .json(&body)
            .send()
            .await
            .map_err(|e| AppError::External(format!("gagal menghubungi Graph API: {e}")))?;

        let status = res.status();
        let json: Value = res.json().await.unwrap_or(Value::Null);
        if !status.is_success() {
            let msg = json
                .pointer("/error/message")
                .and_then(|v| v.as_str())
                .unwrap_or("respons error dari Meta");
            return Err(AppError::External(format!(
                "WhatsApp error {status}: {msg}"
            )));
        }
        let message_id = json
            .pointer("/messages/0/id")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        match message_id {
            Some(id) => Ok(SendOutcome {
                status: "sent".into(),
                message_id: Some(id),
                detail: None,
            }),
            None => Err(AppError::External("respons sukses tanpa message id".into())),
        }
    }
}

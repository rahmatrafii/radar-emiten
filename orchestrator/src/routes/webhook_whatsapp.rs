use axum::{
    Json,
    body::Bytes,
    extract::{Query, State},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
};
use serde_json::{Value, json};

use crate::auth;
use crate::errors::AppError;
use crate::models::webhook::{self, Incoming, WebhookPayload};
use crate::repositories::{inbound, traces, users};
use crate::state::AppState;

/// GET /webhook/whatsapp — verifikasi `hub.challenge` dari Meta.
pub async fn verify_webhook(
    State(state): State<AppState>,
    Query(params): Query<std::collections::HashMap<String, String>>,
) -> impl IntoResponse {
    let mode = params.get("hub.mode").map(|s| s.as_str()).unwrap_or("");
    let token = params
        .get("hub.verify_token")
        .map(|s| s.as_str())
        .unwrap_or("");
    let challenge = params.get("hub.challenge").cloned().unwrap_or_default();

    let Some(expected) = state.config.whatsapp_verify_token.as_deref() else {
        tracing::error!("WHATSAPP_VERIFY_TOKEN belum dikonfigurasi");
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    };

    if mode == "subscribe" && token == expected && !challenge.is_empty() {
        (StatusCode::OK, challenge).into_response()
    } else {
        StatusCode::FORBIDDEN.into_response()
    }
}

/// POST /webhook/whatsapp — raw body, signature HMAC, dedupe wamid, simpan pesan.
pub async fn receive_webhook(
    headers: HeaderMap,
    State(state): State<AppState>,
    body: Bytes,
) -> Result<Json<Value>, AppError> {
    let Some(secret) = state.config.whatsapp_app_secret.as_deref() else {
        tracing::error!("WHATSAPP_APP_SECRET belum dikonfigurasi");
        return Err(AppError::External("webhook belum dikonfigurasi".into()));
    };

    let signature = headers
        .get("x-hub-signature-256")
        .and_then(|v| v.to_str().ok());
    if !auth::verify_meta_signature(&body, signature, secret) {
        return Err(AppError::Unauthorized);
    }

    let payload: WebhookPayload = serde_json::from_slice(&body)
        .map_err(|_| AppError::BadRequest("payload bukan JSON webhook yang valid".into()))?;

    let mut processed = 0u32;
    let mut ignored = 0u32;
    let mut duplicates = 0u32;

    for item in webhook::extract_incoming(&payload) {
        match item {
            Incoming::Text { wamid, from, body } => {
                let nomor = from
                    .chars()
                    .filter(|c| c.is_ascii_digit())
                    .collect::<String>();
                let user_id = users::upsert_by_wa(&state.pool, &nomor).await?;
                let is_new =
                    inbound::insert_if_new(&state.pool, &wamid, Some(user_id), &nomor, Some(&body))
                        .await?;
                if !is_new {
                    duplicates += 1;
                    continue; // replay Meta: balas 200, tanpa side effect kedua
                }
                inbound::set_status(&state.pool, &wamid, "processing").await?;
                // Command router: tesis & perintah; balasan dikirim via klien WhatsApp.
                match crate::services::command_router::route(&state, user_id, &nomor, &body).await {
                    Ok(reply) => {
                        match state.whatsapp.send_text(&nomor, &reply).await {
                            Ok(outcome) => {
                                tracing::info!(
                                    "balasan ke {}: {} ({})",
                                    nomor,
                                    outcome.status,
                                    wamid
                                );
                            }
                            Err(e) => tracing::error!("gagal mengirim balasan: {e}"),
                        }
                        inbound::set_status(&state.pool, &wamid, "processed").await?;
                    }
                    Err(e) => {
                        tracing::error!("command router error: {e}");
                        inbound::set_status(&state.pool, &wamid, "failed").await?;
                    }
                }
                traces::trace(
                    &state.pool,
                    "scout",
                    "message_received",
                    Some("processed"),
                    None,
                    Some(json!({ "wamid": wamid })),
                )
                .await?;
                processed += 1;
            }
            Incoming::Unsupported { wamid, from, kind } => {
                let nomor = from
                    .chars()
                    .filter(|c| c.is_ascii_digit())
                    .collect::<String>();
                users::upsert_by_wa(&state.pool, &nomor).await.ok();
                let is_new =
                    inbound::insert_if_new(&state.pool, &wamid, None, &nomor, None).await?;
                if is_new {
                    inbound::set_status(&state.pool, &wamid, "ignored").await?;
                    traces::trace(
                        &state.pool,
                        "scout",
                        "message_ignored",
                        Some("unsupported_type"),
                        None,
                        Some(json!({ "wamid": wamid, "kind": kind })),
                    )
                    .await?;
                }
                ignored += 1;
            }
        }
    }

    Ok(Json(json!({
        "status": "ok",
        "processed": processed,
        "ignored": ignored,
        "duplicates": duplicates,
    })))
}

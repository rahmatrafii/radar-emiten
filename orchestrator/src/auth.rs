use axum::http::{HeaderMap, HeaderValue};

use crate::config::Config;

/// Samakan dua string secara constant-time (hindari timing leak untuk token).
fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    a.iter()
        .zip(b.iter())
        .fold(0u8, |acc, (x, y)| acc | (x ^ y))
        == 0
}

/// Verifikasi header `X-Hub-Signature-256` Meta: `sha256=<hex HMAC-SHA256(raw_body, app_secret)>`.
pub fn verify_meta_signature(raw_body: &[u8], header: Option<&str>, app_secret: &str) -> bool {
    use hmac::{Hmac, Mac};
    use sha2::Sha256;

    let Some(header) = header else { return false };
    let Some(hex_sig) = header.strip_prefix("sha256=") else {
        return false;
    };
    let Ok(expected) = hex::decode(hex_sig) else {
        return false;
    };
    let Ok(mut mac) = <Hmac<Sha256> as Mac>::new_from_slice(app_secret.as_bytes()) else {
        return false;
    };
    mac.update(raw_body);
    let computed = mac.finalize().into_bytes();
    constant_time_eq(&expected, &computed)
}

pub fn require_internal_token(headers: &HeaderMap, config: &Config) -> bool {
    let Some(expected) = config.internal_api_token.as_deref() else {
        return false; // fail closed bila token belum dikonfigurasi
    };
    if expected.is_empty() {
        return false;
    }
    headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|v: &HeaderValue| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .map(|token| constant_time_eq(token.as_bytes(), expected.as_bytes()))
        .unwrap_or(false)
}

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

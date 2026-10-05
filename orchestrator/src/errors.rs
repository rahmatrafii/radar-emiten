use axum::{Json, http::StatusCode, response::IntoResponse};
use serde_json::json;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("database error")]
    Database(#[from] sqlx::Error),
    #[error("bad request: {0}")]
    BadRequest(String),
    #[error("not found")]
    NotFound,
    #[error("unauthorized")]
    Unauthorized,
    #[error("external service error: {0}")]
    External(String),
}

impl IntoResponse for AppError {
    fn into_response(self) -> axum::response::Response {
        let (status, message) = match &self {
            AppError::Database(_) => (StatusCode::INTERNAL_SERVER_ERROR, "database error"),
            AppError::BadRequest(m) => (StatusCode::BAD_REQUEST, m.as_str()),
            AppError::NotFound => (StatusCode::NOT_FOUND, "not found"),
            AppError::Unauthorized => (StatusCode::UNAUTHORIZED, "unauthorized"),
            AppError::External(m) => (StatusCode::BAD_GATEWAY, m.as_str()),
        };
        // Tidak pernah membocorkan detail internal/stack trace ke client.
        (status, Json(json!({ "error": message }))).into_response()
    }
}

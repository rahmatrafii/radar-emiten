// src/sectors/error.rs — error types terstruktur untuk Sectors API client

use thiserror::Error;

/// Error yang bisa terjadi saat berinteraksi dengan Sectors API v2.
#[derive(Debug, Error)]
pub enum SectorsError {
    /// API key tidak ditemukan di environment variable `SECTORS_API_KEY`.
    #[error("SECTORS_API_KEY tidak ditemukan di environment. Setel variabel tersebut terlebih dahulu.")]
    MissingApiKey,

    /// Terjadi error jaringan / transport saat memanggil API.
    #[error("Reqwest HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    /// API mengembalikan status error (non-2xx).
    #[error("Sectors API mengembalikan HTTP {status}: {body}")]
    ApiError { status: u16, body: String },

    /// Endpoint v1 sudah dimatikan (HTTP 410 Gone).
    #[error("Endpoint v1 sudah dimatikan (HTTP 410). Gunakan endpoint v2.")]
    EndpointGone,

    /// Gagal meng-deserialize JSON respons.
    #[error("Gagal parse JSON respons: {0}")]
    Json(#[from] serde_json::Error),

    /// Field yang diminta tidak tersedia dalam respons.
    #[error("Field '{field}' tidak ditemukan dalam respons untuk ticker '{ticker}'")]
    FieldNotFound { ticker: String, field: String },

    /// Error lain yang tidak terkategori.
    #[error("Error tidak terduga: {0}")]
    Other(#[from] anyhow::Error),
}

/// Alias Result agar mudah dipakai di seluruh crate.
pub type Result<T> = std::result::Result<T, SectorsError>;

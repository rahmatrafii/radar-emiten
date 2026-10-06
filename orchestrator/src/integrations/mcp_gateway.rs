//! Gateway MCP untuk mengakses tools Sectors API v2 milik Representative.
//!
//! Rafi HANYA menjadi client/gateway; implementasi MCP server (`rmcp`), Sectors
//! HTTP client, dan pemilihan transport (stdio/SSE/HTTP) adalah milik
//! Representative. Karena transport belum disepakati (lihat
//! `docs/integration-decisions.md`), adapter real saat ini adalah stub jujur
//! yang mengembalikan `McpError::TransportUnavailable`, dan `MockMcpGateway`
//! dipakai untuk mengetes pipeline.
//!
//! Enam tools yang dikenali sesuai `ARCHITECTURE.md`:
//! 1. `list_subsectors`
//! 2. `screen_companies`
//! 3. `get_subsector_report`
//! 4. `get_company_evidence`
//! 5. `get_previous_snapshot`
//! 6. `record_finding`

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use chrono::{DateTime, Utc};

use crate::config::{AppMode, Config};

/// Future berkotak agar trait object-safe (`Arc<dyn McpGateway>`).
pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// Error terstruktur dari gateway MCP. Tidak pernah memuat secret.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum McpError {
    #[error("transport MCP belum disepakati/dikonfigurasi: {0}")]
    TransportUnavailable(String),
    #[error("tool MCP tidak dikenal: {0}")]
    ToolNotFound(String),
    #[error("Sectors API deprecated (HTTP 410); jangan fallback ke v1")]
    ApiVersionDeprecated,
    #[error("respons MCP tidak valid: {0}")]
    InvalidResponse(String),
    #[error("timeout saat memanggil MCP")]
    Timeout,
}

/// Satu observasi evidence mentah dari sumber (Sectors/MCP). `raw` menjaga
/// respons asli yang belum kita verifikasi agar tidak menebak field.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CompanyEvidence {
    pub ticker: String,
    pub subsector: String,
    pub metric_name: String,
    pub period: String,
    pub value: Option<f64>,
    pub source: String,
    pub observed_at: DateTime<Utc>,
    #[serde(default)]
    pub raw: serde_json::Value,
}

/// Parameter penyaringan emiten. Kosong = tanpa filter.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct ScreenParams {
    #[serde(default)]
    pub subsector: Option<String>,
}

/// Hasil `screen_companies`: daftar ticker kandidat.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ScreenResult {
    pub tickers: Vec<String>,
}

/// Laporan agregat subsektor. Field rincian dipertahankan sebagai `raw`
/// sampai schema Sectors diverifikasi tim.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SubsectorReport {
    pub subsector: String,
    #[serde(default)]
    pub raw: serde_json::Value,
}

/// Kontrak adapter MCP. Implementasi boleh mock (tes) atau real (nanti).
pub trait McpGateway: Send + Sync {
    fn list_subsectors(&self) -> BoxFuture<'_, Result<Vec<String>, McpError>>;
    fn screen_companies(
        &self,
        params: ScreenParams,
    ) -> BoxFuture<'_, Result<ScreenResult, McpError>>;
    fn get_subsector_report(
        &self,
        subsector: String,
    ) -> BoxFuture<'_, Result<SubsectorReport, McpError>>;
    fn get_company_evidence(
        &self,
        ticker: String,
        period: Option<String>,
    ) -> BoxFuture<'_, Result<CompanyEvidence, McpError>>;
    /// Snapshot pembanding historis; sebaiknya dibaca dari DB backend via
    /// endpoint internal, bukan dibuat ulang dari Sectors.
    fn get_previous_snapshot(
        &self,
        ticker: String,
        metric_name: String,
    ) -> BoxFuture<'_, Result<Option<CompanyEvidence>, McpError>>;
    /// Registrasi Finding yang sudah lolos validasi Axum.
    fn record_finding(&self, finding: serde_json::Value)
    -> BoxFuture<'_, Result<String, McpError>>;
}

/// Pilih implementasi gateway berdasarkan `APP_MODE` / konfigurasi.
pub fn from_config(config: &Config) -> Arc<dyn McpGateway> {
    match config.app_mode {
        AppMode::Mock => Arc::new(MockMcpGateway::new()),
        AppMode::Real => Arc::new(RealMcpGateway::new(config)),
    }
}

// ---------------------------------------------------------------------------
// MockMcpGateway — fixture deterministik untuk tes & demo mock.
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MockBehavior {
    Ok,
    /// Simulasikan Sectors API deprecated (HTTP 410).
    Deprecated,
    /// Simulasikan transport tidak tersedia.
    Unavailable,
}

#[derive(Clone)]
pub struct MockMcpGateway {
    behavior: MockBehavior,
}

impl Default for MockMcpGateway {
    fn default() -> Self {
        Self::new()
    }
}

impl MockMcpGateway {
    pub fn new() -> Self {
        Self {
            behavior: MockBehavior::Ok,
        }
    }

    /// Gateway yang selalu gagal dengan `ApiVersionDeprecated` (tes proteksi 410).
    pub fn deprecated() -> Self {
        Self {
            behavior: MockBehavior::Deprecated,
        }
    }

    /// Gateway yang selalu gagal transport (tes error path).
    pub fn unavailable() -> Self {
        Self {
            behavior: MockBehavior::Unavailable,
        }
    }

    fn guard(&self) -> Result<(), McpError> {
        match self.behavior {
            MockBehavior::Ok => Ok(()),
            MockBehavior::Deprecated => Err(McpError::ApiVersionDeprecated),
            MockBehavior::Unavailable => {
                Err(McpError::TransportUnavailable("mock transport down".into()))
            }
        }
    }

    /// Observasi pembanding (periode lama) untuk cold-start pipeline.
    fn previous_fixture(ticker: &str, metric_name: &str) -> Option<CompanyEvidence> {
        if ticker != "BBCA" || metric_name != "net_profit_margin" {
            return None;
        }
        Some(CompanyEvidence {
            ticker: "BBCA".into(),
            subsector: "banks".into(),
            metric_name: "net_profit_margin".into(),
            period: "Q2 2024".into(),
            value: Some(43.12),
            source: "Sectors API v2 /v2/company/report/BBCA".into(),
            observed_at: DateTime::parse_from_rfc3339("2026-07-02T10:00:00Z")
                .unwrap()
                .with_timezone(&Utc),
            raw: serde_json::json!({"fixture": true}),
        })
    }
}

impl McpGateway for MockMcpGateway {
    fn list_subsectors(&self) -> BoxFuture<'_, Result<Vec<String>, McpError>> {
        Box::pin(async move {
            self.guard()?;
            Ok(vec!["banks".into(), "telecommunication".into()])
        })
    }

    fn screen_companies(
        &self,
        params: ScreenParams,
    ) -> BoxFuture<'_, Result<ScreenResult, McpError>> {
        Box::pin(async move {
            self.guard()?;
            let tickers = match params.subsector.as_deref() {
                Some("banks") | None => vec!["BBCA".into(), "BBRI".into()],
                Some("telecommunication") => vec!["TLKM".into()],
                Some(other) => {
                    return Err(McpError::InvalidResponse(format!(
                        "subsektor tidak dikenal di fixture: {other}"
                    )));
                }
            };
            Ok(ScreenResult { tickers })
        })
    }

    fn get_subsector_report(
        &self,
        subsector: String,
    ) -> BoxFuture<'_, Result<SubsectorReport, McpError>> {
        Box::pin(async move {
            self.guard()?;
            Ok(SubsectorReport {
                subsector,
                raw: serde_json::json!({"fixture": true}),
            })
        })
    }

    fn get_company_evidence(
        &self,
        ticker: String,
        period: Option<String>,
    ) -> BoxFuture<'_, Result<CompanyEvidence, McpError>> {
        Box::pin(async move {
            self.guard()?;
            if ticker != "BBCA" {
                return Err(McpError::ToolNotFound(format!(
                    "evidence fixture hanya tersedia untuk BBCA, diminta {ticker}"
                )));
            }
            Ok(CompanyEvidence {
                ticker: "BBCA".into(),
                subsector: "banks".into(),
                metric_name: "net_profit_margin".into(),
                period: period.unwrap_or_else(|| "Q3 2024".into()),
                value: Some(46.85),
                source: "Sectors API v2 /v2/company/report/BBCA".into(),
                observed_at: DateTime::parse_from_rfc3339("2026-10-02T10:15:30Z")
                    .unwrap()
                    .with_timezone(&Utc),
                raw: serde_json::json!({"fixture": true}),
            })
        })
    }

    fn get_previous_snapshot(
        &self,
        ticker: String,
        metric_name: String,
    ) -> BoxFuture<'_, Result<Option<CompanyEvidence>, McpError>> {
        Box::pin(async move {
            self.guard()?;
            Ok(Self::previous_fixture(&ticker, &metric_name))
        })
    }

    fn record_finding(
        &self,
        finding: serde_json::Value,
    ) -> BoxFuture<'_, Result<String, McpError>> {
        Box::pin(async move {
            self.guard()?;
            if finding.get("ticker").is_none() {
                return Err(McpError::InvalidResponse(
                    "finding mock wajib memiliki field ticker".into(),
                ));
            }
            Ok("mock-finding-id".into())
        })
    }
}

// ---------------------------------------------------------------------------
// RealMcpGateway — stub jujur sampai transport Representative disepakati.
// ---------------------------------------------------------------------------

pub struct RealMcpGateway {
    config: Config,
}

impl RealMcpGateway {
    pub fn new(config: &Config) -> Self {
        Self {
            config: config.clone(),
        }
    }

    fn unavailable(&self) -> McpError {
        let transport = self
            .config
            .mcp_transport
            .as_deref()
            .unwrap_or("(belum diisi)");
        McpError::TransportUnavailable(format!(
            "MCP_TRANSPORT={transport}; transport/binary MCP Representative belum disepakati — adapter real belum terverifikasi"
        ))
    }
}

impl McpGateway for RealMcpGateway {
    fn list_subsectors(&self) -> BoxFuture<'_, Result<Vec<String>, McpError>> {
        Box::pin(async move { Err(self.unavailable()) })
    }
    fn screen_companies(
        &self,
        _params: ScreenParams,
    ) -> BoxFuture<'_, Result<ScreenResult, McpError>> {
        Box::pin(async move { Err(self.unavailable()) })
    }
    fn get_subsector_report(
        &self,
        _subsector: String,
    ) -> BoxFuture<'_, Result<SubsectorReport, McpError>> {
        Box::pin(async move { Err(self.unavailable()) })
    }
    fn get_company_evidence(
        &self,
        _ticker: String,
        _period: Option<String>,
    ) -> BoxFuture<'_, Result<CompanyEvidence, McpError>> {
        Box::pin(async move { Err(self.unavailable()) })
    }
    fn get_previous_snapshot(
        &self,
        _ticker: String,
        _metric_name: String,
    ) -> BoxFuture<'_, Result<Option<CompanyEvidence>, McpError>> {
        Box::pin(async move { Err(self.unavailable()) })
    }
    fn record_finding(
        &self,
        _finding: serde_json::Value,
    ) -> BoxFuture<'_, Result<String, McpError>> {
        Box::pin(async move { Err(self.unavailable()) })
    }
}

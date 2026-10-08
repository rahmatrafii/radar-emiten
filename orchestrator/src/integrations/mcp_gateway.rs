//! Gateway MCP untuk mengakses tools Sectors API v2 milik Rafi.
//!
//! Rafi HANYA menjadi client/gateway; implementasi MCP server (`rmcp`), Sectors
//! HTTP client, dan pemilihan transport (stdio/SSE/HTTP) adalah milik
//! Rafi. Karena transport belum disepakati (lihat
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
use tokio::io::{AsyncBufReadExt, AsyncWriteExt};

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
// RealMcpGateway — implementasi stdio: spawn subprocess mcp-server,
// komunikasi via JSON-RPC MCP protocol di stdin/stdout.
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

    /// Spawn binary mcp-server sebagai subprocess dan kembalikan client-nya.
    async fn client(&self) -> Result<StdioMcpClient, McpError> {
        let command = self
            .config
            .mcp_server_command
            .as_deref()
            .unwrap_or("../target/debug/mcp-server");

        // Env var SECTORS_API_KEY harus diteruskan ke subprocess.
        let sectors_key = std::env::var("SECTORS_API_KEY").unwrap_or_default();

        StdioMcpClient::spawn(command, &sectors_key).await
    }
}

impl McpGateway for RealMcpGateway {
    fn list_subsectors(&self) -> BoxFuture<'_, Result<Vec<String>, McpError>> {
        Box::pin(async move {
            let mut client = self.client().await?;
            let result = client
                .call_tool("list_subsectors", serde_json::json!({}))
                .await?;
            // Respons: {"subsectors": ["banks", "telecommunication", ...]}
            let subsectors = result
                .pointer("/subsectors")
                .and_then(|v| v.as_array())
                .ok_or_else(|| McpError::InvalidResponse("list_subsectors: field 'subsectors' tidak ditemukan".into()))?
                .iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect();
            Ok(subsectors)
        })
    }

    fn screen_companies(
        &self,
        params: ScreenParams,
    ) -> BoxFuture<'_, Result<ScreenResult, McpError>> {
        Box::pin(async move {
            let mut client = self.client().await?;
            let args = serde_json::json!({
                "subsector": params.subsector,
                "limit": 20,
            });
            let result = client.call_tool("screen_companies", args).await?;
            // Respons: {"tickers": ["BBCA", "BBRI", ...]}
            let tickers = result
                .pointer("/tickers")
                .and_then(|v| v.as_array())
                .ok_or_else(|| McpError::InvalidResponse("screen_companies: field 'tickers' tidak ditemukan".into()))?
                .iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect();
            Ok(ScreenResult { tickers })
        })
    }

    fn get_subsector_report(
        &self,
        subsector: String,
    ) -> BoxFuture<'_, Result<SubsectorReport, McpError>> {
        Box::pin(async move {
            let mut client = self.client().await?;
            let args = serde_json::json!({ "subsector": subsector.clone() });
            let result = client.call_tool("get_subsector_report", args).await?;
            Ok(SubsectorReport {
                subsector,
                raw: result,
            })
        })
    }

    fn get_company_evidence(
        &self,
        ticker: String,
        period: Option<String>,
    ) -> BoxFuture<'_, Result<CompanyEvidence, McpError>> {
        Box::pin(async move {
            let mut client = self.client().await?;
            // period → n_quarters: ambil 2 kuartal default, atau 1 bila period spesifik
            let n_quarters: u32 = if period.is_some() { 1 } else { 2 };
            let args = serde_json::json!({
                "ticker": ticker,
                "n_quarters": n_quarters,
            });
            let raw = client.call_tool("get_company_evidence", args).await?;

            // Respons: array record keuangan kuartalan.
            // Ambil record pertama (terbaru) dan petakan ke CompanyEvidence.
            let records = raw
                .as_array()
                .ok_or_else(|| McpError::InvalidResponse("get_company_evidence: expected array".into()))?;
            let first = records
                .first()
                .ok_or_else(|| McpError::InvalidResponse(format!("get_company_evidence: tidak ada data untuk {ticker}")))?;

            let period_str = first
                .pointer("/period")
                .or_else(|| first.pointer("/fiscal_quarter"))
                .or_else(|| first.pointer("/period_end"))
                .and_then(|v| v.as_str())
                .unwrap_or("unknown")
                .to_string();

            // Cari nilai metrik — coba revenue, net_income, dll dari record.
            // Default ke field pertama yang numerik.
            let (metric_name, value) = extract_primary_metric(first);

            let subsector = first
                .pointer("/sub_sector")
                .or_else(|| first.pointer("/subsector"))
                .and_then(|v| v.as_str())
                .unwrap_or("unknown")
                .to_string();

            Ok(CompanyEvidence {
                ticker: ticker.clone(),
                subsector,
                metric_name,
                period: period_str,
                value,
                source: format!("Sectors API v2 /v2/financials/quarterly/{ticker}/"),
                observed_at: Utc::now(),
                raw: raw.clone(),
            })
        })
    }

    fn get_previous_snapshot(
        &self,
        _ticker: String,
        _metric_name: String,
    ) -> BoxFuture<'_, Result<Option<CompanyEvidence>, McpError>> {
        // Tool 5 di mcp-server masih stub (butuh endpoint internal).
        // Kembalikan None agar pipeline tetap jalan dengan baseline mode.
        Box::pin(async move { Ok(None) })
    }

    fn record_finding(
        &self,
        finding: serde_json::Value,
    ) -> BoxFuture<'_, Result<String, McpError>> {
        Box::pin(async move {
            let mut client = self.client().await?;
            let args = serde_json::json!({ "finding": finding });
            let result = client.call_tool("record_finding", args).await?;
            let note = result
                .pointer("/note")
                .or_else(|| result.pointer("/status"))
                .and_then(|v| v.as_str())
                .unwrap_or("accepted")
                .to_string();
            Ok(note)
        })
    }
}

// ---------------------------------------------------------------------------
// StdioMcpClient — JSON-RPC MCP over stdin/stdout.
// ---------------------------------------------------------------------------

/// Klien MCP yang berkomunikasi dengan subprocess via stdin/stdout.
/// Setiap pemanggilan `call_tool` melakukan:
///   1. Initialize handshake (jika belum)
///   2. Kirim `tools/call` JSON-RPC request ke stdin
///   3. Baca satu baris JSON-RPC response dari stdout
///   4. Ekstrak `content[0].text` dan parse sebagai JSON
pub struct StdioMcpClient {
    child: tokio::process::Child,
    stdin: tokio::process::ChildStdin,
    stdout: tokio::io::BufReader<tokio::process::ChildStdout>,
    next_id: u64,
}

impl StdioMcpClient {
    /// Spawn binary MCP server dan lakukan initialize handshake.
    pub async fn spawn(command: &str, sectors_api_key: &str) -> Result<Self, McpError> {
        let mut child = tokio::process::Command::new(command)
            .env("SECTORS_API_KEY", sectors_api_key)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            // stderr ke inherit agar log mcp-server tampil di terminal (tidak ganggu stdout)
            .stderr(std::process::Stdio::inherit())
            .spawn()
            .map_err(|e| McpError::TransportUnavailable(format!(
                "gagal spawn mcp-server '{command}': {e}. Pastikan binary sudah di-build: cargo build --bin mcp-server"
            )))?;

        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| McpError::TransportUnavailable("tidak bisa ambil stdin subprocess".into()))?;
        let stdout_raw = child
            .stdout
            .take()
            .ok_or_else(|| McpError::TransportUnavailable("tidak bisa ambil stdout subprocess".into()))?;
        let stdout = tokio::io::BufReader::new(stdout_raw);

        let mut client = Self {
            child,
            stdin,
            stdout,
            next_id: 1,
        };

        // MCP initialize handshake.
        client.initialize().await?;

        Ok(client)
    }

    /// Kirim JSON-RPC request dan tunggu response dengan timeout 30 detik.
    async fn send_request(&mut self, method: &str, params: serde_json::Value) -> Result<serde_json::Value, McpError> {

        let id = self.next_id;
        self.next_id += 1;

        let request = serde_json::json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params,
        });
        let mut line = serde_json::to_string(&request)
            .map_err(|e| McpError::InvalidResponse(format!("serialisasi request gagal: {e}")))?;
        line.push('\n');

        self.stdin
            .write_all(line.as_bytes())
            .await
            .map_err(|e| McpError::TransportUnavailable(format!("tulis stdin gagal: {e}")))?;
        self.stdin
            .flush()
            .await
            .map_err(|e| McpError::TransportUnavailable(format!("flush stdin gagal: {e}")))?;

        // Baca baris response (dengan timeout).
        let mut response_line = String::new();
        let read_fut = self.stdout.read_line(&mut response_line);
        tokio::time::timeout(std::time::Duration::from_secs(30), read_fut)
            .await
            .map_err(|_| McpError::Timeout)?
            .map_err(|e| McpError::TransportUnavailable(format!("baca stdout gagal: {e}")))?;

        if response_line.is_empty() {
            return Err(McpError::TransportUnavailable("subprocess ditutup (EOF)".into()));
        }

        let response: serde_json::Value = serde_json::from_str(response_line.trim())
            .map_err(|e| McpError::InvalidResponse(format!("JSON-RPC response tidak valid: {e} — raw: {response_line}")))?;

        // Cek JSON-RPC error.
        if let Some(err) = response.get("error") {
            return Err(McpError::InvalidResponse(format!("JSON-RPC error: {err}")));
        }

        Ok(response["result"].clone())
    }

    /// MCP initialize handshake (wajib sebelum tools/call).
    async fn initialize(&mut self) -> Result<(), McpError> {
        let params = serde_json::json!({
            "protocolVersion": "2024-11-05",
            "capabilities": {},
            "clientInfo": {
                "name": "orchestrator",
                "version": "0.1.0"
            }
        });
        self.send_request("initialize", params).await?;

        // Kirim initialized notification (tidak punya response).
        let notif = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "notifications/initialized",
        });
        let mut line = serde_json::to_string(&notif).unwrap_or_default();
        line.push('\n');
        let _ = self.stdin.write_all(line.as_bytes()).await;
        let _ = self.stdin.flush().await;

        Ok(())
    }

    /// Panggil satu MCP tool dan kembalikan konten teks ter-parse sebagai JSON.
    pub async fn call_tool(&mut self, tool_name: &str, arguments: serde_json::Value) -> Result<serde_json::Value, McpError> {
        let params = serde_json::json!({
            "name": tool_name,
            "arguments": arguments,
        });
        let result = self.send_request("tools/call", params).await?;

        // MCP CallToolResult: {"content": [{"type": "text", "text": "..."}], "isError": false}
        let is_error = result.pointer("/isError").and_then(|v| v.as_bool()).unwrap_or(false);
        let text = result
            .pointer("/content/0/text")
            .and_then(|v| v.as_str())
            .ok_or_else(|| McpError::InvalidResponse(format!("{tool_name}: respons tanpa content[0].text")))?;

        if is_error {
            return Err(McpError::InvalidResponse(format!("{tool_name} error: {text}")));
        }

        // Parse text sebagai JSON (semua tool mcp-server mengembalikan JSON string).
        serde_json::from_str(text)
            .map_err(|e| McpError::InvalidResponse(format!("{tool_name}: content bukan JSON valid: {e} — raw: {text}")))
    }
}

impl Drop for StdioMcpClient {
    fn drop(&mut self) {
        // Pastikan subprocess dibersihkan saat client di-drop.
        let _ = self.child.start_kill();
    }
}

// ---------------------------------------------------------------------------
// Helper: ekstrak metric utama dari record keuangan kuartalan Sectors.
// ---------------------------------------------------------------------------

/// Ambil (metric_name, value) dari satu record JSON keuangan kuartalan.
/// Prioritas: revenue > net_income > gross_profit > operating_profit > earnings.
fn extract_primary_metric(record: &serde_json::Value) -> (String, Option<f64>) {
    let priority = [
        "revenue",
        "net_income",
        "gross_profit",
        "operating_profit",
        "earnings",
        "total_assets",
        "total_equity",
    ];
    for field in priority {
        if let Some(v) = record.get(field).and_then(|v| v.as_f64()) {
            return (field.to_string(), Some(v));
        }
    }
    // Fallback: ambil field numerik pertama yang ada.
    if let Some(obj) = record.as_object() {
        for (k, v) in obj {
            if let Some(f) = v.as_f64() {
                return (k.clone(), Some(f));
            }
        }
    }
    ("unknown".to_string(), None)
}

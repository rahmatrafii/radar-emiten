//! Biner MCP Server — Data Ingestion & MCP Layer (IDX Sentinel).
//!
//! Mengekspos 6 core tools via MCP (transport stdio) yang membungkus
//! `SectorsClient` (Sectors API v2 + cache + credit tracker).
//!
//! Endpoint mengikuti dokumentasi resmi (terverifikasi live 7 Okt 2026):
//! - `GET /v2/subsectors/` (1 kredit)
//! - `GET /v2/companies/` + `where` (1 kredit)
//! - `GET /v2/subsector/report/{sub}/` + `sections` (1 kredit/section)
//! - `GET /v2/financials/quarterly/{symbol}/` (1 kredit/quarter)
//!
//! Jalankan:
//! ```sh
//! SECTORS_API_KEY=... cargo run --bin mcp-server
//! ```

use std::sync::Arc;

use radar_emiten::SectorsClient;
use rmcp::{
    handler::server::wrapper::Parameters,
    model::{CallToolResult, ContentBlock},
    schemars, tool, tool_handler, tool_router, ErrorData as McpError, ServerHandler, ServiceExt,
};
use serde::Deserialize;

// ---------------------------------------------------------------------------
// Parameter struct untuk tiap tool
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct ScreenParams {
    /// Filter subsektor slug kebab-case (opsional), contoh: "banks".
    /// Kosong = semua emiten.
    #[serde(default)]
    subsector: Option<String>,
    /// Jumlah hasil (opsional, maks 50). Default: 20.
    #[serde(default)]
    limit: Option<u32>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct SubsectorParams {
    /// Slug subsektor kebab-case, contoh: "banks".
    subsector: String,
    /// Section laporan (opsional). Default: ["statistics"] (= 1 kredit).
    /// Valid: statistics, market_cap, stability, valuation, growth, companies.
    #[serde(default)]
    sections: Option<Vec<String>>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct EvidenceParams {
    /// Kode saham BEI, contoh: "BBCA" (suffix ".JK" opsional).
    ticker: String,
    /// Jumlah kuartal terbaru (opsional, maks 12). Default: 2.
    #[serde(default)]
    n_quarters: Option<u32>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct SnapshotParams {
    ticker: String,
    metric_name: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct RecordParams {
    /// Payload finding yang sudah lolos validasi Compliance Checker.
    finding: serde_json::Value,
}

// ---------------------------------------------------------------------------
// Server
// ---------------------------------------------------------------------------

#[derive(Clone)]
struct McpServer {
    client: Arc<SectorsClient>,
}

#[tool_router]
impl McpServer {
    /// Tool 1: daftar slug subsektor resmi dari `GET /v2/subsectors/`.
    #[tool(description = "Daftar slug subsektor BEI resmi (kebab-case)")]
    async fn list_subsectors(&self) -> Result<CallToolResult, McpError> {
        match self.client.fetch_subsectors().await {
            Ok(items) => {
                let slugs: Vec<String> = items.into_iter().map(|s| s.subsector).collect();
                Ok(CallToolResult::success(vec![ContentBlock::text(
                    serde_json::json!({ "subsectors": slugs }).to_string(),
                )]))
            }
            Err(e) => Ok(tool_err(&e)),
        }
    }

    /// Tool 2: skrining emiten via `GET /v2/companies/` + filter `where`.
    #[tool(description = "Skrining emiten kandidat dari Companies Screener, bisa difilter subsektor")]
    async fn screen_companies(
        &self,
        Parameters(params): Parameters<ScreenParams>,
    ) -> Result<CallToolResult, McpError> {
        let limit = params.limit.unwrap_or(20).clamp(1, 50);
        match self
            .client
            .screen_companies(params.subsector.as_deref(), limit)
            .await
        {
            Ok(rows) => {
                // Normalisasi ke ticker 4 huruf untuk kontrak internal.
                let tickers: Vec<String> = rows.iter().map(|r| r.ticker_short()).collect();
                Ok(CallToolResult::success(vec![ContentBlock::text(
                    serde_json::json!({ "tickers": tickers }).to_string(),
                )]))
            }
            Err(e) => Ok(tool_err(&e)),
        }
    }

    /// Tool 3: laporan agregat subsektor via `GET /v2/subsector/report/{sub}/`.
    #[tool(description = "Laporan agregat subsektor (default section statistics = 1 kredit)")]
    async fn get_subsector_report(
        &self,
        Parameters(params): Parameters<SubsectorParams>,
    ) -> Result<CallToolResult, McpError> {
        let sections: Vec<String> = params
            .sections
            .unwrap_or_else(|| vec!["statistics".to_string()]);
        let section_refs: Vec<&str> = sections.iter().map(|s| s.as_str()).collect();
        match self
            .client
            .fetch_subsector_report(&params.subsector, &section_refs)
            .await
        {
            Ok(report) => Ok(CallToolResult::success(vec![ContentBlock::text(
                report.to_string(),
            )])),
            Err(e) => Ok(tool_err(&e)),
        }
    }

    /// Tool 4: evidence mentah keuangan kuartalan via
    /// `GET /v2/financials/quarterly/{ticker}/`.
    #[tool(description = "Ambil keuangan kuartalan mentah N periode terakhir untuk satu ticker")]
    async fn get_company_evidence(
        &self,
        Parameters(params): Parameters<EvidenceParams>,
    ) -> Result<CallToolResult, McpError> {
        let n = params.n_quarters.unwrap_or(2);
        match self.client.fetch_quarterly_financials(&params.ticker, n).await {
            Ok(records) => Ok(CallToolResult::success(vec![ContentBlock::text(
                serde_json::to_string(&records).unwrap_or_default(),
            )])),
            Err(e) => Ok(tool_err(&e)),
        }
    }

    /// Tool 5: snapshot pembanding historis.
    #[tool(description = "Ambil snapshot metrik historis untuk perbandingan")]
    async fn get_previous_snapshot(
        &self,
        Parameters(params): Parameters<SnapshotParams>,
    ) -> Result<CallToolResult, McpError> {
        Ok(CallToolResult::error(vec![ContentBlock::text(format!(
            "Snapshot historis untuk {} / {} belum tersedia: perlu endpoint internal backend \
             (GET /internal/snapshots) — akan diintegrasikan di fase berikutnya.",
            params.ticker, params.metric_name
        ))]))
    }

    /// Tool 6: registrasi finding yang sudah lolos Compliance Checker.
    #[tool(description = "Registrasi finding terverifikasi; stub validasi struktur (persistensi via backend)")]
    async fn record_finding(
        &self,
        Parameters(params): Parameters<RecordParams>,
    ) -> Result<CallToolResult, McpError> {
        let ok = params.finding.get("ticker").is_some()
            && params.finding.get("metric_name").is_some()
            && params.finding.get("value").is_some();
        if ok {
            Ok(CallToolResult::success(vec![ContentBlock::text(
                serde_json::json!({ "status": "accepted", "note": "persistensi diarahkan ke endpoint internal backend" })
                    .to_string(),
            )]))
        } else {
            Ok(CallToolResult::error(vec![ContentBlock::text(
                "finding tidak valid: wajib punya field ticker, metric_name, value",
            )]))
        }
    }
}

#[tool_handler(
    name = "radar-emiten-mcp",
    version = "0.1.0",
    instructions = "MCP server IDX Sentinel: akses data Sectors API v2 untuk agent market research"
)]
impl ServerHandler for McpServer {}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Mapping error Sectors ke tool-level error (pesan terlihat oleh pemanggil).
fn tool_err(e: &radar_emiten::sectors::error::SectorsError) -> CallToolResult {
    CallToolResult::error(vec![ContentBlock::text(format!("{e}"))])
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "radar_emiten=info,warn".into()),
        )
        // Log ke stderr agar tidak mengganggu protokol MCP di stdout.
        .with_writer(std::io::stderr)
        .init();

    let client = SectorsClient::from_env().map_err(|e| {
        anyhow::anyhow!("Gagal inisialisasi SectorsClient (cek SECTORS_API_KEY): {e}")
    })?;

    let server = McpServer {
        client: Arc::new(client),
    };

    let service = server.serve(rmcp::transport::stdio()).await?;
    service.waiting().await?;
    Ok(())
}

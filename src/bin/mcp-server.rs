//! Biner MCP Server — Data Ingestion & MCP Layer (IDX Sentinel).
//!
//! Mengekspos 6 core tools via MCP (transport stdio) yang membungkus
//! `SectorsClient` (Sectors API v2 + cache + credit tracker).
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
    /// Filter subsektor (opsional), contoh: "Banks". Kosong = semua.
    #[serde(default)]
    subsector: Option<String>,
    /// Tanggal acuan laporan kuartalan, format YYYY-MM-DD. Default: 1 tahun lalu.
    #[serde(default)]
    since: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct SubsectorParams {
    /// Nama subsektor, contoh: "Banks".
    subsector: String,
    #[serde(default)]
    since: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct EvidenceParams {
    /// Kode saham BEI, contoh: "BBCA".
    ticker: String,
    /// Field keuangan yang diinginkan, contoh: ["net_interest_margin_q"].
    #[serde(default)]
    fields: Option<Vec<String>>,
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
    /// Tool 1: daftar subsektor unik yang muncul di laporan kuartalan terbaru.
    #[tool(description = "Daftar subsektor BEI yang muncul di laporan kuartalan terbaru")]
    async fn list_subsectors(&self) -> Result<CallToolResult, McpError> {
        let since = default_since();
        match self.client.fetch_quarterly_reports(&since).await {
            Ok(reports) => {
                let mut set = std::collections::BTreeSet::new();
                for r in &reports {
                    if let Some(v) = r.extra.get("sub_sector").and_then(|v| v.as_str()) {
                        set.insert(v.to_string());
                    } else if let Some(v) = r.extra.get("sector").and_then(|v| v.as_str()) {
                        set.insert(v.to_string());
                    }
                }
                Ok(CallToolResult::success(vec![ContentBlock::text(
                    serde_json::json!({ "subsectors": set.into_iter().collect::<Vec<_>>() })
                        .to_string(),
                )]))
            }
            Err(e) => Ok(tool_err(&e)),
        }
    }

    /// Tool 2: skrining emiten kandidat berdasarkan subsektor & periode.
    #[tool(description = "Skrining emiten kandidat dari laporan kuartalan, bisa difilter subsektor")]
    async fn screen_companies(
        &self,
        Parameters(params): Parameters<ScreenParams>,
    ) -> Result<CallToolResult, McpError> {
        let since = params.since.unwrap_or_else(default_since);
        match self.client.fetch_quarterly_reports(&since).await {
            Ok(mut reports) => {
                if let Some(sub) = &params.subsector {
                    reports.retain(|r| {
                        r.extra
                            .get("sub_sector")
                            .and_then(|v| v.as_str())
                            .map(|s| s.eq_ignore_ascii_case(sub))
                            .unwrap_or(false)
                    });
                }
                let tickers: Vec<String> = reports.into_iter().map(|r| r.symbol).collect();
                Ok(CallToolResult::success(vec![ContentBlock::text(
                    serde_json::json!({ "tickers": tickers }).to_string(),
                )]))
            }
            Err(e) => Ok(tool_err(&e)),
        }
    }

    /// Tool 3: laporan agregat kuartalan untuk satu subsektor.
    #[tool(description = "Ringkasan agregat (jumlah emiten, total pendapatan, laba bersih) per subsektor")]
    async fn get_subsector_report(
        &self,
        Parameters(params): Parameters<SubsectorParams>,
    ) -> Result<CallToolResult, McpError> {
        let since = params.since.unwrap_or_else(default_since);
        match self.client.fetch_quarterly_reports(&since).await {
            Ok(reports) => {
                let filtered: Vec<_> = reports
                    .into_iter()
                    .filter(|r| {
                        r.extra
                            .get("sub_sector")
                            .and_then(|v| v.as_str())
                            .map(|s| s.eq_ignore_ascii_case(&params.subsector))
                            .unwrap_or(false)
                    })
                    .collect();
                let total_revenue: f64 = filtered.iter().filter_map(|r| r.revenue).sum();
                let total_income: f64 = filtered.iter().filter_map(|r| r.net_income).sum();
                Ok(CallToolResult::success(vec![ContentBlock::text(
                    serde_json::json!({
                        "subsector": params.subsector,
                        "emiten_count": filtered.len(),
                        "total_revenue": total_revenue,
                        "total_net_income": total_income,
                    })
                    .to_string(),
                )]))
            }
            Err(e) => Ok(tool_err(&e)),
        }
    }

    /// Tool 4: evidence mentah metrik keuangan untuk satu ticker.
    #[tool(description = "Ambil metrik keuangan kuartalan mentah dari Companies Screener v2 untuk satu ticker")]
    async fn get_company_evidence(
        &self,
        Parameters(params): Parameters<EvidenceParams>,
    ) -> Result<CallToolResult, McpError> {
        let fields: Vec<String> = params.fields.unwrap_or_else(|| {
            vec![
                "net_interest_margin_q".into(),
                "pertumbuhan_laba_q".into(),
                "pertumbuhan_pendapatan_q".into(),
            ]
        });
        let field_refs: Vec<&str> = fields.iter().map(|s| s.as_str()).collect();
        match self
            .client
            .fetch_financial_metrics(&params.ticker, field_refs)
            .await
        {
            Ok(row) => Ok(CallToolResult::success(vec![ContentBlock::text(
                serde_json::to_string(&row).unwrap_or_default(),
            )])),
            Err(e) => Ok(tool_err(&e)),
        }
    }

    /// Tool 5: snapshot pembanding historis.
    #[tool(description = "Ambil snapshot metrik historis untuk perbandingan Q_{t-1}")]
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

/// Default: 365 hari ke belakang (format YYYY-MM-DD).
fn default_since() -> String {
    (chrono::Utc::now() - chrono::Duration::days(365))
        .format("%Y-%m-%d")
        .to_string()
}

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

// src/main.rs — Entry point demo IDX Sentinel

use radar_emiten::SectorsClient;
use tracing_subscriber::{fmt, EnvFilter};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Inisialisasi structured logging
    // Atur level dengan: RUST_LOG=radar_emiten=debug
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("radar_emiten=info,warn")),
        )
        .with_target(true)
        .with_line_number(false)
        .compact()
        .init();

    tracing::info!("=== IDX Sentinel — Sectors API v2 Demo ===");

    // Inisialisasi client (API key dibaca dari env SECTORS_API_KEY atau .env)
    let client = SectorsClient::from_env().map_err(|e| {
        eprintln!("\n❌  Error: {e}");
        eprintln!("💡  Setel environment variable: export SECTORS_API_KEY=<api_key_anda>\n");
        anyhow::anyhow!("{e}")
    })?;

    tracing::info!("✅  Client berhasil dibuat: {:?}", client);

    // -----------------------------------------------------------------------
    // 1. Ambil laporan kuartalan terbaru
    // -----------------------------------------------------------------------
    tracing::info!("--- [1] Fetch Quarterly Reports ---");
    let reports = client.fetch_quarterly_reports("2024-07-01").await?;
    tracing::info!("Ditemukan {} laporan kuartalan", reports.len());

    for report in reports.iter().take(3) {
        tracing::info!(
            ticker = %report.symbol,
            periode = %report.period,
            "  📋 Report"
        );
    }

    // -----------------------------------------------------------------------
    // 2. Ambil metrik keuangan untuk BBCA
    // -----------------------------------------------------------------------
    tracing::info!("--- [2] Fetch Financial Metrics (BBCA) ---");
    let fields = vec!["net_interest_margin_q", "pertumbuhan_laba_q"];

    match client.fetch_financial_metrics("BBCA", fields.clone()).await {
        Ok(row) => {
            tracing::info!(ticker = %row.symbol, "Metrics diterima");

            // Konversi ke Finding
            let findings = client.row_to_findings(&row, &fields, "2024-Q3");
            for f in &findings {
                println!(
                    "\n📊 Finding:\n  Ticker   : {}\n  Indikator: {}\n  Sebelum  : {:.4}\n  Sekarang : {:.4}\n  Delta    : {:.2}%\n  Periode  : {}\n  Sumber   : {}\n  Keyakinan: {}",
                    f.ticker,
                    f.indikator,
                    f.nilai_sebelum,
                    f.nilai_sekarang,
                    f.delta_pct(),
                    f.periode,
                    f.sumber,
                    f.tingkat_keyakinan,
                );
            }

            // Serialisasi ke JSON untuk tim lain
            let json = serde_json::to_string_pretty(&findings)?;
            println!("\n📦 JSON output:\n{json}");
        }
        Err(e) => {
            tracing::warn!("Gagal mengambil metrics BBCA: {e}");
        }
    }

    // -----------------------------------------------------------------------
    // Status akhir
    // -----------------------------------------------------------------------
    tracing::info!("{}", client.status());
    tracing::info!("=== Selesai ===");

    Ok(())
}

// src/main.rs — Entry point demo IDX Sentinel

use radar_emiten::SectorsClient;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize structured logging
    // Set level with: RUST_LOG=radar_emiten=debug
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

    // Initialize client (API key read from SECTORS_API_KEY env or .env)
    let client = SectorsClient::from_env().map_err(|e| {
        eprintln!("\n❌  Error: {e}");
        eprintln!("💡  Set environment variable: export SECTORS_API_KEY=<your_api_key>\n");
        anyhow::anyhow!("{e}")
    })?;

    tracing::info!("✅  Client created: {:?}", client);

    // -----------------------------------------------------------------------
    // 1. List official subsector slugs
    // -----------------------------------------------------------------------
    tracing::info!("--- [1] Fetch Subsectors ---");
    let subsectors = client.fetch_subsectors().await?;
    tracing::info!("Found {} subsectors", subsectors.len());

    for s in subsectors.iter().take(5) {
        tracing::info!(sector = %s.sector, subsector = %s.subsector, "  📋 Subsector");
    }

    // -----------------------------------------------------------------------
    // 2. Quarterly financials for BBCA (latest 2 quarters)
    // -----------------------------------------------------------------------
    tracing::info!("--- [2] Fetch Quarterly Financials (BBCA) ---");
    let records = client.fetch_quarterly_financials("BBCA", 2).await?;
    tracing::info!("Received {} quarterly records", records.len());

    // Convert to Findings (compare latest vs previous quarter)
    let fields = vec!["revenue", "earnings"];
    let source = "sectors_api_v2/financials/quarterly/BBCA";
    let findings = client.financials_to_findings(&records, &fields, source);
    for f in &findings {
        println!(
            "\n📊 Finding:\n  Ticker   : {}\n  Metric   : {}\n  Previous : {:.2}\n  Current  : {:.2}\n  Delta    : {:.2}%\n  Period   : {}\n  Source   : {}\n  Confidence: {}",
            f.ticker,
            f.metric_name,
            f.previous_value,
            f.current_value,
            f.delta_pct(),
            f.period,
            f.source,
            f.confidence_level,
        );
    }

    // Serialize to JSON for other teams
    let json = serde_json::to_string_pretty(&findings)?;
    println!("\n📦 JSON output:\n{json}");

    // -----------------------------------------------------------------------
    // Final status
    // -----------------------------------------------------------------------
    tracing::info!("{}", client.status());
    tracing::info!("=== Done ===");

    Ok(())
}

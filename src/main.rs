// src/main.rs — Integration Test Runner (End-to-End) IDX Sentinel
//
// Runner ini mengeksekusi skenario integrasi penuh:
//   1. Inisialisasi SectorsClient dari .env
//   2. Fetch data kuartalan live BBCA (net_interest_margin_q, pertumbuhan_laba_q)
//   3. Ambil baseline Q_{t-1} dari modul seed
//   4. Konversi ke Vec<Finding> (nilai_sebelum = seed, nilai_sekarang = API)
//   5. Cetak CreditTracker summary
//   6. Cetak Vec<Finding> sebagai JSON terstruktur ke stdout

use radar_emiten::{
    get_baseline_period, get_baseline_snapshot, Finding, SectorsClient,
    findings::TingkatKeyakinan,
};
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // -------------------------------------------------------------------------
    // Inisialisasi structured logging
    // Atur level dengan env: RUST_LOG=radar_emiten=debug
    // -------------------------------------------------------------------------
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("radar_emiten=info,warn")),
        )
        .with_target(true)
        .with_line_number(false)
        .compact()
        .init();

    tracing::info!("=== IDX Sentinel — Integration Test Runner (E2E) ===");
    tracing::info!("Baseline periode: {}", get_baseline_period());

    // -------------------------------------------------------------------------
    // a. Inisialisasi SectorsClient::from_env()
    //    Penanganan error ramah jika SECTORS_API_KEY tidak ditemukan
    // -------------------------------------------------------------------------
    let client = match SectorsClient::from_env() {
        Ok(c) => {
            tracing::info!("✅  SectorsClient berhasil diinisialisasi");
            c
        }
        Err(e) => {
            eprintln!("\n❌  Error inisialisasi client: {e}");
            eprintln!("💡  Solusi: Setel environment variable sebelum menjalankan:");
            eprintln!("     export SECTORS_API_KEY=<api_key_anda>");
            eprintln!("   atau buat file .env dengan isi:");
            eprintln!("     SECTORS_API_KEY=<api_key_anda>\n");
            // Keluar dengan kode non-zero tanpa panic
            std::process::exit(1);
        }
    };

    // -------------------------------------------------------------------------
    // b. Ambil data kuartalan live BBCA dari Sectors API v2
    //    Field yang diminta: net_interest_margin_q, pertumbuhan_laba_q
    // -------------------------------------------------------------------------
    let ticker = "BBCA";
    let fields = vec!["net_interest_margin_q", "pertumbuhan_laba_q"];

    tracing::info!("--- [1] Fetch Financial Metrics live ({ticker}) ---");

    let live_row = match client.fetch_financial_metrics(ticker, fields.clone()).await {
        Ok(row) => {
            tracing::info!(
                ticker = %row.symbol,
                "✅  Data live diterima dari Sectors API v2"
            );
            Some(row)
        }
        Err(e) => {
            tracing::warn!("⚠️   Gagal mengambil data live {ticker}: {e}");
            eprintln!("⚠️   Gagal mengambil data live untuk {ticker}: {e}");
            None
        }
    };

    // -------------------------------------------------------------------------
    // c. Ambil data baseline Q_{t-1} dari modul seed
    //    d. Konversi menjadi Vec<Finding>
    // -------------------------------------------------------------------------
    tracing::info!("--- [2] Bangun Vec<Finding> dari seed + live data ---");

    let mut findings: Vec<Finding> = Vec::new();

    if let Some(ref row) = live_row {
        for &field in &fields {
            // Nilai terkini dari Sectors API v2
            let nilai_sekarang = match row.get_f64(field) {
                Some(v) => v,
                None => {
                    tracing::warn!(
                        ticker = %ticker,
                        field = %field,
                        "Field tidak ditemukan di respons API, skip"
                    );
                    continue;
                }
            };

            // c. Nilai sebelumnya dari modul seed (baseline Q_{t-1})
            let nilai_sebelum = get_baseline_snapshot(ticker, field).unwrap_or_else(|| {
                tracing::warn!(
                    ticker = %ticker,
                    field = %field,
                    "Baseline seed tidak tersedia, gunakan 0.0"
                );
                0.0
            });

            // Tingkat keyakinan: Tinggi jika seed tersedia, Sedang jika fallback
            let tingkat_keyakinan = if get_baseline_snapshot(ticker, field).is_some() {
                TingkatKeyakinan::Tinggi
            } else {
                TingkatKeyakinan::Sedang
            };

            findings.push(Finding::new(
                ticker,
                field,
                nilai_sebelum,
                nilai_sekarang,
                "2026-Q2",  // periode terkini yang di-fetch
                "sectors_api_v2/screener",
                tingkat_keyakinan,
            ));
        }
    } else {
        // Jika API gagal, tetap hasilkan Finding dari seed saja sebagai demo
        tracing::info!("⚠️   API tidak tersedia — menggunakan data seed murni sebagai demo");
        for &field in &fields {
            if let Some(nilai_sebelum) = get_baseline_snapshot(ticker, field) {
                findings.push(Finding::new(
                    ticker,
                    field,
                    nilai_sebelum,
                    nilai_sebelum, // nilai_sekarang = nilai_sebelum (tidak ada data terkini)
                    get_baseline_period(),
                    "seed/demo",
                    TingkatKeyakinan::Rendah,
                ));
            }
        }
    }

    // -------------------------------------------------------------------------
    // e. Cetak status CreditTracker (sisa kredit dan total pemakaian)
    // -------------------------------------------------------------------------
    tracing::info!("--- [3] Status CreditTracker ---");
    let credit_summary = client.credits.summary();
    println!("\n💳  Credit Tracker: {credit_summary}");
    println!(
        "    Kredit dipakai : {}",
        client.credits.used()
    );
    println!(
        "    Kredit tersisa : {}",
        client.credits.remaining()
    );

    // -------------------------------------------------------------------------
    // f. Cetak output Vec<Finding> dalam format JSON terstruktur ke stdout
    // -------------------------------------------------------------------------
    tracing::info!("--- [4] Output Vec<Finding> sebagai JSON ---");

    let json = serde_json::to_string_pretty(&findings)?;
    println!("\n📦  Findings JSON Output:");
    println!("{json}");

    // Ringkasan singkat di log
    tracing::info!(
        jumlah_findings = findings.len(),
        "✅  Vec<Finding> berhasil dibuat"
    );
    for f in &findings {
        tracing::info!(
            ticker = %f.ticker,
            indikator = %f.indikator,
            nilai_sebelum = %f.nilai_sebelum,
            nilai_sekarang = %f.nilai_sekarang,
            delta_pct = format!("{:.2}%", f.delta_pct()),
            "  📊 Finding"
        );
    }

    tracing::info!("{}", client.status());
    tracing::info!("=== Integration Test Selesai ===");

    Ok(())
}

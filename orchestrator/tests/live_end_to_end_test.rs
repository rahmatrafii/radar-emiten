//! Test Nyata End-to-End ("Tes Asli")
//! Menguji seluruh siklus sistem dengan data riil:
//! 1. DeepSeek API: Ekstraksi kalimat tesis pengguna secara live
//! 2. PostgreSQL (Docker): Penyimpanan pengguna, tesis, dan aktivasi ("YA")
//! 3. Sectors API v2: Penarikan data laporan keuangan kuartalan riil BBCA via RealMcpGateway
//! 4. Agent Pipeline: Verifikasi 3 gate, evaluasi status tesis, dan pembentukan alert
//! 5. Audit Log & Jejak: Verifikasi compliance dan trace tersimpan di DB

use orchestrator::config::Config;
use orchestrator::orchestrator::agent_pipeline::AgentPipeline;
use orchestrator::repositories::{theses, traces, users};
use orchestrator::services::command_router;
use orchestrator::state::AppState;
use sqlx::PgPool;

const TEST_PHONE: &str = "628999888777";

#[tokio::test]
async fn tes_asli_end_to_end_radar_emiten() {
    dotenvy::dotenv().ok();

    let db_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://postgres:postgres@localhost:5434/idx_sentinel".into());
    let pool = match PgPool::connect(&db_url).await {
        Ok(p) => p,
        Err(e) => {
            eprintln!("Melewati tes asli: database PostgreSQL belum terhubung ({e})");
            return;
        }
    };

    // Jalankan migrasi skema
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("Migrasi database harus berhasil");

    let _sectors_key = match std::env::var("SECTORS_API_KEY") {
        Ok(k) if !k.is_empty() => k,
        _ => {
            eprintln!("Melewati tes asli: SECTORS_API_KEY belum diset di .env");
            return;
        }
    };

    let deepseek_key = match std::env::var("DEEPSEEK_API_KEY") {
        Ok(k) if !k.is_empty() => k,
        _ => {
            eprintln!("Melewati tes asli: DEEPSEEK_API_KEY belum diset di .env");
            return;
        }
    };

    println!("\n=======================================================");
    println!("MEMULAI TES ASLI END-TO-END RADAR EMITEN");
    println!("=======================================================");

    // Bersihkan data tes sebelumnya
    let _ = sqlx::query("DELETE FROM finding_tesis WHERE tesis_id IN (SELECT id FROM tesis WHERE pengguna_id IN (SELECT id FROM pengguna WHERE nomor_wa=$1))")
        .bind(TEST_PHONE)
        .execute(&pool)
        .await;
    let _ = sqlx::query("DELETE FROM alerts WHERE pengguna_id IN (SELECT id FROM pengguna WHERE nomor_wa=$1)")
        .bind(TEST_PHONE)
        .execute(&pool)
        .await;
    let _ = sqlx::query("DELETE FROM indikator_tesis WHERE tesis_id IN (SELECT id FROM tesis WHERE pengguna_id IN (SELECT id FROM pengguna WHERE nomor_wa=$1))")
        .bind(TEST_PHONE)
        .execute(&pool)
        .await;
    let _ = sqlx::query("DELETE FROM tesis WHERE pengguna_id IN (SELECT id FROM pengguna WHERE nomor_wa=$1)")
        .bind(TEST_PHONE)
        .execute(&pool)
        .await;
    let _ = sqlx::query("DELETE FROM pengguna WHERE nomor_wa=$1")
        .bind(TEST_PHONE)
        .execute(&pool)
        .await;

    // 1. Setup Config & AppState dari .env
    let mut config = Config::from_env().unwrap_or_else(|_| Config::default());
    config.database_url = db_url.clone();
    config.llm_provider = "deepseek".into();
    config.deepseek_api_key = Some(deepseek_key.clone());
    config.deepseek_model = Some("deepseek-chat".into());
    config.policy_disclaimer_conflict_acknowledged = true;

    let state = AppState::new(pool.clone(), config);

    // -------------------------------------------------------------------------
    // TAHAP 1: EKSTRAKSI KALIMAT TESIS VIA DEEPSEEK API SECARA LIVE
    // -------------------------------------------------------------------------
    println!("\n[1/5] Memanggil DeepSeek API secara live...");
    let user_sentence = "Saya amati bahwa BBCA net_profit_margin akan bertumbuh pada kuartal ini";
    let extracted = state
        .gemini
        .extract_thesis(user_sentence)
        .await
        .expect("DeepSeek harus berhasil mengekstrak kalimat tesis");

    println!("      -> Ticker diekstrak: {}", extracted.ticker);
    println!("      -> Metrik diekstrak: {} ({})", extracted.metrics[0].metric_name, extracted.metrics[0].desired_direction);
    assert_eq!(extracted.ticker, "BBCA");
    assert_eq!(extracted.metrics[0].metric_name, "net_profit_margin");

    // -------------------------------------------------------------------------
    // TAHAP 2: SIMULASI PENDAFTARAN & KONFIRMASI TESIS DI DATABASE
    // -------------------------------------------------------------------------
    println!("\n[2/5] Mendaftarkan tesis ke PostgreSQL...");
    let user_id = users::upsert_by_wa(&pool, TEST_PHONE).await.expect("Buat pengguna");

    // Simpan tesis dalam status PENDING_CONFIRMATION
    let pending_thesis_id = theses::insert_pending(
        &pool,
        user_id,
        user_sentence,
        &extracted.ticker,
    )
    .await
    .expect("Simpan pending thesis");

    for m in &extracted.metrics {
        theses::add_indicator(&pool, pending_thesis_id, &m.metric_name, &m.desired_direction)
            .await
            .expect("Tambah indikator");
    }
    println!("      -> Tesis dibuat (ID: {pending_thesis_id}, Status: PENDING_CONFIRMATION)");

    // Pengguna membalas "YA"
    let reply = command_router::route(&state, user_id, TEST_PHONE, "YA")
        .await
        .expect("Proses balasan konfirmasi");
    println!("      -> Respons balasan bot: \"{}\"", reply.lines().next().unwrap_or(""));
    assert!(reply.contains("diaktifkan"));

    let (active_status, is_active): (String, bool) = sqlx::query_as(
        "SELECT status, aktif FROM tesis WHERE id=$1",
    )
    .bind(pending_thesis_id)
    .fetch_one(&pool)
    .await
    .expect("Tesis harus ada");

    assert!(is_active);
    println!("      -> Status tesis terkonfirmasi: AKTIF (status: {active_status})");

    // -------------------------------------------------------------------------
    // TAHAP 3: AMBIL DATA FUNDAMENTAL LANGSUNG DARI SECTORS API V2
    // -------------------------------------------------------------------------
    println!("\n[3/5] Mengambil data laporan keuangan BBCA dari Sectors API v2...");
    let evidence_res = state
        .mcp
        .get_company_evidence("BBCA".to_string(), Some("net_profit_margin".into()), None)
        .await
        .expect("Sectors API v2 harus mengembalikan bukti riil");

    println!("      -> Bukti diterima: Subsektor = {}, Periode = {}", evidence_res.subsector, evidence_res.period);
    println!("      -> Nilai terbaru  : {:.2}%", evidence_res.value.unwrap_or(0.0));
    println!("      -> Sumber data    : {}", evidence_res.source);
    assert!(evidence_res.value.is_some(), "Nilai terbaru harus ada");

    // -------------------------------------------------------------------------
    // TAHAP 4: MENJALANKAN SIKLUS PIPELINE PEMANTAUAN (AGENT PIPELINE)
    // -------------------------------------------------------------------------
    println!("\n[4/5] Menjalankan siklus Agent Pipeline...");
    let pipeline = AgentPipeline::new(state.clone());
    let report = pipeline.run_cycle().await;

    println!("      -> Laporan siklus:");
    println!("         * Snapshots disimpan : {}", report.snapshots_stored);
    println!("         * Findings diterima  : {}", report.findings_accepted);
    println!("         * Findings ditolak   : {}", report.findings_rejected);
    println!("         * Alert terkirim/mock: {}", report.alerts_sent);
    println!("         * Errors             : {:?}", report.errors);

    assert_eq!(report.errors.len(), 0, "Pipeline tidak boleh menghasilkan error");
    assert!(report.snapshots_stored >= 1, "Snapshot harus tersimpan");
    assert!(report.alerts_sent >= 1, "Alert harus berhasil dibentuk/dikirim");

    // -------------------------------------------------------------------------
    // TAHAP 5: VERIFIKASI AKHIR DATABASE & STATUS TESIS
    // -------------------------------------------------------------------------
    println!("\n[5/5] Verifikasi hasil akhir di database...");
    
    // Status tesis terbarui
    let (updated_status, _): (String, bool) = sqlx::query_as(
        "SELECT status, aktif FROM tesis WHERE id=$1",
    )
    .bind(pending_thesis_id)
    .fetch_one(&pool)
    .await
    .expect("Ambil status tesis akhir");
    
    println!("      -> Status tesis akhir: {}", updated_status);
    assert_ne!(updated_status, "BELUM_CUKUP_DATA", "Status harus terbarui setelah observasi dihitung");

    // Cek alert tersimpan
    let alert_rows: Vec<(String, String, String)> = sqlx::query_as(
        "SELECT header, isi_pesan, status_kirim FROM alerts WHERE pengguna_id=$1 ORDER BY id DESC",
    )
    .bind(user_id)
    .fetch_all(&pool)
    .await
    .expect("Ambil alert");

    println!("      -> Jumlah alert tersimpan: {}", alert_rows.len());
    assert!(!alert_rows.is_empty());
    println!("      -> Header alert: {}", alert_rows[0].0);
    println!("      -> Status kirim: {}", alert_rows[0].2);
    println!("      -> Teks alert WhatsApp:");
    println!("-------------------------------------------------------");
    println!("{}", alert_rows[0].1);
    println!("-------------------------------------------------------");

    // Cek jejak audit agent
    let traces_list = traces::list_recent(&pool, 5, 0).await.expect("Ambil traces");
    println!("      -> Jejak audit agent (5 terbaru):");
    for t in traces_list {
        println!("         * [{}] {} -> {:?} (ticker: {:?})", t.agent, t.aksi, t.outcome, t.ticker);
    }

    println!("\n=======================================================");
    println!("TES ASLI END-TO-END SELESAI DENGAN SUKSES 100%!");
    println!("=======================================================\n");
}

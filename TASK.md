Saya adalah Data Lead (Dian) untuk proyek "IDX Sentinel". Tolong selesaikan sisa modul Tugas 1 (Data & MCP) dan jalankan git commit secara otomatis dalam satu langkah.

Lakukan tugas berikut secara berurutan:

1. Buat Modul Seed Data Q_{t-1} (src/sectors/seed.rs):
   - Buat struct SeedSnapshot yang menyimpan data baseline historis (kuartal 2026-Q1 / 2024-Q2) untuk emiten sampel demo: "BBCA" dan "BMRI".
   - Masukkan nilai indikator baseline realistis:
     * BBCA: net_interest_margin_q = 5.8, pertumbuhan_laba_q = 8.1
     * BMRI: net_interest_margin_q = 5.4, pertumbuhan_laba_q = 7.5
   - Sediakan fungsi publik:
     pub fn get_baseline_snapshot(ticker: &str, indicator: &str) -> Option<f64>
     pub fn get_baseline_period() -> &'static str (mengembalikan "2026-Q1")
   - Export modul ini di src/sectors/mod.rs dan re-export di src/lib.rs.

2. Update Script Integration Test (src/main.rs):
   - Perbarui src/main.rs (tokio::main async) sebagai runner tes integrasi end-to-end:
     a. Inisialisasi SectorsClient::from_env() dari file .env.
     b. Ambil data kuartalan live BBCA untuk "net_interest_margin_q" dan "pertumbuhan_laba_q".
     c. Ambil data baseline Q_{t-1} dari modul seed.
     d. Konversi menjadi Vec<Finding> dengan nilai_sebelum (dari seed) dan nilai_sekarang (dari Sectors API v2).
     e. Cetak status CreditTracker (sisa kredit dan total pemakaian).
     f. Cetak output Vec<Finding> dalam format JSON terstruktur (serde_json::to_string_pretty) ke stdout.
     g. Berikan penanganan error ramah jika SECTORS_API_KEY tidak ditemukan tanpa panic.

3. Verifikasi & Auto-Commit Git:
   - Jalankan `cargo check` di terminal untuk memastikan tidak ada error kompilasi.
   - Jika lulus, jalankan `git add .`
   - Buat git commit dengan perintah:
     git commit -m "feat(mcp): add Q_{t-1} seed baseline, main e2e runner, and integration test"

Kerjakan seluruh perintah di atas dan beritahu saya jika git commit sudah selesai dibuat.

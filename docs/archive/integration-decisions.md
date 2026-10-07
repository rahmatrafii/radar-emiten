# Integration Decisions & Conflict Log

Dokumen ini mencatat konflik spesifikasi, komponen yang belum tersedia, dan keputusan yang perlu dikonfirmasi tim. Pisahkan **fakta terkonfirmasi** dari hal yang **belum dipastikan**.

## Hasil Audit Tahap 0 (5 Oktober 2026)

- Branch `main`, working tree bersih sebelum audit.
- Baseline `cargo check`: **PASS** (exit 0).
- Komponen sudah ada: crate `orchestrator` (Axum minimal, `/health` + graceful shutdown, port 8080), 3 migrasi (skema awal, kontrak Finding, penyelarasan runbook — sudah diterapkan ke DB `sentinel`), `docker-compose.yml` + healthcheck, `.env` (lokal, tidak ter-commit), `.env.example`, README, 4 dokumen `docs/`.
- Komponen belum ada: MCP server/client (Rafiq), WhatsApp client & webhook, `AgentPipeline`, Gemini client, fixture E2E skenario A–E.
- Endpoint internal/dashboard (`/snapshots`, `/findings`, `/jejak`, `/kredit`, `/pantauan`, `/internal/*`, `/api/*`) **sudah ada** (Tahap 3, 5 Oktober 2026).
- Risiko: Gemini API key pernah terekspos — rotasi direkomendasikan.

## Konflik yang sudah diselesaikan

### 1. Disclaimer & kata terlarang — SELESAI (7 Oktober 2026)
- **Masalah:** Konstanta disclaimer di `CONTRACTS.md` mengandung frasa **"rekomendasi transaksi"** yang dilarang oleh aturan produk, sementara `PROMPTS.md` (Chief Presenter) memakai disclaimer berbeda.
- **Solusi:** Disclaimer resmi diubah ke versi yang lebih ketat (tidak mengandung kata "rekomendasi"):
  `Disclaimer: Informasi ini hanya bersifat edukasi & pemantauan data historis. TIDAK memberikan saran investasi (beli/jual/tahan) maupun eksekusi transaksi.`
- **File yang diubah:** `docs/CONTRACTS.md`, `orchestrator/src/models/whatsapp.rs`
- **Status:** Selesai. Tim dapat set `POLICY_DISCLAIMER_CONFLICT_ACKNOWLEDGED=true` untuk mengaktifkan pengiriman WhatsApp real.

## Komponen eksternal yang belum tersedia (belum terverifikasi)

| Komponen | Pemilik | Status |
|---|---|---|
| MCP server `rmcp` + transport (stdio/SSE/HTTP) | Rafiq | Belum ada di repo → buat `McpGateway` trait + `MockMcpGateway`; adapter real menyusul |
| Daftar metric final (whitelist) | Tim | Belum final → buat metric registry yang dapat dikonfigurasi; fixture hanya untuk tes |
| Prompt extractor tesis | Dian | `PROMPTS.md` ada (4 agent prompt), D-02 compliance completed (7 Okt 2026), D-03 evidence verifier completed (7 Okt 2026), D-04 fixtures completed (7 Okt 2026) |
| Gemini API key/model | Tim | Ada di `.env` lokal; model belum diisi |
| WhatsApp Cloud API credentials | Tim | Belum diisi → mock mode (hard gate disclaimer sudah diselesaikan) |
| Biaya Sectors per endpoint | Rafiq | Jangan dikarang; terima dari `POST /kredit` |

## WhatsApp Real — Status (7 Oktober 2026)

- **Hard gate disclaimer:** `POLICY_DISCLAIMER_CONFLICT_ACKNOWLEDGED=true` (konflik disclaimer sudah diselesaikan 7 Okt 2026).
- **Credentials:** Sudah ada di `.env` lokal (`WHATSAPP_ACCESS_TOKEN`, `WHATSAPP_PHONE_NUMBER_ID`, `WHATSAPP_VERIFY_TOKEN`, `WHATSAPP_GRAPH_API_VERSION=v25.0`). `WHATSAPP_APP_SECRET` kosong — hanya dibutuhkan untuk verifikasi signature webhook masuk, bukan untuk kirim pesan.
- **Blocker aktual:** `APP_MODE=mock` + `POLICY_DISCLAIMER_CONFLICT_ACKNOWLEDGED=false` di `.env` lokal. Untuk aktifkan real: set `APP_MODE=real` + `POLICY_DISCLAIMER_CONFLICT_ACKNOWLEDGED=true`.
- **Mode real:** `APP_MODE=real` + credentials terisi → `dispatch_real()` aktif (POST ke Graph API Meta, cek HTTP status + `message_id`).
- **Mode mock:** `APP_MODE=mock` → `mock_sent` (default, untuk development/testing).
- **Kode:** `orchestrator/src/services/whatsapp_client.rs` — `send_text()`, `send_template()`, `send_alert()` sudah mendukung mode real.
- **Hasil uji (7 Okt 2026):** webhook signature valid → `processed:1` + log `sent`; template `hello_world` → pesan masuk di HP. Balasan teks atas webhook simulasi mengembalikan `sent` tapi tidak sampai di HP (tidak ada jendela 24 jam di sisi Meta).
- **Dashboard (7 Okt 2026):** `dashboard/index.html` satu file (5 section F11 + jejak F12, auto-refresh 5 detik, disclaimer resmi) diserve Axum di `/dashboard` via `ServeDir` (`tower-http` feature `fs`). HTML statis bersih dari kata terlarang (satu-satunya kemunculan adalah disclaimer resmi).

## Keputusan teknis yang sudah diambil

- **Binary/crate:** satu crate `orchestrator/` sebagai backend Axum di port **8080** (bukan 3000).
- **Kepemilikan DB:** hanya Axum yang memegang pool PostgreSQL; MCP server harus mendelegasikan via endpoint internal.
- **Nama tabel:** migrasi lama memakai nama Indonesia (`pengguna`, `tesis`, ...); dipertahankan untuk tidak memutus migrasi, dipetakan ke konsep runbook (`users`, `theses`, dst).
- **Status tesis:** `PENDING_CONFIRMATION`, `MENGUAT`, `NETRAL`, `MELEMAH`, `BELUM_CUKUP_DATA`.
- **Dedupe alert:** `(pengguna_id, tesis_id, metric_name, period)` + dedupe pesan masuk via `wa_message_id`.
- **Mock vs real:** semua integrasi eksternal default ke mock sampai kredensial/transport terverifikasi.
- **Metric registry (Tahap 3):** belum final dari tim; default development di kode (`net_profit_margin`, `operating_margin`, `gross_margin`, `roe`, `roa`, `eps`, `revenue`, `net_income`, `der`, `current_ratio`), dapat ditimpa env `METRIC_REGISTRY` (koma-separated). Ganti saat tim memberikan daftar final.
- **`raw_data` snapshot:** DTO `POST /snapshots` menerima `raw_data` tetapi skema `snapshot_data` belum punya kolom tersebut; saat ini diterima lalu dibuang eksplisit. Migrasi terpisah diperlukan bila evidence mentah ingin disimpan.
- **Tahap 3 selesai:** route API + internal auth + Finding gate + dashboard JSON dengan nomor WA tersamarkan.
- **Tahap 6 selesai:** command router + Gemini extractor (mock deterministic) + tesis pending→YA→aktif + batas 3 tesis & 3 metric.
- **Prompt extractor tesis:** `docs/PROMPTS.md` (milik Dian) mencakup 4 agen (Scout/Analyst/Compliance/Chief Presenter) tetapi belum ada prompt khusus ekstraksi tesis. Schema JSON ekstraksi tesis didefinisikan Rafi (`src/services/gemini_client.rs`), divalidasi ketat di Rust, dan mengikuti gaya structured-output PROMPTS.md. Perlu konfirmasi Dian apakah prompt ini akan di-adopt.
- **Field bahasa Inggris:** `src/findings/mod.rs` telah diubah dari field bahasa Indonesia (`indikator`, `nilai_sebelum`, `nilai_sekarang`, `periode`, `sumber`, `tingkat_keyakinan`) ke bahasa Inggris (`metric_name`, `previous_value`, `current_value`, `period`, `source`, `confidence_level`) untuk konsistensi dengan `CONTRACTS.md` dan `orchestrator/models/finding.rs`.
- **Koreksi client Sectors (7 Okt 2026):** endpoint di `src/sectors/client.rs` + `ARCHITECTURE.md` sebelumnya salah (`companies/reports/quarterly`, `companies/screener/`, param `fields`, konvensi `_q`/`_prev`) — keduanya 404 di API live. Ditulis ulang mengikuti docs resmi + verifikasi live: `GET /v2/subsectors/`, `GET /v2/companies/` (tanpa `fields`), `GET /v2/companies/quarterly-financial-dates/`, `GET /v2/financials/quarterly/{symbol}/`, `GET /v2/subsector/report/{sub}/`. Simbol `.JK` dinormalisasi ke ticker 4 huruf. Demo live: BBCA revenue Q2 2026 +0.86% vs Q1. Catatan: `ARCHITECTURE.md` versi awal (`/v2/subsectors/`, `/v2/subsector/report/`) ternyata benar — koreksinya adalah mengembalikannya + menambah 3 endpoint lain.
- **Kredit pengujian 7 Okt 2026:** eksplorasi + verifikasi + demo ≈ 10–14 kredit dari 1000 (beberapa 404 baseline ikut tertagih 1 kredit sesuai billing resmi; demo `radar-emiten` terukur 3 kredit).
- **`/diam` durasi:** belum disepakati tim; saat ini mute sampai `/lanjut` (diset 2099-12-31).
- **Tahap 7 selesai (5 Oktober 2026):** `src/integrations/mcp_gateway.rs` — trait `McpGateway` (6 tools ARCHITECTURE.md), `MockMcpGateway` (fixture + mode 410/transport-down), `RealMcpGateway` stub jujur (`TransportUnavailable` sampai transport disepakati), factory `from_config`, `AppState.mcp`. 7 test baru PASS; format/fmt/check/test bersih. Transport MCP real tetap **blocked**: schema response Sectors belum diverifikasi Rafiq; DTO memakai `raw: serde_json::Value` untuk field mentah.
- **Tahap 8 selesai (5 Oktober 2026):** `src/orchestrator/agent_pipeline.rs` — `AgentPipeline::run_cycle()`, `PipelineRunReport`, snapshot→previous→delta deterministik Rust→`Finding` resmi→`accept_finding` (evidence gate + compliance audit)→link `finding_tesis`→status tesis→alert (dedupe unik, batas harian WIB, mute `/diam`)→`WhatsAppClient` (mock_sent) + jejak `jejak_agent`. Tambahan repository: `theses::watchlist_with_user`, `theses::link_finding`, `theses::support_counts`, `users::is_muted`. 5 test pipeline DB-backed PASS + 3 unit test arah dukungan.
  - Asumsi terdokumentasi: confidence 0.9 konservatif sampai tim punya model confidence; `NEUTRAL_RELATIVE_THRESHOLD=0.02` asumsi, bukan validasi data; arah `any` = netral sampai definisi tim.
  - Catatan: `findings` unique index (ticker, metric_name, period WHERE lolos) — untuk beberapa tesis pada ticker/metric yang sama, Finding hanya lolos sekali; alert per tesis tetap unik via `(pengguna_id, tesis_id, metric_name, period)`.

## Keamanan

- `.env` berisi Gemini API key — **sudah terekspos sebelumnya; rotasi key direkomendasikan**.
- `.env` sudah di-`.gitignore`; `.env.example` hanya berisi nama variabel.

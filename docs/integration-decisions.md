# Integration Decisions & Conflict Log

Dokumen ini mencatat konflik spesifikasi, komponen yang belum tersedia, dan keputusan yang perlu dikonfirmasi tim. Pisahkan **fakta terkonfirmasi** dari hal yang **belum dipastikan**.

## Hasil Audit Tahap 0 (5 Oktober 2026)

- Branch `main`, working tree bersih sebelum audit.
- Baseline `cargo check`: **PASS** (exit 0).
- Komponen sudah ada: crate `orchestrator` (Axum minimal, `/health` + graceful shutdown, port 8080), 3 migrasi (skema awal, kontrak Finding, penyelarasan runbook — sudah diterapkan ke DB `sentinel`), `docker-compose.yml` + healthcheck, `.env` (lokal, tidak ter-commit), `.env.example`, README, 4 dokumen `docs/`.
- Komponen belum ada: MCP server/client (Representative), WhatsApp client & webhook, `AgentPipeline`, Gemini client, fixture E2E skenario A–E.
- Endpoint internal/dashboard (`/snapshots`, `/findings`, `/jejak`, `/kredit`, `/pantauan`, `/internal/*`, `/api/*`) **sudah ada** (Tahap 3, 5 Oktober 2026).
- Risiko: Gemini API key pernah terekspos — rotasi direkomendasikan.

## Konflik yang belum terselesaikan

### 1. Disclaimer & kata terlarang — BLOCKER KEPATUHAN SEBELUM DEMO PUBLIK
- Konstanta disclaimer di `CONTRACTS.md` mengandung frasa **"rekomendasi transaksi"**:
  `Disclaimer: Informasi ini hanya bersifat edukasi & pemantauan data historis. Bukan anjuran investasi atau rekomendasi transaksi.`
- Sementara aturan produk melarang teks `rekomendasi` pada pesan, dan `PROMPTS.md` (Chief Presenter) memakai disclaimer berbeda:
  `Ringkasan ini hanya bersifat edukasi & pemantauan data historis. TIDAK memberikan saran investasi (beli/jual/tahan) maupun eksekusi transaksi.`
- **Status:** belum diputuskan tim. Selama `POLICY_DISCLAIMER_CONFLICT_ACKNOWLEDGED=false`, hard gate memblokir pengiriman WhatsApp real (audit reason `DISCLAIMER_POLICY_CONFLICT`, status bukan `sent`). Mock mode melaporkan `blocked_by_policy_conflict`.
- **Keputusan yang dibutuhkan:** tim memilih (a) memperbarui konstanta disclaimer + schema secara resmi, atau (b) mengesahkan pengecualian eksplisit untuk disclaimer.

## Komponen eksternal yang belum tersedia (belum terverifikasi)

| Komponen | Pemilik | Status |
|---|---|---|
| MCP server `rmcp` + transport (stdio/SSE/HTTP) | Representative | Belum ada di repo → buat `McpGateway` trait + `MockMcpGateway`; adapter real menyusul |
| Daftar metric final (whitelist) | Tim | Belum final → buat metric registry yang dapat dikonfigurasi; fixture hanya untuk tes |
| Prompt extractor tesis | Dian | `PROMPTS.md` ada (4 agent prompt), D-02 compliance in progress, D-03 pending |
| Gemini API key/model | Tim | Ada di `.env` lokal; model belum diisi |
| WhatsApp Cloud API credentials | Tim | Belum diisi → mock mode |
| Biaya Sectors per endpoint | Representative | Jangan dikarang; terima dari `POST /kredit` |

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
- **`/diam` durasi:** belum disepakati tim; saat ini mute sampai `/lanjut` (diset 2099-12-31).
- **Tahap 7 selesai (5 Oktober 2026):** `src/integrations/mcp_gateway.rs` — trait `McpGateway` (6 tools ARCHITECTURE.md), `MockMcpGateway` (fixture + mode 410/transport-down), `RealMcpGateway` stub jujur (`TransportUnavailable` sampai transport disepakati), factory `from_config`, `AppState.mcp`. 7 test baru PASS; format/fmt/check/test bersih. Transport MCP real tetap **blocked**: schema response Sectors belum diverifikasi Representative; DTO memakai `raw: serde_json::Value` untuk field mentah.
- **Tahap 8 selesai (5 Oktober 2026):** `src/orchestrator/agent_pipeline.rs` — `AgentPipeline::run_cycle()`, `PipelineRunReport`, snapshot→previous→delta deterministik Rust→`Finding` resmi→`accept_finding` (evidence gate + compliance audit)→link `finding_tesis`→status tesis→alert (dedupe unik, batas harian WIB, mute `/diam`)→`WhatsAppClient` (mock_sent) + jejak `jejak_agent`. Tambahan repository: `theses::watchlist_with_user`, `theses::link_finding`, `theses::support_counts`, `users::is_muted`. 5 test pipeline DB-backed PASS + 3 unit test arah dukungan.
  - Asumsi terdokumentasi: confidence 0.9 konservatif sampai tim punya model confidence; `NEUTRAL_RELATIVE_THRESHOLD=0.02` asumsi, bukan validasi data; arah `any` = netral sampai definisi tim.
  - Catatan: `findings` unique index (ticker, metric_name, period WHERE lolos) — untuk beberapa tesis pada ticker/metric yang sama, Finding hanya lolos sekali; alert per tesis tetap unik via `(pengguna_id, tesis_id, metric_name, period)`.

## Keamanan

- `.env` berisi Gemini API key — **sudah terekspos sebelumnya; rotasi key direkomendasikan**.
- `.env` sudah di-`.gitignore`; `.env.example` hanya berisi nama variabel.

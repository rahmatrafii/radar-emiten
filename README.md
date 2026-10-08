# Financial Market Research Agent 🚀

> [!CAUTION]
> **PENTING / DISCLAIMER:**
> Aplikasi ini hanya menyediakan analisis informasi dan pemantauan data historis. **TIDAK** memberikan saran investasi (rekomendasi beli/jual/tahan) maupun eksekusi transaksi otomatis.

---

## 📌 Ringkasan Proyek

**Financial Market Research Agent** adalah sistem intelijen pasar modal berbasis arsitektur *Agentic AI* dan *Model Context Protocol* (MCP) yang dirancang untuk memantau, mendeteksi anomali, menganalisis fundamental emiten, serta mendistribusikan ringkasan berbasis data secara objektif ke kanal komunikasi (WhatsApp).

### Arsitektur Singkat
```text
Sectors API v2 ──► MCP Rust Server (stdio) ──► Axum Backend + Gemini 2.0 Flash ──► PostgreSQL ──► WhatsApp Cloud API
```

1. **Sectors API v2**: Sumber data pasar modal Indonesia (emiten, sektor, rasio keuangan).
2. **MCP Rust Server (stdio)**: Lapisan tool abstraction berkinerja tinggi menggunakan Rust untuk menyediakan data real-time dan snapshot historis, berkomunikasi via transport stdio.
3. **Axum Backend + Gemini 2.0 Flash**: Orkestrator alur agentic multi-stage (Scout, Fundamental, Compliance, Presenter) dengan output terstruktur JSON.
4. **PostgreSQL**: Penyimpanan snapshot data, log verifikasi evidensi (*audit trail*), dan riwayat *findings*.
5. **WhatsApp Cloud API**: Pengiriman sinyal informasi dan ringkasan pasar terverifikasi kepada pengguna akhir.

---

## 👥 Tim Pengembang & Pembagian Tugas

| Nama | Peran | Tanggung Jawab Utama |
|---|---|---|
| **Dian** | Data & MCP Lead | Integrasi Sectors API v2, Modul `CreditTracker` & `MemCache`, Arsitektur MCP Server Gateway (`stdio`), Seed Baseline Snapshot $Q_{t-1}$ |
| **Rafiq** | Analyst Agent & Dashboard Lead | Logika Analyst Agent (Perhitungan % Perubahan Fundamental), *Evidence Checker / Evidence Gate* (Filter Penolakan Temuan), Single-Page Dashboard (`dashboard/index.html`) |
| **Rafi** | Backend & Platform Lead | Axum Backend Engine, Skema Database PostgreSQL, Integrasi Gemini 2.0 Flash (Ekstraksi Tesis), Webhook WhatsApp Business API & *Compliance Filter* |

---

## 🛠️ Prasyarat Sistem

- **Rust** (MSRV 1.78+) & `cargo`
- **Docker** & **Docker Compose**
- **API Keys**:
  - Sectors API v2
  - Google Gemini API (`gemini-2.0-flash` / `gemini-1.5-flash`)
  - Meta for Developers (WhatsApp Cloud API)

---

## 🚀 Panduan Memulai (Quickstart)

### 1. Salin Konfigurasi Environment
```bash
cp .env.example .env
```
Isi variabel yang diperlukan pada file `.env`.

### 2. Jalankan Database (PostgreSQL)
Jalankan instance database lokal menggunakan Docker:
```bash
docker compose up -d
```

### 3. Jalankan MCP Server (Data & MCP Layer)
```bash
cargo run --bin mcp-server
```
Binary `mcp-server` menyediakan 6 core tools via protokol MCP (transport stdio):
`list_subsectors`, `screen_companies`, `get_subsector_report`, `get_company_evidence`,
`get_previous_snapshot`, `record_finding`.

### 4. Build Binary MCP Server
Sebelum menjalankan orchestrator, pastikan binary `mcp-server` sudah ter-build:
```bash
cargo build --bin mcp-server
```
Binary hasil build akan berada di `target/debug/mcp-server` (atau `target/release/mcp-server` jika menggunakan flag `--release`). Orchestrator akan me-spawn binary ini via transport **stdio**.

### 5. Jalankan Backend (orchestrator)
```bash
cd orchestrator
cargo run
```
Backend Axum berjalan di port `8080`. Health check: `curl http://localhost:8080/health`.
Dashboard: `http://localhost:8080/dashboard`.

> Migrasi TIDAK auto-run. Jalankan manual dulu:
> ```bash
> sqlx migrate run --source orchestrator/migrations
> ```
> Mode default `APP_MODE=mock` (aman untuk dev). Scheduler default nonaktif (`SCHEDULER_ENABLED=false`).

---

## 📚 Dokumentasi Lengkap

Mulai dari [docs/README.md](docs/README.md) (indeks + status handover), lalu berurutan:

- [docs/01-mulai-dari-nol.md](docs/01-mulai-dari-nol.md): setup `.env`, database, cara jalan, troubleshooting, sisa kerja terbuka.
- [docs/02-arsitektur.md](docs/02-arsitektur.md): diagram alur data, 6 MCP tools, alur tesis → alert.
- [docs/03-kontrak-data.md](docs/03-kontrak-data.md): schema `Finding`, hasil compliance, payload WhatsApp.
- [docs/04-agent-prompts.md](docs/04-agent-prompts.md): 4 system prompt + daftar kata terlarang.
- [docs/05-api-backend.md](docs/05-api-backend.md): semua endpoint Axum + contoh `curl` + 3 gate validasi.
- [docs/06-sectors-api.md](docs/06-sectors-api.md): endpoint Sectors v2 terverifikasi + biaya kredit.
- [docs/07-indikator-testing.md](docs/07-indikator-testing.md): whitelist metric + cara `cargo test`.
- [docs/archive/](docs/archive/): histori masa hackathon (audit, runbook, brief, pembagian tugas) — bukan acuan teknis.

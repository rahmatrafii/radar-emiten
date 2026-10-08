# Financial Market Research Agent (IDX Sentinel) 🚀

> [!CAUTION]
> **⚠️ DISCLAIMER — Baca Sebelum Menggunakan:**
> Sistem ini **hanya** menyediakan ringkasan informasi dan pemantauan data historis pasar modal.
> **TIDAK** memberikan saran investasi (rekomendasi beli / jual / tahan) maupun eksekusi transaksi otomatis.
> Seluruh output bersifat informatif dan tidak menggantikan analisis profesional berlisensi.

---

## 📌 Ringkasan Proyek

**IDX Sentinel** adalah sistem intelijen pasar modal Indonesia berbasis arsitektur *Agentic AI* dan *Model Context Protocol* (MCP) yang dirancang untuk:

- 🔍 **Memantau & mendeteksi anomali** fundamental emiten secara periodik
- 🧠 **Menganalisis perubahan fundamental** kuartal-ke-kuartal via Gemini 2.0 Flash
- 🛡️ **Memverifikasi evidensi** sebelum temuan dikirimkan (Evidence Gate — zero hallucination guardrail)
- 📲 **Mendistribusikan ringkasan terverifikasi** ke WhatsApp Business secara otomatis

Dibangun di atas stack **Rust (Axum + rmcp) + PostgreSQL + Google Gemini 2.0 Flash**, dengan data bersumber dari **Sectors API v2** (IDX).

---

## 🏗️ Arsitektur Sistem

### Alur Data End-to-End

```mermaid
sequenceDiagram
    autonumber
    participant WA as 📲 WhatsApp Webhook
    participant AX as ⚙️ Axum Backend
    participant GM as 🧠 Gemini 2.0 Flash
    participant MCP as 🦀 MCP Server (stdio)
    participant SA as 📊 Sectors API v2

    WA->>AX: POST /webhook (trigger / query user)
    AX->>GM: Kirim konteks + system prompt (Scout Stage)
    GM->>AX: Tool call request: screen_companies / get_company_evidence
    AX->>MCP: Spawn subprocess, kirim JSON-RPC via stdin
    MCP->>SA: HTTP GET (dengan CreditTracker guard)
    SA-->>MCP: JSON data fundamental emiten
    MCP-->>AX: Hasil tool via stdout (JSON-RPC response)
    AX->>GM: Inject evidence ke prompt (Analyst Stage)
    GM->>AX: Tesis perubahan fundamental (structured JSON)
    AX->>AX: Evidence Gate — validasi kelengkapan data
    AX->>GM: Compliance check (Compliance Stage)
    GM-->>AX: Sinyal LOLOS / TOLAK
    AX->>WA: Kirim ringkasan terverifikasi via WhatsApp Cloud API
```

### Stack Teknologi

| Layer | Teknologi |
|---|---|
| **Data Source** | Sectors API v2 (IDX) |
| **MCP Gateway** | Rust (`rmcp`) — transport **stdio** (subprocess) |
| **Orchestrator / Backend** | Rust · Axum · Tokio async |
| **AI Engine** | Google Gemini 2.0 Flash (structured JSON output) |
| **Database** | PostgreSQL (snapshot, audit trail, findings log) |
| **Delivery Channel** | WhatsApp Cloud API (Meta for Developers) |
| **Dashboard** | Single-page HTML (`dashboard/index.html`) |

---

## ⚡ Key Technical Highlights (USP)

### 🛡️ 1. Evidence Gate — Zero Hallucination Guardrail
Setiap temuan fundamental divalidasi oleh *Evidence Gate* sebelum diproses Gemini.
Gate memeriksa kelengkapan data kuantitatif (field wajib, nilai non-null, batas threshold) dan **menolak** temuan yang datanya tidak mencukupi — memastikan AI hanya bekerja dengan fakta nyata, bukan imputasi.

### 🔢 2. Atomic Credit Tracker
`CreditTracker` adalah modul **thread-safe** berbasis `Arc<Mutex<>>` yang melacak konsumsi kuota Sectors API v2 (limit **1.000 kredit**) secara real-time. Setiap tool call didebet atomik; jika kuota habis, request diblokir otomatis sebelum menyentuh API — mencegah overcharge dan rate-limit error.

### 🔌 3. Subprocess MCP `stdio` Transport
Alih-alih membuka port HTTP terpisah, orchestrator men-**spawn** binary `mcp-server` sebagai *child process* dan berkomunikasi lewat **stdin/stdout** menggunakan JSON-RPC 2.0. Hasilnya: latency sub-millisecond antar-komponen, tidak ada overhead network stack, dan zero port conflict.

---

## 👥 Tim Pengembang & Pembagian Tugas

| Nama | Peran | Tanggung Jawab Utama |
|---|---|---|
| **Dian** | Data & MCP Lead | Integrasi Sectors API v2, Modul `CreditTracker` & `MemCache`, Arsitektur MCP Server Gateway (`stdio`), Seed Baseline Snapshot $Q_{t-1}$ |
| **Rafiq** | Analyst Agent & Dashboard Lead | Logika Analyst Agent (Perhitungan % Perubahan Fundamental), *Evidence Checker / Evidence Gate* (Filter Penolakan Temuan), Single-Page Dashboard (`dashboard/index.html`) |
| **Rafi** | Backend & Platform Lead | Axum Backend Engine, Skema Database PostgreSQL, Integrasi Gemini 2.0 Flash (Ekstraksi Tesis), Webhook WhatsApp Business API & *Compliance Filter* |

---

## 🔑 Konfigurasi Environment (`.env` Spec)

Salin template: `cp .env.example .env`, lalu isi variabel berikut:

| Variabel | Contoh Nilai | Keterangan |
|---|---|---|
| `APP_MODE` | `mock` / `live` | `mock` = data dummy (aman untuk dev/test); `live` = data nyata |
| `MCP_TRANSPORT` | `stdio` | Protokol komunikasi ke MCP server |
| `MCP_SERVER_COMMAND` | `./target/debug/mcp-server` | Path ke binary mcp-server yang akan di-spawn |
| `GEMINI_MODEL` | `gemini-2.0-flash` | Model Gemini yang digunakan untuk analisis |
| `GEMINI_API_KEY` | `AIza...` | Google AI Studio API Key |
| `SECTORS_API_KEY` | `sec_...` | Sectors API v2 Key (limit 1.000 kredit) |
| `DATABASE_URL` | `postgres://user:pass@localhost/radar` | PostgreSQL connection string |
| `WHATSAPP_TOKEN` | `EAAG...` | Meta for Developers — WhatsApp Cloud API token |
| `WHATSAPP_PHONE_ID` | `1234567890` | ID nomor pengirim WhatsApp Business |
| `VERIFY_TOKEN` | `my_verify_token` | Token verifikasi webhook WhatsApp |
| `SCHEDULER_ENABLED` | `false` / `true` | Aktifkan scheduler periodik (default: `false`) |
| `PORT` | `8080` | Port Axum backend |

---

## 🚀 Quickstart

### Prasyarat Sistem

- **Rust** (MSRV 1.78+) & `cargo`
- **Docker** & **Docker Compose**
- API Keys: Sectors API v2, Google Gemini, Meta for Developers (WhatsApp)

---

### Langkah 1 — Konfigurasi Environment

```bash
cp .env.example .env
# Edit .env dan isi semua API key yang diperlukan
```

### Langkah 2 — Jalankan Database

```bash
docker compose up -d
```

Lalu jalankan migrasi schema:

```bash
sqlx migrate run --source orchestrator/migrations
```

### Langkah 3 — Build Binary MCP Server

> [!IMPORTANT]
> Binary `mcp-server` **harus di-build terlebih dahulu** sebelum menjalankan orchestrator, karena orchestrator akan men-spawn-nya sebagai subprocess via `stdio`.

```bash
cargo build --bin mcp-server
# Hasil: target/debug/mcp-server
# Untuk production: cargo build --release --bin mcp-server
```

Binary menyediakan **6 MCP tools** berikut:

| Tool | Deskripsi |
|---|---|
| `list_subsectors` | Daftar semua subsektor IDX |
| `screen_companies` | Filter emiten berdasarkan kriteria fundamental |
| `get_subsector_report` | Laporan fundamental agregat per subsektor |
| `get_company_evidence` | Data fundamental spesifik satu emiten |
| `get_previous_snapshot` | Baseline snapshot kuartal sebelumnya ($Q_{t-1}$) |
| `record_finding` | Simpan temuan terverifikasi ke PostgreSQL |

### Langkah 4 — Jalankan Orchestrator

```bash
cd orchestrator
cargo run
```

| Endpoint | Fungsi |
|---|---|
| `GET /health` | Health check — pastikan server berjalan |
| `GET /dashboard` | Single-page monitoring dashboard |
| `POST /webhook` | Endpoint penerima WhatsApp webhook |
| `POST /api/scan` | Trigger scan manual (mode `live`) |

```bash
# Verifikasi server berjalan:
curl http://localhost:8080/health
```

---

## 🧪 Verifikasi & Unit Testing

Jalankan seluruh unit test suite:

```bash
cargo test --lib
```

> ✅ **35 / 35 unit tests passed** — mencakup `CreditTracker`, `MemCache`, Evidence Gate logic, MCP tool parsing, dan compliance filter.

Untuk menjalankan test spesifik per modul:

```bash
# Contoh: test modul credit tracker
cargo test --lib credit_tracker

# Contoh: test modul evidence gate
cargo test --lib evidence
```

---

## 📚 Dokumentasi Lengkap

Mulai dari [`docs/README.md`](docs/README.md) (indeks + status handover):

| Dokumen | Isi |
|---|---|
| [`docs/01-mulai-dari-nol.md`](docs/01-mulai-dari-nol.md) | Setup `.env`, database, troubleshooting, sisa kerja terbuka |
| [`docs/02-arsitektur.md`](docs/02-arsitektur.md) | Diagram alur data, 6 MCP tools, alur tesis → alert |
| [`docs/03-kontrak-data.md`](docs/03-kontrak-data.md) | Schema `Finding`, hasil compliance, payload WhatsApp |
| [`docs/04-agent-prompts.md`](docs/04-agent-prompts.md) | 4 system prompt + daftar kata terlarang |
| [`docs/05-api-backend.md`](docs/05-api-backend.md) | Semua endpoint Axum + contoh `curl` + 3 gate validasi |
| [`docs/06-sectors-api.md`](docs/06-sectors-api.md) | Endpoint Sectors v2 terverifikasi + biaya kredit |
| [`docs/07-indikator-testing.md`](docs/07-indikator-testing.md) | Whitelist metric + cara `cargo test` |

> [!NOTE]
> [`docs/archive/`](docs/archive/) — histori internal masa hackathon (audit, runbook, brief). Bukan acuan teknis aktif.

---

<div align="center">
  <sub>Built with ❤️ in Rust · Powered by Gemini 2.0 Flash & Sectors API v2</sub>
</div>

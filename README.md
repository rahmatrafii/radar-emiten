# Financial Market Research Agent 🚀

> [!CAUTION]
> **PENTING / DISCLAIMER:**
> Aplikasi ini hanya menyediakan analisis informasi dan pemantauan data historis. **TIDAK** memberikan saran investasi (rekomendasi beli/jual/tahan) maupun eksekusi transaksi otomatis.

---

## 📌 Ringkasan Proyek

**Financial Market Research Agent** adalah sistem intelijen pasar modal berbasis arsitektur *Agentic AI* dan *Model Context Protocol* (MCP) yang dirancang untuk memantau, mendeteksi anomali, menganalisis fundamental emiten, serta mendistribusikan ringkasan berbasis data secara objektif ke kanal komunikasi (WhatsApp).

### Arsitektur Singkat
```text
Sectors API v2 ──► MCP Rust Server ──► Axum Backend + Gemini Flash ──► PostgreSQL ──► WhatsApp Cloud API
```

1. **Sectors API v2**: Sumber data pasar modal Indonesia (emiten, sektor, rasio keuangan).
2. **MCP Rust Server**: Lapisan tool abstraction berkinerja tinggi menggunakan Rust untuk menyediakan data real-time dan snapshot historis.
3. **Axum Backend + Gemini Flash**: Orkestrator alur agentic multi-stage (Scout, Fundamental, Compliance, Presenter) dengan output terstruktur JSON.
4. **PostgreSQL**: Penyimpanan snapshot data, log verifikasi evidensi (*audit trail*), dan riwayat *findings*.
5. **WhatsApp Cloud API**: Pengiriman sinyal informasi dan ringkasan pasar terverifikasi kepada pengguna akhir.

---

## 👥 Tim Pengembang

| Nama | Peran | Tanggung Jawab Utama |
|---|---|---|
| **Representative** | Data & MCP Rust Engineer | Integrasi Sectors API v2, MCP Rust Server (`rmcp`), Caching & Rate-limit Handling |
| **Dian** | Agent Logic & Prompt Engineer | Desain Master Prompts (JSON Structured Output), Compliance & Rule Guardrails, Few-Shot Test Cases |
| **Rahmat** | Backend & Platform Engineer | Axum Backend Engine, PostgreSQL Schema & Repository, WhatsApp Cloud API Webhook & Alerting |

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

### 3. Jalankan Backend (orchestrator)
```bash
cd orchestrator
cargo run
```
Backend Axum berjalan di port `8080`. Health check: `curl http://localhost:8080/health`.

> Catatan: MCP server (`mcp-server`) disediakan oleh Representative dan belum ada di repo ini.

---

## 📚 Dokumentasi Lengkap

Untuk panduan arsitektur dan spesifikasi teknis mendalam, silakan baca dokumen pada direktori `docs/`:
- [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md): Diagram alur data, spesifikasi tool MCP, dan penanganan Sectors API v2.
- [docs/CONTRACTS.md](docs/CONTRACTS.md): Kontrak schema JSON resmi untuk pertukaran data antarkomponen (`Finding`).
- [docs/PROMPTS.md](docs/PROMPTS.md): Master Prompt untuk ke-4 agent (Scout, Fundamental, Compliance, Presenter).
- [docs/TASKS.md](docs/TASKS.md): Rincian pembagian kerja dan milestone tim hackathon.

# Breakdown Tugas Tim (Task Distribution)

Dokumen ini memetakan pembagian tanggung jawab, deliverable spesifik, dan ketergantungan antaranggotat tim untuk proyek Hackathon **Financial Market Research Agent**.

---

## 👥 Rangkuman Alokasi Peran

| Anggota Tim | Domain Utama | Fokus Teknologi |
|---|---|---|
| **Rafiq** | Data Ingestion & MCP Layer | Rust, `rmcp`, Sectors API v2, Caching Layer |
| **Dian** | Agentic Intelligence & Compliance | Prompt Engineering, JSON Structured Outputs, Gemini API, Guardrails |
| **Rafi** | Application Core & Distribution | Rust (Axum), PostgreSQL (SQLx), Meta WhatsApp Cloud API |

---

## 📋 Rincian Tugas & Deliverable

### 1. Rafiq (Data & MCP Rust Server)

| No | Modul / Tugas | Deskripsi Deliverable | Output / Kontrak | Status |
|---|---|---|---|---|
| R-01 | **Sectors API v2 Client** | Implementasi HTTP Client Rust untuk konsumsi endpoint `/v2/subsectors/`, `/v2/companies/` (screener), `/v2/companies/quarterly-financial-dates/`, `/v2/financials/quarterly/{symbol}/`, `/v2/subsector/report/{sub}/`. | Modul `sectors_client` dengan proteksi error HTTP 410. | `Completed` |
| R-02 | **Caching & Rate-Limiter** | In-memory cache (LRU / TTL) untuk menghindari kehabisan kuota query Sectors API. | Middleware cache dengan waktu kadaluarsa konfigurabel. | `Completed` |
| R-03 | **MCP Rust Server (`rmcp`)** | Implementasi server MCP berbasis JSON-RPC yang mengekspos 6 core tools: `list_subsectors`, `screen_companies`, `get_subsector_report`, `get_company_evidence`, `get_previous_snapshot`, `record_finding`. | Biner `mcp-server` yang dapat dipanggil oleh backend/agent. | `Completed` |
| R-04 | **Data DTO & Serialization** | Struct Rust dengan atribut `serde` untuk pemetaan data raw Sectors API v2 ke format konsumsi agen. | Modul `models::sectors`. | `Completed` |

---

### 2. Dian (Agent Logic, Prompts & Compliance Guardrail)

| No | Modul / Tugas | Deskripsi Deliverable | Output / Kontrak | Status |
|---|---|---|---|---|
| D-01 | **Master System Prompts** | Finalisasi dan tuning prompt 4 agen (`Market Scout`, `Fundamental Analyst`, `Compliance Checker`, `Chief Presenter`) dengan JSON Structured Output. | File [docs/PROMPTS.md](PROMPTS.md) & template prompt teruji. | `Completed` |
| D-02 | **Compliance Rule Engine** | Mekanisme validasi kata terlarang (*anti-recommendation filter*: beli, jual, tahan, rekomendasi, target harga, cuan). | Regex & logic sanitizer pada tahap verifikasi finding. | `Completed` |
| D-03 | **Zero-Hallucination Verifier** | Logika perbandingan 1:1 antara metrik angka yang disebutkan oleh model dengan data raw Sectors API v2. | Validasi skema `ComplianceVerificationResult`. | `Completed` |
| D-04 | **Few-Shot Test Cases** | Kumpulan dataset skenario pengujian (emiten perbankan, telekomunikasi, komoditas) untuk mengevaluasi akurasi reasoning model Gemini Flash. | File `tests/fixtures/agent_cases.json`. | `Completed` |

---

### 3. Rafi (Axum Backend, Database & WhatsApp Delivery)

| No | Modul / Tugas | Deskripsi Deliverable | Output / Kontrak | Status |
|---|---|---|---|---|
| M-01 | **Axum Web Server Framework** | Setup fondasi backend Axum, routing REST/Webhook, graceful shutdown, dan konfigurasi environment. | Biner `backend` pada port 8080. | `Completed` |
| M-02 | **PostgreSQL & Database Migrations** | Skema tabel database: `snapshots`, `findings`, `compliance_audit_logs`, dan integrasi queries via `sqlx`. | File migrasi SQL & repository pattern. | `Completed` |
| M-03 | **Agent Pipeline Orchestration** | Pengendali alur kerja sequensial: memicu MCP Tool -> invoke Gemini Flash -> verifikasi compliance -> simpan database. | Service `orchestrator::AgentPipeline`. | `Completed` |
| M-04 | **WhatsApp Cloud API Integration** | Integrasi pengiriman pesan WhatsApp (Meta Graph API) untuk ringkasan temuan pasar, serta webhook handler. | Client `whatsapp::Client` & format payload resmi. | `Completed` |

> **Catatan verifikasi (7 Okt 2026):** Keempat deliverable M-01 s/d M-04 terimplementasi penuh dan teruji — 57 test orkestrator PASS (termasuk 5 test pipeline DB-backed). Catatan operasional: (1) migrasi tidak auto-run saat startup, jalankan `sqlx migrate run` manual; (2) `RealMcpGateway` masih stub `TransportUnavailable` sampai transport MCP disepakati; (3) pengiriman WhatsApp real terhalang hard gate `POLICY_DISCLAIMER_CONFLICT_ACKNOWLEDGED=false` (bloker kepatuhan, bukan bug). (4) Konflik disclaimer telah diselesaikan — disclaimer resmi diubah ke versi yang tidak mengandung kata "rekomendasi".

---

## 🔄 Titik Temu Integrasi (Integration Touchpoints)

1. **Rafiq ➔ Rafi**:
   - Backend memanggil MCP Server melalui stdio/HTTP untuk mengakses tools Sectors API v2.
2. **Dian ➔ Rafi**:
   - Master prompt dan JSON schema dieksekusi oleh backend saat memanggil Gemini Flash API.
3. **Rafi ➔ Dian**:
   - Log kepatuhan dan audit trail disimpan di PostgreSQL untuk diinspeksi akurasinya oleh tim prompt/compliance.

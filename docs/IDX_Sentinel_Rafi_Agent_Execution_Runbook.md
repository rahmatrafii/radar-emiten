# IDX Sentinel / Financial Market Research Agent
## Runbook Eksekusi Lengkap untuk Agent Coding — Bagian Rahmat Rafi

**Versi dokumen:** 1.0  
**Tanggal penyusunan:** 4 Oktober 2026  
**Pemilik bagian:** Rahmat Rafi  
**Bahasa/backend:** Rust + Axum + Tokio  
**Database:** PostgreSQL + SQLx  
**Port backend:** `8080`  
**Tujuan:** Menjadi instruksi kerja yang bisa diberikan langsung kepada AI coding agent untuk memeriksa repository, mengimplementasikan bagian Rafi dari awal sampai akhir, menjalankan pengujian, dan melaporkan hasil nyata.

> **Peringatan penting:** Jangan pernah meminta pengguna mengirim API key/token/password ke chat atau menuliskannya ke repository. Gunakan nama variabel environment yang ada di dokumen ini. Jangan mengaku sebuah integrasi berhasil sebelum benar-benar menjalankan tesnya.

---

# 0. Instruksi utama untuk AI coding agent

Kamu bertindak sebagai **coding agent yang mengeksekusi pekerjaan**, bukan hanya memberi saran atau membuat rencana.

1. Periksa repository dan file yang sudah ada sebelum membuat atau mengubah apa pun.
2. Baca `ARCHITECTURE.md`, `CONTRACTS.md`, `TASKS.md`, `README.md`, file konfigurasi, kode, migration, serta `docs/PROMPTS.md` jika tersedia.
3. Jangan menimpa implementasi yang sudah ada tanpa memeriksanya. Pertahankan perubahan pengguna dan perubahan anggota tim lain.
4. Kerjakan satu fase pada satu waktu. Setelah setiap fase, jalankan pemeriksaan yang relevan dan perbaiki kegagalan sebelum lanjut.
5. Implementasikan backend Rafi. Jangan mengambil alih implementasi MCP/Sectors milik **Representative** atau prompt master/compliance engine milik **Dian**, kecuali diperlukan adapter integrasi kecil yang sudah disepakati.
6. Bila komponen anggota lain belum ada, buat **trait/interface dan mock adapter** agar bagian Rafi tetap dapat diuji. Jangan mengarang endpoint transport MCP, field Sectors, biaya kredit, atau indikator yang belum dikonfirmasi.
7. Jika ditemukan konflik spesifikasi, ikuti aturan prioritas pada Bagian 1. Catat konflik yang belum dapat diselesaikan di `docs/integration-decisions.md`; jangan menyembunyikannya.
8. Setelah tiap fase, laporkan: file yang dibuat/diubah, command yang dijalankan, hasil tes, dan blocker yang sebenarnya. Jangan melaporkan tes berhasil jika command gagal atau tidak dijalankan.
9. Utamakan satu alur end-to-end yang benar dibanding banyak stub yang diklaim selesai. Semua stub harus diberi nama `mock`, `stub`, atau `not implemented` secara jelas.
10. Jangan membuat commit otomatis kecuali diminta pemilik repository. Jangan memasukkan `.env`, token, nomor WhatsApp mentah, atau credential ke commit/log.

## Definisi selesai secara umum

Bagian Rafi dianggap selesai hanya apabila:

- Backend Axum menyala di port `8080`, bisa graceful shutdown, dan bisa mengakses PostgreSQL.
- Migration dapat membangun database dari keadaan kosong.
- Kontrak HTTP backend terdokumentasi dan dapat dites tanpa anggota lain menggunakan mock.
- Pesan WhatsApp dapat dikirim dari kode Rust, bila kredensial valid tersedia.
- Webhook WhatsApp memverifikasi challenge dan signature, serta mencegah pesan duplikat diproses dua kali.
- User dapat mengirim tesis, mendapat konfirmasi, membalas `YA`, dan melihat tesis aktif tersimpan.
- Pipeline dapat mengambil/memakai snapshot, membangun `Finding` sesuai kontrak resmi, memverifikasi bukti, menyimpan hasil verifikasi, membentuk alert, menerapkan cooldown, dan mencatat jejak.
- Endpoint JSON untuk dashboard mengembalikan data dari database dan menyamarkan nomor WhatsApp.
- Unit test, integration test yang dapat dijalankan lokal, `cargo fmt --check`, `cargo check`, dan `cargo test` dijalankan serta hasilnya dilaporkan.
- Integrasi nyata yang belum dapat dites karena credential, transport MCP, atau akses API belum tersedia disebut sebagai **belum terverifikasi**, bukan dianggap selesai.

---

# 1. Sumber kebenaran dan aturan konflik spesifikasi

Gunakan sumber berikut dengan urutan prioritas ini:

1. **`CONTRACTS.md` terbaru** adalah sumber kebenaran untuk bentuk payload eksternal `Finding`, `ComplianceVerificationResult`, dan `WhatsAppAlertPayload`.
2. **`ARCHITECTURE.md` terbaru** adalah sumber kebenaran untuk gambaran arsitektur dan enam MCP tools.
3. **`TASKS.md` terbaru** adalah sumber kebenaran untuk deliverable Rafi M-01 sampai M-04, termasuk Axum port `8080`, SQLx, `orchestrator::AgentPipeline`, dan WhatsApp Cloud API.
4. Brief IDX Sentinel sebelumnya tetap menjadi sumber perilaku produk yang tidak dibatalkan oleh dokumen terbaru: tesis melalui WhatsApp, konfirmasi `YA`, daftar perintah, tiga status tesis, bukti yang dapat diaudit, cooldown, pencatatan kredit, jejak agent, dan API dashboard.
5. Dokumen implementasi atau kode yang sudah disepakati dalam repository juga perlu diperiksa sebelum mengubah integrasi yang sudah berjalan.

## Keputusan teknis yang wajib diikuti

### 1.1 Kontrak `Finding` resmi

Jangan memakai nama field dari kontrak lama seperti `indikator`, `arah_dukungan`, `nilai_sebelum`, `nilai_sekarang`, `periode`, `sumber`, atau `tingkat_keyakinan` sebagai field eksternal `Finding`.

Gunakan field resmi dari `CONTRACTS.md`:

```json
{
  "ticker": "BBCA",
  "subsector": "banks",
  "metric_name": "net_profit_margin",
  "current_value": 46.85,
  "previous_value": 43.12,
  "period": "Q3 2024",
  "source": "Sectors API v2 /v2/company/report/BBCA",
  "observed_at": "2026-10-02T10:15:30Z",
  "confidence_score": 0.98,
  "finding_summary": "Net Profit Margin BBCA pada periode ini berbeda dibandingkan periode pembanding."
}
```

Field wajib: `ticker`, `subsector`, `metric_name`, `current_value`, `previous_value`, `period`, `source`, `observed_at`, `confidence_score`, `finding_summary`. `additionalProperties` dilarang. Nilai `current_value` dan `previous_value` secara schema dapat berupa number/string/null, tetapi **Finding yang akan menjadi alert wajib mempunyai kedua nilai numerik yang valid**. Null/string yang tidak bisa diparse menjadi angka boleh tercatat sebagai temuan tidak valid, tetapi tidak boleh lolos ke alert.

Jika kode lama masih memakai kontrak lama, buat adapter internal yang eksplisit. Jangan diam-diam mengubah `CONTRACTS.md` atau mengirim dua bentuk payload yang berbeda di endpoint yang sama.

### 1.2 Database

`TASKS.md` secara khusus menyebut tabel inti `snapshots`, `findings`, dan `compliance_audit_logs`. Untuk mendukung perilaku produk sebelumnya, buat juga tabel operasional yang dibutuhkan, misalnya pengguna, tesis, indikator tesis, pesan masuk, alerts, kredit, dan jejak agent. Jangan menghapus tiga tabel inti tersebut.

**Axum backend milik Rafi adalah satu-satunya pemilik koneksi dan query PostgreSQL.** MCP server milik Representative tidak boleh membuat koneksi database kedua yang menulis ke tabel yang sama. Bila MCP tools `get_previous_snapshot` dan `record_finding` membutuhkan database, implementasi tool tersebut harus mendelegasikan operasi ke endpoint internal Axum yang diamankan, atau koordinasikan perubahan arsitektur dengan tim.

### 1.3 Port dan struktur aplikasi

Gunakan port `8080` seperti `TASKS.md`, bukan port `3000` yang pernah muncul pada contoh terdahulu. Jika repository sudah mempunyai konfigurasi port lain yang berfungsi, ubah secara sadar dan perbarui `.env.example` serta README.

### 1.4 Indikator dan data Sectors

Contoh metrik di `CONTRACTS.md` adalah contoh schema, bukan bukti bahwa seluruh metrik tersedia pada akun Sectors. Gunakan daftar indikator yang disetujui tim dan hasil tes API. Jangan menebak nama field dari respons Sectors. Jika daftar final belum ada, buat metric registry yang dapat dikonfigurasi dan gunakan fixture hanya untuk tes.

### 1.5 Integrasi Gemini

Periksa `docs/PROMPTS.md` karena `TASKS.md` mencatat prompt master Dian sebagai pekerjaan yang sudah selesai. Gunakan prompt/schema milik Dian jika tersedia. Jangan membuat versi prompt tandingan atau mengganti schema tanpa koordinasi. Backend bertanggung jawab memanggil Gemini, mem-parse JSON, memvalidasi hasilnya, dan menolak output tidak valid.

### 1.6 Konflik disclaimer dan kata terlarang — WAJIB DICATAT

Ada konflik antara aturan produk terdahulu yang melarang teks `rekomendasi` di pesan dan konstanta disclaimer dalam `CONTRACTS.md` yang secara literal mengandung frasa **“rekomendasi transaksi”**:

`Disclaimer: Informasi ini hanya bersifat edukasi & pemantauan data historis. Bukan anjuran investasi atau rekomendasi transaksi.`

Jangan diam-diam mengubah kontrak, dan jangan mengklaim seluruh aturan kata terlarang sudah terpenuhi tanpa menyelesaikan konflik ini. Implementasikan validasi pada field dinamis (`header`, `body`, ringkasan LLM) dan validasi kesamaan disclaimer terhadap konstanta schema. Catat konflik ini di `docs/integration-decisions.md` sebagai **BLOCKER KEPATUHAN SEBELUM DEMO PUBLIK**. Selama `POLICY_DISCLAIMER_CONFLICT_ACKNOWLEDGED=false`, hard gate harus memblokir pengiriman WhatsApp real, menyimpan audit reason `DISCLAIMER_POLICY_CONFLICT`, dan melaporkan bahwa pengiriman diblokir; jangan menandainya `sent`. Mock mode boleh menguji payload, tetapi harus melaporkan `blocked_by_policy_conflict` dan tidak mengklaim seolah alur real dapat mengirim. Alert tidak boleh dinyatakan production/compliance-ready sampai tim memutuskan salah satu: memperbarui konstanta disclaimer dan schema secara resmi, atau mengesahkan pengecualian eksplisit untuk disclaimer tetap. Jangan memasukkan pengecualian tersembunyi.

---

# 2. Batas tanggung jawab Rafi

## Yang harus diimplementasikan Rafi

- M-01: Axum Web Server, REST routes, webhook routes, konfigurasi environment, graceful shutdown, backend binary port `8080`.
- M-02: PostgreSQL, SQLx migrations, repository pattern, tabel snapshots, findings, compliance audit logs, dan tabel operasional yang diperlukan oleh fitur produk.
- M-03: `orchestrator::AgentPipeline` untuk mengorkestrasi alur MCP → Gemini/agent prompts → compliance verification → persistence → alert dispatch.
- M-04: WhatsApp Cloud API client, format payload, pengiriman pesan, webhook verification, webhook message parsing, signature validation, serta deduplikasi pesan.
- Workflow tesis WhatsApp dan konfirmasi pengguna.
- Tiga status tesis (Menguat, Netral, Melemah), kartu bukti, daftar perintah WhatsApp, cooldown/batas alert, pencatatan kredit, audit trail/jejak agent, serta API JSON dashboard.
- Integrasi dengan kontrak MCP Representative dan prompt/compliance Dian tanpa mengambil alih ownership mereka.

## Yang bukan tugas utama Rafi

- Mengimplementasikan Sectors API client, caching, rate limiter, atau MCP server `rmcp` dari nol. Itu tanggung jawab Representative.
- Menulis ulang master prompt empat agent yang dikerjakan Dian. Rafi mengintegrasikan prompt dan memvalidasi outputnya.
- Membuat dashboard frontend penuh bila dashboard telah dialokasikan ke Dian/anggota lain. Bagian Rafi menyediakan API JSON dan memastikan nomor pengguna disamarkan.
- Menyediakan nasihat keuangan, rekomendasi transaksi, target harga, atau eksekusi transaksi.

---

# 3. Alur sistem yang harus jadi

```text
Pesan WhatsApp pengguna
  ↓
GET verification / POST webhook
  ↓
Verifikasi signature + parse payload + dedupe message ID
  ↓
Simpan pesan masuk
  ↓
Command router / ekstraksi tesis oleh Gemini
  ↓
Rust memvalidasi ticker + whitelist metric
  ↓
Minta konfirmasi pengguna
  ↓
Pengguna membalas YA
  ↓
Tesis aktif tersimpan di PostgreSQL
  ↓
Scheduler atau trigger demo menjalankan AgentPipeline
  ↓
Axum meminta data melalui MCP client (data berasal dari Sectors API v2)
  ↓
Simpan snapshot terbaru; baca snapshot periode pembanding
  ↓
Perhitungan numerik deterministik di Rust
  ↓
Bentuk Finding sesuai CONTRACTS.md
  ↓
Evidence/compliance checks + audit log
  ├─ Gagal → status ditolak + alasan + jejak; jangan kirim alert
  └─ Lolos
       ↓
  Hubungkan Finding dengan tesis yang relevan
       ↓
  Hitung status tesis: Menguat / Netral / Melemah
       ↓
  Terapkan deduplikasi + cooldown + batas alert harian
       ↓
  Bangun WhatsAppAlertPayload dari template kode
       ↓
  Validasi field dinamis, angka, dan disclaimer
       ↓
  Kirim melalui WhatsApp Cloud API
       ↓
  Simpan status pengiriman + jejak agent
       ↓
  Endpoint /api/* menyediakan data untuk dashboard
```

Prinsip mutlak: **kode Rust menghitung angka dan menetapkan aturan; LLM tidak menjadi sumber angka faktual.** LLM digunakan untuk ekstraksi bahasa dan/atau kalimat ringkasan sesuai prompt yang disetujui. Jika bukti tidak cukup atau hasil validasi gagal, jangan mengirim alert.

---

# 4. Tahap 0 — Audit repository sebelum coding

> ✅ **STATUS: SELESAI (5 Oktober 2026)** — branch `main` bersih, baseline `cargo check` PASS, inventaris komponen dicatat di `docs/integration-decisions.md`, tidak ada file penting yang akan tertimpa.


Jalankan dari root repository:

```bash
git status --short
 git branch --show-current
find . -maxdepth 3 -type f | sort
```

Perintah `git status` dan daftar file harus diperiksa sebelum file diubah. Sesuaikan command `find` jika menggunakan PowerShell.

## Langkah

1. Temukan crate Rust yang sudah ada. Jika sudah ada `orchestrator/`, `backend/`, atau folder backend, **gunakan crate itu**; jangan membuat backend kedua.
2. Baca `Cargo.toml`, `src/main.rs`, router Axum yang sudah ada, konfigurasi environment, migration, tests, `.gitignore`, `docker-compose.yml`, README, `ARCHITECTURE.md`, `CONTRACTS.md`, `TASKS.md`, dan `docs/PROMPTS.md` jika ada.
3. Periksa apakah Representative sudah menyediakan MCP server/binary dan transport yang disepakati.
4. Periksa apakah Dian sudah menyediakan prompt, JSON schema, compliance output, dan fixture.
5. Buat atau perbarui `docs/integration-decisions.md`. Catat setidaknya konflik disclaimer, transport MCP, daftar metric final, dan status prompt. Pisahkan fakta terkonfirmasi dari hal yang belum dipastikan.
6. Catat command baseline yang bisa dijalankan (`cargo check`, `cargo test`, atau kegagalan baseline). Jangan menganggap kegagalan lama disebabkan perubahan baru.

**Selesai jika:** agent memahami struktur saat ini, tidak ada file penting yang akan tertimpa, dan ada daftar komponen yang sudah ada vs belum ada.

---

# 5. Tahap 1 — Fondasi Rust, Axum, konfigurasi, dan keamanan

> ✅ **STATUS: SELESAI (5 Oktober 2026)** — struktur `config.rs`/`state.rs`/`errors.rs`/`routes/`, `Config::from_env()` + validasi mode real, AppState dengan PgPool, tracing, `GET /health` JSON + cek DB terverifikasi (`{"status":"ok","database":"ok"}`), graceful shutdown, port 8080, `cargo check` PASS.


## 5.1 Struktur kode yang disarankan

Sesuaikan dengan konvensi repository saat ini. Jangan memindahkan kode besar jika tidak diperlukan.

```text
orchestrator/                         # atau crate backend yang sudah ada
├── Cargo.toml
├── .env                              # lokal, tidak boleh commit
├── .env.example
├── docker-compose.yml                # bila belum ada di root
├── migrations/
│   ├── 0001_initial_schema.sql
│   └── 0002_indexes_and_constraints.sql  # bila butuh migration terpisah
├── src/
│   ├── main.rs
│   ├── config.rs
│   ├── state.rs
│   ├── errors.rs
│   ├── models/
│   │   ├── mod.rs
│   │   ├── finding.rs
│   │   ├── compliance.rs
│   │   ├── whatsapp.rs
│   │   ├── thesis.rs
│   │   └── snapshot.rs
│   ├── routes/
│   │   ├── mod.rs
│   │   ├── health.rs
│   │   ├── webhook_whatsapp.rs
│   │   ├── internal.rs
│   │   └── dashboard_api.rs
│   ├── repositories/
│   │   ├── mod.rs
│   │   ├── users.rs
│   │   ├── theses.rs
│   │   ├── snapshots.rs
│   │   ├── findings.rs
│   │   ├── alerts.rs
│   │   ├── credits.rs
│   │   └── traces.rs
│   ├── services/
│   │   ├── mod.rs
│   │   ├── whatsapp_client.rs
│   │   ├── gemini_client.rs
│   │   ├── thesis_service.rs
│   │   ├── finding_service.rs
│   │   ├── compliance_service.rs
│   │   ├── cooldown_service.rs
│   │   └── status_service.rs
│   ├── integrations/
│   │   ├── mod.rs
│   │   └── mcp_gateway.rs
│   ├── orchestrator/
│   │   ├── mod.rs
│   │   └── agent_pipeline.rs
│   └── scheduler.rs
├── tests/
│   ├── api_tests.rs
│   ├── webhook_tests.rs
│   └── pipeline_tests.rs
└── README.md
```

Nama crate/folder tidak wajib sama jika repository sudah mempunyai struktur sendiri. Nama service yang diminta di `TASKS.md` adalah `orchestrator::AgentPipeline`; pertahankan nama API konseptual itu atau berikan adapter yang jelas.

## 5.2 Dependency

Periksa dependency yang sudah ada terlebih dahulu. Tambahkan hanya yang diperlukan dan gunakan versi yang kompatibel. Kebutuhan umum:

- `axum`, `tokio`, `tower`, `tower-http`
- `serde`, `serde_json`
- `sqlx` dengan PostgreSQL/runtime Tokio/UUID/chrono
- `reqwest` dengan JSON dan TLS yang sesuai
- `dotenvy` atau konfigurasi environment yang sudah digunakan
- `uuid`, `chrono`
- `thiserror` atau error type yang konsisten
- `hmac`, `sha2`, dan hex encoding untuk verifikasi signature Meta
- crate validasi JSON schema hanya jika memang digunakan; jika tidak, validasikan DTO dan aturan kontrak di Rust dan tutup unknown field dengan Serde.

Jangan menambahkan framework agent besar, Redis, Kafka, Kubernetes, vector database, atau frontend framework untuk tugas ini.

## 5.3 Environment variables

Buat `.env.example` dengan nama variabel saja, tanpa rahasia:

```env
APP_MODE=mock
HOST=0.0.0.0
PORT=8080
DATABASE_URL=postgres://postgres:postgres@localhost:5432/idx_sentinel

GEMINI_API_KEY=
GEMINI_MODEL=

WHATSAPP_ACCESS_TOKEN=
WHATSAPP_PHONE_NUMBER_ID=
WHATSAPP_APP_SECRET=
WHATSAPP_VERIFY_TOKEN=
WHATSAPP_GRAPH_API_VERSION=
WHATSAPP_TEMPLATE_NAME=hello_world
WHATSAPP_TEMPLATE_LANGUAGE=en_US

ADMIN_PHONE=
INTERNAL_API_TOKEN=

SECTORS_CREDIT_BUDGET=1000
MAX_ALERTS_PER_DAY=3
NEUTRAL_RELATIVE_THRESHOLD=0.02
SCHEDULER_INTERVAL_SECONDS=86400

MCP_TRANSPORT=
MCP_SERVER_COMMAND=
MCP_SERVER_ARGS=
MCP_CALLBACK_BASE_URL=http://127.0.0.1:8080
POLICY_DISCLAIMER_CONFLICT_ACKNOWLEDGED=false
```

Catatan:

- `APP_MODE=mock` harus memungkinkan unit/integration tests tanpa memerlukan token eksternal. Untuk `APP_MODE=real`, validasi konfigurasi integrasi saat startup atau ketika integrasi pertama kali dipakai.
- `GEMINI_MODEL` dan `WHATSAPP_GRAPH_API_VERSION` diisi berdasarkan konfigurasi layanan dan keputusan tim; jangan menebak versi API yang belum diverifikasi.
- API key Sectors tetap dikelola Representative. Jangan menambahkan key Sectors ke `.env` Rafi jika arsitektur tim tidak memerlukannya.
- `POLICY_DISCLAIMER_CONFLICT_ACKNOWLEDGED` bukan pengganti keputusan tim. Default harus tetap `false` sampai konflik di Bagian 1.6 diselesaikan secara eksplisit.

Pastikan `.gitignore` memuat sekurang-kurangnya:

```gitignore
.env
.env.*.local
target/
*.pem
```

Jangan abaikan `.env.example`.

## 5.4 Docker Compose PostgreSQL

Jika repository belum memiliki layanan database, buat `docker-compose.yml` di lokasi yang sesuai:

```yaml
services:
  postgres:
    image: postgres:16
    environment:
      POSTGRES_USER: postgres
      POSTGRES_PASSWORD: postgres
      POSTGRES_DB: idx_sentinel
    ports:
      - "5432:5432"
    volumes:
      - postgres_data:/var/lib/postgresql/data
    healthcheck:
      test: ["CMD-SHELL", "pg_isready -U postgres -d idx_sentinel"]
      interval: 5s
      timeout: 3s
      retries: 10

volumes:
  postgres_data:
```

Gunakan kredensial contoh tersebut hanya untuk pengembangan lokal. Jangan gunakan password contoh untuk deployment publik.

Jalankan:

```bash
docker compose up -d
docker compose ps
```

## 5.5 Axum server

Implementasikan:

- `Config::from_env()` dan validasi input/config yang dibutuhkan.
- `AppState` berisi `PgPool`, HTTP client yang digunakan ulang, dan service/integration dependency yang dibutuhkan.
- Router dengan `GET /health`.
- `tokio::signal` untuk graceful shutdown.
- Log terstruktur; jangan pernah mencetak access token, app secret, verify token, atau raw credential.
- Error response JSON yang konsisten dan tidak mengembalikan stack trace kepada client.

Contoh response `/health`:

```json
{
  "status": "ok",
  "database": "ok"
}
```

Status `ok` harus didukung pemeriksaan koneksi DB, bukan hanya menunjukkan proses HTTP hidup. Bila DB gagal, kembalikan status service yang tidak sehat (misalnya HTTP 503).

**Selesai jika:** `cargo run` menjalankan binary `backend` di port `8080`, `GET /health` merespons, database connect, dan proses berhenti dengan graceful shutdown.

---

# 6. Tahap 2 — Database dan migration SQLx

> ✅ **STATUS: SELESAI (5 Oktober 2026)** — migrasi 3 file sudah jalan di DB `sentinel`; ditambah `src/models/` (Finding strict per kontrak, Snapshot), `src/repositories/` (11 modul: users/theses/snapshots/findings/compliance/alerts/credits/traces/inbound), `lib.rs` sebagai crate library, dan `tests/repository_tests.rs` — 4 test repository PASS terhadap PostgreSQL lokal.


## 6.1 Pola repository

Pisahkan query SQL dari handler HTTP:

- route/handler: parse request, validasi dasar, panggil service, bentuk response;
- service: aturan bisnis/orchestration;
- repository: query PostgreSQL melalui SQLx;
- model/DTO: bentuk data dan serialisasi.

Gunakan transaction untuk operasi multi-tabel yang harus konsisten. Gunakan parameterized query/SQLx bind, bukan string concatenation untuk nilai dari pengguna.

## 6.2 Tabel inti wajib

### `snapshots`

Simpan observasi historis yang berasal dari data Sectors/MCP:

- `id UUID PRIMARY KEY`
- `ticker VARCHAR(4) NOT NULL`
- `subsector TEXT NOT NULL`
- `metric_name TEXT NOT NULL`
- `value NUMERIC` (nilai numerik hasil normalisasi; nilai non-numerik tidak dipakai sebagai bukti)
- `period TEXT NOT NULL`
- `source TEXT NOT NULL`
- `observed_at TIMESTAMPTZ NOT NULL`
- `raw_data JSONB` untuk evidence mentah yang boleh disimpan
- `created_at TIMESTAMPTZ NOT NULL DEFAULT now()`
- unique constraint yang mencegah duplikasi observasi yang sama, minimal atas `ticker`, `metric_name`, `period`, dan `source`.

Tambahkan index untuk pencarian berdasarkan `(ticker, metric_name, observed_at DESC)` dan `(ticker, metric_name, period)`.

### `findings`

Simpan semua Finding yang diterima untuk pemeriksaan dan audit, termasuk temuan yang ditolak:

- `id UUID PRIMARY KEY`
- sepuluh field kontrak resmi: `ticker`, `subsector`, `metric_name`, `current_value`, `previous_value`, `period`, `source`, `observed_at`, `confidence_score`, `finding_summary`
- metadata internal di luar payload kontrak: `validation_status` (`pending`, `accepted`, `rejected`), `rejection_reason`, `created_at`
- jika diperlukan, simpan data mentah yang tervalidasi sebagai JSONB terpisah. Jangan mengeluarkan kolom internal ini sebagai bagian dari payload `Finding` yang strict.

`current_value` dan `previous_value` sebaiknya disimpan sebagai `NUMERIC` nullable agar temuan yang gagal verifikasi dapat dicatat, tetapi alert hanya boleh menggunakan nilai numerik yang valid.

### `compliance_audit_logs`

Setiap percobaan pemeriksaan harus punya jejak:

- `id UUID PRIMARY KEY`
- `finding_id UUID` FK ke `findings`, nullable untuk kegagalan sebelum Finding dibuat
- `is_compliant BOOLEAN NOT NULL`
- `evidence_verified BOOLEAN NOT NULL`
- `prohibited_words_detected JSONB NOT NULL DEFAULT '[]'`
- `rejection_reason TEXT`
- `stage TEXT NOT NULL` (misalnya `finding`, `presenter_payload`, `whatsapp_dispatch`)
- `details JSONB`
- `created_at TIMESTAMPTZ NOT NULL DEFAULT now()`

Jangan hanya menyimpan hasil yang berhasil. Penolakan justru merupakan bukti penting untuk audit/demo.

## 6.3 Tabel operasional yang dibutuhkan oleh workflow produk

Tambahkan tabel pendukung seperlunya, minimal:

### `users`

- `id UUID PRIMARY KEY`
- `wa_number TEXT UNIQUE NOT NULL` (format ter-normalisasi tanpa tanda `+` untuk API WhatsApp)
- `opt_in BOOLEAN NOT NULL DEFAULT false`
- `muted_until TIMESTAMPTZ NULL`
- `last_inbound_at TIMESTAMPTZ NULL`
- `created_at TIMESTAMPTZ NOT NULL DEFAULT now()`

Jangan mengembalikan nomor mentah dari API dashboard. Masking dilakukan di response layer.

### `theses`

- `id UUID PRIMARY KEY`
- `user_id UUID NOT NULL REFERENCES users(id)`
- `original_text TEXT NOT NULL`
- `ticker VARCHAR(4) NOT NULL`
- `subsector TEXT`
- `status TEXT NOT NULL` dengan nilai internal yang terdokumentasi, paling tidak `PENDING_CONFIRMATION`, `MENGUAT`, `NETRAL`, `MELEMAH`, `BELUM_CUKUP_DATA`
- `active BOOLEAN NOT NULL DEFAULT false`
- `created_at`, `updated_at` TIMESTAMPTZ

Maksimum tiga tesis aktif per pengguna ditegakkan di service dengan transaction/lock yang sesuai, bukan hanya dengan pemeriksaan tak terlindungi yang rentan race condition.

### `thesis_metrics`

- `id UUID PRIMARY KEY`
- `thesis_id UUID NOT NULL REFERENCES theses(id) ON DELETE CASCADE`
- `metric_name TEXT NOT NULL`
- `desired_direction TEXT NOT NULL` (`increase`, `decrease`, atau `any`; jelaskan arti nilai pada kode)
- `created_at TIMESTAMPTZ NOT NULL DEFAULT now()`
- unique `(thesis_id, metric_name)`

Maksimum tiga metric per tesis ditegakkan di service. `desired_direction` berasal dari makna tesis yang diekstrak/ditanyakan kepada pengguna dan divalidasi; jangan menyimpulkannya dari ticker saja.

### `inbound_messages`

- `wa_message_id TEXT PRIMARY KEY` (ID message dari Meta; unique untuk deduplikasi)
- `user_id UUID REFERENCES users(id)`
- `from_number TEXT NOT NULL`
- `body TEXT`
- `processing_status TEXT NOT NULL` (`received`, `processing`, `processed`, `failed`, `ignored`)
- `attempt_count INTEGER NOT NULL DEFAULT 0`
- `last_error TEXT`
- `received_at TIMESTAMPTZ NOT NULL DEFAULT now()`
- `processed_at TIMESTAMPTZ`

### `thesis_findings` atau tabel relasi setara

Gunakan tabel relasi untuk menghubungkan satu Finding pasar dengan tesis pengguna tanpa menambah field non-kontrak ke JSON `Finding`:

- `thesis_id UUID REFERENCES theses(id)`
- `finding_id UUID REFERENCES findings(id)`
- `support_direction TEXT` (`supports`, `weakens`, `neutral`)
- `created_at TIMESTAMPTZ NOT NULL DEFAULT now()`
- unique `(thesis_id, finding_id)`

### `alerts`

- `id UUID PRIMARY KEY`
- `user_id UUID REFERENCES users(id)`
- `thesis_id UUID REFERENCES theses(id)`
- `finding_id UUID REFERENCES findings(id)`
- `metric_name TEXT NOT NULL`
- `period TEXT NOT NULL`
- `payload JSONB NOT NULL`
- `status TEXT NOT NULL` (`pending`, `sent`, `failed`, `deferred`, `rejected`)
- `error_message TEXT`
- `created_at TIMESTAMPTZ NOT NULL DEFAULT now()`
- `sent_at TIMESTAMPTZ`
- unique `(user_id, thesis_id, metric_name, period)` untuk mencegah alert indikator/periode yang sama dikirim berulang.

### `credit_usage`

- `id UUID PRIMARY KEY`
- `endpoint TEXT NOT NULL`
- `credits NUMERIC NOT NULL CHECK (credits >= 0)`
- `from_cache BOOLEAN NOT NULL`
- `call_reference TEXT`
- `recorded_at TIMESTAMPTZ NOT NULL DEFAULT now()`

Biaya aktual dikirim oleh Representative berdasarkan pengukuran akun Sectors. Jangan mengarang biaya endpoint di kode backend.

### `agent_traces`

- `id UUID PRIMARY KEY`
- `agent TEXT NOT NULL`
- `action TEXT NOT NULL`
- `outcome TEXT NOT NULL`
- `ticker VARCHAR(4)`
- `details JSONB`
- `created_at TIMESTAMPTZ NOT NULL DEFAULT now()`

Tambahkan tabel `conversation_state` hanya jika dibutuhkan untuk state dialog seperti konfirmasi penghapusan; jika bisa dimodelkan dengan kolom/tabel yang sudah ada tanpa membuat state ambigu, hindari tabel tambahan yang tidak perlu.

## 6.4 SQLx migration

1. Buat migration dengan `sqlx migrate add ...` jika SQLx CLI sudah tersedia. Jika tidak tersedia, buat file migration dengan nama timestamp/urutan yang konsisten sesuai konvensi SQLx.
2. Implementasikan foreign key, unique constraint, check constraint, dan index.
3. Jalankan migration terhadap database kosong.
4. Jalankan migration kedua kali untuk memastikan startup/deploy tidak mencoba membuat tabel ulang secara manual.
5. Tambahkan repository queries untuk insert/upsert snapshot, insert Finding, read snapshots previous/current, update thesis, insert inbound message, insert alert, record credit, insert trace, dan read data dashboard.
6. Query untuk mencari snapshot sebelumnya harus menggunakan periode/timestamp yang valid dan pengurutan eksplisit; jangan membandingkan string periode secara alfabetis jika format periodenya tidak dijamin sortable.

**Selesai jika:** semua migration berjalan pada PostgreSQL kosong, FK dan constraint valid, dan operasi repository utama memiliki test.

---

# 7. Tahap 3 — API internal dan API dashboard

> ✅ **STATUS: SELESAI (5 Oktober 2026)** — endpoint `/snapshots` (POST/GET), `/findings`, `/jejak`, `/kredit`, `/pantauan`, `/internal/snapshots/previous`, `/internal/findings`, dan `/api/tesis` `/api/findings` `/api/ditolak` `/api/jejak` `/api/kredit`; endpoint internal dijaga `INTERNAL_API_TOKEN` (constant-time compare); Finding divalidasi deterministik + cross-check snapshot + audit; masking nomor WA; 5 test API + 9 unit test PASS.

Semua endpoint untuk pertukaran data tim harus melalui Axum; anggota lain tidak boleh mengakses PostgreSQL langsung. Lindungi endpoint internal dengan `Authorization: Bearer <INTERNAL_API_TOKEN>` jika bisa dijangkau jaringan. Jangan memakai token literal di source code.

## 7.1 Endpoint status

### `GET /health`

Tujuan: memeriksa proses dan koneksi database.

### `POST /snapshots`

Menerima snapshot hasil dari MCP/Representative atau adapter ingestion.

Contoh body internal:

```json
{
  "ticker": "BBCA",
  "subsector": "banks",
  "metric_name": "net_profit_margin",
  "value": 46.85,
  "period": "Q3 2024",
  "source": "Sectors API v2 /v2/company/report/BBCA",
  "observed_at": "2026-10-02T10:15:30Z",
  "raw_data": {"example": true}
}
```

Validasi ticker, metric registry, periode, source, nilai, dan timestamp. Snapshot duplikat ditangani dengan upsert/unique constraint yang eksplisit; jangan membuat observasi kedua seolah data baru.

### `GET /snapshots?ticker=BBCA&metric_name=net_profit_margin`

Mengembalikan observasi sekarang dan sebelumnya/riwayat terbatas yang disimpan. Validasi query parameter. Tidak memanggil Sectors API untuk sekadar membaca histori.

### `GET /pantauan`

Mengembalikan ticker + metric aktif yang perlu dipantau, tanpa nomor WhatsApp mentah. Data bersumber dari tesis aktif dan `thesis_metrics`.

### `POST /findings`

Menerima **payload `Finding` resmi**. Jangan menambahkan property internal ke JSON kontrak ini. Proses melalui service pipeline/validation yang sama, simpan audit, dan kembalikan `finding_id`, status (`accepted`/`rejected`), dan alasan penolakan yang aman.

Contoh sukses:

```json
{
  "finding_id": "<uuid>",
  "status": "accepted",
  "rejection_reason": null
}
```

Contoh ditolak:

```json
{
  "finding_id": "<uuid-atau-null>",
  "status": "rejected",
  "rejection_reason": "current_value tidak cocok dengan snapshot tersimpan"
}
```

### `POST /jejak`

Menyimpan langkah agent secara terstruktur, misalnya agent `Scout`, action `evidence_fetched`, outcome singkat, ticker, dan metadata non-rahasia.

### `POST /kredit`

Menyimpan pemakaian kredit yang dilaporkan Representative: endpoint, credits, from_cache, dan reference bila ada. Endpoint ini tidak boleh menerima token/key.

## 7.2 Endpoint internal untuk integrasi MCP

Transport MCP pada dokumen proyek disebut dapat berupa stdio/SSE atau sesuai implementasi tim. **Periksa transport sebenarnya sebelum coding. Jangan mengarang URL MCP.**

Karena Axum satu-satunya pemilik PostgreSQL, sediakan endpoint internal terlindungi jika MCP server perlu membaca snapshot sebelumnya atau menyimpan Finding:

- `GET /internal/snapshots/previous?ticker=...&metric_name=...`
- `POST /internal/findings` untuk memasukkan hasil yang sudah melewati service validasi Axum.

Kunci endpoint ini dengan `INTERNAL_API_TOKEN`. MCP server harus mendelegasikan operasi database melalui endpoint ini, bukan membuka pool PostgreSQL sendiri. Pipeline Axum yang sedang berjalan tidak perlu memanggil dirinya sendiri lewat MCP untuk menyimpan Finding; ia boleh menggunakan repository/service langsung.

Jika tim sudah punya kontrak internal setara, gunakan kontrak tim itu dan perbarui dokumentasi, jangan membuat endpoint duplikat.

## 7.3 Endpoint untuk dashboard

Sediakan setidaknya:

- `GET /api/tesis` — daftar tesis dan status, tanpa nomor mentah.
- `GET /api/findings` — Finding yang diterima beserta nilai/sumber/periode.
- `GET /api/ditolak` — Finding atau payload yang ditolak beserta alasan.
- `GET /api/jejak` — urutan kerja agent.
- `GET /api/kredit` — budget konfigurasi, kredit terpakai, kredit tersisa, dan waktu update.

Tambahkan batas `limit`/pagination yang wajar dan stabil. Jangan mengembalikan API key, credential, raw webhook signature, atau nomor WhatsApp mentah. Masking dilakukan di backend; frontend tidak boleh menjadi satu-satunya lapisan privasi.

Bentuk respons konsisten, terdokumentasi, dan diujikan. Dashboard dikerjakan oleh anggota yang ditunjuk; Rafi menyediakan data JSON.

**Selesai jika:** API bisa dites dengan curl/HTTP test atau mock tanpa mengakses database secara langsung dari agent lain.

---

# 8. Tahap 4 — WhatsApp Cloud API client

> ✅ **STATUS: SELESAI (5 Oktober 2026)** — `services/whatsapp_client.rs` (`send_text`/`send_template`/`send_alert`), DTO `WhatsAppAlertPayload` sesuai CONTRACTS.md, mock transport (`mock_sent`, catatan request), real mode POST ke Graph API dengan cek status HTTP + message_id, hard gate `DISCLAIMER_POLICY_CONFLICT` (mock melaporkan `blocked_by_policy_conflict`), token tidak masuk log. 5 test WhatsApp PASS.

Buat service terpisah, misalnya `services/whatsapp_client.rs`. Jangan memasukkan HTTP request Meta langsung ke setiap handler.

## 8.1 Pengiriman pesan

Implementasikan fungsi dengan tanggung jawab jelas, misalnya:

- `send_text(recipient, text)` untuk pesan text saat jendela layanan 24 jam tersedia.
- `send_template(recipient, template_name, language, parameters)` untuk template yang telah disetujui jika diperlukan di luar jendela layanan.
- `send_alert(WhatsAppAlertPayload)` untuk memvalidasi payload internal lalu memetakan ke format request WhatsApp Cloud API.

Endpoint Meta dibangun dari `WHATSAPP_GRAPH_API_VERSION` dan `WHATSAPP_PHONE_NUMBER_ID` dari environment. Header `Authorization: Bearer ...` dibuat pada saat request; token tidak masuk URL/log.

`WhatsAppAlertPayload` internal mengikuti schema `CONTRACTS.md`:

```json
{
  "recipient_number": "6281234567890",
  "header": "BBCA · Net Profit Margin",
  "body": "Nilai periode pembanding: 43.12. Nilai terbaru: 46.85. Periode: Q3 2024. Sumber: Sectors API v2.",
  "disclaimer": "Disclaimer: Informasi ini hanya bersifat edukasi & pemantauan data historis. Bukan anjuran investasi atau rekomendasi transaksi."
}
```

Angka di contoh hanya untuk ilustrasi/test. Pada runtime, nilai harus berasal dari Finding dan snapshot tervalidasi.

Mapping ke API Meta dilakukan di client: format resmi request Meta dibedakan dari DTO internal `WhatsAppAlertPayload`. Normalisasi `recipient_number` mengikuti pola kontrak (digit tanpa tanda `+`, 10–15 digit, dimulai 1–9). Jangan menganggap sukses hanya karena HTTP request tidak melempar error: periksa status HTTP dan response ID message dari Meta, kemudian simpan `status_kirim`/message ID.

## 8.2 Penanganan error

- Timeout jaringan harus memiliki batas waktu.
- Tangani HTTP error dan response Meta dengan pesan aman.
- Jangan retry tanpa batas. Retry hanya error transien, dengan batas dan jeda yang masuk akal.
- Simpan kegagalan pengiriman ke `alerts.status = failed` dan audit/trace; jangan menandai `sent` sebelum provider mengonfirmasi.
- Jangan mengirim ulang alert tanpa cek idempotensi.
- Jangan menaruh access token di error log.

## 8.3 Mode mock

Dalam `APP_MODE=mock`, gunakan fake transport yang menyimpan request tiruan/log lokal terstruktur. Fake tidak boleh menyatakan bahwa pesan sungguhan sudah sampai. Beri response seperti `mock_sent` dan tampilkan jelas di log/test. Dalam `APP_MODE=real`, jalankan HTTP request yang nyata.

## 8.4 Tes pengiriman nyata

Dengan kredensial milik pemilik akun yang disimpan lokal:

1. Pastikan nomor penerima demo sudah diizinkan/terdaftar di Meta.
2. Kirim template `hello_world` atau template yang telah disetujui.
3. Jalankan fungsi pengiriman dari Rust, bukan lewat Postman saja.
4. Pastikan HP menerima pesan dan simpan bukti hasil test tanpa mengekspos token atau nomor penuh di repository.

**Selesai jika:** mock tests lolos, dan jika credentials tersedia, satu pesan nyata dikirim dengan response Meta yang valid. Jika tidak ada credentials, tandai integrasi nyata sebagai `blocked: credentials unavailable`, bukan gagal secara fungsional atau sukses.

---

# 9. Tahap 5 — WhatsApp webhook, signature, dan deduplikasi

## 9.1 `GET /webhook/whatsapp`

Implementasikan verification Meta dengan query parameter `hub.mode`, `hub.verify_token`, dan `hub.challenge`:

- Jika mode dan verify token cocok, balas challenge persis sesuai format verification.
- Jika token tidak cocok/parameter tidak valid, kembalikan status penolakan yang tepat.
- Jangan mencetak verify token atau challenge sensitif yang tidak diperlukan ke log.

## 9.2 `POST /webhook/whatsapp`

Urutan wajib:

1. Ambil **raw request body bytes** sebelum JSON parsing.
2. Ambil header `X-Hub-Signature-256`.
3. Hitung HMAC-SHA256 dari raw body menggunakan `WHATSAPP_APP_SECRET`.
4. Bandingkan signature dengan cara constant-time; jangan memakai perbandingan string biasa untuk rahasia.
5. Bila signature tidak valid, tolak request dan jangan proses pesan.
6. Setelah signature valid, parse JSON webhook Meta.
7. Ambil `message_id`/`wamid`, nomor pengirim, tipe pesan, dan isi text bila ada.
8. Masukkan pesan ke `inbound_messages` memakai unique primary key `wa_message_id`.
9. Bila ID sudah ada dan pesan telah diproses, jangan menjalankan workflow lagi; balas 200 untuk mencegah retry berulang dari provider.
10. Abaikan event status delivery yang bukan pesan masuk dan tangani media/non-text dengan response yang ramah/aman bila belum didukung.
11. Catat status pemrosesan (`received`, `processing`, `processed`, `failed`, `ignored`) agar kesalahan dapat ditelusuri.
12. Jangan memproses payload dari user yang tidak diizinkan sebagai demo atau belum opt-in untuk alert; jangan mengirim alert proaktif sebelum ketentuan opt-in terpenuhi.

Hindari memproses pesan ganda hanya dengan `HashSet` di memori. Deduplikasi harus tetap berlaku setelah proses restart, menggunakan constraint unik PostgreSQL.

## 9.3 Payload parser

Buat DTO Serde untuk struktur webhook yang dibutuhkan dan uji terhadap fixture yang disanitasi. Jangan mengasumsikan semua webhook memiliki `messages`; status update/provider events bisa mempunyai struktur berbeda. Parser harus menangani field opsional tanpa panic.

## 9.4 Tunneling lokal

Untuk uji real, webhook perlu URL publik melalui tunnel yang disetujui tim (contoh ngrok/Cloudflare Tunnel). Jangan simpan URL tunnel sementara sebagai konfigurasi production. Dokumentasikan cara menjalankan tunnel secara lokal; jangan mengklaim webhook real sudah berfungsi jika request hanya dites dengan fixture lokal.

**Selesai jika:** challenge verification lolos, signature valid diterima, signature invalid ditolak, satu `wamid` diproses sekali, dan replay tidak membuat side effect kedua.

---

# 10. Tahap 6 — Workflow tesis WhatsApp dan Gemini extractor

## 10.1 Command router

Normalisasi command secara aman (trim, case-insensitive bila relevan). Dukungan minimum:

| Command | Perilaku wajib |
|---|---|
| `/tesis` | Memulai workflow input tesis; kalimat biasa juga dapat diperlakukan sebagai tesis jika tidak dikenali sebagai command. |
| `YA` | Mengaktifkan tesis terakhir yang menunggu konfirmasi milik nomor pengirim tersebut. |
| `TIDAK` | Membatalkan tesis yang menunggu konfirmasi, bila diimplementasikan sebagai opsi konfirmasi. |
| `/status` | Mengembalikan tesis aktif dan status informasi saat ini. |
| `/bukti` | Mengembalikan data sumber/Finding terakhir dari database, tanpa memanggil Sectors API lagi. |
| `/diam` | Menghentikan alert proaktif sampai dilanjutkan. |
| `/lanjut` | Mengaktifkan kembali alert proaktif. |
| `/hapus` | Meminta pengguna memilih/mengonfirmasi tesis yang dihapus; jangan hapus semua tanpa konfirmasi. |
| `/bantuan` | Menampilkan daftar command dan disclaimer sesuai keputusan kepatuhan yang disetujui. |
| `/kredit` | Menampilkan meter kredit hanya kepada nomor admin yang dikonfigurasi. |

Pesan yang tidak dikenali harus dibalas dengan bantuan, bukan ditebak secara bebas. Command yang diproses tetap harus melalui deduplikasi ID pesan.

## 10.2 Ekstraksi tesis oleh Gemini

1. Gunakan prompt/JSON structured-output milik Dian dari `docs/PROMPTS.md` jika tersedia.
2. Input termasuk kalimat tesis pengguna, bukan data rahasia lain.
3. Minta keluaran berisi ticker dan metric names **hanya dari registry yang disetujui**, serta arah yang dimaksud oleh pengguna untuk setiap metric jika dibutuhkan (`increase`, `decrease`, `any`). Bentuk JSON internal harus disepakati dengan prompt Dian; jangan mengubah kontrak `Finding` untuk menampung field ekstraksi ini.
4. Rust mem-parse JSON ke DTO yang ketat; reject output malformed, extra fields bila schema melarang, ticker yang tidak valid, metric di luar whitelist, atau jumlah metric lebih dari tiga.
5. Jika teks pengguna ambigu (“laba bagus” tetapi tidak jelas metrik mana yang bisa diobservasi) atau metric tidak tersedia, jangan menebak. Minta pengguna memilih salah satu metric yang tersedia.
6. Jangan biarkan LLM menulis SQL, URL, endpoint, atau query database bebas.
7. Simpan tesis dalam status `PENDING_CONFIRMATION`, lalu kirim ringkasan metric yang akan dipantau.
8. Hanya setelah pengguna membalas `YA`, lakukan transaction untuk memastikan batas maksimum tiga tesis aktif per pengguna, lalu set `active=true` dan `status` awal (`BELUM_CUKUP_DATA` atau status awal yang disepakati).
9. Tesis yang sedang menunggu konfirmasi tidak termasuk hitungan tesis aktif, tetapi jumlah pending per pengguna boleh dibatasi agar tidak menumpuk.

Contoh format balasan (teks yang dikirim dari template, bukan LLM bebas):

```text
Saya akan memantau tesis berikut:
Ticker: BBCA
Metric: net_profit_margin
Arah yang dimaksud: increase

Balas YA untuk mengaktifkan pemantauan.
```

Jangan membuat status aktif sebelum `YA`. User harus dapat menghentikan notifikasi dengan `/diam` dan menghapus tesis melalui workflow konfirmasi.

## 10.3 Sanitasi nomor telepon

Simpan nomor dalam satu format kanonis yang cocok dengan WhatsApp Cloud API. Semua endpoint dan dashboard yang menampilkan nomor harus menggunakan satu helper masking di backend, misalnya `+62 812-••••-7890`; jangan menampilkan nomor penuh di log atau video demo.

**Selesai jika:** test membuktikan input tesis menghasilkan tesis pending, metric yang tak dikenal ditolak, `YA` mengaktifkan tesis, batas tiga tesis diterapkan, dan data nomor disamarkan.

---

# 11. Tahap 7 — MCP gateway dan sumber data

MCP server/Rust `rmcp` dibuat oleh Representative. Rafi membuat **MCP client/gateway adapter** untuk dipanggil oleh `AgentPipeline`.

## 11.1 Enam MCP tools yang harus dikenali oleh adapter

Sesuai `ARCHITECTURE.md`:

1. `list_subsectors` — daftar subsektor.
2. `screen_companies` — menyaring emiten berdasarkan subsektor/parameter yang diizinkan.
3. `get_subsector_report` — statistik agregat subsektor.
4. `get_company_evidence` — bukti/laporan fundamental untuk ticker/periode.
5. `get_previous_snapshot` — pembanding historis dari database milik backend.
6. `record_finding` — registrasi Finding melalui jalur yang telah diverifikasi.

Sesuai pembagian tim, jangan implementasikan Sectors HTTP client atau MCP server itu sendiri di bagian Rafi. Fokus pada komunikasi client dan penanganan response/error.

## 11.2 Transport

1. Periksa dokumentasi MCP server yang benar-benar tersedia dan pilihan transport yang disepakati (stdio/SSE/HTTP).
2. Jika implementasi nyata sudah tersedia, pakai transport dan nama tools persis seperti kontrak Representative.
3. Jika belum tersedia, buat trait seperti `McpGateway` dan `MockMcpGateway` untuk test pipeline. Siapkan adapter real terpisah agar dapat dilengkapi tanpa mengubah `AgentPipeline`.
4. Jangan menebak URL, command binary, atau schema response MCP. Bila transport belum disepakati, catat blocker di `docs/integration-decisions.md` dan beri status integrasi real sebagai belum terverifikasi.
5. Tangani MCP error sebagai tipe error terstruktur dan log ringkas tanpa secret.

## 11.3 Proteksi Sectors API v2 dan HTTP 410

Backend harus mengenali error `API_VERSION_DEPRECATED`/HTTP `410` dari MCP dan:

- tidak mengalihkan ke endpoint v1;
- tidak menebak data atau menggunakan response lama sebagai data baru;
- mencatat kegagalan di `agent_traces`;
- tidak menghasilkan alert dari panggilan yang gagal;
- mengembalikan error yang mudah ditindaklanjuti.

`BASE_URL` dan path Sectors v2 terutama dikelola oleh Representative. Rafi tidak menyimpan key Sectors jika bukan pemilik key.

## 11.4 Snapshot cold-start

Pipeline membutuhkan dua observasi yang benar-benar dapat dibandingkan. Bila snapshot sebelumnya belum ada:

- simpan snapshot terbaru sebagai baseline;
- log “baseline created / insufficient comparison data”;
- jangan mengeluarkan perubahan palsu dari nilai sebelumnya yang tidak ada;
- baru buat Finding perubahan setelah current dan previous tersedia.

**Selesai jika:** `MockMcpGateway` menjalankan unit/integration test lengkap, dan adapter real dapat diuji saat Representative menyediakan transport dan binary.

---

# 12. Tahap 8 — `orchestrator::AgentPipeline`

Ini deliverable utama M-03. Bentuk pipeline sebagai service yang komponennya mudah dites dan bukan satu fungsi besar di handler.

## 12.1 Kontrak tanggung jawab pipeline

Pipeline melakukan:

1. mengambil daftar tesis aktif dan metric yang dipantau;
2. meminta data ke MCP gateway;
3. menyimpan snapshot dan membaca snapshot sebelumnya dari repository;
4. membuat data perbandingan yang konsisten;
5. menghitung delta/arah secara deterministik di Rust;
6. membangun objek `Finding` sesuai `CONTRACTS.md`;
7. menjalankan Evidence/Compliance checks dan menulis audit log;
8. menyimpan Finding berstatus diterima/ditolak;
9. menghubungkan Finding yang lolos dengan tesis relevan;
10. menghitung status tesis;
11. membuat payload pesan menggunakan template kode;
12. menerapkan deduplikasi/cooldown/mute sebelum pengiriman;
13. mengirim alert melalui WhatsApp client;
14. mencatat hasil di database dan `agent_traces`.

Gunakan struktur hasil yang eksplisit, misalnya `PipelineRunReport` dengan count snapshot, finding accepted, finding rejected, alert sent, alert deferred, dan errors. Jangan mengembalikan sukses global bila sebagian tahap gagal tanpa tercatat.

## 12.2 Urutan eksekusi yang disarankan

```text
run_cycle()
  ├─ load_watchlist()
  ├─ for each unique ticker/metric:
  │    ├─ fetch_evidence_through_mcp()
  │    ├─ normalize_and_store_snapshot()
  │    └─ load_previous_snapshot()
  ├─ build_candidate_finding()
  ├─ deterministic_evidence_validation()
  ├─ compliance_service.check_finding()
  ├─ persist_finding_and_audit()
  ├─ associate_finding_to_theses()
  ├─ recompute_thesis_status()
  ├─ for each eligible alert:
  │    ├─ check_mute_and_daily_limit()
  │    ├─ reserve_unique_alert_key()
  │    ├─ build_whatsapp_payload()
  │    ├─ validate_payload_and_policy()
  │    ├─ dispatch_whatsapp()
  │    └─ persist_delivery_status()
  └─ persist_pipeline_summary_trace()
```

Simpan checkpoint/audit pada fase penting agar kegagalan HTTP eksternal tidak menghilangkan jejak database.

## 12.3 Perhitungan numerik

- Hitung semua delta di Rust; Gemini tidak boleh menghitung `current_value`, `previous_value`, selisih, persen, atau status tesis.
- Untuk perubahan relatif, formula lazim adalah `(current - previous) / abs(previous)`; dokumentasikan formula dan perlakuan nilai sebelumnya nol. Jangan menghasilkan `NaN`/infinity.
- `NEUTRAL_RELATIVE_THRESHOLD=0.02` adalah nilai awal yang pernah diusulkan pada brief, **belum merupakan hasil validasi data**. Jadikan konfigurasi dan tuliskan sebagai asumsi sementara. Uji nilai di bawah, tepat pada, dan di atas threshold. Jangan mengunci threshold sebagai fakta bisnis final tanpa review tim.
- Perlakuan metric dengan arah `decrease` berbeda dari metric `increase`. Contoh: naiknya metrik bukan selalu dukungan jika metric tersebut dimaksudkan turun. Untuk `any` atau arah yang ambigu, status arah harus netral sampai definisi dipastikan.
- Bila data null/non-numerik, period mismatch, ticker mismatch, atau snapshot belum cukup, tidak boleh menyusun alert perubahan.
- Jangan membandingkan nilai untuk periode yang tidak sebanding tanpa dokumentasi (misalnya FY vs Q tanpa normalisasi).

## 12.4 Membentuk `Finding`

Bangun field kontrak dari data yang sudah dinormalisasi:

- `ticker`: kode ticker tervalidasi.
- `subsector`: subsektor resmi dari data Sectors/MCP, bukan tebakan LLM.
- `metric_name`: metric dari registry whitelist.
- `current_value`: snapshot saat ini.
- `previous_value`: snapshot pembanding yang cocok.
- `period`: label periode current yang jelas.
- `source`: endpoint/dataset sumber yang nyata.
- `observed_at`: timestamp RFC3339/ISO 8601.
- `confidence_score`: nilai 0..1 dengan dasar yang dijelaskan. Jika belum ada model confidence yang disepakati, buat aturan deterministik konservatif yang didokumentasikan atau minta sumber confidence dari agent sesuai schema; jangan mengarang presisi palsu.
- `finding_summary`: ringkasan faktual maksimal 500 karakter, tanpa arahan investasi.

Strict validation harus menolak property tambahan dan format yang tidak sesuai. Jangan masukkan `thesis_id`, `support_direction`, atau status persistence ke object JSON `Finding`; metadata itu disimpan di tabel relasi/kolom internal.

## 12.5 Status tesis

Status dihitung dari Finding yang lolos dan sesuai metric thesis, bukan dari pesan LLM:

- **MENGUAT** bila Finding mendukung tesis lebih banyak daripada yang melemahkan.
- **MELEMAH** bila Finding yang melemahkan lebih banyak.
- **NETRAL** bila hitungan dukungan dan pelemahan sama.
- Jika tidak ada data baru yang memenuhi syarat, jangan membuat perubahan status palsu; simpan status yang ada dan catat “belum ada pembaruan”/`BELUM_CUKUP_DATA` sesuai model yang disetujui.

Satu Finding yang sama bisa relevan ke beberapa tesis. Tentukan `support_direction` per hubungan Finding–tesis berdasarkan `desired_direction`, bukan menulis field baru pada Finding global.

**Selesai jika:** unit test memperlihatkan status benar untuk kasus mendukung, melemahkan, tie/netral, metric tanpa arah, baseline kosong, nilai nol, data tak valid, dan Finding duplikat.

---

# 13. Tahap 9 — Evidence Gate dan Compliance audit

Gunakan schema `ComplianceVerificationResult` yang ada di `CONTRACTS.md`:

```json
{
  "finding_id": "<string>",
  "is_compliant": true,
  "prohibited_words_detected": [],
  "evidence_verified": true,
  "rejection_reason": null
}
```

Schema tersebut memerlukan `finding_id`, `is_compliant`, `prohibited_words_detected`, `evidence_verified`, dan `rejection_reason`.

## 13.1 Validasi deterministik minimum

Finding boleh diteruskan ke tahap alert hanya jika:

1. payload cocok dengan kontrak `Finding` dan tidak memiliki field tambahan;
2. `ticker` cocok pola kontrak `^[A-Z]{4}$`;
3. `subsector`, `metric_name`, `period`, dan `source` tersedia serta tidak hanya whitespace;
4. `metric_name` berada dalam metric registry yang disetujui;
5. `current_value` dan `previous_value` dapat dinormalisasi menjadi angka finite;
6. ticker, metric, periode, nilai, dan source konsisten dengan snapshot di database;
7. snapshot pembanding adalah periode yang benar dan tidak tertukar;
8. `confidence_score` berada pada rentang 0.0–1.0;
9. `observed_at` merupakan timestamp valid;
10. periode/metric/ticker belum menghasilkan alert yang sama untuk tesis itu;
11. `finding_summary` mematuhi panjang schema dan policy yang disetujui.

Jika salah satu pemeriksaan evidensi gagal, set Finding `rejected`, simpan alasan jelas dan audit row, lalu jangan kirim alert. Jangan membuang penolakan tanpa catatan.

## 13.2 Kata terlarang dan LLM output

Rule engine Dian adalah pemilik utama compliance logic. Integrasikan jika tersedia; backend tetap mempunyai hard gate sebelum pengiriman. Minimal pemeriksaan case-insensitive harus mencakup istilah yang tercantum pada dokumen tim, termasuk `beli`, `jual`, `tahan`, `rekomendasi`, `target harga`, dan `cuan`, pada teks dinamis yang akan dikirim. Perhatikan frasa multi-kata dan batas kata agar tidak salah mendeteksi potongan kata yang tidak relevan.

Jangan mengirim output LLM secara langsung. Periksa ringkasan, angka, field payload, dan disclaimer. Angka utama pada pesan dibangun dari nilai `Finding` menggunakan formatter deterministik.

Catat semua penolakan di `compliance_audit_logs` dengan `evidence_verified=false` jika evidence tidak cocok dan isi `prohibited_words_detected` bila ada.

## 13.3 Jangan samakan schema-valid dengan alert-eligible

JSON dengan `current_value: null` bisa valid terhadap union type di schema, tetapi tetap tidak memenuhi persyaratan bukti numeric untuk alert. Gunakan status yang jelas, misalnya `schema_valid=true`, `evidence_verified=false`, `is_compliant=false`, `rejection_reason=missing_numeric_evidence`.

**Selesai jika:** minimal satu Finding valid diterima; missing value, metric asing, mismatch snapshot, periode salah, summary melanggar policy, dan duplicate ditolak dengan alasan dan audit log.

---

# 14. Tahap 10 — Kartu bukti dan presenter

Presenter boleh dibantu Gemini melalui prompt yang sudah disepakati Dian, tetapi **template dan angka akhir dibentuk/ditetapkan oleh Rust**.

## 14.1 Payload internal

Bangun DTO yang sesuai `WhatsAppAlertPayload`:

- `recipient_number`: angka nomor yang sudah dinormalisasi, tanpa `+`, 10–15 digit sesuai schema.
- `header`: maksimal 100 karakter.
- `body`: maksimal 2048 karakter.
- `disclaimer`: sama persis dengan konstanta di `CONTRACTS.md` sampai tim mengubah kontrak secara resmi.

## 14.2 Isi kartu bukti

Kartu bukti menyertakan sekurang-kurangnya:

- ticker + nama metric yang mudah dipahami;
- nilai previous dan current yang bersumber dari Finding;
- periode;
- status tesis (Menguat/Netral/Melemah);
- sumber;
- confidence score yang bisa ditampilkan secara jujur bila dibutuhkan;
- keterangan bahwa ini informasi/pemantauan, bukan nasihat investasi;
- petunjuk `/bukti` bila tetap sesuai format produk.

Jangan menampilkan confidence score sebagai “98% pasti” jika definisi confidence belum divalidasi; label dan cara interpretasinya harus sesuai keputusan tim.

## 14.3 Validasi nomor di pesan

- Angka data utama harus sama dengan data yang tersimpan di Finding.
- Jika Presenter LLM menulis angka yang tidak terdapat pada evidence, buang atau reject ringkasannya.
- Pilihan paling aman: gunakan LLM untuk satu kalimat tanpa angka baru, lalu render nilai melalui template kode.
- Validasi field dinamis terhadap daftar kata terlarang sebelum request Meta dibuat.
- Validasi disclaimer secara terpisah dan tandai konflik spesifikasi pada Bagian 1.6. Jika `POLICY_DISCLAIMER_CONFLICT_ACKNOWLEDGED=false`, blokir dispatch real dengan reason `DISCLAIMER_POLICY_CONFLICT`; jangan tandai compliance selesai selama konflik belum diselesaikan.

**Selesai jika:** payload lulus validasi panjang/pola, angka tidak dapat diubah LLM, disclaimer diverifikasi, dan policy violation mencegah pengiriman.

---

# 15. Tahap 11 — Deduplikasi alert, cooldown, mute, dan scheduler

## 15.1 Deduplikasi alert

Satu alert per kombinasi:

`user_id + thesis_id + metric_name + period`

Tegakkan dengan unique constraint database dan transaksi. Pemeriksaan di kode saja tidak cukup untuk race condition.

## 15.2 Batas harian

Maksimum default yang direncanakan adalah tiga alert per pengguna per hari (`MAX_ALERTS_PER_DAY=3`). Nilai konfigurasi dapat diganti. Definisikan batas tanggal menggunakan timezone yang terdokumentasi, dengan WIB/`Asia/Jakarta` jika itu kebijakan tim; hindari membandingkan tanggal server UTC tanpa penjelasan.

Bila batas sudah tercapai:

- jangan kirim alert keempat;
- simpan alert sebagai `deferred`/tertunda;
- simpan alasan dan jejak;
- jika ada beberapa alert pada batch yang sama, urutkan berdasarkan dampak pada status tesis sesuai aturan yang disetujui.

## 15.3 `/diam` dan `/lanjut`

- `/diam`: set `muted_until` atau status mute dalam database. Untuk implementasi pertama, bila durasi belum disepakati, gunakan setting eksplisit (`QUIET_HOURS`) atau mute sampai `/lanjut`; jangan berpura-pura kebijakan durasinya sudah ditetapkan.
- `/lanjut`: hapus mute/atur status aktif.
- Selagi muted, jangan kirim alert proaktif. Balasan langsung terhadap command pengguna masih boleh dikirim selama sesuai policy dan window layanan WhatsApp.
- Gunakan DB untuk mute; jangan hanya menyimpan status di memory.

## 15.4 Scheduler

Implementasikan scheduler sederhana berbasis Tokio atau scheduler crate yang sudah dipakai. Hindari menjalankan dua cycle bersamaan. Gunakan distributed/process lock bila aplikasi bisa berjalan lebih dari satu instance; untuk prototype satu instance, gunakan lock in-process dan dokumentasikan batasannya.

- Interval dikonfigurasi melalui `SCHEDULER_INTERVAL_SECONDS`.
- Untuk real mode, jangan memicu alert berulang hanya karena timer tick; pipeline hanya mengeluarkan alert untuk evidence/periode baru dan unique key mencegah duplikasi.
- Dalam mock mode, sediakan trigger manual terlindungi untuk menjalankan satu cycle demo. Jangan expose trigger ini ke publik tanpa auth.
- Tangani error per ticker/metric agar satu error tidak selalu menghentikan seluruh batch, tetapi catat hasil kegagalan.

**Selesai jika:** test alert 1–3 dapat dikirim, alert ke-4 ditunda, alert duplicate hanya tercatat satu kali, `/diam` memblokir alert proaktif, `/lanjut` mengaktifkannya lagi, dan cycle tidak overlap.

---

# 16. Tahap 12 — Chief/pipeline status dan dispatch

`Chief Presenter Agent` bertugas merangkum hasil, tetapi pembentukan status dan bukti berasal dari aturan kode.

1. Ambil Findings yang accepted saja.
2. Hubungkan ke tesis berdasarkan ticker dan metric yang dipantau.
3. Hitung arah dukungan per metric sesuai `desired_direction`.
4. Agregasikan ke `MENGUAT`, `NETRAL`, atau `MELEMAH` berdasarkan jumlah bukti valid. Jika tidak ada bukti baru, tandai “belum ada pembaruan” dan jangan menciptakan status/alert baru.
5. Jika banyak temuan masuk, urutkan sesuai dampak status tesis memakai aturan deterministic yang disetujui tim; dokumentasikan aturan ranking itu.
6. Panggil Presenter/Gemini hanya untuk kalimat ringkasan bila prompt tersedia. Jika Gemini gagal, jangan mengganti data dengan karangan; gunakan template netral yang dibangun dari evidence atau tandai dispatch gagal sesuai policy.
7. Bangun payload internal.
8. Jalankan compliance check payload dan konflik disclaimer.
9. Periksa mute, quota, dedupe sebelum dispatch.
10. Simpan hasil pengiriman (sent/failed/deferred/rejected) dan message ID provider bila tersedia.
11. Simpan trace untuk Scout, Analyst, Compliance, Presenter, serta langkah dispatch. Jika agent berjalan sebagai deterministic code dan bukan panggilan LLM, trace harus tetap jujur dan tidak mengklaim agent LLM melakukan reasoning yang tidak terjadi.

**Selesai jika:** pipeline mock yang dipicu manual menghasilkan Finding dan trace yang bisa diaudit; Finding invalid ditolak; alert yang sama tidak dikirim dua kali.

---

# 17. Tahap 13 — Meter kredit dan jejak agent

## 17.1 Meter kredit

- Representative mengirim biaya setiap panggilan Sectors ke `POST /kredit`.
- Simpan endpoint, nilai kredit, `from_cache`, waktu, dan call reference bila tersedia.
- `GET /api/kredit` menghitung `total_used` dari row database dan `remaining = max(0, configured_budget - total_used)`.
- `SECTORS_CREDIT_BUDGET` default rencana `1000`, tetapi tampilkan sebagai budget konfigurasi, bukan mengklaim telah cocok dengan akun jika data saldo eksternal belum diverifikasi.
- Endpoint `/kredit` di WhatsApp hanya boleh menampilkan detail kepada `ADMIN_PHONE`. Bandingkan nomor setelah normalisasi.
- Jangan menyimpan key Sectors di row kredit atau log.

## 17.2 Agent trace

Simpan satu trace untuk tiap tahap signifikan, minimal:

- Scout mulai/selesai mengambil evidence;
- snapshot inserted/duplicate/baseline;
- Analyst menghitung perubahan atau melewati data;
- Compliance accepted/rejected beserta alasan aman;
- Presenter menghasilkan payload/ditolak;
- cooldown sent/deferred/muted;
- WhatsApp dispatched/failed;
- siklus selesai dengan summary.

Gunakan hasil singkat yang aman, bukan full prompt, bearer token, API key, raw secret, atau isi pribadi berlebihan. Dashboard API membaca `agent_traces` dari database.

**Selesai jika:** `/api/kredit`, `/kredit` admin, dan `/api/jejak` menampilkan data yang bersumber dari database, bukan angka hard-coded.

---

# 18. Tahap 14 — Pengujian wajib

Jalankan tiap jenis tes setelah fase terkait. Tulis fixture JSON di `tests/fixtures/` dan jangan menggunakan secret asli.

## 18.1 Unit tests

- `Finding` valid sesuai `CONTRACTS.md` diterima oleh parser.
- Field tambahan ditolak.
- Ticker tidak cocok pola ditolak.
- `confidence_score` di luar 0..1 ditolak.
- timestamp malformed ditolak.
- current/previous null atau non-numeric tidak dapat menjadi alert.
- `metric_name` di luar registry ditolak.
- current/previous yang tidak cocok dengan snapshot ditolak.
- source/periode kosong ditolak.
- delta positif/negatif/di bawah threshold/d tepat threshold/di atas threshold.
- previous=0 tidak menghasilkan NaN/Infinity.
- perubahan arah `increase` dan `decrease` dihitung sesuai tesis.
- agregasi MENGUAT/NETRAL/MELEMAH benar.
- nomor WhatsApp dinormalisasi dan dimasking benar.
- kata terlarang pada field dinamis menolak payload; test konflik disclaimer dijalankan secara eksplisit.
- alert quota harian, mute, dan unique idempotency.

## 18.2 Integration/API tests

- `/health` memberi status sesuai kondisi DB.
- `POST /snapshots` + `GET /snapshots` menyimpan dan membaca data.
- `GET /pantauan` hanya menampilkan tesis aktif.
- `POST /findings` menyimpan accepted finding dan compliance audit.
- invalid finding tersimpan sebagai rejected dengan reason.
- `/api/tesis`, `/api/findings`, `/api/ditolak`, `/api/jejak`, `/api/kredit` mengeluarkan response shape konsisten.
- internal endpoints menolak request tanpa token yang valid.

## 18.3 Webhook tests

- GET challenge dengan verify token benar berhasil.
- GET challenge dengan token salah ditolak.
- POST signature benar diproses.
- POST signature salah ditolak dan tidak menambah row.
- payload dengan message ID baru disimpan dan diproses satu kali.
- payload dengan message ID sama tidak menghasilkan side effect kedua.
- status update/non-message tidak menyebabkan panic.
- payload text kosong/media yang belum didukung ditangani deterministik.

## 18.4 Pipeline tests

Gunakan `MockMcpGateway`, `MockGeminiClient`, dan `MockWhatsAppClient`:

1. baseline snapshot baru tanpa pembanding tidak membuat Finding perubahan.
2. data valid menghasilkan Finding sesuai kontrak, audit accepted, status tesis dan trace.
3. mismatch evidence menghasilkan reject + audit + tidak ada send.
4. output Gemini berisi metric asing tidak mengaktifkan tesis.
5. Presenter mengembalikan angka palsu/kata terlarang tidak dikirim.
6. alert duplicate hanya dikirim sekali.
7. alert keempat dalam sehari deferred.
8. muted user tidak menerima alert proaktif.
9. mock mode menandai `mock_sent`, bukan `sent` sungguhan.

## 18.5 Commands minimal sebelum menyatakan selesai

Jalankan dari crate Rust yang benar:

```bash
cargo fmt --check
cargo check
cargo test
```

Jalankan `cargo clippy --all-targets --all-features -- -D warnings` jika bisa tanpa merusak konvensi proyek. Jalankan migration test dengan PostgreSQL lokal/Docker. Jika salah satu command tidak dapat dijalankan, laporkan alasannya dan output yang relevan; jangan mengganti hasilnya dengan asumsi.

---

# 19. Tahap 15 — E2E demo scenario

Buat scenario yang dapat diulang menggunakan mock, lalu ulangi dengan integrasi real jika semua credentials/layanan tersedia.

## Scenario A — tesis aktif

1. Pengguna demo mengirim kalimat tesis tentang ticker dan metric yang benar-benar ada di registry.
2. Webhook signature lolos dan message ID tersimpan.
3. Gemini extractor menghasilkan JSON terstruktur.
4. Rust menolak/memperbaiki tidak boleh; jika hasil tak valid, minta pengguna klarifikasi.
5. Backend mengirim ringkasan tesis pending.
6. Pengguna membalas `YA`.
7. Tesis menjadi aktif di PostgreSQL dan muncul di `/api/tesis`.

## Scenario B — evidence valid

1. Mock MCP menyediakan current snapshot dan previous snapshot dengan periode/sumber.
2. Pipeline menghitung perbandingan dengan Rust.
3. Finding mengikuti kontrak resmi.
4. Evidence gate mencocokkan nilainya dengan snapshot.
5. Finding dan compliance audit tersimpan.
6. Status tesis diperbarui.
7. Quota/mute/dedupe lolos.
8. Payload dibentuk dari template dan mock WhatsApp mencatat `mock_sent`.
9. Dashboard API menunjukkan Finding, status, credit usage yang tersedia, dan trace.

## Scenario C — evidence invalid

Kirim fixture yang nilai `current_value`-nya berbeda dari snapshot atau `previous_value` kosong.

Expected:

- Finding ditandai rejected;
- compliance audit menyimpan alasan;
- `/api/ditolak` menampilkan alasan;
- WhatsApp tidak dipanggil;
- trace mencatat penolakan.

## Scenario D — cooldown dan duplicate

- Kirim tiga alert yang valid untuk satu user dalam satu hari: sesuai konfigurasi, maksimal tiga dikirim.
- Alert keempat harus `deferred`.
- Kirim payload periode/metric yang sama dua kali: hanya satu alert yang dapat berstatus sent.
- Aktifkan `/diam`: alert proaktif tidak dikirim sampai `/lanjut`.

## Scenario E — integrasi real

Hanya setelah credential lokal dan akun layanan tersedia:

- kirim pesan melalui WhatsApp Cloud API dari Rust;
- gunakan webhook publik sementara yang disetujui tim;
- uji signature yang benar dan salah;
- pakai MCP server nyata dari Representative;
- pakai Gemini model/prompt yang telah dikonfirmasi Dian;
- simpan bukti test tanpa menampilkan secret atau nomor telepon penuh.

Jangan menyebut mock sebagai integrasi real.

---

# 20. Urutan implementasi dua hari

Urutan ini adalah prioritas eksekusi, bukan jaminan bahwa semua integrasi eksternal pasti tersedia.

## Hari 1 — fondasi, DB, kontrak API, pengiriman WhatsApp

### Blok 1: audit + setup

- [ ] Audit repository dan tentukan crate backend yang benar.
- [ ] Validasi config/environment dan `.gitignore`.
- [ ] Buat/validasi Docker Compose PostgreSQL.
- [ ] Buat Axum binary `backend` di port `8080`.
- [ ] Implementasikan `/health` dan graceful shutdown.

**Exit criteria:** backend dan DB hidup, secret tidak ter-track.

### Blok 2: migration + repository

- [ ] Buat tiga tabel inti dan tabel operasional yang dibutuhkan.
- [ ] Tambahkan FK, unique constraints, indexes, dan timestamps.
- [ ] Buat repositories SQLx untuk user/thesis/snapshot/finding/audit/alert/credit/trace.
- [ ] Tes migration dari database kosong.

**Exit criteria:** migration dan repository smoke tests lolos.

### Blok 3: kontrak API

- [ ] Implementasikan `/snapshots`, `/pantauan`, `/findings`, `/jejak`, `/kredit`.
- [ ] Lindungi internal routes dengan token konfigurasi.
- [ ] Implementasikan semua endpoint dashboard JSON.
- [ ] Tambahkan tests accepted/rejected dan masking.

**Exit criteria:** semua route diuji pakai mock/database lokal.

### Blok 4: WhatsApp send + GET webhook

- [ ] Implementasikan WhatsApp client dan mock transport.
- [ ] Map `WhatsAppAlertPayload` ke request Meta.
- [ ] Implementasikan GET webhook challenge verification.
- [ ] Jalankan test mock; jika token valid tersedia, kirim `hello_world` dari Rust.

**Exit criteria:** mock send lolos; pengiriman real dinyatakan berhasil hanya bila response provider dan penerimaan pesan diverifikasi.

## Hari 2 — webhook, tesis, pipeline, compliance, cooldown, E2E

### Blok 5: POST webhook

- [ ] Validasi raw-body signature HMAC-SHA256.
- [ ] Parse payload secara aman.
- [ ] Deduplikasi message ID dengan PostgreSQL unique key.
- [ ] Simpan status processing dan trace.

**Exit criteria:** signature invalid tidak menghasilkan side effect; replay tidak memproses dua kali.

### Blok 6: tesis + Gemini

- [ ] Integrasikan prompt Dian.
- [ ] Implementasikan JSON parser ketat + metric allowlist.
- [ ] Implementasikan pending thesis + `YA` confirmation.
- [ ] Implementasikan `/status`, `/bantuan`, `/diam`, `/lanjut`, `/hapus`, `/bukti`, `/kredit`.

**Exit criteria:** tesis pending sampai `YA`; batas tesis/metric diterapkan; command utama teruji.

### Blok 7: MCP gateway + AgentPipeline

- [ ] Integrasikan MCP transport bila tersedia; jika belum, gunakan `MockMcpGateway` dan catat blocker.
- [ ] Implementasikan baseline/current/previous snapshot flow.
- [ ] Hitung delta di Rust dan bentuk Finding resmi.
- [ ] Implementasikan evidence gate/compliance audit.
- [ ] Hitung status tesis dan bangun evidence card.

**Exit criteria:** valid fixture diterima dan fixture invalid ditolak, dengan audit.

### Blok 8: cooldown + scheduler + dispatch

- [ ] Implementasikan daily limit dan unique alert key.
- [ ] Implementasikan mute/continue.
- [ ] Implementasikan scheduler non-overlap + manual mock trigger terlindungi.
- [ ] Hubungkan ke mock WhatsApp lalu real WhatsApp jika tersedia.

**Exit criteria:** alert duplicate tidak terkirim ganda, alert keempat deferred, mute berfungsi.

### Blok 9: full regression + dokumen

- [ ] Jalankan test unit, integration, webhook, dan pipeline.
- [ ] Jalankan `cargo fmt --check`, `cargo check`, `cargo test`.
- [ ] Perbarui README cara setup dari nol.
- [ ] Perbarui `docs/api-internal.md`, `docs/architecture.md`, dan `docs/integration-decisions.md` sesuai kode aktual.
- [ ] Catat status integrasi real vs mock secara eksplisit.
- [ ] Periksa secret dan nomor pengguna sebelum demo.

**Exit criteria:** skenario A–D bisa diulang. Skenario E hanya ditandai lolos bila layanan eksternal nyata sudah diverifikasi.

---

# 21. Prioritas jika waktu dua hari tidak cukup

## P0 — wajib untuk alur MVP

- Axum/8080 + config + health + Postgres/migrations.
- Internal API dan repository.
- WhatsApp client + webhook verification/signature/dedupe.
- Tesis pending → `YA` → active.
- Finding canonical schema + evidence validation + audit persistence.
- Status tesis, template bukti, kata terlarang pada field dinamis, disclaimer conflict blocker.
- Idempotency/cooldown, mock E2E, agent trace.

## P1 — setelah P0 stabil

- Semua WhatsApp commands yang belum lengkap.
- Credit meter dan API dashboard read models.
- Scheduler otomatis lengkap.
- Adapter MCP real setelah Representative menyediakan transport/tool.
- Gemini Presenter jika seluruh hard checks sudah ada.

## P2 — hanya setelah semua wajib lolos

- Command tanya “kenapa?” atau ringkasan tambahan.
- Pembanding subsektor.
- UI/dashboard polish (jika dimiliki Rafi; defaultnya hanya API).
- Deployment atau infrastruktur tambahan yang tidak diwajibkan.

Jangan memotong DB persistence, webhook deduplication, evidence gate, atau bukti status mock/real. Jika bagian eksternal belum tersedia, siapkan mock dan laporkan sebagai blocker, bukan berpura-pura selesai.

---

# 22. Security dan compliance checklist sebelum demo/submit

- [ ] Tidak ada `.env`, API key, token, password, `WHATSAPP_APP_SECRET`, atau `INTERNAL_API_TOKEN` dalam repository atau log.
- [ ] `.env.example` hanya berisi nama variabel dan contoh non-rahasia.
- [ ] Sectors API key tetap dimiliki Representative, sesuai pembagian tim.
- [ ] Raw webhook signature dipakai untuk validasi dan tidak dipublikasikan.
- [ ] Nomor WhatsApp disamarkan di semua `/api/*`, screenshot, dan video.
- [ ] Internal endpoints memerlukan token bila dapat diakses jaringan.
- [ ] Tidak ada saran atau eksekusi transaksi.
- [ ] Nilai di alert berasal dari Finding/snapshot, bukan angka buatan LLM.
- [ ] Semua Finding ditolak memiliki reason/audit trail.
- [ ] Alert dihentikan ketika evidence/policy check gagal.
- [ ] Konflik disclaimer terdokumentasi dan belum ditandai solved tanpa keputusan tim.
- [ ] Payload demo mock tidak disebut pesan WhatsApp sungguhan.
- [ ] README menggambarkan batasan nyata, tidak mengklaim fitur yang belum ada.

---

# 23. Dokumen yang harus dihasilkan/diperbarui agent

Di akhir implementasi, pastikan tersedia:

1. `orchestrator/` atau crate backend yang konsisten dengan repository.
2. SQLx migrations untuk semua tabel inti + operasional.
3. `.env.example` dan `.gitignore` yang benar.
4. Backend Axum binary di port `8080`.
5. Routes dan services untuk health, webhook WhatsApp, internal APIs, dashboard APIs.
6. Repository SQLx dengan query teruji.
7. `orchestrator::AgentPipeline` beserta MCP/Gemini/Compliance/WhatsApp adapters.
8. Unit/integration tests dan fixture tanpa secret.
9. `docs/api-internal.md` berisi method, path, auth, request, response, dan error code.
10. `docs/architecture.md` atau update diagram sesuai kode aktual.
11. `docs/integration-decisions.md` untuk konflik, pemilik komponen, transport yang dipilih, metric registry, dan keterbatasan.
12. README cara menjalankan dari nol, mode mock, mode real, migration, test, webhook tunnel, dan konfigurasi yang dibutuhkan.
13. Laporan akhir agent yang membedakan `implemented + tested`, `implemented but integration unverified`, `blocked`, dan `not implemented`.

---

# 24. Format laporan akhir yang wajib diberikan AI coding agent

Setelah mengerjakan, laporkan dengan format berikut:

```text
## Ringkasan
- Apa yang sudah dibuat:
- Apa yang belum selesai:

## File berubah
- path — tujuan perubahan

## Cara menjalankan
1. command...
2. command...

## Hasil pemeriksaan
- cargo fmt --check: PASS/FAIL/NOT RUN
- cargo check: PASS/FAIL/NOT RUN
- cargo test: PASS/FAIL/NOT RUN
- migration: PASS/FAIL/NOT RUN
- mock E2E: PASS/FAIL/NOT RUN
- WhatsApp real: VERIFIED / BLOCKED / NOT RUN
- MCP real: VERIFIED / BLOCKED / NOT RUN
- Gemini real: VERIFIED / BLOCKED / NOT RUN

## Keputusan/konflik yang perlu pemilik/tim
- ...

## Risiko yang masih ada
- ...
```

Tidak boleh menulis PASS hanya karena source code terlihat benar. PASS harus didukung oleh command/test yang benar-benar dijalankan.

---

# 25. Prompt pendek untuk memulai agent coding

Salin prompt berikut bersama dokumen ini:

> Baca seluruh `IDX_Sentinel_Rafi_Agent_Execution_Runbook.md`, `ARCHITECTURE.md`, `CONTRACTS.md`, dan `TASKS.md` sebelum coding. Periksa repository aktual terlebih dahulu dan jangan menimpa perubahan yang sudah ada. Implementasikan seluruh tugas Rahmat Rafi secara berurutan dalam fase runbook. Kerjakan perubahan file nyata, bukan hanya memberi rencana. Gunakan `CONTRACTS.md` terbaru sebagai kontrak `Finding`, jalankan test setelah setiap fase, dan perbaiki error yang disebabkan perubahanmu. Jangan mengimplementasikan ownership Representative (MCP/Sectors server) atau Dian (master prompts/compliance engine) dari nol; buat adapter/mock jika komponen itu belum tersedia. Jangan menebak transport MCP, field Sectors, metric whitelist, atau biaya kredit. Catat konflik disclaimer dan kata terlarang sebagai blocker kepatuhan; jangan menyembunyikannya. Jangan menampilkan atau meminta secret. Di akhir, gunakan format laporan pada Bagian 24 dan bedakan dengan jujur fitur yang benar-benar dites dari integrasi yang belum terverifikasi.

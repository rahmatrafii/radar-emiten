# Laporan Audit Status Proyek — 7 Oktober 2026

> Audit ulang teliti berbasis bukti langsung (baca file + eksekusi test).
> Setiap klaim mencantumkan lokasi bukti `file:baris` atau hasil perintah.
> Status: **fakta terverifikasi** vs **belum terverifikasi / belum dikerjakan** dipisahkan.

---

## 1. Ringkasan Eksekutif

| Area | Status |
|------|--------|
| Tugas tim (R/D/M, 12 item di `docs/TASKS.md`) | 12/12 `Completed` (terverifikasi di `docs/TASKS.md:23-48`) |
| Test suite | Terakhir full run: **78 passed** (35 lib + 43 integration); re-verifikasi lib hari ini: **35 passed** |
| Scheduler F2 | Terimplementasi (`orchestrator/src/scheduler.rs` ada) |
| WhatsApp real | Teruji sebagian: webhook ✅, template ✅, balasan teks simulasi ⚠️ (tidak sampai di HP) |
| Inkonsistensi docs-vs-code awal | Sebagian diperbaiki; **2 sisa** ditemukan (rinci di §5) |
| Git | Branch aktif `feat/rafi-mcp-server`, working tree **tidak bersih** (banyak modified + untracked) |
| Submit / video / medsos | Tidak ada bukti di repo → **belum terverifikasi** |

---

## 2. Tugas per Anggota (terverifikasi)

Sumber: `docs/TASKS.md:23-48` (dibaca langsung).

| Anggota | ID | Status tertulis |
|---------|----|-----------------|
| Rafiq | R-01, R-02, R-03, R-04 | `Completed` semua |
| Dian | D-01, D-02, D-03, D-04 | `Completed` semua |
| Rafi | M-01, M-02, M-03, M-04 | `Completed` semua |

Bukti implementasi D-02 s/d D-04 + scheduler F2 (file ada, ditemukan via glob):
- `orchestrator/src/services/compliance_service.rs` ✅
- `orchestrator/src/services/evidence_verifier.rs` ✅
- `orchestrator/tests/fixtures/agent_cases.json` ✅
- `orchestrator/tests/agent_cases_test.rs` ✅
- `orchestrator/src/scheduler.rs` ✅

Tim di `README.md:30-32`: Rafiq (Data & MCP), Dian (Agent Logic & Prompt), Rafi (Backend & Platform) — konsisten dengan pembagian tugas.

---

## 3. Verifikasi Test (dieksekusi)

| Perintah | Hasil |
|----------|-------|
| `cargo test --lib` (dijalankan saat audit) | **35 passed**, 0 failed (`orchestrator`) |
| `cargo test` full (run terakhir sebelum audit) | **78 passed** (35 lib + 43 integration: 5 agent_cases + 5 api + 7 mcp_gateway + 5 pipeline + 4 repository + 6 thesis + 6 webhook + 5 whatsapp), 0 failed |
| `cargo fmt` | bersih (tidak ada diff) |

Catatan: sejak full run 78-test, perubahan kode hanya penghapusan debug log di `orchestrator/src/main.rs` (terverifikasi: tidak ada string `DEBUG`/file `webhook_test.json`/`wa_template_test.json` di repo). Lib 35 test di-run ulang dan tetap lolos.

**Koreksi terhadap `docs/TASKS.md:50`:** catatan verifikasi di sana masih menulis "57 test orkestrator PASS" — angka itu **kedaluwarsa** (sekarang 78). Juga masih menulis hard gate `POLICY_...=false` sebagai blocker — kondisi itu **sudah berubah** (lihat §4).

---

## 4. Status `.env` Lokal (dibaca langsung, 7 Okt 2026)

| Variabel | Nilai | Keterangan |
|----------|-------|------------|
| `APP_MODE` | `real` | diubah untuk test WhatsApp real |
| `POLICY_DISCLAIMER_CONFLICT_ACKNOWLEDGED` | `true` | konflik disclaimer sudah diselesaikan |
| `WHATSAPP_ACCESS_TOKEN` | terisi | token baru diregenerasi saat test |
| `WHATSAPP_PHONE_NUMBER_ID` | `1233178056554020` | cocok dengan `docs/WA.jpeg` |
| `WHATSAPP_APP_SECRET` | terisi (32 char) | sudah diisi saat test webhook |
| `WHATSAPP_GRAPH_API_VERSION` | `v25.0` | — |
| `GEMINI_MODEL` | **kosong** | belum diisi |
| `SCHEDULER_ENABLED` | **tidak ada** di `.env` | kode default nonaktif bila env hilang (`main.rs`); `.env.example:33` menulis `false` |
| `DATABASE_URL` | `postgres://postgres:rahasia@localhost:5432/sentinel` | — |
| Secret/API key lain | terisi (`GEMINI_API_KEY`, `SECTORS_API_KEY`, `INTERNAL_API_TOKEN`) | `.env` ter-`.gitignore` (`git check-ignore` ✅), tidak muncul di `git status` ✅ |

---

## 5. Inkonsistensi yang Masih Ada (temuan audit)

### 5.1 `docs/TASKS.md:23` (R-01) masih mencantumkan endpoint lama ❌
Teks R-01: konsumsi endpoint `/v2/subsectors/`, `/v2/companies/`, `/v2/company/report/`.
Fakta kode + `docs/ARCHITECTURE.md:61,101-103` (sudah diperbaiki): endpoint aktual adalah `GET /v2/companies/reports/quarterly?since=...` dan `GET /v2/companies/screener/?fields=...&where=...`.
**Aksi:** selaraskan baris R-01 dengan endpoint aktual.

### 5.2 `docs/TASKS.md:50` catatan verifikasi kedaluwarsa ❌
Menulis "57 test" dan "`POLICY_...=false` (bloker)". Fakta saat ini: 78 test passed; `.env` memakai `true`; konflik disclaimer dinyatakan selesai (`docs/integration-decisions.md:16-21`).
**Aksi:** perbarui angka test + status hard gate.

### Yang sudah konsisten ✅
- `docs/ARCHITECTURE.md:61,101-103` ↔ `src/sectors/client.rs` (quarterly + screener).
- `src/findings/mod.rs:36-42` memakai field Inggris (`metric_name`, `previous_value`, `current_value`, ...) — tidak ada lagi `indikator`/`nilai_sebelum`.
- Disclaimer: `docs/CONTRACTS.md:183` ↔ `orchestrator/src/models/whatsapp.rs:4` (teks baru tanpa kata "rekomendasi").

---

## 6. Yang Belum Dikerjakan / Belum Terverifikasi

### 6.1 Docs placeholder (masih ada, dibaca langsung)
- `docs/api-internal.md:3` — "Placeholder — akan diisi Rafi." (tabel endpoint ada, contoh permintaan belum ada).
- `docs/indikator.md:3,18` — placeholder + "aturan arah_baik: Belum final".
- `docs/sectors-api-notes.md:3,15` — placeholder + "biaya kredit: Belum terverifikasi".
- `docs/evaluasi.md` — **sudah terisi** (tidak ada placeholder lagi).

### 6.2 Kode / fitur terbuka
- `RealMcpGateway` = stub jujur (`TransportUnavailable` sampai transport disepakati) — `orchestrator/src/integrations/mcp_gateway.rs:287-307`.
- `raw_data` snapshot diterima lalu dibuang — `orchestrator/src/routes/snapshots.rs:22,64-65`.
- `PROMPTS.md` **tidak memuat** prompt ekstraksi tesis (grep tidak menemukan; schema JSON hanya ada di `gemini_client.rs`).
- Metric registry final belum ada (default dev + override `METRIC_REGISTRY`).
- Durasi `/diam` belum disepakati (mute sampai `/lanjut`).
- Migrasi tidak auto-run saat startup (jalankan `sqlx migrate run` manual).
- Dashboard frontend: folder `dashboard/` **tidak ada** (glob kosong); hanya API JSON `/api/*` yang siap.
- `GEMINI_MODEL` kosong di `.env`.

### 6.3 Webhook publik (ditunda atas permintaan)
Server localhost tidak dapat dijangkau Meta. Butuh tunnel + registrasi webhook + subscribe `messages`. Lihat detail di respons audit sebelumnya.

### 6.4 Non-teknis (tidak ada bukti di repo)
Video teaser 1 menit, video juri ≤3 menit, postingan medsos, pernyataan masalah 1 kalimat, pilihan track, dan submit portal — **tidak ditemukan di repo**, status belum terverifikasi.

---

## 7. Temuan Git (penting, dikoreksi dari klaim lama)

| Klaim lama | Fakta saat audit |
|------------|------------------|
| Branch `main`, working tree bersih (`docs/integration-decisions.md:7`) | Branch aktif: **`feat/rafi-mcp-server`**; working tree **tidak bersih**: 18 modified + 13 untracked (output `git status --short`) |
| Docs ter-commit lengkap | `git ls-files docs` hanya mencatat **6 file**: `ARCHITECTURE.md`, `CONTRACTS.md`, `IDX_Sentinel_Rafi_Agent_Execution_Runbook.md`, `integration-decisions.md`, `PROMPTS.md`, `TASKS.md`. File berikut **untracked** (ada di disk, belum di-commit): `api-internal.md`, `evaluasi.md`, `indikator.md`, `sectors-api-notes.md`, `DIAN_Task_Plan.md`, `IDX_Sentinel_Brief_Lengkap.md`, `IDX_Sentinel_Rincian_Tugas_Tim.md`, `WA.jpeg`, dst. |
| Secret aman | ✅ `.env` di-ignore (`.gitignore:3`) dan tidak muncul di `git status`. Namun `.env` lokal memuat token/API key aktif — jangan pernah `git add -f .env`. |

**Aksi yang disarankan:** commit/push file docs + kode baru sebelum submit; pastikan repo publik; verifikasi tidak ada secret di riwayat commit.

---

## 8. Hasil Uji WhatsApp Real (7 Okt 2026)

| Test | Hasil |
|------|-------|
| Webhook + signature HMAC valid | ✅ `{"status":"ok","processed":1}` |
| `dispatch_real()` ke Graph API | ✅ log `sent` + `message_id` |
| Template `hello_world` langsung | ✅ pesan masuk di HP |
| Balasan teks atas webhook **simulasi** | ⚠️ log `sent` tapi tidak sampai di HP (pola khas tidak ada jendela 24 jam di sisi Meta; webhook simulasi lokal tidak membuka jendela) |
| Token expired (401) di tengah jalan | Diatasi dengan regenerasi token via dashboard Meta |

Catatan: file `webhook_test.json` / `wa_template_test.json` sudah dihapus; debug log sementara di `main.rs` sudah dibersihkan.

---

## 9. Koreksi atas Laporan Sebelumnya

1. Pernyataan "Test WhatsApp real masih blocked — butuh token dari tim" **salah** — token sudah ada di `.env`; blocker aktual adalah `APP_MODE` + flag disclaimer (keduanya sudah diubah untuk test). Dilaporkan dan diperbaiki di `docs/integration-decisions.md` + `docs/evaluasi.md`.
2. Angka "57 test" di `docs/TASKS.md:50` **kedaluwarsa** — angka terverifikasi saat ini 78 (35 lib + 43 integration).
3. Klaim "branch main, working tree bersih" **kedaluwarsa** — branch aktif `feat/rafi-mcp-server`, tree tidak bersih.
4. Daftar docs "lengkap" sebelumnya **tidak akurat** — 3 file masih placeholder (§6.1) dan banyak docs belum ter-commit (§7).

---

## 10. Koreksi Setelah Audit (masih 7 Okt 2026)

- **Client Sectors ditulis ulang** (`src/sectors/client.rs`, `models/`, `mcp-server.rs`, `main.rs`, `tests.rs`): endpoint lama 404 di API live. Endpoint benar (semua terverifikasi live 200): `/v2/subsectors/`, `/v2/companies/`, `/v2/companies/quarterly-financial-dates/`, `/v2/financials/quarterly/{symbol}/`, `/v2/subsector/report/{sub}/`. Test: 13/13 unit lolos + demo live BBCA berhasil (revenue +0.86%, earnings +1.13%).
- **Koreksi atas koreksi:** `ARCHITECTURE.md` versi awal (`/v2/subsectors/`, `/v2/subsector/report/`) ternyata benar sesuai docs resmi; perbaikan 7 Okt siang yang menggantinya adalah kesalahan, sudah dikembalikan + dilengkapi.
- **Kredit:** pengujian live ≈ 10–14 kredit dari 1000 (rinci di `docs/sectors-api-notes.md`).
- **`docs/sectors-api-notes.md` sudah terisi** (tidak lagi placeholder); `docs/TASKS.md` R-01 diperbaiki.

## 11. Saran Prioritas (sebelum submit 8 Okt 23.59)

1. Perbaiki 2 inkonsistensi §5 (R-01 endpoint + catatan verifikasi TASKS).
2. Commit semua file baru/berubah; pastikan repo publik dan bersih dari secret.
3. Bekukan fitur; rekam video cadangan dengan data mock (stabil).
4. Lengkapi placeholder kecil (§6.1) bila sempat — tidak memengaruhi kode.
5. Siapkan artefak non-teknis (§6.4) dan submit lebih awal.

# 05 — API Backend (Axum :8080)

Daftar ini disalin dari `orchestrator/src/routes/mod.rs:16-49`. Tidak ada endpoint lain yang resmi.

## 1. Ringkasan endpoint

| Method + Path | Fungsi | Auth |
|---|---|---|
| `GET /health` | Cek hidup + koneksi DB (`{"status":"ok","database":"ok"}`) | — |
| `POST /snapshots` | Simpan/upsert snapshot observasi | `INTERNAL_API_TOKEN` (jika disetel) |
| `GET /snapshots?ticker=&metric_name=&limit=` | Baca riwayat snapshot (`ticker` + `metric_name` wajib) | — |
| `POST /findings` | Terima `Finding` → gate → `accepted`/`rejected` | `INTERNAL_API_TOKEN` (jika disetel) |
| `POST /jejak` | Catat langkah agent (`agent` harus salah satu dari 8 nama resmi) | `INTERNAL_API_TOKEN` (jika disetel) |
| `POST /kredit` | Catat pemakaian kredit Sectors | `INTERNAL_API_TOKEN` (jika disetel) |
| `GET /pantauan` | Ticker + metric dari tesis aktif (tanpa nomor WA) | — |
| `GET /webhook/whatsapp` | Verifikasi webhook Meta (`hub.mode/verify_token/challenge`) | verify token |
| `POST /webhook/whatsapp` | Terima pesan (HMAC-SHA256 `X-Hub-Signature-256` + dedupe `wa_message_id`) | app secret |
| `GET /internal/snapshots/previous?ticker=&metric_name=` | Snapshot pembanding untuk MCP | `INTERNAL_API_TOKEN` |
| `POST /internal/findings` | Sama dengan `POST /findings`, untuk MCP | `INTERNAL_API_TOKEN` |
| `GET /api/tesis` | Tesis + status (nomor disamarkan) | — |
| `GET /api/findings` | Finding `lolos` | — |
| `GET /api/ditolak` | Finding `ditolak` + alasan | — |
| `GET /api/jejak` | Jejak agent | — |
| `GET /api/kredit` | Budget vs terpakai vs sisa | — |
| `GET /dashboard` | File statis `dashboard/index.html` (resolusi fleksibel) | — |

Semua list mendukung `?limit=&offset=` (default 50, maks 200).

## 2. Contoh request

```bash
BASE=http://localhost:8080

# Snapshot (raw_data diterima tapi saat ini dibuang — lihat 01 §5)
curl -X POST $BASE/snapshots -H 'Content-Type: application/json' -d '{
  "ticker": "BBCA", "subsector": "banks", "metric_name": "net_profit_margin",
  "value": 46.85, "period": "Q3 2024",
  "source": "Sectors API v2 /v2/financials/quarterly/BBCA",
  "observed_at": "2026-10-02T10:15:30Z"
}'

# Finding (kontrak persis 03 §1)
curl -X POST $BASE/findings -H 'Content-Type: application/json' -d '{
  "ticker": "BBCA", "subsector": "banks", "metric_name": "net_profit_margin",
  "current_value": 46.85, "previous_value": 43.12, "period": "Q3 2024",
  "source": "Sectors API v2 /v2/financials/quarterly/BBCA",
  "observed_at": "2026-10-02T10:15:30Z",
  "confidence_score": 0.98, "finding_summary": "Net Profit Margin BBCA Q3 2024 tercatat 46.85%."
}'

# Jejak (agent hanya: scout, analyst, evidence_checker, chief, compliance, presenter, scheduler, dispatcher)
curl -X POST $BASE/jejak -H 'Content-Type: application/json' -d '{
  "agent": "scout", "action": "evidence_fetched", "outcome": "ok", "ticker": "BBCA"
}'

# Kredit
curl -X POST $BASE/kredit -H 'Content-Type: application/json' -d '{
  "endpoint": "/v2/financials/quarterly/BBCA", "credits": 2, "from_cache": false
}'

# Internal (butuh token)
curl $BASE/internal/snapshots/previous'ticker=BBCA&metric_name=net_profit_margin' \
  -H "Authorization: Bearer $INTERNAL_API_TOKEN"
```

## 3. Tiga gate `POST /findings` (kode: `routes/findings.rs:13-230`)

1. **Format** (`validation.rs`): ticker 4 huruf kapital, metric ada di registry, angka finite, `observed_at` RFC3339, summary 1–500 karakter.
2. **Compliance** (`compliance_service.rs`): ada kata terlarang → `rejected: kata terlarang terdeteksi: ...`.
3. **Evidence** (`evidence_verifier.rs`): angka tidak 1:1 dengan snapshot → `rejected: ... tidak cocok dengan snapshot tersimpan`.

Lolos semua → `lolos`, tulis audit `compliance_audit_logs` + trace `evidence_checker/finding_accepted`. Duplikat `(ticker, metric, period)` yang sudah lolos → `rejected: finding duplikat`.

## 4. Perintah WhatsApp (kode: `services/command_router.rs`)

`/tesis` (lalu kirim kalimat bebas) → `YA`/`TIDAK` → `/status` → `/bukti` (dari DB, tanpa panggil API) → `/diam` (mute sampai `/lanjut`) → `/lanjut` → `/hapus <id>` → `/bantuan` → `/kredit` (admin saja). Pesan tak dikenal dibalas daftar perintah, bukan ditebak. Batas: 3 tesis aktif/user, 3 metric/tesis.

## 5. Batasan yang perlu kamu tahu

- `GET /snapshots` wajib kirim `ticker` + `metric_name` (tanpa itu = 400).
- Webhook menolak signature salah dan mengabaikan event `statuses`/media secara aman.
- Nomor WA tidak pernah keluar mentah dari `/api/*` — masking di backend (`masking.rs`).

# 03 — Kontrak Data

Sumber kebenaran payload. Implementasi: `orchestrator/src/models/finding.rs`, `whatsapp.rs`, `orchestrator/src/validation.rs`.

## 1. `Finding` (wajib 10 field, `additionalProperties: false`)

| Field | Aturan (divalidasi di `validation::validate_finding`) |
|---|---|
| `ticker` | Tepat 4 huruf kapital A–Z, mis. `BBCA` |
| `subsector` | Tidak kosong, mis. `banks` |
| `metric_name` | Harus ada di whitelist (`07-indikator-testing.md`) |
| `current_value` / `previous_value` | Harus numerik finite (angka atau string angka). `null`/teks bebas = ditolak sebagai alert |
| `period` | Tidak kosong, mis. `Q3 2024` |
| `source` | Tidak kosong, mis. `Sectors API v2 /v2/financials/quarterly/BBCA` |
| `observed_at` | RFC3339 valid, mis. `2026-10-02T10:15:30Z` |
| `confidence_score` | 0.0–1.0 finite |
| `finding_summary` | 1–500 karakter, faktual, tanpa kata terlarang |

Contoh valid:

```json
{
  "ticker": "BBCA",
  "subsector": "banks",
  "metric_name": "net_profit_margin",
  "current_value": 46.85,
  "previous_value": 43.12,
  "period": "Q3 2024",
  "source": "Sectors API v2 /v2/financials/quarterly/BBCA",
  "observed_at": "2026-10-02T10:15:30Z",
  "confidence_score": 0.98,
  "finding_summary": "Net Profit Margin BBCA Q3 2024 tercatat 46.85%, naik 3.73 poin dari 43.12% periode pembanding."
}
```

Contoh ditolak (angka tidak cocok snapshot → Gate 2):

```json
{
  "finding_id": 42,
  "status": "rejected",
  "rejection_reason": "current_value tidak cocok dengan snapshot tersimpan"
}
```

## 2. Hasil verifikasi compliance (`compliance_audit_logs`)

Setiap `POST /findings` menulis 1 baris audit, **termasuk yang ditolak**:

| Field | Arti |
|---|---|
| `is_compliant` | `false` bila ada kata terlarang di `finding_summary` |
| `evidence_verified` | `false` bila angka tidak 1:1 dengan snapshot DB |
| `prohibited_words_detected` | Array kata yang kena, mis. `["beli"]` |
| `rejection_reason` | Alasan aman untuk API/dashboard |
| `stage` | `finding` / `presenter_payload` / `whatsapp_dispatch` |

Hanya `is_compliant=true DAN evidence_verified=true` yang berstatus `accepted`/`lolos`.

## 3. Payload WhatsApp (`WhatsAppAlertPayload`)

Kode: `orchestrator/src/models/whatsapp.rs:4-44`. Aturan:

- `recipient_number`: 10–15 digit, tanpa `+`, digit pertama bukan `0`.
- `header` ≤ 100 karakter, `body` ≤ 2048 karakter, keduanya tidak kosong.
- `disclaimer` harus **persis sama** dengan konstanta (satu-satunya teks resmi):

```text
Disclaimer: Informasi ini hanya bersifat edukasi & pemantauan data historis. TIDAK memberikan saran investasi (beli/jual/tahan) maupun eksekusi transaksi.
```

Status pengiriman jujur: `mock_sent` (mode mock) | `sent` (Meta konfirmasi `message_id`) | `blocked_by_policy_conflict` | `failed`. Mock tidak pernah diklaim `sent`.

## 4. Kalau mau ubah kontrak

1. Ubah struct di `orchestrator/src/models/` + validasi di `validation.rs` + migrasi bila kolom DB berubah.
2. Perbarui file ini + `04-agent-prompts.md` bila menyentuh kata terlarang/disclaimer.
3. Beri tahu seluruh tim — kontrak adalah titik temu 3 peran. Jangan kirim dua bentuk payload berbeda di endpoint yang sama.

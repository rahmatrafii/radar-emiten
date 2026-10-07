# 07 — Indikator & Testing

## 1. Whitelist metric (sumber kebenaran: `orchestrator/src/metrics.rs:14-25`)

Default dev (dipakai bila `METRIC_REGISTRY` kosong):

```text
net_profit_margin, operating_margin, gross_margin, roe, roa,
eps, revenue, net_income, der, current_ratio
```

Override: `METRIC_REGISTRY="net_profit_margin,roe,revenue"` (koma-separated, spasi diabaikan). Setiap `metric_name` di `POST /snapshots` dan `POST /findings` ditolak (400) kalau tidak ada di daftar ini.

### Aturan `arah_baik` (untuk status tesis MENGUAT/MELEMAH)

| Metric | Naik berarti |
|---|---|
| `net_profit_margin`, `operating_margin`, `gross_margin`, `roe`, `roa`, `eps`, `revenue`, `net_income`, `current_ratio` | Mendukung tesis "naik itu baik" (`increase`) |
| `der` | Sebaliknya — naik = melemahkan tesis "keuangan sehat" (`decrease`) |

`desired_direction` per tesis disimpan di `thesis_metrics` (`increase`/`decrease`/`any`). `any`/ambigu = netral sampai definisi disepakati. Ambang netral: perubahan relatif < `NEUTRAL_RELATIVE_THRESHOLD` (default 0.02 = 2%) dianggap netral.

## 2. Cara test

```bash
cargo test                 # full suite — butuh PostgreSQL lokal
cargo test --lib           # unit saja (compliance, validation, metrics)
cargo test --test agent_cases_test   # 5 kasus fixture Dian
cargo fmt --check && cargo check      # wajib bersih sebelum commit
```

Fixture: `orchestrator/tests/fixtures/agent_cases.json` (runner `tests/agent_cases_test.rs` = 4 validasi struktur + 1 end-to-end `POST /findings`).

| ID | Skenario | Harapan |
|---|---|---|
| `case_001` (TSTA, banks) | Laba naik, compliant, angka cocok | `accepted` |
| `case_002` (TSTB, telekomunikasi) | Laba turun, compliant, angka cocok | `accepted` |
| `case_003` (TSTC, komoditas) | Valuasi berubah, compliant, angka cocok | `accepted` |
| `case_004` (TSTD) | Summary mengandung "beli" | `rejected` (compliance) |
| `case_005` (TSTE) | Angka tidak sesuai snapshot | `rejected` (evidence) |

Catatan jujur: fixture memakai ticker sintetis (TSTA–TSTE), bukan data Sectors live. Tanpa `DATABASE_URL` hanya validasi struktur yang jalan. Gemini API langsung belum diuji — yang diuji compliance (D-02) + evidence (D-03).

## 3. Uji WhatsApp (hasil 7 Okt 2026)

- Webhook + signature HMAC valid → `{"status":"ok","processed":1}`. ✅
- Template `hello_world` via Graph API → masuk HP. ✅
- Balasan teks atas webhook **simulasi** → `sent` + `message_id` tapi tidak sampai HP (tidak ada jendela 24 jam di sisi Meta). Untuk demo: user kirim pesan asli dulu (buka jendela 24 jam), atau pakai template di luar jendela.

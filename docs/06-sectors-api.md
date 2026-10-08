# 06 — Sectors API v2 (Catatan Lapangan Terverifikasi)

Referensi resmi: `https://docs.sectors.app/api-references/v2/indonesia/`. Semua di bawah terverifikasi live 7 Okt 2026 via curl + `cargo run --bin radar-emiten`. Pemilik awal: Rafiq.

## 1. Endpoint yang dipakai kode

| Endpoint | Fungsi | Biaya |
|---|---|---|
| `GET /v2/subsectors/` | Daftar slug `[{sector, subsector}]`, mis. `banks` | 1 kredit |
| `GET /v2/companies/?where=...` | Screener. Respons `{"results": [...], "pagination": {...}}` | 1 kredit (structured) |
| `GET /v2/companies/quarterly-financial-dates/?since=&limit=` | Feed tanggal laporan universe `[{symbol, date, quarter}]`, `limit` maks 30 | 1 kredit/halaman |
| `GET /v2/financials/quarterly/{symbol}/?n_quarters=` | Keuangan per emiten, JSON array terbaru-dulu | 1 kredit/quarter |
| `GET /v2/subsector/report/{sub}/?sections=` | Agregat subsektor. `sections` **wajib eksplisit** (`statistics,market_cap,stability,valuation,growth,companies`) | 1 kredit/section |

Uji 7 Okt 2026 habis ~10–14 kredit dari 1000. Demo `radar-emiten` terukur 3 kredit (1 subsectors + 2 quarters).

## 2. Aturan yang sering menjebak (baca sebelum coding)

1. **Suffix `.JK`**: respons memakai `"BBCA.JK"`, input menerima `BBCA` maupun `BBCA.JK`. Kode menormalisasi ke ticker 4 huruf untuk kontrak internal.
2. **`fields` tidak didukung screener** → 400 `UNSUPPORTED_PARAMETERS` (gratis, tidak menagih).
3. **`where`**: operator `=` tunggal + string dikutip satu, mis. `sub_sector = 'banks'`.
4. **Field kuartalan**: notasi kurung `revenue_q[Q1-2024]`. Tidak ada konvensi `_prev` — bandingkan 2 record via `n_quarters=2`.
5. **Metrik bank** ada di objek nested `financials_sector_metrics` (`net_interest_income`, `gross_loan`, `total_deposit`). Field flat yang dipakai: `revenue`, `earnings`, `symbol`, `company_name`, `date`.
6. **Billing**: 2xx menagih; 400/401/403/429/5xx gratis; **404 ikut menagih 1 kredit**. Jangan panggil `subsector/report` tanpa `sections` (default 6 section = 6 kredit).
7. **Feed universe** ~950 emiten ≈ 32 halaman ≈ 32 kredit — selalu pakai `since` inkremental.
8. **HTTP 410** = memanggil endpoint v1 yang sudah dimatikan. Kode mengunci prefix `/v2/` dan mengembalikan `API_VERSION_DEPRECATED` tanpa fallback.

## 3. Contoh live

- `BBCA` revenue Q2 2026 = 28.677T vs Q1 2026 = 28.434T (+0.86%).
- Screener banks: `GET /v2/companies/?where=sub_sector = 'banks'`.
- Feed inkremental: `GET /v2/companies/quarterly-financial-dates/?since=2026-07-01&limit=30`.

## 4. Untuk penerus

- Jangan menebak nama field — cek `src/sectors/models/` dulu.
- Biaya baru hanya dari pengukuran (`POST /kredit`), bukan dari tebakan.
- Kalau endpoint 404: cek ejaan path v2 + `sections`/`where` sebelum menyalahkan API key.

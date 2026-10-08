# Dokumentasi Radar Emiten

Panduan handover untuk penerus proyek. Semua dalam Bahasa Indonesia, diurut dari yang wajib dibaca lebih dulu.

| Urutan | File | Isi |
|---|---|---|
| 1 | [01-mulai-dari-nol.md](01-mulai-dari-nol.md) | Setup `.env`, database, cara jalan + troubleshooting. **Baca ini dulu.** |
| 2 | [02-arsitektur.md](02-arsitektur.md) | Alur data end-to-end, 6 MCP tools, batasan yang masih berlaku. |
| 3 | [03-kontrak-data.md](03-kontrak-data.md) | Schema `Finding`, hasil compliance, payload WhatsApp + contoh valid/ditolak. |
| 4 | [04-agent-prompts.md](04-agent-prompts.md) | 4 system prompt + daftar kata terlarang + keterbatasannya. |
| 5 | [05-api-backend.md](05-api-backend.md) | Daftar endpoint Axum yang benar-benar ada di kode + contoh request. |
| 6 | [06-sectors-api.md](06-sectors-api.md) | Endpoint Sectors v2 yang dipakai, aturan `.JK`, biaya kredit. |
| 7 | [07-indikator-testing.md](07-indikator-testing.md) | Whitelist metric + cara menjalankan test. |

## Aturan main

- Sumber kebenaran kode: `orchestrator/src/routes/mod.rs` (endpoint), `orchestrator/src/models/` (kontrak), `orchestrator/src/metrics.rs` (metric), `src/sectors/` (Sectors client).
- Prinsip produk: kode menghitung angka, LLM hanya menulis kalimat. Tanpa bukti, tanpa pesan. Setiap alert wajib ada disclaimer.
- File lama masa hackathon dipindah ke [`archive/`](archive/) — hanya histori, jangan dijadikan acuan teknis.

## Status handover (per Okt 2026)

- Backend + pipeline + compliance + WhatsApp mock/real sudah jalan. Detail sisa kerja terbuka ada di `01-mulai-dari-nol.md` bagian "Yang belum selesai".
- Kalau ragu antara docs vs kode: **percaya kode**, lalu perbarui docs.

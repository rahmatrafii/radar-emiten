# Integration Decisions & Conflict Log

Dokumen ini mencatat konflik spesifikasi, komponen yang belum tersedia, dan keputusan yang perlu dikonfirmasi tim. Pisahkan **fakta terkonfirmasi** dari hal yang **belum dipastikan**.

## Konflik yang belum terselesaikan

### 1. Disclaimer & kata terlarang — BLOCKER KEPATUHAN SEBELUM DEMO PUBLIK
- Konstanta disclaimer di `CONTRACTS.md` mengandung frasa **"rekomendasi transaksi"**:
  `Disclaimer: Informasi ini hanya bersifat edukasi & pemantauan data historis. Bukan anjuran investasi atau rekomendasi transaksi.`
- Sementara aturan produk melarang teks `rekomendasi` pada pesan, dan `PROMPTS.md` (Chief Presenter) memakai disclaimer berbeda:
  `Ringkasan ini hanya bersifat edukasi & pemantauan data historis. TIDAK memberikan saran investasi (beli/jual/tahan) maupun eksekusi transaksi.`
- **Status:** belum diputuskan tim. Selama `POLICY_DISCLAIMER_CONFLICT_ACKNOWLEDGED=false`, hard gate memblokir pengiriman WhatsApp real (audit reason `DISCLAIMER_POLICY_CONFLICT`, status bukan `sent`). Mock mode melaporkan `blocked_by_policy_conflict`.
- **Keputusan yang dibutuhkan:** tim memilih (a) memperbarui konstanta disclaimer + schema secara resmi, atau (b) mengesahkan pengecualian eksplisit untuk disclaimer.

## Komponen eksternal yang belum tersedia (belum terverifikasi)

| Komponen | Pemilik | Status |
|---|---|---|
| MCP server `rmcp` + transport (stdio/SSE/HTTP) | Representative | Belum ada di repo → buat `McpGateway` trait + `MockMcpGateway`; adapter real menyusul |
| Daftar metric final (whitelist) | Tim | Belum final → buat metric registry yang dapat dikonfigurasi; fixture hanya untuk tes |
| Prompt extractor tesis | Dian | `PROMPTS.md` ada (4 agent prompt), D-02 compliance in progress, D-03 pending |
| Gemini API key/model | Tim | Ada di `.env` lokal; model belum diisi |
| WhatsApp Cloud API credentials | Tim | Belum diisi → mock mode |
| Biaya Sectors per endpoint | Representative | Jangan dikarang; terima dari `POST /kredit` |

## Keputusan teknis yang sudah diambil

- **Binary/crate:** satu crate `orchestrator/` sebagai backend Axum di port **8080** (bukan 3000).
- **Kepemilikan DB:** hanya Axum yang memegang pool PostgreSQL; MCP server harus mendelegasikan via endpoint internal.
- **Nama tabel:** migrasi lama memakai nama Indonesia (`pengguna`, `tesis`, ...); dipertahankan untuk tidak memutus migrasi, dipetakan ke konsep runbook (`users`, `theses`, dst).
- **Status tesis:** `PENDING_CONFIRMATION`, `MENGUAT`, `NETRAL`, `MELEMAH`, `BELUM_CUKUP_DATA`.
- **Dedupe alert:** `(pengguna_id, tesis_id, metric_name, period)` + dedupe pesan masuk via `wa_message_id`.
- **Mock vs real:** semua integrasi eksternal default ke mock sampai kredensial/transport terverifikasi.

## Keamanan

- `.env` berisi Gemini API key — **sudah terekspos sebelumnya; rotasi key direkomendasikan**.
- `.env` sudah di-`.gitignore`; `.env.example` hanya berisi nama variabel.

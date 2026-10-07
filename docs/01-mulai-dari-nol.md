# 01 — Mulai dari Nol (Handover)

Untuk teman yang meneruskan proyek. Target: dari clone sampai `GET /health` = ok dalam ~30 menit.

> Aplikasi ini hanya analisis informasi dan pemantauan data historis. **Bukan saran investasi** (tanpa kata beli/jual/tahan, tanpa eksekusi transaksi).

## 1. Prasyarat

- Rust 1.78+ (`cargo --version`), Docker + Docker Compose, PostgreSQL client opsional.
- API keys (simpan di `.env`, tidak pernah di-commit): `SECTORS_API_KEY`, `GEMINI_API_KEY`, kredensial WhatsApp (lihat bawah).

## 2. Setup (5 langkah)

```bash
# 1. Clone + masuk repo
git clone <repo-url> radar-emiten && cd radar-emiten

# 2. Isi .env
cp .env.example .env
# Wajib isi: DATABASE_URL, SECTORS_API_KEY
# Contoh lokal:
#   POSTGRES_USER=postgres
#   POSTGRES_PASSWORD=postgres
#   POSTGRES_DB=idx_sentinel
#   DATABASE_URL=postgres://postgres:postgres@localhost:5432/idx_sentinel

# 3. Naikkan database
docker compose up -d
docker compose ps   # tunggu status healthy

# 4. Migrasi (MANUAL — tidak auto-run saat startup, lihat orchestrator/src/main.rs)
cargo install sqlx-cli --no-default-features --features postgres,rustls  # sekali saja
sqlx migrate run --source orchestrator/migrations

# 5a. Jalankan backend (port 8080)
cd orchestrator && cargo run
curl http://localhost:8080/health
# harapannya: {"status":"ok","database":"ok"}

# 5b. (terminal lain, root repo) Demo Sectors / MCP server
cargo run --bin radar-emiten        # demo live: subsectors + BBCA 2 kuartal (butuh SECTORS_API_KEY)
cargo run --bin mcp-server          # 6 tools via MCP stdio
```

Dashboard: buka `http://localhost:8080/dashboard` (file `dashboard/index.html`, auto-refresh 5 detik).

## 3. Konfigurasi penting di `.env`

| Variabel | Default | Artinya |
|---|---|---|
| `APP_MODE` | `mock` | `mock` = WhatsApp/Gemini tiruan (`mock_sent`), aman untuk dev. `real` = kirim beneran, butuh semua kredensial (divalidasi di `orchestrator/src/config.rs:101-116`). |
| `SCHEDULER_ENABLED` | `false` | Penjadwal harian F2 (`orchestrator/src/scheduler.rs`). Set `true` + `SCHEDULER_INTERVAL_SECONDS` kalau mau pemantauan otomatis jalan. |
| `MAX_ALERTS_PER_DAY` | `3` | Batas alert per user per hari. |
| `NEUTRAL_RELATIVE_THRESHOLD` | `0.02` | Perubahan relatif < 2% dianggap netral. Asumsi sementara, bukan hasil validasi data. |
| `METRIC_REGISTRY` | (10 metric default) | Override whitelist, koma-separated. Lihat `07-indikator-testing.md`. |
| `POLICY_DISCLAIMER_CONFLICT_ACKNOWLEDGED` | `true` | Harus `true` agar mode `real` mau kirim (konflik disclaimer sudah selesai 7 Okt 2026). |
| `INTERNAL_API_TOKEN` | — | Wajib untuk `POST /internal/*`. Dibandingkan constant-time (`orchestrator/src/auth.rs`). |
| `ADMIN_PHONE` | — | Nomor admin untuk `/kredit` (digit saja, tanpa `+`). |

Mode WhatsApp real butuh: `APP_MODE=real` + `WHATSAPP_ACCESS_TOKEN`, `WHATSAPP_PHONE_NUMBER_ID`, `WHATSAPP_APP_SECRET`, `WHATSAPP_VERIFY_TOKEN`, `INTERNAL_API_TOKEN`, `GEMINI_API_KEY`. Token Meta cepat kedaluwarsa — regenerasi via dashboard Meta kalau dapat 401.

## 4. Perintah harian

```bash
cargo check            # cek kompilasi cepat
cargo fmt --check      # format harus bersih
cargo test             # full suite (butuh DB lokal untuk integration test)
cargo test --lib       # unit test saja, tanpa DB
sqlx migrate run --source orchestrator/migrations   # tiap habis pull migrasi baru
```

## 5. Yang belum selesai (jujur, per audit 7 Okt 2026)

1. `RealMcpGateway` masih stub `TransportUnavailable` (`orchestrator/src/integrations/mcp_gateway.rs`) — transport MCP real belum disepakati.
2. `raw_data` di `POST /snapshots` diterima lalu dibuang (`orchestrator/src/routes/snapshots.rs:64-65`) — butuh migrasi kolom `raw_data JSONB` kalau mau simpan evidence mentah.
3. Durasi `/diam` belum disepakati — saat ini mute sampai `/lanjut` (2099-12-31, lihat `services/command_router.rs:98-108`).
4. `GEMINI_MODEL` kosong di `.env` contoh — isi sesuai model yang dipakai tim.
5. Prompt ekstraksi tesis tidak ada di `04-agent-prompts.md` — schema-nya ada di `services/gemini_client.rs`, perlu konfirmasi pemilik prompt.
6. Webhook publik butuh tunnel (ngrok/Cloudflare) + registrasi di Meta + subscribe `messages`. Balasan teks di luar jendela 24 jam butuh template yang disetujui.
7. Deteksi kata terlarang tidak menangkap imbuhan ("membeli" lolos dari "beli") — keterbatasan `compliance_service.rs`, didokumentasikan di `04-agent-prompts.md`.

## 6. Troubleshooting

| Gejala | Penyebab umum | Perintah |
|---|---|---|
| `Gagal konek ke PostgreSQL` | DB belum naik / `DATABASE_URL` salah | `docker compose ps`, `docker compose logs db` |
| `/health` balas `database: fail` / 503 | Migrasi belum jalan | `sqlx migrate run --source orchestrator/migrations` |
| `APP_MODE=real` langsung exit | Kredensial real belum lengkap | Isi 6 variabel wajib (lihat §3) atau kembali ke `mock` |
| WhatsApp `sent` tapi tidak sampai HP | Di luar jendela 24 jam Meta | Minta user kirim pesan dulu, atau pakai template `hello_world` |
| `metric_name ... tidak ada di registry` | Metric di luar whitelist | Cek `07-indikator-testing.md`, set `METRIC_REGISTRY` |
| `401` dari Graph API | Token Meta kedaluwarsa | Regenerasi token di dashboard Meta |

Jangan pernah `git add -f .env`. File ini sudah di-`.gitignore` — kalau bocor, rotasi key dulu baru bersihkan riwayat.

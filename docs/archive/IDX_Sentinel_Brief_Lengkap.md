# IDX Sentinel — Brief Proyek Lengkap

> Dokumen ini dibuat agar bisa **ditempel utuh ke AI lain** sebagai konteks. Semua yang perlu diketahui ada di sini: lomba, tim, produk, fitur, arsitektur, data, aturan, jadwal, dan hal yang belum diputuskan.
> Terakhir diperbarui: 7 Oktober 2026. **Jangan menempelkan API key, token, atau password ke AI mana pun.**

---

## 1. Konteks singkat

- **Lomba:** Sectors Hackathon 2026 (penyelenggara: Sectors, data pasar modal Indonesia/IDX).
- **Tim:** 3 orang mahasiswa. Saya (Rafi) memegang **Orchestrator & WhatsApp**.
- **Tingkat saya:** pemula di AI agent dan baru belajar Rust. Mohon jelaskan langkah dengan sederhana, berurutan, dan beri tanda "selesai jika".
- **Bahasa pemrograman:** **Rust** (sudah diputuskan, tidak diganti).
- **Batas waktu:** build dan pengumpulan tutup **8 Oktober 2026 pukul 23.59 WIB**. Pendaftaran tim tutup 7 Oktober. Setelah submit, repo **dibekukan** (tidak boleh commit lagi). Hari ini 7 Oktober, jadi sisa sekitar 1,5 hari.
- **Track yang disarankan:** Track 1 (AI Agents & Assistants). Keputusan akhir track diambil bersama tim.

## 2. Aturan lomba yang paling berpengaruh

1. Repo GitHub **publik**, dibuat selama masa build, dan tetap publik minimal 90 hari setelah pengumuman. Juri bisa memeriksa riwayat commit, jadi commit rutin dari ketiga anggota.
2. **Tidak boleh ada API key di repo.** Jika terlanjur ter-commit: ganti key dulu di layanan asal, bersihkan repo, dan kabari kanal support penyelenggara.
3. Data Sectors (MCP atau REST API) harus menjadi **sumber data inti**.
4. Harus ada prototipe yang **berjalan ujung ke ujung**. Deploy live tidak wajib; repo publik + video demo sudah cukup.
5. **Dilarang memberi nasihat keuangan** dan **dilarang eksekusi jual/beli otomatis**. Produk harus berposisi sebagai alat informasi dan analisis, dengan disclaimer.
6. Teknologi tidak boleh dipalsukan untuk demo (juri memeriksa kode).
7. Yang dikumpulkan: link repo publik, video teaser 1 menit (publik), video juri maksimal 3 menit, pernyataan masalah satu kalimat, pilihan track dan nama anggota, serta postingan media sosial yang menandai akun resmi Sectors.
8. Bobot penilaian: **kegunaan nyata 40%**, **video demo dan cerita 30%**, **kedalaman teknis 30%**.
9. Kredit API tim: 1.000 kredit, berlaku sampai 8 Oktober 2026.

## 3. Ide produk (sudah disepakati tim)

**Satu kalimat:** pengguna mengirim alasannya memegang sebuah saham (disebut *tesis*) lewat WhatsApp; agent memantau data Sectors dan **hanya mengabari ketika bukti berubah**, lengkap dengan angka, periode, dan sumber. Hasil juga tampil di **dashboard informasi**.

**Pengguna sasaran:** investor ritel Indonesia yang punya alasan memegang saham tetapi tidak sempat memeriksa laporan tiap kuartal.

**Contoh tesis:** "Saya pegang BBCA karena labanya terus tumbuh dan valuasinya masih wajar."

**Yang membuat produk menarik dan berbeda:**
- Setiap alert **berbukti** (angka sebelum/sekarang, periode, sumber).
- Sistem **berani menolak** klaim yang datanya kurang, dan itu ditampilkan.
- **Diam adalah fitur:** ada batas alert harian dan cooldown agar tidak membanjiri pengguna.
- **Kode menghitung, model bahasa menulis:** semua angka dihitung di Rust, LLM hanya dipakai untuk memahami kalimat tesis dan menyusun ringkasan.

**Prinsip desain:**
1. Kode menghitung, LLM menulis.
2. Tanpa bukti, tanpa pesan.
3. Diam adalah fitur.
4. Informasi, bukan saran (tanpa kata beli/jual/tahan, tanpa target harga).
5. Daftar indikator tertutup (whitelist) di kode, bukan dibebaskan ke LLM.

## 4. Keputusan terbaru tim (2 Oktober)

- **Dashboard informasi ditambahkan** di samping WhatsApp (bukan menggantikan). Dashboard dan WhatsApp membaca database yang sama.
- **Tetap multi-agent**, dimulai dari **4 agent** (bukan 10).
- Output dashboard **hanya informasi**, bukan saran tindakan.

## 5. Daftar fitur

| Kode | Fitur | Prioritas |
|---|---|---|
| F1 | Tesis lewat WhatsApp | Wajib |
| F2 | Pemantauan terjadwal & deteksi perubahan | Wajib |
| F3 | Status tesis tiga tingkat (Menguat / Netral / Melemah) | Wajib |
| F4 | Kartu bukti pada setiap alert | Wajib |
| F5 | Menu perintah WhatsApp | Wajib |
| F6 | Penolak bukti (evidence gate) | Sangat disarankan |
| F7 | Cooldown & batas alert harian | Sangat disarankan |
| F8 | Meter kredit API | Sangat disarankan |
| F11 | Dashboard informasi | Disarankan (dikerjakan setelah alur utama jalan) |
| F12 | Jejak agent (log kerja multi-agent) | Disarankan (bagian dari dashboard) |
| F9 | Tanya "kenapa?" | Jika sempat |
| F10 | Pembanding subsektor | Opsional |

### F1. Tesis lewat WhatsApp
- Pengguna mengirim kalimat bebas atau mengetik `/tesis`.
- Webhook menerima pesan dan mencatat ID pesan (agar pesan yang dikirim ulang WhatsApp tidak diproses dua kali).
- Gemini diminta mengekstrak **hanya ticker dan indikator dari daftar tertutup**, keluaran JSON terstruktur.
- Kode memvalidasi hasilnya; yang tidak dikenal dibuang.
- Agent meminta konfirmasi ("Balas YA untuk mulai"); tesis aktif setelah pengguna membalas YA.
- Batas: maksimal 3 tesis aktif per pengguna, maksimal 3 indikator per tesis. Pengguna harus opt-in dan selalu bisa `/diam` atau `/hapus`.

### F2. Pemantauan terjadwal & deteksi perubahan
- Penjadwal harian (jam dapat diatur) memicu alur.
- Langkah: ambil daftar ticker yang dipantau → panggil endpoint tanggal laporan kuartalan (dengan `since`) untuk mengecek ada laporan baru atau tidak → jika ada, ambil data lengkap → simpan snapshot → bandingkan dengan snapshot periode sebelumnya → hitung arah dukungan terhadap tesis (mendukung / melemahkan / netral) → bentuk `Finding`.
- Perubahan di bawah ambang dianggap netral. **Usulan awal ambang: 2% relatif**, disesuaikan setelah melihat data asli.
- Data fundamental berubah per kuartal, jadi **snapshot kuartal sebelumnya disiapkan sejak awal (cold-start)** agar alert pertama tidak kosong.
- Semua perhitungan di kode Rust.

### F3. Status tesis tiga tingkat
- **Menguat:** indikator yang mendukung lebih banyak daripada yang melemahkan.
- **Melemah:** indikator yang melemahkan lebih banyak.
- **Netral:** jumlahnya sama atau data belum cukup.
- Indikator tanpa data baru tidak dihitung dan ditandai "belum ada pembaruan".
- Status hanya menggambarkan kecocokan data dengan alasan pengguna, bukan ajakan bertindak.

### F4. Kartu bukti pada setiap alert
Format tetap dibentuk oleh kode (template), bukan ditulis bebas oleh LLM. LLM hanya boleh mengisi satu kalimat penjelasan. Contoh (angka hanya ilustrasi):

```
BBCA · Pertumbuhan laba
Sebelum: 8,1%  →  Sekarang: 5,4%  (2026-Q2)
Status tesis: MELEMAH (2 dari 3 indikator berlawanan)
Sumber: Sectors API v2 · Keyakinan: sedang
Ketik /bukti untuk melihat data lengkap.
Informasi ini bukan saran investasi.
```
Kode memeriksa bahwa setiap angka di pesan identik dengan angka di `Finding`.

### F5. Menu perintah WhatsApp
| Perintah | Fungsi |
|---|---|
| `/tesis` | Mulai menambah tesis (kalimat bebas juga diterima) |
| `/status` | Semua tesis aktif beserta statusnya |
| `/bukti` | Data di balik alert terakhir (dari database, tanpa memanggil API) |
| `/diam` / `/lanjut` | Jeda dan lanjutkan alert |
| `/hapus` | Hapus satu tesis (dengan konfirmasi) |
| `/bantuan` | Daftar perintah + disclaimer |
| `/kredit` | Meter kredit API, khusus nomor admin tim |

Pesan yang tidak dipahami dijawab dengan daftar perintah, bukan ditebak.

### F6. Penolak bukti (evidence gate)
Sebuah `Finding` lolos hanya jika:
- sumber dan periode terisi,
- nilai sebelum dan sekarang tersedia dan berupa angka,
- indikator ada di daftar tertutup,
- ticker, periode, dan angka konsisten dengan snapshot di database,
- periode itu belum pernah dikirim.

Jika gagal: status "ditolak" + alasan disimpan, tidak dikirim. Penolakan ditampilkan di dashboard dan di demo sebagai kekuatan.

### F7. Cooldown & batas alert harian
- Maksimal **3 alert per pengguna per hari** (parameter).
- **Satu alert per indikator per periode** (kunci unik: pengguna + tesis + indikator + periode).
- Jika alert melebihi batas, urutkan berdasarkan dampak pada status tesis; sisanya ditunda ke hari berikutnya.
- Pengguna yang `/diam` tidak dikirimi alert sampai jeda berakhir.

### F8. Meter kredit API
- Setiap panggilan ke Sectors dicatat: endpoint, perkiraan kredit, waktu, dari cache atau panggilan nyata.
- Hasil yang masih segar di-cache agar tidak diambil berulang untuk tesis berbeda.
- Biaya per endpoint berbeda dan **harus diukur sendiri** (catat saldo sebelum dan sesudah panggilan).

### F9. Tanya "kenapa?" (jika sempat)
Pengguna membalas "kenapa?". Agent mengambil `Finding` dan snapshot dari database, meminta Gemini merangkum 3 kalimat, lalu kode memeriksa semua angka identik dengan data asal dan menyaring kata terlarang. Terjadi dalam jendela layanan 24 jam, jadi tidak butuh template pesan.

### F10. Pembanding subsektor (opsional)
Contoh: "5,4% dibanding median subsektor bank 4,9%". Dikerjakan hanya jika inti sudah stabil dan biaya kredit masuk anggaran.

### F11. Dashboard informasi (baru)
Satu halaman yang menampilkan:
1. **Tesis dan statusnya** (Menguat/Netral/Melemah).
2. **Daftar temuan** dalam bentuk kartu bukti (angka sebelum/sekarang, periode, sumber).
3. **Temuan ditolak** beserta alasannya.
4. **Jejak agent** (lihat F12).
5. **Meter kredit dan cooldown** (sisa kredit, alert yang ditahan).

Aturan teknis:
- Axum menyediakan endpoint JSON; dashboard berupa **satu file HTML + JavaScript biasa** yang menyegarkan data tiap beberapa detik. Tanpa framework.
- **Nomor WhatsApp wajib disamarkan** (contoh `+62 812-••••-3456`) karena repo publik dan demo direkam.
- Tidak perlu deploy, cukup berjalan di laptop saat merekam video.
- Tidak boleh memuat saran beli/jual/target harga; tetap memuat disclaimer.

### F12. Jejak agent (baru)
Log urutan kerja multi-agent yang tampil di dashboard, misalnya:
`Scout menemukan laporan baru BBCA → Analyst menghitung perubahan → Evidence Checker meloloskan → Chief merangkum → pesan terkirim`.
Disimpan di tabel `jejak_agent`. Ini bagian yang paling menarik untuk video karena multi-agent terlihat nyata.

## 6. Multi-agent: 4 agent awal

| Agent | Tugas | Pakai LLM? |
|---|---|---|
| **Scout** | Memeriksa laporan kuartalan baru dan memilih ticker yang perlu diambil | Tidak (aturan) |
| **Analyst** | Membandingkan nilai sebelum dan sekarang, menentukan arah dukungan | Tidak (kode Rust) |
| **Evidence Checker** | Menolak temuan yang datanya kurang (F6) | Tidak (aturan) |
| **Chief** | Menggabungkan temuan, menghitung status tesis, menyusun ringkasan | Ya, hanya untuk kalimat |

Catatan jujur: LLM hanya dipakai di dua tempat, yaitu **memahami kalimat tesis (F1)** dan **menulis ringkasan (Chief, F9)**. Jangan menyebut semua agent "pakai AI" jika sebenarnya aturan/kode; juri membuka repo. Menjelaskan bahwa angka dihitung kode adalah nilai plus.

## 7. Arsitektur dan tech stack

| Lapisan | Pilihan |
|---|---|
| Bahasa | **Rust** |
| Backend & orchestrator | Axum + Tokio |
| Database | PostgreSQL + SQLx (dijalankan lewat Docker Compose) |
| HTTP klien | reqwest + serde |
| Konfigurasi rahasia | file `.env` + paket `dotenvy` (tidak pernah masuk repo) |
| Penjadwal | `orchestrator/src/scheduler.rs` — `run_scheduler()` memanggil `AgentPipeline::run_cycle()` berkala (interval `SCHEDULER_INTERVAL_SECONDS`, aktif bila `SCHEDULER_ENABLED=true`). |
| MCP server | Rust + `rmcp` (dikerjakan anggota data & MCP) |
| LLM | Gemini API (kelas Flash), keluaran JSON terstruktur |
| Pesan | WhatsApp Business Platform Cloud API (mode real: credentials terisi + `APP_MODE=real`; mode mock: `mock_sent`) |
| Dashboard | satu file HTML + JavaScript biasa, membaca endpoint JSON dari Axum |

Tidak perlu: Redis, Kafka, Kubernetes, vector database, atau framework frontend.

### Endpoint backend yang direncanakan
| Endpoint | Fungsi |
|---|---|
| `GET /health` | Cek aplikasi hidup |
| `GET/POST /webhook/whatsapp` | Verifikasi webhook dan penerimaan pesan masuk |
| `POST /snapshots` | Simpan data snapshot dari agent |
| `GET /snapshots` | Ambil snapshot untuk pembanding |
| `POST /findings` | Menerima `Finding` dari agent analisis |
| `POST /jejak` | Catat jejak kerja agent |
| `POST /kredit` | Catat penggunaan kredit API |
| `GET /pantauan` | Daftar ticker & indikator aktif |
| `GET /internal/snapshots/previous` | Snapshot pembanding historis |
| `POST /internal/findings` | Terima Finding dari MCP server |
| `GET /api/tesis` | Daftar tesis + status (nomor disamarkan) |
| `GET /api/findings` | Temuan yang lolos |
| `GET /api/ditolak` | Temuan ditolak + alasan |
| `GET /api/jejak` | Jejak kerja agent |
| `GET /api/kredit` | Penggunaan dan sisa kredit |

### Detail teknis WhatsApp
- Gunakan **nomor uji** dari Meta dan daftarkan nomor HP penerima demo. Verifikasi bisnis penuh tidak wajib untuk tahap uji.
- Token sementara cepat kedaluwarsa; buat token permanen lewat System User di Business Settings.
- Menerima pesan butuh **webhook** yang bisa diakses internet; saat pengembangan bisa memakai tunnel (ngrok atau Cloudflare Tunnel).
- Pengguna yang mengirim pesan lebih dulu membuka **jendela layanan 24 jam** (balasan bebas diperbolehkan). Alert yang kita mulai di luar jendela itu butuh **template yang disetujui Meta**. Untuk demo, atur skenario di dalam jendela 24 jam.
- Validasi tanda tangan webhook (`X-Hub-Signature-256`) dan simpan ID pesan untuk mencegah pemrosesan ganda.

## 8. Data dari Sectors API v2

**Sudah terkonfirmasi dari dokumentasi resmi:**
- Endpoint **v1 sudah usang**; gunakan **v2**.
- **Laporan perusahaan** dapat diambil per bagian: overview, valuation, future, peers, financials, dividend, management, ownership.
- Endpoint **tanggal laporan kuartalan terbaru** untuk semua emiten IDX, mendukung parameter `since` untuk memeriksa data baru (murah, cocok untuk pemicu).
- Endpoint **harga penutupan harian** untuk semua ticker IDX pada satu tanggal.
- **Data keuangan kuartalan historis** tidak lagi ada di laporan perusahaan; hanya dari **Companies Screener**, yang mendukung penulisan field kuartalan dengan akhiran `_q` pada parameter `where`.
- Companies Screener mendukung filter terstruktur (`where`, misalnya subsektor `banks`) dan kueri bahasa alami (`q`).

**Sudah terverifikasi dari kode:**
- Field yang dipakai: `net_interest_margin_q`, `pertumbuhan_laba_q`, `pertumbuhan_pendapatan_q` (screener)
- Laporan kuartalan: `companies/reports/quarterly?since=YYYY-MM-DD`
- Screener: `companies/screener/?fields=...&where=symbol==TICKER`

**Belum terverifikasi (WAJIB diuji sebelum mengunci fitur):**
1. **Biaya kredit per panggilan.** Informasi awal (belum saya verifikasi): subsektor ±1 kredit, kueri terstruktur ±1 kredit, kueri bahasa alami ±3 kredit. Ukur sendiri.
2. **Akses akun API tim** sudah aktif (sempat muncul pesan "Limited to Sectors Insider subscribers" di awal).
3. **Ambang perubahan netral** yang masuk akal (lihat sebaran perubahan antar kuartal).
4. **Subsektor demo**: pilih satu dengan data paling lengkap (kandidat: `banks`).

**Uji 30 menit di API Playground:**
1. Companies Screener untuk subsektor `banks`, pilih beberapa field, coba versi `_q`; catat field yang muncul.
2. Laporan perusahaan satu ticker dengan `sections=financials,valuation`.
3. Endpoint tanggal laporan kuartalan dengan `since=`.
4. Endpoint harga penutupan harian untuk satu tanggal.
5. Catat saldo kredit sebelum dan sesudah tiap panggilan.

**Aturan keputusan:** aplikasi layak jika ditemukan minimal **2–3 indikator** untuk subsektor yang dipilih, dengan **nilai kuartal sebelumnya dan sekarang**, dan biaya kredit muat dalam 1.000 kredit. Jika field terbatas, sesuaikan indikator tesis (misalnya pertumbuhan laba, valuasi, harga) dan perbarui kontrak `Finding`; ide tidak perlu diganti.

## 9. Kontrak data `Finding` (titik temu ketiga anggota tim)

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
  "finding_summary": "Net Profit Margin (NPM) BBCA pada Q3 2024 tercatat 46.85%, naik 3.73 poin dibanding Q3 2023 sebesar 43.12%."
}
```

> **Catatan (7 Okt 2026):** Field `Finding` di `src/findings/mod.rs` telah diubah dari bahasa Indonesia ke bahasa Inggris untuk konsistensi dengan `CONTRACTS.md` dan `orchestrator/models/finding.rs`.
- `confidence_score`: 0.0–1.0; **rendah (<0.5) tidak dikirim**.
- Kontrak final mengikuti `docs/CONTRACTS.md` — field memakai bahasa Inggris sesuai implementasi kode.
- Perubahan kontrak harus diberitahukan ke semua anggota.

## 10. Rancangan tabel PostgreSQL

| Tabel | Kolom utama |
|---|---|
| `pengguna` | id, nomor_wa, opt_in, jeda_sampai, dibuat_pada |
| `tesis` | id, pengguna_id, teks_asli, ticker, status, aktif, dibuat_pada |
| `indikator_tesis` | id, tesis_id, indikator, arah_baik |
| `snapshot_data` | id, ticker, indikator, periode, nilai, sumber, diambil_pada |
| `findings` | id, tesis_id, indikator, arah_dukungan, nilai_sebelum, nilai_sekarang, periode, sumber, tingkat_keyakinan, status (lolos/ditolak), alasan_tolak, dibuat_pada |
| `alerts` | id, pengguna_id, tesis_id, finding_id, isi_pesan, status_kirim, dikirim_pada |
| `pesan_masuk` | id_pesan_wa (unik), pengguna_id, isi, diterima_pada |
| `penggunaan_kredit` | id, endpoint, kredit, dari_cache, dicatat_pada |
| `jejak_agent` | id, agent, aksi, hasil_singkat, ticker, dicatat_pada |

## 11. Alur ujung ke ujung

1. Pengguna mengirim tesis ke WhatsApp → webhook diverifikasi dan pesan dicatat.
2. Orchestrator meminta Gemini mengekstrak ticker dan indikator, lalu memvalidasi dengan daftar tertutup.
3. Agent mengonfirmasi indikator; pengguna membalas YA; tesis tersimpan.
4. Penjadwal berjalan: **Scout** memeriksa laporan baru dan mengambil data; snapshot disimpan.
5. **Analyst** menghitung perubahan dan arah dukungan → `Finding`.
6. **Evidence Checker** meloloskan atau menolak (alasan dicatat).
7. **Chief** menghitung status tesis dan menyusun ringkasan; cooldown memeriksa batas harian.
8. Kartu bukti dibentuk dari template dan dikirim lewat WhatsApp; status pengiriman dicatat.
9. Setiap langkah dicatat di `jejak_agent` dan tampil di dashboard.
10. Pengguna dapat membalas `/bukti` atau "kenapa?" dan mendapat jawaban dari database.

## 12. Pembagian tanggung jawab (usulan, perlu disepakati)

| Peran | Nama | Fokus |
|---|---|---|
| **Orchestrator & WhatsApp** | Rafi | F1, F3, F4, F5, F7, F8; webhook, database, penjadwal, pengiriman WhatsApp, filter kata terlarang, endpoint JSON dashboard, jejak agent |
| **Agent Logic & Compliance** | Dian | Master prompts (4 agent), compliance rule engine, zero-hallucination verifier, few-shot test cases |
| **Data & MCP** | Rafiq | Klien Sectors API v2, pengambilan data dan snapshot (Scout), cache, pengukuran kredit; memegang API key tim |

Hanya Rafiq yang memegang kunci Sectors; yang lain mengakses data lewat MCP/backend tanpa menaruh kunci di repo.

## 13. Aturan kepatuhan (wajib)

- Dilarang memuat kata **beli, jual, tahan, rekomendasi, target harga**. Kode memblokir pesan yang mengandungnya sebelum dikirim.
- Setiap alert memuat: *"Informasi ini bukan saran investasi."*
- Tidak ada eksekusi transaksi apa pun.
- Pengguna demo adalah orang yang setuju menerima pesan.
- Kunci API (Sectors, Gemini, WhatsApp) hanya di `.env` yang masuk `.gitignore`.
- Nomor WhatsApp pengguna disamarkan di dashboard dan video.
- Instruksi ke Gemini: jangan menyarankan beli/jual/tahan dan jangan menyebut target harga.

## 14. Struktur repo yang direncanakan

```
radar-emiten/
├── src/
│   ├── bin/
│   │   └── mcp-server.rs      # MCP server (Rafiq)
│   ├── sectors/               # Sectors API v2 client + cache + credit tracker
│   ├── findings/              # DTO Finding
│   ├── lib.rs
│   └── main.rs
├── orchestrator/              # Backend Axum (Rafi)
│   ├── src/
│   │   ├── orchestrator/      # Agent pipeline
│   │   ├── routes/            # HTTP endpoints
│   │   ├── repositories/      # Database queries (SQLx)
│   │   ├── services/          # WhatsApp, Gemini, MCP gateway
│   │   ├── models/            # DTO internal
│   │   ├── integrations/      # MCP gateway trait
│   │   └── ...
│   ├── migrations/            # SQL migrations
│   └── tests/                 # Integration tests
├── docs/
│   ├── ARCHITECTURE.md
│   ├── CONTRACTS.md
│   ├── PROMPTS.md
│   ├── TASKS.md
│   ├── integration-decisions.md
│   ├── sectors-api-notes.md
│   ├── indikator.md
│   ├── evaluasi.md
│   └── api-internal.md
├── dashboard/                 # SELESAI (7 Okt 2026): index.html satu file + diserve Axum di /dashboard
├── docker-compose.yml         # PostgreSQL
├── .env.example
├── .gitignore                 # harus memuat: /target, .env, *.pem
├── Cargo.toml                 # Workspace: 2 crate
└── README.md
```
Kerja di branch masing-masing dan gabungkan lewat Pull Request. Commit kecil dengan pesan jelas.

## 15. Jadwal (2 – 8 Oktober 2026)

| Tanggal | Pekerjaan |
|---|---|
| 2 – 3 Okt | Uji API Sectors (30 menit), kunci indikator dan kontrak `Finding`. Backend Axum, tabel, webhook, kirim WhatsApp dari Rust, validasi dan pencegah duplikat (F1, F5, F7). |
| 4 – 5 Okt | Gemini untuk ekstraksi tesis dan ringkasan, kartu bukti, status tesis, penjadwal harian (F2, F3, F4). |
| 6 Okt | Sambung dengan agent dan data tim, penolak bukti, meter kredit, endpoint dashboard, jejak agent, uji ujung ke ujung, rekam video cadangan (F6, F8, F11, F12). |
| 7 Okt | **Bekukan fitur.** README, video teaser dan juri, postingan media sosial, periksa tidak ada kunci di repo. |
| 8 Okt | Submit **lebih awal di hari itu** (batas 23.59 WIB). Setelah submit repo dibekukan. |

**Urutan prioritas jika waktu mepet:** alur WhatsApp + database dulu → agent → dashboard terakhir. F9 dan F10 boleh dilewati tanpa mengurangi inti produk. Dashboard yang rapi bernilai, tetapi tidak boleh membuat alur utama terlambat.

**Titik cek jujur:** jika sampai 3 Oktober pesan WhatsApp belum bisa terkirim dari kode Rust, bicarakan dengan tim untuk berbagi beban (bukan mengganti bahasa).

## 16. Skenario video demo

1. Pengguna mengirim tesis lewat WhatsApp; agent menjawab indikator yang akan dipantau.
2. Pengguna membalas YA; tesis aktif.
3. Penjadwal berjalan; **dashboard menampilkan jejak agent** bekerja (Scout → Analyst → Checker → Chief).
4. Alert kartu bukti masuk di WhatsApp; status tesis berubah menjadi **Melemah**.
5. Pengguna mengetik `/bukti` dan "kenapa?" lalu mendapat data asalnya.
6. Satu temuan sengaja dibuat tidak lengkap dan **DITOLAK**; alasannya tampil di dashboard.
7. Tampilkan meter kredit dan bukti cooldown anti-spam.

Penyampaian: kalimat produk, siapa penggunanya, alur utama, dan satu hal yang membuktikan sistem tidak mengarang (penolakan bukti).

## 17. Yang sudah diputuskan vs yang masih terbuka

**Sudah diputuskan:**
- Bahasa Rust; konsep tesis lewat WhatsApp; fitur F1–F8 plus kartu bukti, status tiga tingkat, menu perintah, cooldown, meter kredit.
- Dashboard informasi ditambahkan; tetap multi-agent (4 agent); output hanya informasi.

**Masih terbuka:**
- Nama final repo/produk (saran: `idx-sentinel`; alternatif `thesis-watch`, `radar-emiten`).
- Track final (disarankan Track 1).
- Indikator final dan nama field Sectors (menunggu uji API).
- Ambang perubahan netral dan batas alert harian (usulan: 2% dan 3 alert/hari).
- Siapa yang membuat tampilan dashboard.
- Subsektor demo (kandidat `banks`).

## 18. Risiko utama dan penanganannya

| Risiko | Penanganan |
|---|---|
| Rust sulit bagi pemula | Belajar sambil mengerjakan; mulai dari hal kecil (Axum "hello", lalu kirim WhatsApp, lalu SQLx); minta bantuan tim; AI coding tools diizinkan |
| Data fundamental berubah per kuartal, alert "live" jarang | Siapkan snapshot kuartal sebelumnya sejak awal; demo membandingkan kuartal terbaru vs sebelumnya |
| Template WhatsApp belum disetujui | Atur skenario demo di dalam jendela 24 jam; rekam video cadangan |
| Kredit API habis | Cache hasil, pakai endpoint murah untuk pemicu, catat biaya per panggilan |
| Kunci bocor ke repo | `.gitignore` sejak commit pertama, cek `git status`, ganti kunci jika bocor |
| Waktu mepet | Ikuti urutan prioritas; bekukan fitur 7 Oktober |
| Teknologi dianggap dipalsukan | Perhitungan nyata di kode; jejak agent dan log tersimpan di database |

---

## Lampiran A. Cara memakai dokumen ini dengan AI lain

Tempel seluruh isi dokumen ini, lalu tambahkan salah satu permintaan berikut:

- *"Berdasarkan brief di atas, buatkan skema SQL PostgreSQL lengkap untuk tabel-tabel yang dirancang."*
- *"Buatkan kerangka proyek Rust (Axum + SQLx) untuk bagian orchestrator, dengan struktur modul dan penjelasan untuk pemula."*
- *"Buatkan kode Rust untuk mengirim pesan WhatsApp Cloud API dan menerima webhook beserta validasi tanda tangannya."*
- *"Buatkan prompt Gemini dengan keluaran JSON terstruktur untuk mengekstrak ticker dan indikator dari kalimat tesis, dengan whitelist indikator."*
- *"Buatkan fungsi Rust untuk menghitung status tesis (Menguat/Netral/Melemah) dan blokir kata terlarang."*
- *"Rancang halaman dashboard satu file HTML yang membaca endpoint JSON di atas, dengan nomor WhatsApp disamarkan."*
- *"Tinjau rencana ini dan sebutkan risiko terbesar yang mungkin saya lewatkan."*

## Lampiran B. Instruksi tetap untuk AI lain

Mohon patuhi hal berikut dalam semua jawaban:
1. Jangan menyarankan fitur yang memberi saran beli/jual atau menjalankan transaksi.
2. Jangan meminta atau menampilkan API key; gunakan nama variabel dari `.env.example`.
3. Jelaskan untuk pemula Rust dan pemula AI agent, dengan langkah berurutan dan tanda "selesai jika".
4. Perhitungan angka harus di kode, bukan oleh LLM.
5. Jika ada informasi yang tidak pasti (misalnya nama field Sectors atau biaya kredit), katakan dengan jelas dan sarankan cara mengujinya, jangan menebak.
6. Tetap dalam batas waktu: selesai sebelum 8 Oktober 2026 pukul 23.59 WIB.

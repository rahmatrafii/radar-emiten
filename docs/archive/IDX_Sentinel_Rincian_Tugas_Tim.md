# IDX Sentinel — Rincian Tugas Tim (3 Anggota)

> Sectors Hackathon 2026 — Disusun 2 Oktober 2026 — Diperbarui 7 Oktober 2026 — Batas submit 8 Oktober 2026 pukul 23.59 WIB

## 1. Cara membaca dokumen ini

Dokumen ini memecah proyek menjadi tugas kecil untuk tiga peran. Setiap tugas punya ID, target tanggal, dan tanda "selesai jika" agar jelas kapan boleh lanjut. Pembagian ini usulan awal yang disusun dari peran yang sudah disebut di chat tim. Sesuaikan dengan keahlian masing-masing, tetapi pastikan setiap tugas punya satu pemilik.

| Peran | Nama | Tanggung jawab inti | Folder di repo |
|---|---|---|---|
| Data & MCP | Rafiq | Mengambil data Sectors API v2, Scout agent, cache, pencatatan kredit | `src/bin/mcp-server.rs`, `src/sectors/` |
| Agent Analisis | Dian | Master prompts, compliance rule engine, zero-hallucination verifier, few-shot test cases | `orchestrator/src/orchestrator/`, `orchestrator/src/services/` |
| Orchestrator & WhatsApp | Rafi | Database, WhatsApp, Chief agent, status tesis, kartu bukti, cooldown, API dashboard | `orchestrator/` |

Aturan emas: satu pemilik database, yaitu Rafi. Rafiq dan Dian tidak mengakses PostgreSQL langsung, tetapi lewat endpoint HTTP (bagian 6). Ini mencegah bentrok skema dan mempermudah pemeriksaan.

## 2. Tugas bersama (semua anggota)

| ID | Tugas dan rincian | Target | Selesai jika |
|---|---|---|---|
| T1 | Hari ini: sepakati tiga hal. Kontrak Finding (`docs/CONTRACTS.md`). Satu subsektor demo dan 2-3 indikator (setelah uji API). Track lomba dan nama produk. | 2 Okt | Ketiganya menyetujui di grup dan file kontrak sudah di repo. |
| T2 | Titik cek integrasi: 3 Okt: pesan WhatsApp terkirim dari kode Rust. 5 Okt: alur ujung ke ujung dengan data tiruan. 6 Okt: alur ujung ke ujung dengan data Sectors asli. | 3, 5, 6 Okt | Setiap titik cek dicatat lolos atau tidak di grup. |
| T3 | Sinkron singkat 10 menit setiap hari (pagi atau malam): apa yang selesai, apa yang menghambat, apa yang dikerjakan besok. Jika macet lebih dari 2 jam, langsung minta bantuan, jangan menunggu. | Harian | Tidak ada hambatan yang menggantung lebih dari sehari. |
| T4 | README dan diagram alur: Rafi menulis kerangka dan bagian WhatsApp; Rafiq dan Dian menulis bagian masing-masing. Cara menjalankan harus bisa diikuti dari nol. | 7 Okt | Orang di luar tim bisa menjalankan proyek dengan README saja. |
| T5 | Video: teaser 1 menit dan video juri maksimal 3 menit. Usulan: Rafi merekam bagian WhatsApp, Dian merekam dashboard dan jejak agent, Rafiq menjelaskan data dan penghematan kredit. Satu orang menyunting. | 7 Okt | Dua video publik di YouTube atau media sosial, durasi sesuai aturan. |
| T6 | Postingan media sosial yang menandai akun resmi Sectors, memakai template thumbnail dari penyelenggara. | 7 Okt | Link postingan siap dimasukkan ke formulir submit. |
| T7 | Pemeriksaan akhir: tidak ada API key di repo atau riwayat commit, repo publik, disclaimer ada, nomor WhatsApp disamarkan. | 7 Okt | Daftar periksa bagian 8 terpenuhi semua. |
| T8 | Submit lewat portal oleh perwakilan tim. Lakukan lebih awal pada 8 Okt, jangan menunggu menit terakhir. Setelah submit, repo dibekukan dan tidak boleh ada commit lagi. | 8 Okt | Submission tercatat di portal. |

## 3. Rafiq: Data & MCP

Tujuan: menyediakan data Sectors yang bersih, hemat kredit, dan terdokumentasi, serta menjalankan Scout agent. Pekerjaan Rafiq membuka jalan bagi dua anggota lain, jadi D1 harus selesai paling dulu.

| ID | Tugas dan rincian | Target | Selesai jika |
|---|---|---|---|
| D1 | Uji API 30 menit di API Playground (URGENT). Companies Screener untuk subsektor banks, coba field versi _q; catat field yang tersedia. Laporan perusahaan satu ticker (financials dan valuation). Endpoint tanggal laporan kuartalan dengan since. Endpoint harga penutupan harian. Catat saldo kredit sebelum dan sesudah tiap panggilan. Serahkan hasilnya di `docs/sectors-api-notes.md` (tanpa API key). | 2 Okt | Ada daftar field yang tersedia, biaya kredit per panggilan, dan usulan 2-3 indikator. |
| D2 | Pastikan akses API tim aktif: buat key, satu panggilan berstatus 200. Jika masih muncul batasan Insider, kirim bukti ke dukungan penyelenggara. Simpan key hanya di .env milik perwakilan; jangan membagikannya lewat chat. | 2 Okt | Panggilan uji berstatus 200. |
| D3 | Klien Sectors v2 di Rust (reqwest dan serde): baca key dari .env, tangani error (termasuk 410 untuk endpoint lama), batasi percobaan ulang. | 3 Okt | Klien bisa mengambil data satu ticker dan mengembalikan JSON yang sudah dirapikan. |
| D4 | MCP server (rmcp) dengan 4-6 tools bertipe ketat, misalnya: daftar subsektor, saring emiten, laporan perusahaan, tanggal laporan kuartalan (since), harga penutupan. Parameter dibatasi nilai yang sah; agent tidak boleh menyusun URL bebas. | 3-5 Okt | Setiap tool bisa dipanggil dan mengembalikan format seragam beserta catatan sumber. |
| D5 | Cache hasil yang masih segar agar satu data dipakai banyak tesis tanpa memanggil API lagi. | 4 Okt | Panggilan kedua untuk data yang sama tidak mengurangi kredit. |
| D6 | Scout agent: tanya `GET /pantauan` untuk daftar ticker dan indikator aktif, cek tanggal laporan baru dengan since, ambil data yang perlu, kirim ke `POST /snapshots`, lalu catat langkahnya di `POST /jejak`. | 4-5 Okt | Dengan data nyata, Scout mengisi snapshot dan jejak untuk satu ticker. |
| D7 | Baseline cold-start: ambil data kuartal sebelumnya untuk ticker demo dan kirim ke `POST /snapshots` agar alert pertama punya pembanding. | 4 Okt | Untuk ticker demo, ada snapshot dua periode berurutan. |
| D8 | Pencatatan kredit: setiap panggilan ke Sectors dikirim ke `POST /kredit` (endpoint, kredit, dari cache atau nyata). | 5 Okt | Total kredit di database cocok dengan saldo di akun Sectors. |
| D9 | Dokumentasi: bagian data di README dan `docs/sectors-api-notes.md` (endpoint, field, biaya, batasan). | 7 Okt | Anggota lain bisa memahami sumber data tanpa bertanya. |
| D10 | Jika key bocor: ganti key di Sectors, bersihkan repo, dan kabari kanal dukungan penyelenggara sesuai aturan. | Jika terjadi | Key lama dinonaktifkan dan repo bersih. |

## 4. Dian: Agent Analisis & Compliance

Tujuan: menentukan indikator, menghitung perubahan, dan menolak temuan yang datanya kurang. Semua angka dihitung kode, bukan LLM. Dian juga dapat memegang dashboard (atau diserahkan ke anggota yang mengusulkannya).

| ID | Tugas dan rincian | Target | Selesai jika |
|---|---|---|---|
| A1 | Daftar indikator tertutup (whitelist) bersama Rafiq, berdasarkan hasil D1. Untuk tiap indikator tentukan: nama, satuan, dan arah_baik (naik atau turun dianggap baik bagi tesis). Tulis di `docs/indikator.md`. | 3 Okt | Ada 2-3 indikator final untuk subsektor demo. |
| A2 | Ambang perubahan netral (usulan awal 2% relatif) dan aturan confidence_score (0.0-1.0). Sesuaikan setelah melihat sebaran data asli. | 3 Okt | Aturan tertulis dan disetujui tim. |
| A3 | Data uji (fixture) berupa Finding palsu yang menghasilkan tiga status: Menguat, Netral, Melemah. Serahkan ke Rafi untuk menguji F3. | 3 Okt | Tiga berkas JSON uji tersedia di repo. |
| A4 | Analyst: ambil snapshot sekarang dan sebelumnya, hitung persentase perubahan, tentukan arah_dukungan dan confidence_score, hasilkan Finding. Tambahkan uji otomatis. | 3-5 Okt | Uji otomatis lolos untuk kasus naik, turun, dan hampir sama. |
| A5 | Evidence Checker: sebuah Finding lolos hanya jika sumber dan periode terisi, kedua nilai angka tersedia, indikator ada di whitelist, ticker dan periode konsisten dengan snapshot, dan periode belum pernah dikirim. Jika gagal: status ditolak dengan kode alasan yang jelas. | 4-5 Okt | Satu Finding sengaja tidak lengkap ditolak dengan alasan tercatat. |
| A6 | Integrasi: ambil data lewat `GET /snapshots`, kirim hasil ke `POST /findings`, catat langkah di `POST /jejak`. | 5 Okt | Satu alur Analyst → Checker → /findings berjalan dengan data tiruan. |
| A7 | Dashboard (satu file HTML dan JavaScript biasa) di folder `dashboard/`. Mulai dari JSON tiruan agar tidak menunggu backend. Bagian: tesis dan status, daftar temuan (kartu bukti), temuan ditolak, jejak agent, meter kredit dan cooldown. Nomor WhatsApp harus disamarkan. Tidak ada saran beli atau jual; ada disclaimer. | 4-6 Okt | Dashboard menampilkan data nyata dari /api/* dan segar tiap beberapa detik. |
| A8 | Evaluasi: jalankan minimal 5 kasus uji dan catat hasilnya di `docs/evaluasi.md`. Dokumentasikan logika agent di `docs/ARCHITECTURE.md`. | 6-7 Okt | Dokumen evaluasi berisi kasus, hasil, dan keterbatasan. |
| A9 | Opsional jika waktu cukup: pembanding subsektor (F10). | 6 Okt | Hanya dikerjakan setelah semua tugas lain lolos. |

## 5. Rafi: Orchestrator & WhatsApp

Tujuan: menjadi pusat sistem: database, WhatsApp dua arah, Chief agent, status tesis, kartu bukti, cooldown, dan API untuk dashboard. Urutan kerja sengaja dibuat dari yang paling mandiri ke yang bergantung pada anggota lain.

| ID | Tugas dan rincian | Target | Selesai jika |
|---|---|---|---|
| R0 | Persiapan: pasang Rust, Docker, VS Code; jalankan PostgreSQL lewat docker-compose; proyek Axum "hello"; buat .env (tanpa commit) dan pastikan .gitignore memuat .env. Buat kunci Gemini di Google AI Studio dan simpan di .env. | 2 Okt | Server menyala, database hidup, git status tidak menampilkan .env. |
| R1 | WhatsApp: buat app Meta tipe Business, tambahkan produk WhatsApp, daftarkan nomor HP penerima, kirim template hello_world, lalu buat fungsi kirim_wa di Rust (reqwest). | 2-3 Okt | Pesan WhatsApp masuk ke HP dari kode Rust. |
| R2 | Skema PostgreSQL dengan migrasi SQLx: pengguna, tesis, indikator_tesis, snapshot_data, findings, alerts, pesan_masuk, penggunaan_kredit, jejak_agent. | 3 Okt | Semua tabel terbentuk dari migrasi dan bisa diisi data uji. |
| R3 | Endpoint internal untuk anggota lain: `POST /findings`, `POST /snapshots`, `GET /snapshots`, `GET /pantauan`, `POST /jejak`, `POST /kredit`, `GET /health`. Validasi input dan tolak data tak lengkap. | 3-4 Okt | Rafiq dan Dian dapat memanggil endpoint dengan data tiruan. |
| R4 | Webhook WhatsApp: verifikasi, validasi tanda tangan X-Hub-Signature-256, catat ID pesan agar pesan ulang tidak diproses dua kali. Menu perintah: /tesis, /status, /bukti, /diam, /lanjut, /hapus, /bantuan, /kredit (admin). Pesan tak dikenal dibalas daftar perintah. | 3-4 Okt | Setiap perintah dijawab benar, pesan ganda diabaikan. |
| R5 | F1: ekstraksi tesis. Gemini mengembalikan JSON berisi ticker dan indikator dari whitelist; kode memvalidasi; agent meminta konfirmasi YA; tesis disimpan. Batas 3 tesis aktif per pengguna. Instruksi ke Gemini melarang saran beli, jual, tahan, dan target harga. | 4-5 Okt | Kalimat bebas menghasilkan tesis valid; indikator tak dikenal ditolak dengan pesan jelas. |
| R6 | F3 dan F4: hitung status tesis (Menguat, Netral, Melemah), bentuk kartu bukti dari template tetap, periksa setiap angka di pesan identik dengan Finding, blokir kata terlarang, dan tambahkan disclaimer. | 4-5 Okt | Tiga data uji dari A3 menghasilkan tiga status benar dan pesan terformat. |
| R7 | F7 dan penjadwal: kunci unik per pengguna + tesis + indikator + periode, batas 3 alert per hari, penundaan sisa alert, dan jeda /diam. **Catatan (7 Okt): Penjadwal otomatis sudah diimplementasikan** (`orchestrator/src/scheduler.rs` + spawn di `main.rs`, aktif bila `SCHEDULER_ENABLED=true`, default nonaktif). Fitur F2 (pemantauan terjadwal) aktif. | 5 Okt | Finding yang sama dikirim dua kali hanya menghasilkan satu pesan; alert ke-4 ditunda. |
| R8 | Chief agent: gabungkan Finding yang lolos, urutkan berdasarkan dampak pada status tesis, susun ringkasan (LLM hanya untuk kalimat), catat jejak, lalu kirim lewat WhatsApp. | 5 Okt | Alur lengkap menghasilkan satu pesan WhatsApp dari Finding yang lolos. |
| R9 | F8: tampilkan penggunaan kredit lewat /kredit dan /api/kredit memakai data dari POST /kredit. | 5-6 Okt | Total terpakai dan sisa tampil dan cocok dengan akun Sectors. |
| R10 | API dashboard: `GET /api/tesis` (nomor disamarkan), `/api/findings`, `/api/ditolak`, `/api/jejak`, `/api/kredit`. | 6 Okt | Dashboard milik Dian menampilkan data nyata dari endpoint ini. |
| R11 | Uji ujung ke ujung dengan data Sectors asli dan siapkan data demo (baseline, satu Finding yang melemahkan, satu Finding yang sengaja ditolak). Rekam video cadangan. | 6 Okt | Skenario demo bisa diulang tanpa error. |
| R12 | Opsional (F9): balasan "kenapa?" dari database, dengan pengecekan angka dan filter kata terlarang. | 6 Okt | Hanya jika R1-R11 sudah lolos. |
| R13 | Penyerahan: bagian WhatsApp di README, merekam bagian WhatsApp untuk video, memastikan token Meta dan semua rahasia aman, dan membuat token permanen lewat System User. | 7 Okt | Daftar periksa bagian 8 untuk bagian ini terpenuhi. |

## 6. Antarmuka dan serah terima antar anggota

Tabel ini menjelaskan siapa mengirim apa ke siapa. Semua data memakai JSON dengan kontrak Finding dan disepakati di T1.

| Dari | Ke | Apa | Cara |
|---|---|---|---|
| Rafiq | Semua | Hasil uji API: field, biaya kredit, usulan indikator | `docs/sectors-api-notes.md` |
| Dian | Rafi | Fixture tiga status dan aturan ambang | Berkas JSON di repo |
| Rafi | Rafiq dan Dian | Daftar endpoint internal dan contoh permintaan | `docs/api-internal.md` |
| Rafiq (Scout) | Rafi | Snapshot data, jejak, catatan kredit | `POST /snapshots`, `/jejak`, `/kredit` |
| Rafi | Rafiq (Scout) | Daftar ticker dan indikator yang dipantau | `GET /pantauan` |
| Dian (Analyst/Checker) | Rafi | Finding lolos atau ditolak + alasan | `POST /findings` |
| Rafi | Dian (dashboard) | Data tesis, temuan, ditolak, jejak, kredit | `GET /api/*` |

Catatan: daftar endpoint ini usulan agar tiga orang bisa bekerja paralel. Jika tim memilih cara lain, yang wajib dipertahankan hanyalah kontrak Finding dan satu pemilik database.

## 7. Matriks jadwal harian

| Tanggal | Rafiq: Data & MCP | Dian: Agent Analisis | Rafi: Orchestrator |
|---|---|---|---|
| Jum 2 Okt | D1 uji API, D2 akses key | A1 indikator (draf), kontrak T1 | R0 persiapan, mulai R1 |
| Sab 3 Okt | D3 klien, mulai D4 | A1-A3 final, mulai A4 | R1 selesai, R2, R3, mulai R4. Titik cek: pesan Rust terkirim |
| Min 4 Okt | D4, D5, D6, D7 | A4, A5, mulai A7 | R4, R5, R6 |
| Sen 5 Okt | D6, D8 | A5, A6, A7 | R6, R7, R8, R9. Titik cek: alur data tiruan |
| Sel 6 Okt | Bantu uji dengan data asli | A7, A8, A9 (opsional) | R9, R10, R11. Titik cek: data asli |
| Rab 7 Okt | D9, README, video | Evaluasi, video | R13, README, video, T7. Bekukan fitur |
| Kam 8 Okt | Submit lebih awal | Cadangan | Cadangan |

Jika waktu mepet, urutan prioritasnya: alur WhatsApp dan database dulu, lalu agent, dashboard di akhir. Tugas bertanda opsional (A9, R12) boleh dilewati tanpa mengurangi inti produk.

## 8. Daftar periksa sebelum submit (7-8 Oktober)

- [ ] Repo publik dan tidak diubah menjadi privat.
- [ ] Tidak ada API key, token, atau file .env di repo dan riwayat commit.
- [ ] `.env.example` ada dan berisi nama variabel tanpa nilai.
- [ ] README: masalah, cara kerja, diagram alur, cara menjalankan, pembagian tugas, dan disclaimer.
- [ ] Riwayat commit menunjukkan kontribusi ketiga anggota.
- [ ] Tidak ada kata beli, jual, tahan, atau target harga di pesan maupun dashboard.
- [ ] Nomor WhatsApp pengguna disamarkan di dashboard dan video.
- [ ] Alur utama bisa dijalankan dari nol mengikuti README.
- [ ] Video teaser 1 menit dan video juri maksimal 3 menit sudah publik.
- [ ] Postingan media sosial menandai akun resmi Sectors dengan template thumbnail.
- [ ] Pernyataan masalah satu kalimat, pilihan track, dan nama anggota siap diisi.
- [ ] Fitur dibekukan sebelum submit, dan tidak ada commit setelahnya.

## 9. Aturan kerja bersama

- Satu branch per anggota (mcp, agents, orchestrator), digabung lewat Pull Request yang ditinjau minimal satu anggota lain.
- Commit kecil dan sering dengan pesan jelas, misalnya "feat: kirim pesan WhatsApp via Cloud API".
- Jangan pernah commit .env, kunci, atau token. Jika terlanjur, ganti kunci di layanan asal dulu, baru bersihkan repo.
- Perubahan kontrak Finding atau endpoint wajib diumumkan ke seluruh tim sebelum dikerjakan.
- Perhitungan angka selalu di kode. LLM hanya untuk memahami kalimat tesis dan menulis ringkasan.
- Pertanyaan tentang apakah suatu fitur melanggar aturan diajukan ke kanal resmi penyelenggara sebelum dibuat.

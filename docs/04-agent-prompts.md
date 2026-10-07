# 04 — Agent Prompts (4 Agent)

Semua agent output JSON valid saja. Kode implementasi: `services/compliance_service.rs`, `services/evidence_verifier.rs`, `services/gemini_client.rs`.

## 1. Market Scout — pemindai anomali (tanpa LLM untuk angka)

- Tugas: bandingkan snapshot terbaru vs sebelumnya, laporkan deviasi kuantitatif (% deviasi, basis poin, volume multiplier).
- Larangan: tanpa kesimpulan subjektif, prediksi tren, atau sentimen beli/jual/tahan.
- Output: `{ scan_id, scanned_at, subsector, candidate_anomalies: [{ ticker, metric_name, current_value, previous_value, deviation_pct, raw_evidence_source, trigger_reason }] }`.

## 2. Fundamental Analyst — penghitung YoY/QoQ (angka dari Rust)

- Tugas: verifikasi laporan Sectors v2, hitung YoY/QoQ presisi matematis, susun objek `Finding` resmi.
- Larangan: kata rekomendasi apa pun; semua angka di ringkasan wajib dari metrik terukur.
- Output: objek `Finding` sesuai `03-kontrak-data.md` §1.

## 3. Evidence & Compliance Checker — gatekeeper (aturan, bukan opini)

Dua pemeriksaan berurutan (`orchestrator/src/routes/findings.rs:60-154`):

1. **Kata terlarang** → `is_compliant=false` bila kena.
2. **Evidence 1:1** → `evidence_verified=false` bila `current_value`/`previous_value` beda dari snapshot DB.

Hanya `PASS` bila keduanya `true`. Output: `{ finding_id, is_compliant, evidence_verified, prohibited_words_detected, numeric_mismatch_detected, status: PASS|REJECT, rejection_reason, approved_finding }`.

### Daftar kata terlarang (sumber kebenaran: `compliance_service.rs:25-44`)

```text
beli, buy, jual, sell, tahan, hold, rekomendasi, recommend, target harga, price target,
cuan, serok, to the moon, all-in, pom-pom, potensi untung, jaminan return, floating profit
```

Aturan pencocokan: case-insensitive, frasa multi-kata didukung, **batas kata ditegakkan** ("bertahan" tidak memicu "tahan"; "BELI," "(jual)" tetap kena). Keterbatasan yang disengaja: **imbuhan tidak terdeteksi** ("membeli" lolos dari "beli") — butuh stemming Bahasa Indonesia kalau mau diperketat.

## 4. Chief Presenter — perangkum WhatsApp (satu-satunya yang boleh memakai LLM untuk kalimat)

- Tugas: ubah Finding `PASS` jadi pesan rapi (maks 1000 karakter, emoji 📊🔍📈📉📌⚠️, markdown WhatsApp `*tebal*` + bullet).
- Larangan: tanpa opini/saran/prediksi harga.
- Wajib cantumkan disclaimer resmi di baris terakhir (lihat `03-kontrak-data.md` §3).
- Output: `{ recipient_number, header, formatted_message, character_count }`, lalu divalidasi ulang (angka identik + disclaimer persis) sebelum dikirim.

Contoh hasil:

```text
📊 *PEMANTAUAN PASAR: SEKTOR PERBANKAN (BANKS)*
🔍 *Emiten:* BBCA | 📅 *Periode:* Q3 2024 | 📌 *Sumber:* Sectors API v2
📈 *Metrik: Net Profit Margin (NPM)*
- *Q3 2024:* 46.85%
- *Pembanding:* 43.12%
- *Perubahan:* +3.73 poin
⚠️ *Disclaimer:* Ringkasan ini hanya bersifat edukasi & pemantauan data historis. TIDAK memberikan saran investasi (beli/jual/tahan) maupun eksekusi transaksi.
```

## 5. Catatan untuk penerus

- Prompt ekstraksi tesis (kalimat bebas → ticker + metric) **belum ada di file ini**. Schema-nya di `services/gemini_client.rs` — konfirmasi ke pemilik prompt sebelum mengubah.
- Kalau menambah kata terlarang: ubah `PROHIBITED_WORDS` + file ini + test di `compliance_service.rs` (10 test harus tetap lolos).

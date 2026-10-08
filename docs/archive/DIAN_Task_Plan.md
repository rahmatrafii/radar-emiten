# Dian — Rencana Tugas Agent Logic & Prompt Engineer

> **Pemilik:** Dian  
> **Tanggal:** 7 Oktober 2026  
> **Status:** D-01 Completed, D-02 Completed (7 Okt 2026), D-03 Completed (7 Okt 2026), D-04 Completed (7 Okt 2026)

---

## Ringkasan Tugas

| ID | Tugas | Status | Prioritas |
|----|-------|--------|-----------|
| D-01 | Master System Prompts (4 agent) | **Completed** | — |
| D-02 | Compliance Rule Engine | **Completed** (7 Okt 2026) | Tinggi |
| D-03 | Zero-Hallucination Verifier | **Completed** (7 Okt 2026) | Tinggi |
| D-04 | Few-Shot Test Cases | **Completed** (7 Okt 2026) | Sedang |

---

## D-02: Compliance Rule Engine

> ✅ **STATUS: SELESAI (7 Oktober 2026)** — `orchestrator/src/services/compliance_service.rs` + Gate 1.5 di `accept_finding()`. 10 unit test PASS, 67 test total PASS (29 lib + 38 integration), `cargo fmt` bersih.

### Tujuan
Mekanisme validasi kata terlarang (*anti-recommendation filter*) pada tahap verifikasi finding.

### Referensi
- `docs/PROMPTS.md` — Daftar kata terlarang (bagian 3)
- `docs/CONTRACTS.md` — Schema `ComplianceVerificationResult`
- `docs/IDX_Sentinel_Rafi_Agent_Execution_Runbook.md` — Bagian 13.2

### Daftar Kata Terlarang
```text
beli, buy, jual, sell, tahan, hold, rekomendasi, recommend, target harga, price target,
cuan, serok, to the moon, all-in, pom-pom, potensi untung, jaminan return, floating profit
```

### Langkah Implementasi

1. **Buat file `orchestrator/src/services/compliance_service.rs`**
   - Fungsi `check_compliance(text: &str) -> ComplianceCheckResult`
   - Pengecekan case-insensitive
   - Deteksi frasa multi-kata (contoh: "target harga")
   - Return struct:
     ```rust
     pub struct ComplianceCheckResult {
         pub is_compliant: bool,
         pub prohibited_words_detected: Vec<String>,
     }
     ```

2. **Daftarkan modul di `orchestrator/src/services/mod.rs`**
   ```rust
   pub mod compliance_service;
   ```

3. **Integrasikan ke `orchestrator/src/routes/findings.rs` (`accept_finding`)**
   - Gate 1.5 setelah validasi format, sebelum cross-check snapshot
   - Jika `is_compliant == false`, simpan finding sebagai `ditolak`, tulis audit log ke `compliance_audit_logs` (stage `finding`), return `rejected`
   - `agent_pipeline.rs` tidak diubah — pipeline sudah memanggil `accept_finding()`, jadi otomatis memakai gate yang sama (hindari duplikasi logic)

4. **Tambahkan unit test di `compliance_service.rs`**
   - Test kata terlarang single word
   - Test frasa multi-kata
   - Test case-insensitive
   - Test teks bersih (compliant)
   - Test batas kata (contoh: "bertahan" tidak memicu "tahan")
   - Test tanda baca, hyphenated phrase, multi-match, teks kosong

### Selesai Jika
- [x] Compliance check function mendeteksi semua kata terlarang dari daftar
- [x] Frasa multi-kata terdeteksi dengan benar
- [x] Case-insensitive check berfungsi
- [x] Batas kata enforced (tidak ada false positive potongan kata)
- [x] Unit test lolos untuk semua kasus (10 test PASS)
- [x] Terintegrasi dengan pipeline existing via `accept_finding()` (67 test total PASS)

---

## D-03: Zero-Hallucination Verifier

> ✅ **STATUS: SELESAI (7 Oktober 2026)** — `orchestrator/src/services/evidence_verifier.rs` (`verify_evidence` + `verify_finding` + `ComplianceVerificationResult`) + refactor Gate 2 di `accept_finding()`. 4 unit test PASS, 71 test total PASS (33 lib + 38 integration), `cargo fmt` bersih.

### Tujuan
Logika perbandingan 1:1 antara metrik angka yang disebutkan oleh model dengan data raw Sectors API v2.

### Referensi
- `docs/CONTRACTS.md` — Schema `ComplianceVerificationResult`
- `docs/IDX_Sentinel_Rafi_Agent_Execution_Runbook.md` — Bagian 13.1
- `orchestrator/src/repositories/compliance.rs` — fungsi `log()` sudah ada

### Schema ComplianceVerificationResult
```json
{
  "finding_id": "string",
  "is_compliant": boolean,
  "prohibited_words_detected": ["string"],
  "evidence_verified": boolean,
  "rejection_reason": "string or null"
}
```

### Langkah Implementasi

1. **Buat file `orchestrator/src/services/evidence_verifier.rs`**
   - Fungsi `verify_evidence(finding: &Finding, current_snapshot: &Snapshot, previous_snapshot: &Snapshot) -> VerificationResult`
   - Bandingkan `finding.current_value` dengan `current_snapshot.value`
   - Bandingkan `finding.previous_value` dengan `previous_snapshot.value`
   - Return struct:
     ```rust
     pub struct VerificationResult {
         pub evidence_verified: bool,
         pub rejection_reason: Option<String>,
     }
     ```

2. **Daftarkan modul di `orchestrator/src/services/mod.rs`**
   ```rust
   pub mod evidence_verifier;
   ```

3. **Refactor Gate 2 di `orchestrator/src/routes/findings.rs` (`accept_finding`)**
   - Ganti inline `value_exists`/`previous_exists` dengan `evidence_verifier::verify_evidence()`
   - Tidak ada perubahan perilaku — hanya ekstraksi ke service reusable + testable
   - `agent_pipeline.rs` tidak diubah — sudah memanggil `accept_finding()`

4. **Audit log tetap via `compliance_audit_logs`**
   - Gunakan fungsi `compliance::log()` yang sudah ada (tidak diubah)
   - Simpan `is_compliant`, `evidence_verified`, `prohibited_words_detected`, `rejection_reason`

5. **Tambahkan unit test di `evidence_verifier.rs`**
   - Test struct `VerificationResult` valid/invalid
   - Test serialisasi `ComplianceVerificationResult` sesuai kontrak (accept + reject)

### Selesai Jika
- [x] Evidence verifier membandingkan angka Finding dengan snapshot
- [x] `ComplianceVerificationResult` sesuai schema CONTRACTS.md + berseri dengan benar
- [x] Gate 2 di-refactor tanpa perubahan perilaku (test DB-backed existing tetap lolos)
- [x] Unit test lolos untuk semua kasus (4 test PASS, 71 test total PASS)

---

## D-04: Few-Shot Test Cases

> ✅ **STATUS: SELESAI (7 Oktober 2026)** — `orchestrator/tests/fixtures/agent_cases.json` (5 kasus) + `orchestrator/tests/agent_cases_test.rs` (4 validasi fixture + 1 end-to-end via POST /findings). 5 test PASS, 76 test total PASS (33 lib + 43 integration), `cargo fmt` bersih.

### Tujuan
Kumpulan dataset skenario pengujian untuk mengevaluasi akurasi reasoning model Gemini Flash.

### Referensi
- `docs/TASKS.md` — Deskripsi D-04
- `docs/PROMPTS.md` — Format output agent

### Langkah Implementasi

1. **Buat folder `tests/fixtures/` di root proyek**

2. **Buat file `tests/fixtures/agent_cases.json`**
   - Minimal 5 kasus uji
   - Setiap kasus berisi:
     ```json
     {
       "case_id": "case_001",
       "description": "Deskripsi skenario",
       "input": "Kalimat tesis pengguna",
       "expected_ticker": "BBCA",
       "expected_metrics": ["net_profit_margin"],
       "expected_compliance": true,
       "expected_evidence_verified": true
     }
     ```

3. **Definisikan kasus uji:**

   | Kasus | Sektor | Skenario | Expected |
   |-------|--------|----------|----------|
   | 001 | Perbankan (BBCA) | Laba naik, compliant | Accept |
   | 002 | Telekomunikasi (TLKM) | Laba turun, compliant | Accept |
   | 003 | Komoditas | Valuasi berubah, compliant | Accept |
   | 004 | Perbankan | Finding dengan kata "beli" | Reject (compliance) |
   | 005 | Perbankan | Angka tidak sesuai snapshot | Reject (evidence) |

4. **Buat test runner `tests/agent_cases_test.rs`**
   - Baca `agent_cases.json`
   - Untuk setiap kasus, jalankan pipeline
   - Assert expected result

### Selesai Jika
- [x] File fixture berisi minimal 5 kasus uji (`orchestrator/tests/fixtures/agent_cases.json`)
- [x] Kasus mencakup 3 sektor (banks, telecommunication, commodities) + skenario accept dan reject
- [x] Test runner lolos untuk semua kasus (`orchestrator/tests/agent_cases_test.rs`, 5 test PASS)

---

## Urutan Pekerjaan yang Disarankan

```
D-02 (Compliance Engine) → D-03 (Evidence Verifier) → D-04 (Test Cases)
```

**Alasan:** D-02 dan D-03 saling terkait (compliance check + evidence verification), dan D-04 membutuhkan keduanya untuk test case yang lengkap.

---

## Estimasi Waktu

| Tugas | Estimasi |
|-------|----------|
| D-02 | 2-3 jam |
| D-03 | 2-3 jam |
| D-04 | 1-2 jam |
| **Total** | **5-8 jam** |

---

## Catatan Penting

1. **Jangan mengubah kode milik Rafiq (MCP/Sectors) atau Rafi (Backend/WhatsApp)** tanpa koordinasi
2. **Semua perubahan kontrak harus diberitahukan ke seluruh tim**
3. **Jalankan `cargo test` setelah setiap perubahan** untuk memastikan tidak ada regression
4. **Dokumentasikan semua keputusan desain** di kode comment atau `docs/integration-decisions.md`

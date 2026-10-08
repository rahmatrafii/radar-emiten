//! Compliance Rule Engine (D-02) — milik Dian.
//!
//! Memindai teks dinamis (`finding_summary`, header/body WhatsApp, ringkasan LLM)
//! terhadap daftar kata terlarang (blacklist) dari `docs/PROMPTS.md` bagian 3.
//!
//! Aturan:
//! - Pencocokan case-insensitive.
//! - Frasa multi-kata didukung (contoh: "target harga", "to the moon").
//! - Batas kata (word boundary) enforced agar tidak salah mendeteksi potongan
//!   kata yang tidak relevan (contoh: "bertahan" TIDAK memicu "tahan").
//! - Bentuk berimbuhan (contoh: "membeli" untuk "beli") TIDAK terdeteksi oleh
//!   pencocokan batas kata ini; itu keterbatasan yang didokumentasikan.
//!   Untuk cakupan turunan kata penuh diperlukan stemming Bahasa Indonesia
//!   (di luar scope D-02).

/// Hasil pemeriksaan compliance.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComplianceCheckResult {
    pub is_compliant: bool,
    pub prohibited_words_detected: Vec<String>,
}

/// Daftar kata/frasa terlarang — sumber kebenaran `docs/PROMPTS.md` bagian 3.
/// Semua entri dalam huruf kecil; pencocokan dilakukan case-insensitive.
pub const PROHIBITED_WORDS: &[&str] = &[
    "beli",
    "buy",
    "jual",
    "sell",
    "tahan",
    "hold",
    "rekomendasi",
    "recommend",
    "target harga",
    "price target",
    "cuan",
    "serok",
    "to the moon",
    "all-in",
    "pom-pom",
    "potensi untung",
    "jaminan return",
    "floating profit",
];

/// Periksa satu teks terhadap daftar kata terlarang.
///
/// Mengembalikan `is_compliant=false` + daftar kata yang terdeteksi bila ada
/// pelanggaran, atau `is_compliant=true` + vec kosong bila bersih.
pub fn check_compliance(text: &str) -> ComplianceCheckResult {
    let lower = text.to_lowercase();
    let mut detected = Vec::new();

    for phrase in PROHIBITED_WORDS {
        if contains_phrase_with_boundary(&lower, phrase) {
            detected.push(phrase.to_string());
        }
    }

    ComplianceCheckResult {
        is_compliant: detected.is_empty(),
        prohibited_words_detected: detected,
    }
}

/// Cek apakah `phrase` (sudah lowercase) muncul di `text` (sudah lowercase)
/// dengan batas kata di kedua sisi.
///
/// Batas kata = awal/akhir string, atau karakter non-alfanumerik.
/// Ini mencegah "tahan" cocok di dalam "bertahan", tapi tetap mendeteksi
/// "beli," "BELI", "(jual)", "target harga." dll.
fn contains_phrase_with_boundary(text: &str, phrase: &str) -> bool {
    if phrase.is_empty() {
        return false;
    }

    let text_bytes = text.as_bytes();
    let phrase_bytes = phrase.as_bytes();

    if phrase_bytes.len() > text_bytes.len() {
        return false;
    }

    let mut start = 0;
    while start + phrase_bytes.len() <= text_bytes.len() {
        // Cari kemunculan substring berikutnya.
        let Some(pos) = find_subslice(&text[start..], phrase).map(|p| start + p) else {
            break;
        };

        let before_ok = pos == 0 || !is_word_char(text_bytes[pos - 1] as char);
        let after_pos = pos + phrase_bytes.len();
        let after_ok =
            after_pos >= text_bytes.len() || !is_word_char(text_bytes[after_pos] as char);

        if before_ok && after_ok {
            return true;
        }

        // Lanjut cari setelah posisi ini (overlap-safe: maju 1 char).
        // Hindari infinite loop untuk phrase 1 char dengan maju berdasarkan char boundary.
        start = pos + 1;
        // Pastikan start pada batas char UTF-8.
        while start < text.len() && !text.is_char_boundary(start) {
            start += 1;
        }
    }

    false
}

/// Pencarian subslice byte sederhana (tanpa dependensi `regex`).
fn find_subslice(haystack: &str, needle: &str) -> Option<usize> {
    haystack.find(needle)
}

/// Karakter kata = alfanumerik ASCII atau underscore.
/// Hyphen, spasi, dan tanda baca dianggap pemisah kata.
fn is_word_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn teks_bersih_lolos() {
        let r = check_compliance("Net profit margin naik 5% pada Q1 2026.");
        assert!(r.is_compliant);
        assert!(r.prohibited_words_detected.is_empty());
    }

    #[test]
    fn kata_tunggal_terdeteksi() {
        let r = check_compliance("Saya sarankan beli saham ini");
        assert!(!r.is_compliant);
        assert!(r.prohibited_words_detected.contains(&"beli".to_string()));
    }

    #[test]
    fn frasa_multi_kata_terdeteksi() {
        let r = check_compliance("Target harga 5000 untuk BBCA");
        assert!(!r.is_compliant);
        assert!(
            r.prohibited_words_detected
                .contains(&"target harga".to_string())
        );
    }

    #[test]
    fn case_insensitive() {
        let r = check_compliance("BELI saham BBCA sekarang");
        assert!(!r.is_compliant);
        assert!(r.prohibited_words_detected.contains(&"beli".to_string()));
    }

    #[test]
    fn tanda_baca_tidak_menghalangi_deteksi() {
        let r = check_compliance("Rekomendasi: saham ini bagus.");
        assert!(!r.is_compliant);
        assert!(
            r.prohibited_words_detected
                .contains(&"rekomendasi".to_string())
        );
    }

    #[test]
    fn potongan_kata_tidak_salah_deteksi() {
        // "bertahan" mengandung "tahan" tapi sebagai bagian kata — harus lolos.
        let r = check_compliance("Kinerja bertahan stabil pada kuartal ini.");
        assert!(
            r.is_compliant,
            "harus lolos, dapat: {:?}",
            r.prohibited_words_detected
        );
    }

    #[test]
    fn frasa_inggris_multi_kata_terdeteksi() {
        let r = check_compliance("This stock will go to the moon soon");
        assert!(!r.is_compliant);
        assert!(
            r.prohibited_words_detected
                .contains(&"to the moon".to_string())
        );
    }

    #[test]
    fn beberapa_kata_terlarang_sekaligus() {
        let r = check_compliance("Beli sekarang, dijamin cuan besar");
        assert!(!r.is_compliant);
        assert!(r.prohibited_words_detected.contains(&"beli".to_string()));
        assert!(r.prohibited_words_detected.contains(&"cuan".to_string()));
    }

    #[test]
    fn hyphenated_phrase_terdeteksi() {
        let r = check_compliance("Strategi all-in sangat berisiko");
        assert!(!r.is_compliant);
        assert!(r.prohibited_words_detected.contains(&"all-in".to_string()));
    }

    #[test]
    fn teks_kosong_lolos() {
        let r = check_compliance("");
        assert!(r.is_compliant);
    }
}

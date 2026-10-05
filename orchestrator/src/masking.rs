/// Samarkan nomor WhatsApp sebelum keluar dari backend (log, API, demo).
/// Format kanonik nomor adalah digit tanpa '+' (mis. 6281234567890).
pub fn mask_wa(nomor: &str) -> String {
    let digits: String = nomor.chars().filter(|c| c.is_ascii_digit()).collect();
    if digits.len() < 8 {
        return "••••".to_string();
    }
    // Tampilkan awalan kode negara + sebagian, tutup tengah, sisakan 4 digit akhir.
    let prefix_len = if digits.starts_with("62") && digits.len() >= 10 {
        5.min(digits.len() - 4)
    } else {
        3
    };
    let head: &str = &digits[..prefix_len];
    let tail: &str = &digits[digits.len() - 4..];
    format!("+{head}-••••-{tail}")
}

#[cfg(test)]
mod tests {
    use super::mask_wa;

    #[test]
    fn mask_format_indonesia() {
        let masked = mask_wa("6281234567890");
        assert!(masked.contains("••••"));
        assert!(masked.ends_with("7890"));
        assert!(!masked.contains("8123456"));
    }

    #[test]
    fn mask_nomor_dengan_plus() {
        let masked = mask_wa("+628111222333");
        assert!(masked.contains("••••"));
        assert!(masked.ends_with("2333"));
    }

    #[test]
    fn mask_nomor_pendek() {
        assert_eq!(mask_wa("123"), "••••");
    }
}

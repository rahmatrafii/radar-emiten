// src/sectors/client.rs — tests module

#[cfg(test)]
mod tests {
    use super::*;
    use crate::findings::TingkatKeyakinan;
    use crate::sectors::models::screener::ScreenerRow;
    use std::collections::HashMap;

    // -----------------------------------------------------------------------
    // Helper: buat ScreenerRow palsu untuk test
    // -----------------------------------------------------------------------
    fn mock_screener_row(ticker: &str, fields: HashMap<String, serde_json::Value>) -> ScreenerRow {
        ScreenerRow {
            symbol: ticker.to_string(),
            company_name: Some(format!("{ticker} Corp")),
            sector: Some("Finance".to_string()),
            sub_sector: Some("Banking".to_string()),
            fields,
        }
    }

    #[test]
    fn test_finding_delta_pct_positif() {
        let finding = Finding::new(
            "BBCA",
            "net_interest_margin_q",
            5.0,
            6.0,
            "2024-Q3",
            "sectors_api_v2/screener",
            TingkatKeyakinan::Tinggi,
        );
        let delta = finding.delta_pct();
        assert!((delta - 20.0).abs() < 1e-9, "Delta harus 20%, dapat: {delta}");
        assert!(finding.is_improving());
    }

    #[test]
    fn test_finding_delta_pct_negatif() {
        let finding = Finding::new(
            "BBCA",
            "net_interest_margin_q",
            6.0,
            5.0,
            "2024-Q3",
            "sectors_api_v2/screener",
            TingkatKeyakinan::Tinggi,
        );
        let delta = finding.delta_pct();
        assert!(delta < 0.0, "Delta harus negatif");
        assert!(!finding.is_improving());
    }

    #[test]
    fn test_finding_delta_pct_zero_base() {
        let finding = Finding::new(
            "BBCA",
            "nim_q",
            0.0, // nilai sebelum = 0
            5.0,
            "2024-Q3",
            "sectors_api_v2/screener",
            TingkatKeyakinan::Sedang,
        );
        // Tidak boleh panic (division by zero)
        let delta = finding.delta_pct();
        assert_eq!(delta, 0.0, "Delta harus 0 jika base = 0");
    }

    #[test]
    fn test_screener_row_get_f64_number() {
        let mut fields = HashMap::new();
        fields.insert(
            "net_interest_margin_q".to_string(),
            serde_json::Value::Number(serde_json::Number::from_f64(5.75).unwrap()),
        );
        let row = mock_screener_row("BBCA", fields);
        assert_eq!(row.get_f64("net_interest_margin_q"), Some(5.75));
        assert_eq!(row.get_f64("tidak_ada"), None);
    }

    #[test]
    fn test_screener_row_get_f64_string_coerce() {
        let mut fields = HashMap::new();
        fields.insert(
            "eps_q".to_string(),
            serde_json::Value::String("123.45".to_string()),
        );
        let row = mock_screener_row("BBRI", fields);
        assert_eq!(row.get_f64("eps_q"), Some(123.45));
    }

    #[test]
    fn test_finding_serialization_roundtrip() {
        let finding = Finding::new(
            "BMRI",
            "pertumbuhan_laba_q",
            10.5,
            12.3,
            "2024-Q3",
            "sectors_api_v2/screener",
            TingkatKeyakinan::Tinggi,
        );

        let json = serde_json::to_string(&finding).expect("Serialisasi gagal");
        let deserialized: Finding = serde_json::from_str(&json).expect("Deserialisasi gagal");

        assert_eq!(finding.ticker, deserialized.ticker);
        assert_eq!(finding.indikator, deserialized.indikator);
        assert!((finding.nilai_sekarang - deserialized.nilai_sekarang).abs() < 1e-9);
        assert_eq!(finding.tingkat_keyakinan, deserialized.tingkat_keyakinan);
    }

    #[test]
    fn test_credit_tracker_charge_and_remaining() {
        use crate::sectors::credits::CreditTracker;
        let tracker = CreditTracker::new(100);
        assert_eq!(tracker.used(), 0);
        assert_eq!(tracker.remaining(), 100);

        tracker.charge(30);
        assert_eq!(tracker.used(), 30);
        assert_eq!(tracker.remaining(), 70);

        assert!(tracker.can_afford(70));
        assert!(!tracker.can_afford(71));

        tracker.reset();
        assert_eq!(tracker.used(), 0);
    }

    #[test]
    fn test_cache_set_get_and_expiry() {
        use crate::sectors::cache::MemCache;
        use chrono::Duration;

        let cache: MemCache<String> = MemCache::new(Duration::hours(24));

        // Test SET dan GET
        cache.set("BBCA", "data_bbca".to_string());
        assert_eq!(cache.get("BBCA"), Some("data_bbca".to_string()));

        // Key berbeda harus miss
        assert_eq!(cache.get("BBRI"), None);

        // TTL 0 = langsung expired
        let short_cache: MemCache<String> = MemCache::new(Duration::zero());
        short_cache.set("BBCA", "data".to_string());
        assert_eq!(short_cache.get("BBCA"), None, "Harus expired dengan TTL=0");
    }

    #[test]
    fn test_missing_api_key_error() {
        // Pastikan variabel env tidak ada untuk test ini
        std::env::remove_var("SECTORS_API_KEY");
        let result = SectorsClient::new(String::new(), SectorsConfig::default());
        // Empty string key harus error
        // (from_env akan error, new() menerima string — test MissingApiKey via from_env)
        let result_env = {
            std::env::remove_var("SECTORS_API_KEY");
            SectorsClient::from_env()
        };
        assert!(
            matches!(result_env, Err(SectorsError::MissingApiKey)),
            "Harus error MissingApiKey"
        );
    }
}

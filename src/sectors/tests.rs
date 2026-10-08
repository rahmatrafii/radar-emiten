// src/sectors/tests.rs — tests module (offline; no network calls)

#[cfg(test)]
mod tests {
    use crate::findings::{ConfidenceLevel, Finding};
    use crate::sectors::client::{normalize_ticker, SectorsClient, SectorsConfig};
    use crate::sectors::error::SectorsError;
    use crate::sectors::models::quarterly::QuarterlyFinancials;
    use crate::sectors::models::screener::{ScreenerResponse, SubsectorItem};
    use std::collections::HashMap;

    fn test_client() -> SectorsClient {
        // Dummy key; constructor does no network I/O.
        SectorsClient::new("dummy-key".to_string(), SectorsConfig::default()).unwrap()
    }

    fn financials(
        symbol: &str,
        date: &str,
        revenue: f64,
        earnings: f64,
        net_interest_income: Option<f64>,
    ) -> QuarterlyFinancials {
        let mut extra = HashMap::new();
        extra.insert(
            "revenue".to_string(),
            serde_json::json!(revenue),
        );
        extra.insert(
            "earnings".to_string(),
            serde_json::json!(earnings),
        );
        if let Some(nim) = net_interest_income {
            extra.insert(
                "financials_sector_metrics".to_string(),
                serde_json::json!({ "net_interest_income": nim }),
            );
        }
        QuarterlyFinancials {
            symbol: symbol.to_string(),
            date: Some(date.to_string()),
            extra,
        }
    }

    // -----------------------------------------------------------------------
    // Ticker normalization (API uses BBCA.JK; internal contract needs BBCA)
    // -----------------------------------------------------------------------

    #[test]
    fn test_normalize_ticker_strips_jk_suffix() {
        assert_eq!(normalize_ticker("BBCA.JK"), "BBCA");
        assert_eq!(normalize_ticker("bbca.jk"), "BBCA");
        assert_eq!(normalize_ticker("BBCA"), "BBCA");
        assert_eq!(normalize_ticker(" tlkm.jk "), "TLKM");
    }

    // -----------------------------------------------------------------------
    // Deserialization matches live API shapes (verified 7 Oct 2026)
    // -----------------------------------------------------------------------

    #[test]
    fn test_screener_response_uses_results_key() {
        let raw = r#"{"results":[{"symbol":"BBCA.JK","company_name":"PT Bank Central Asia Tbk."}],"pagination":{"total_count":1}}"#;
        let resp: ScreenerResponse = serde_json::from_str(raw).unwrap();
        assert_eq!(resp.results.len(), 1);
        assert_eq!(resp.results[0].symbol, "BBCA.JK");
        assert_eq!(resp.results[0].ticker_short(), "BBCA");
    }

    #[test]
    fn test_subsector_item_deserialization() {
        let raw = r#"[{"sector":"financials","subsector":"banks"}]"#;
        let items: Vec<SubsectorItem> = serde_json::from_str(raw).unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].subsector, "banks");
    }

    #[test]
    fn test_quarterly_financials_metric_flat_and_nested() {
        let rec = financials("BBCA.JK", "2026-06-30", 28.0, 14.0, Some(21.0));
        assert_eq!(rec.metric("revenue"), Some(28.0));
        assert_eq!(rec.metric("earnings"), Some(14.0));
        // Nested under financials_sector_metrics (bank-specific).
        assert_eq!(rec.metric("net_interest_income"), Some(21.0));
        assert_eq!(rec.metric("nonexistent"), None);
        assert_eq!(rec.ticker_short(), "BBCA");
    }

    #[test]
    fn test_quarterly_financials_extract_derived_metrics() {
        let rec = financials("BBCA.JK", "2026-06-30", 100.0, 25.0, None);
        assert_eq!(rec.extract_metric("net_profit_margin"), Some(25.0));
        assert_eq!(rec.extract_metric("net_income"), Some(25.0));
        assert_eq!(rec.extract_metric("unknown_metric"), None);
    }

    // -----------------------------------------------------------------------
    // financials_to_findings: latest vs previous quarter
    // -----------------------------------------------------------------------

    #[test]
    fn test_financials_to_findings_positive_delta() {
        let client = test_client();
        let records = vec![
            financials("BBCA.JK", "2026-06-30", 28.0, 14.0, None),
            financials("BBCA.JK", "2026-03-31", 25.0, 12.0, None),
        ];
        let findings =
            client.financials_to_findings(&records, &["revenue", "earnings"], "test-source");
        assert_eq!(findings.len(), 2);

        let rev = findings.iter().find(|f| f.metric_name == "revenue").unwrap();
        assert_eq!(rev.ticker, "BBCA");
        assert_eq!(rev.period, "2026-06-30");
        assert!((rev.delta_pct() - 12.0).abs() < 1e-9);
        assert!(rev.is_improving());
        assert_eq!(rev.confidence_level, ConfidenceLevel::High);
    }

    #[test]
    fn test_financials_to_findings_skips_missing_fields() {
        let client = test_client();
        let records = vec![
            financials("BBCA.JK", "2026-06-30", 28.0, 14.0, None),
            financials("BBCA.JK", "2026-03-31", 25.0, 12.0, None),
        ];
        let findings = client.financials_to_findings(
            &records,
            &["revenue", "field_tidak_ada"],
            "test-source",
        );
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].metric_name, "revenue");
    }

    #[test]
    fn test_financials_to_findings_needs_two_records() {
        let client = test_client();
        let records = vec![financials("BBCA.JK", "2026-06-30", 28.0, 14.0, None)];
        let findings = client.financials_to_findings(&records, &["revenue"], "test-source");
        assert!(findings.is_empty());
    }

    // -----------------------------------------------------------------------
    // Finding contract (English fields)
    // -----------------------------------------------------------------------

    #[test]
    fn test_finding_delta_pct_positive() {
        let finding = Finding::new(
            "BBCA",
            "revenue",
            5.0,
            6.0,
            "2026-06-30",
            "sectors_api_v2/financials",
            ConfidenceLevel::High,
        );
        let delta = finding.delta_pct();
        assert!((delta - 20.0).abs() < 1e-9, "Delta should be 20%, got: {delta}");
        assert!(finding.is_improving());
    }

    #[test]
    fn test_finding_delta_pct_zero_base() {
        let finding = Finding::new(
            "BBCA",
            "revenue",
            0.0,
            5.0,
            "2026-06-30",
            "sectors_api_v2/financials",
            ConfidenceLevel::Medium,
        );
        assert_eq!(finding.delta_pct(), 0.0);
    }

    #[test]
    fn test_finding_serialization_roundtrip() {
        let finding = Finding::new(
            "BMRI",
            "earnings",
            10.5,
            12.3,
            "2026-06-30",
            "sectors_api_v2/financials",
            ConfidenceLevel::High,
        );

        let json = serde_json::to_string(&finding).expect("Serialization failed");
        let deserialized: Finding = serde_json::from_str(&json).expect("Deserialization failed");

        assert_eq!(finding.ticker, deserialized.ticker);
        assert_eq!(finding.metric_name, deserialized.metric_name);
        assert!((finding.current_value - deserialized.current_value).abs() < 1e-9);
        assert_eq!(finding.confidence_level, deserialized.confidence_level);
    }

    // -----------------------------------------------------------------------
    // Credits, cache, config (unchanged behavior)
    // -----------------------------------------------------------------------

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

        cache.set("BBCA", "data_bbca".to_string());
        assert_eq!(cache.get("BBCA"), Some("data_bbca".to_string()));
        assert_eq!(cache.get("BBRI"), None);

        let short_cache: MemCache<String> = MemCache::new(Duration::zero());
        short_cache.set("BBCA", "data".to_string());
        assert_eq!(short_cache.get("BBCA"), None, "Should be expired with TTL=0");
    }

    #[test]
    fn test_missing_api_key_error() {
        std::env::set_var("SECTORS_API_KEY", "");
        let result_env = SectorsClient::from_env();
        assert!(
            matches!(result_env, Err(SectorsError::MissingApiKey)),
            "Should error MissingApiKey"
        );
    }
}

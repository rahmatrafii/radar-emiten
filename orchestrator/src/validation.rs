use crate::metrics;
use crate::models::finding::Finding;

/// Normalisasi angka dari JSON: number finite atau string yang bisa diparse.
pub fn to_f64(v: &serde_json::Value) -> Option<f64> {
    match v {
        serde_json::Value::Number(n) => n.as_f64().filter(|x| x.is_finite()),
        serde_json::Value::String(s) => s.trim().parse::<f64>().ok().filter(|x| x.is_finite()),
        _ => None,
    }
}

/// Validasi minimum Finding sesuai runbook Bagian 13.1 (deterministik, tanpa DB).
pub fn validate_finding(f: &Finding) -> Result<(), String> {
    if f.ticker.len() != 4
        || !f
            .ticker
            .chars()
            .all(|c| c.is_ascii_uppercase() && c.is_ascii_alphabetic())
    {
        return Err("ticker harus 4 huruf kapital (mis. BBCA)".into());
    }
    if f.subsector.trim().is_empty()
        || f.metric_name.trim().is_empty()
        || f.period.trim().is_empty()
        || f.source.trim().is_empty()
    {
        return Err("subsector/metric_name/period/source tidak boleh kosong".into());
    }
    if !metrics::is_allowed(&f.metric_name) {
        return Err(format!(
            "metric_name '{}' tidak ada di registry",
            f.metric_name
        ));
    }
    if !(0.0..=1.0).contains(&f.confidence_score) || !f.confidence_score.is_finite() {
        return Err("confidence_score harus pada rentang 0.0..=1.0".into());
    }
    if chrono::DateTime::parse_from_rfc3339(&f.observed_at).is_err() {
        return Err("observed_at harus timestamp RFC3339 yang valid".into());
    }
    if to_f64(&f.current_value).is_none() || to_f64(&f.previous_value).is_none() {
        return Err("current_value/previous_value harus numerik yang valid".into());
    }
    if f.finding_summary.trim().is_empty() || f.finding_summary.len() > 500 {
        return Err("finding_summary wajib diisi, maksimal 500 karakter".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid() -> Finding {
        Finding {
            ticker: "BBCA".into(),
            subsector: "banks".into(),
            metric_name: "net_profit_margin".into(),
            current_value: serde_json::json!(46.85),
            previous_value: serde_json::json!(43.12),
            period: "Q3 2024".into(),
            source: "Sectors API v2".into(),
            observed_at: "2026-10-02T10:15:30Z".into(),
            confidence_score: 0.9,
            finding_summary: "Ringkasan faktual.".into(),
        }
    }

    #[test]
    fn finding_valid_diterima() {
        assert!(validate_finding(&valid()).is_ok());
    }

    #[test]
    fn ticker_buruk_ditolak() {
        let mut f = valid();
        f.ticker = "bbca".into();
        assert!(validate_finding(&f).is_err());
    }

    #[test]
    fn confidence_di_luar_rentang_ditolak() {
        let mut f = valid();
        f.confidence_score = 1.5;
        assert!(validate_finding(&f).is_err());
    }

    #[test]
    fn nilai_null_ditolak_sebagai_alert() {
        let mut f = valid();
        f.current_value = serde_json::Value::Null;
        assert!(validate_finding(&f).is_err());
    }

    #[test]
    fn timestamp_buruk_ditolak() {
        let mut f = valid();
        f.observed_at = "kemarin".into();
        assert!(validate_finding(&f).is_err());
    }

    #[test]
    fn metric_asing_ditolak() {
        let mut f = valid();
        f.metric_name = "lunar_magic".into();
        assert!(validate_finding(&f).is_err());
    }
}

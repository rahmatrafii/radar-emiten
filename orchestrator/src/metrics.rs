/// Registry metric yang diizinkan. Daftar final milik tim belum ada;
/// override lewat env `METRIC_REGISTRY` (koma-separated), default untuk dev/test.
pub fn metric_registry() -> Vec<String> {
    if let Ok(raw) = std::env::var("METRIC_REGISTRY") {
        let list: Vec<String> = raw
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        if !list.is_empty() {
            return list;
        }
    }
    [
        "net_profit_margin",
        "operating_margin",
        "gross_margin",
        "roe",
        "roa",
        "eps",
        "revenue",
        "net_income",
        "der",
        "current_ratio",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect()
}

pub fn is_allowed(metric_name: &str) -> bool {
    metric_registry().iter().any(|m| m == metric_name)
}

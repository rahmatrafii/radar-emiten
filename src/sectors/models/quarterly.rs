// src/sectors/models/quarterly.rs
// Model respons untuk endpoint kuartalan Sectors API v2.
//
// Referensi:
// - Universe dates: https://docs.sectors.app/api-references/v2/indonesia/helper-list/latest-quarterly-dates
//   `GET /v2/companies/quarterly-financial-dates/?since=YYYY-MM-DD&limit=N`
//   → `{"results": [{"symbol", "date", "quarter"}], "pagination": {...}}`
//   Biaya: 1 kredit per halaman.
// - Per-symbol financials: https://docs.sectors.app/api-references/v2/indonesia/report/quarterly-financials
//   `GET /v2/financials/quarterly/{symbol}/?n_quarters=N`
//   → JSON array langsung `[{symbol, date, <metric...>, financials_sector_metrics?}]`
//   Biaya: 1 kredit per quarter yang dikembalikan.
//   Simbol menerima `BBCA` maupun `BBCA.JK` (case-insensitive); respons memakai `.JK`.
// (Terverifikasi live 7 Okt 2026.)

/// Satu baris feed tanggal laporan kuartalan universe.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct QuarterlyDateRow {
    /// Kode saham, contoh: `"BBCA.JK"`.
    pub symbol: String,

    /// Tanggal laporan kuartalan, format `"YYYY-MM-DD"`.
    pub date: String,

    /// Label kuartal: `"q1"`..`"q4"`.
    pub quarter: String,
}

/// Respons feed tanggal kuartalan universe.
#[derive(Debug, serde::Deserialize)]
pub struct QuarterlyDatesResponse {
    #[serde(default)]
    pub results: Vec<QuarterlyDateRow>,

    #[serde(default)]
    pub pagination: Option<serde_json::Value>,
}

/// Satu record keuangan kuartalan untuk satu emiten.
///
/// Field metrik bervariasi per sektor (bank/asuransi punya tambahan di
/// `financials_sector_metrics` seperti `net_interest_income`, `gross_loan`,
/// `total_deposit`). Semua metrik diakses via [`QuarterlyFinancials::metric`].
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct QuarterlyFinancials {
    /// Kode saham, contoh: `"BBCA.JK"`.
    #[serde(default)]
    pub symbol: String,

    /// Tanggal laporan kuartalan, format `"YYYY-MM-DD"`.
    #[serde(default)]
    pub date: Option<String>,

    /// Semua metrik keuangan (flat + objek `financials_sector_metrics`).
    #[serde(flatten)]
    pub extra: std::collections::HashMap<String, serde_json::Value>,
}

impl QuarterlyFinancials {
    /// Mengambil nilai metrik sebagai `f64`.
    ///
    /// Mencari di field flat terlebih dahulu, lalu di dalam objek
    /// `financials_sector_metrics` (untuk emiten sektor finansial).
    /// Mengembalikan `None` jika tidak ada atau bukan angka finite.
    pub fn metric(&self, name: &str) -> Option<f64> {
        if let Some(v) = to_f64_opt(self.extra.get(name)) {
            return Some(v);
        }
        if let Some(nested) = self.extra.get("financials_sector_metrics") {
            if let Some(obj) = nested.as_object() {
                if let Some(v) = to_f64_opt(obj.get(name)) {
                    return Some(v);
                }
            }
        }
        None
    }

    /// Mengambil metrik dengan fallback kalkulasi turunan (derived metrics).
    /// Mendukung net_profit_margin (dari earnings/revenue), operating_margin,
    /// gross_margin, dan alias net_income (dari earnings).
    pub fn extract_metric(&self, name: &str) -> Option<f64> {
        if let Some(v) = self.metric(name) {
            return Some(v);
        }
        match name {
            "net_profit_margin" => {
                let earnings = self.metric("earnings").or_else(|| self.metric("net_income"))?;
                let revenue = self.metric("revenue")?;
                if revenue != 0.0 {
                    Some((earnings / revenue) * 100.0)
                } else {
                    None
                }
            }
            "net_income" => self.metric("earnings"),
            "operating_margin" => {
                let op = self.metric("operating_profit").or_else(|| self.metric("operating_income"))?;
                let revenue = self.metric("revenue")?;
                if revenue != 0.0 {
                    Some((op / revenue) * 100.0)
                } else {
                    None
                }
            }
            "gross_margin" => {
                let gp = self.metric("gross_profit")?;
                let revenue = self.metric("revenue")?;
                if revenue != 0.0 {
                    Some((gp / revenue) * 100.0)
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    /// Ticker 4 huruf untuk kontrak internal (orchestrator validasi `^[A-Z]{4}$`).
    /// `"BBCA.JK"` → `"BBCA"`.
    pub fn ticker_short(&self) -> String {
        crate::sectors::client::normalize_ticker(&self.symbol)
    }
}

fn to_f64_opt(v: Option<&serde_json::Value>) -> Option<f64> {
    match v? {
        serde_json::Value::Number(n) => n.as_f64().filter(|x| x.is_finite()),
        serde_json::Value::String(s) => s.trim().parse::<f64>().ok().filter(|x| x.is_finite()),
        _ => None,
    }
}

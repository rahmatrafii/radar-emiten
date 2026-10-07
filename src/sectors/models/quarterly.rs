// src/sectors/models/quarterly.rs
// Model respons untuk endpoint /v2/companies/reports/quarterly

use serde::{Deserialize, Serialize};

/// Satu entri laporan kuartalan dari endpoint
/// `GET /v2/companies/reports/quarterly?since=YYYY-MM-DD`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuarterlyReport {
    /// Kode saham BEI, contoh: `"BBCA"`.
    pub symbol: String,

    /// Nama perusahaan.
    pub company_name: Option<String>,

    /// Periode pelaporan, contoh: `"2024-Q3"`.
    pub period: String,

    /// Tanggal pelaporan ke bursa (format ISO 8601).
    pub report_date: Option<String>,

    /// Pendapatan bersih (dalam jutaan rupiah).
    pub net_income: Option<f64>,

    /// Pendapatan usaha.
    pub revenue: Option<f64>,

    /// Earnings per share.
    pub eps: Option<f64>,

    /// Field tambahan yang mungkin ada di respons API, disimpan agar tidak hilang.
    #[serde(flatten)]
    pub extra: std::collections::HashMap<String, serde_json::Value>,
}

/// Respons penuh dari endpoint quarterly reports.
/// Sectors API v2 membungkus data dalam field `"data"`.
#[derive(Debug, Deserialize)]
pub struct QuarterlyReportResponse {
    pub data: Vec<QuarterlyReport>,

    #[serde(default)]
    pub total: Option<u64>,

    #[serde(default)]
    pub page: Option<u32>,
}

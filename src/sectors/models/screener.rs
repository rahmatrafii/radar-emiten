// src/sectors/models/screener.rs
// Model respons untuk endpoint Companies Screener v2

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Satu baris hasil dari Companies Screener v2.
/// Field kuartalan bersifat dinamis sehingga disimpan dalam HashMap.
///
/// Contoh field kuartalan:
/// - `"pertumbuhan_laba_q"` → pertumbuhan laba kuartal terakhir (%)
/// - `"net_interest_margin_q"` → NIM kuartal terakhir (%)
/// - `"pertumbuhan_pendapatan_q"` → pertumbuhan pendapatan q-o-q
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScreenerRow {
    /// Kode saham BEI.
    pub symbol: String,

    /// Nama perusahaan.
    pub company_name: Option<String>,

    /// Sektor IDX.
    pub sector: Option<String>,

    /// Sub-sektor IDX.
    pub sub_sector: Option<String>,

    /// Semua field keuangan lainnya (termasuk field kuartalan yang diminta).
    /// Key = nama field, Value = nilai numerik atau null.
    #[serde(flatten)]
    pub fields: HashMap<String, serde_json::Value>,
}

impl ScreenerRow {
    /// Mengambil nilai field keuangan sebagai `f64`.
    /// Mengembalikan `None` jika field tidak ada atau bukan angka.
    pub fn get_f64(&self, field: &str) -> Option<f64> {
        match self.fields.get(field)? {
            serde_json::Value::Number(n) => n.as_f64(),
            serde_json::Value::String(s) => s.parse().ok(),
            _ => None,
        }
    }
}

/// Payload query untuk Companies Screener v2.
/// Dikirim sebagai query parameter ke API.
#[derive(Debug, Serialize)]
pub struct ScreenerQuery<'a> {
    /// Filter kondisi, contoh: `"market_cap > 1000000"`.
    #[serde(rename = "where", skip_serializing_if = "Option::is_none")]
    pub where_clause: Option<&'a str>,

    /// Field yang ingin ditampilkan, dipisahkan koma.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fields: Option<String>,

    /// Jumlah hasil per halaman (default API: 20).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,

    /// Urutan hasil, contoh: `"market_cap desc"`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sort: Option<&'a str>,
}

/// Respons penuh dari Companies Screener v2.
#[derive(Debug, Deserialize)]
pub struct ScreenerResponse {
    pub data: Vec<ScreenerRow>,

    #[serde(default)]
    pub total: Option<u64>,
}

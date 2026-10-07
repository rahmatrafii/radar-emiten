// src/sectors/models/screener.rs
// Model respons untuk endpoint Companies Screener v2.
//
// Referensi: https://docs.sectors.app/api-references/v2/indonesia/screener/companies
// - Endpoint: `GET /v2/companies/`
// - Query: `where` (SQL-like, operator `=`, string dikutip) + `order_by`.
//   Natural language `q` TIDAK dipakai (biaya 3 kredit vs 1 kredit structured).
// - Param `fields` TIDAK didukung API (400 `UNSUPPORTED_PARAMETERS`).
// - Respons: `{"results": [...], "pagination": {...}}` (key `results`, bukan `data`).
// - Simbol memakai suffix `.JK` (contoh: `"BBCA.JK"`).
// - Biaya: 1 kredit per structured query sukses (terverifikasi live 7 Okt 2026).

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Satu baris hasil dari Companies Screener v2.
///
/// Baris bersifat ringkas (umumnya `symbol` + `company_name`); field lain
/// opsional dan bervariasi sehingga field tak dikenal ditampung di `fields`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScreenerRow {
    /// Kode saham, contoh: `"BBCA.JK"` (dengan suffix `.JK`).
    pub symbol: String,

    /// Nama perusahaan.
    pub company_name: Option<String>,

    /// Sektor IDX (bila disertakan respons).
    pub sector: Option<String>,

    /// Sub-sektor IDX (bila disertakan respons).
    pub sub_sector: Option<String>,

    /// Semua field lain yang mungkin ada di respons.
    #[serde(flatten)]
    pub fields: HashMap<String, serde_json::Value>,
}

impl ScreenerRow {
    /// Mengambil nilai field sebagai `f64`.
    /// Mengembalikan `None` jika field tidak ada atau bukan angka.
    pub fn get_f64(&self, field: &str) -> Option<f64> {
        match self.fields.get(field)? {
            serde_json::Value::Number(n) => n.as_f64(),
            serde_json::Value::String(s) => s.parse().ok(),
            _ => None,
        }
    }

    /// Ticker 4 huruf untuk kontrak internal (orchestrator validasi `^[A-Z]{4}$`).
    /// `"BBCA.JK"` → `"BBCA"`.
    pub fn ticker_short(&self) -> String {
        crate::sectors::client::normalize_ticker(&self.symbol)
    }
}

/// Respons penuh dari Companies Screener v2.
#[derive(Debug, Deserialize)]
pub struct ScreenerResponse {
    /// Daftar perusahaan (key resmi: `results`).
    #[serde(default)]
    pub results: Vec<ScreenerRow>,

    /// Metadata paginasi (bentuk tidak dikontrak; disimpan mentah).
    #[serde(default)]
    pub pagination: Option<serde_json::Value>,
}

/// Satu entri dari `GET /v2/subsectors/` (daftar sector/subsector slug).
///
/// Referensi: https://docs.sectors.app/api-references/v2/indonesia/helper-list/subsectors
/// Respons adalah JSON array langsung: `[{"sector": "...", "subsector": "..."}]`.
/// Biaya: 1 kredit (terverifikasi live 7 Okt 2026).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubsectorItem {
    /// Sector slug kebab-case, contoh: `"financials"`.
    pub sector: String,

    /// Subsector slug kebab-case, contoh: `"banks"`.
    pub subsector: String,
}

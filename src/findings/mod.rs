// src/findings/mod.rs — Kontrak output Finding untuk seluruh tim

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

/// Tingkat keyakinan hasil analisis.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TingkatKeyakinan {
    Tinggi,
    Sedang,
    Rendah,
}

impl std::fmt::Display for TingkatKeyakinan {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TingkatKeyakinan::Tinggi => write!(f, "TINGGI"),
            TingkatKeyakinan::Sedang => write!(f, "SEDANG"),
            TingkatKeyakinan::Rendah => write!(f, "RENDAH"),
        }
    }
}

/// Kontrak output data ke seluruh tim IDX Sentinel.
///
/// Dihasilkan oleh [`SectorsClient`] dan dikonsumsi oleh:
/// - MCP Server (Peran 2)  
/// - Alert Engine (Peran 3)
/// - Dashboard (Peran 4)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Finding {
    /// Kode saham BEI, contoh: `"BBCA"`.
    pub ticker: String,

    /// Nama indikator keuangan, contoh: `"net_interest_margin_q"`.
    pub indikator: String,

    /// Nilai indikator pada kuartal **sebelumnya**.
    pub nilai_sebelum: f64,

    /// Nilai indikator pada kuartal **terkini**.
    pub nilai_sekarang: f64,

    /// Periode kuartal terkini, format `"YYYY-QN"`, contoh: `"2024-Q3"`.
    pub periode: String,

    /// Sumber data, contoh: `"sectors_api_v2/screener"`.
    pub sumber: String,

    /// Tingkat keyakinan analisis.
    pub tingkat_keyakinan: TingkatKeyakinan,
}

impl Finding {
    /// Membuat `Finding` baru dengan field yang sudah divalidasi.
    pub fn new(
        ticker: impl Into<String>,
        indikator: impl Into<String>,
        nilai_sebelum: f64,
        nilai_sekarang: f64,
        periode: impl Into<String>,
        sumber: impl Into<String>,
        tingkat_keyakinan: TingkatKeyakinan,
    ) -> Self {
        Self {
            ticker: ticker.into(),
            indikator: indikator.into(),
            nilai_sebelum,
            nilai_sekarang,
            periode: periode.into(),
            sumber: sumber.into(),
            tingkat_keyakinan,
        }
    }

    /// Persentase perubahan nilai indikator.
    pub fn delta_pct(&self) -> f64 {
        if self.nilai_sebelum == 0.0 {
            return 0.0;
        }
        (self.nilai_sekarang - self.nilai_sebelum) / self.nilai_sebelum.abs() * 100.0
    }

    /// Apakah ada peningkatan dibanding periode sebelumnya?
    pub fn is_improving(&self) -> bool {
        self.nilai_sekarang > self.nilai_sebelum
    }
}

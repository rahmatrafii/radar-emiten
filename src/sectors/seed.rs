// src/sectors/seed.rs — Modul Seed Data Q_{t-1} (Baseline Historis)
//
// Menyimpan data baseline kuartalan emiten sampel demo untuk periode
// 2026-Q1 (kuartal sebelumnya / Q_{t-1}) sebagai nilai referensi pembanding.
//
// Data ini digunakan oleh integration test & e2e runner di `main.rs` untuk
// mengisi `Finding::nilai_sebelum` ketika data _prev tidak tersedia di API.

use std::collections::HashMap;

// ---------------------------------------------------------------------------
// Periode Baseline
// ---------------------------------------------------------------------------

/// Periode kuartal baseline yang disimpan dalam modul ini.
const BASELINE_PERIOD: &str = "2026-Q1";

// ---------------------------------------------------------------------------
// Struct SeedSnapshot
// ---------------------------------------------------------------------------

/// Snapshot data baseline historis untuk satu emiten pada periode Q_{t-1}.
///
/// Digunakan sebagai nilai referensi (nilai_sebelum) saat membandingkan
/// metrik keuangan terkini dengan kuartal sebelumnya.
///
/// # Contoh
/// ```rust
/// use radar_emiten::sectors::seed::SeedSnapshot;
///
/// let snapshot = SeedSnapshot::for_bbca();
/// assert_eq!(snapshot.ticker, "BBCA");
/// assert_eq!(snapshot.get("net_interest_margin_q"), Some(5.8));
/// ```
#[derive(Debug, Clone)]
pub struct SeedSnapshot {
    /// Kode saham BEI, contoh: `"BBCA"`.
    pub ticker: String,

    /// Periode kuartal baseline, format `"YYYY-QN"`.
    pub periode: String,

    /// Map indikator → nilai baseline.
    indicators: HashMap<String, f64>,
}

impl SeedSnapshot {
    /// Membuat snapshot baru dengan ticker dan periode tertentu.
    fn new(ticker: &str, periode: &str, indicators: HashMap<String, f64>) -> Self {
        Self {
            ticker: ticker.to_string(),
            periode: periode.to_string(),
            indicators,
        }
    }

    /// Snapshot baseline untuk **BBCA** (Bank Central Asia).
    ///
    /// Nilai indikator kuartal 2026-Q1:
    /// - `net_interest_margin_q` = 5.8
    /// - `pertumbuhan_laba_q` = 8.1
    pub fn for_bbca() -> Self {
        let mut indicators = HashMap::new();
        indicators.insert("net_interest_margin_q".to_string(), 5.8_f64);
        indicators.insert("pertumbuhan_laba_q".to_string(), 8.1_f64);
        Self::new("BBCA", BASELINE_PERIOD, indicators)
    }

    /// Snapshot baseline untuk **BMRI** (Bank Mandiri).
    ///
    /// Nilai indikator kuartal 2026-Q1:
    /// - `net_interest_margin_q` = 5.4
    /// - `pertumbuhan_laba_q` = 7.5
    pub fn for_bmri() -> Self {
        let mut indicators = HashMap::new();
        indicators.insert("net_interest_margin_q".to_string(), 5.4_f64);
        indicators.insert("pertumbuhan_laba_q".to_string(), 7.5_f64);
        Self::new("BMRI", BASELINE_PERIOD, indicators)
    }

    /// Mengambil nilai baseline untuk indikator tertentu.
    ///
    /// Mengembalikan `None` jika indikator tidak ada dalam snapshot.
    pub fn get(&self, indicator: &str) -> Option<f64> {
        self.indicators.get(indicator).copied()
    }
}

// ---------------------------------------------------------------------------
// Fungsi publik level-modul
// ---------------------------------------------------------------------------

/// Mengambil nilai baseline Q_{t-1} untuk emiten dan indikator tertentu.
///
/// # Arguments
/// * `ticker` — Kode saham BEI (case-insensitive), contoh: `"BBCA"`.
/// * `indicator` — Nama indikator keuangan, contoh: `"net_interest_margin_q"`.
///
/// # Returns
/// `Some(f64)` jika data tersedia, `None` jika ticker atau indikator tidak dikenal.
///
/// # Contoh
/// ```rust
/// use radar_emiten::sectors::seed::get_baseline_snapshot;
///
/// let nim = get_baseline_snapshot("BBCA", "net_interest_margin_q");
/// assert_eq!(nim, Some(5.8));
///
/// let unknown = get_baseline_snapshot("TLKM", "net_interest_margin_q");
/// assert_eq!(unknown, None);
/// ```
pub fn get_baseline_snapshot(ticker: &str, indicator: &str) -> Option<f64> {
    let snapshot = match ticker.to_uppercase().as_str() {
        "BBCA" => SeedSnapshot::for_bbca(),
        "BMRI" => SeedSnapshot::for_bmri(),
        _ => return None,
    };
    snapshot.get(indicator)
}

/// Mengembalikan periode kuartal baseline yang digunakan modul ini.
///
/// Format: `"YYYY-QN"`, contoh: `"2026-Q1"`.
pub fn get_baseline_period() -> &'static str {
    BASELINE_PERIOD
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bbca_nim() {
        assert_eq!(
            get_baseline_snapshot("BBCA", "net_interest_margin_q"),
            Some(5.8)
        );
    }

    #[test]
    fn test_bbca_pertumbuhan_laba() {
        assert_eq!(
            get_baseline_snapshot("BBCA", "pertumbuhan_laba_q"),
            Some(8.1)
        );
    }

    #[test]
    fn test_bmri_nim() {
        assert_eq!(
            get_baseline_snapshot("BMRI", "net_interest_margin_q"),
            Some(5.4)
        );
    }

    #[test]
    fn test_bmri_pertumbuhan_laba() {
        assert_eq!(
            get_baseline_snapshot("BMRI", "pertumbuhan_laba_q"),
            Some(7.5)
        );
    }

    #[test]
    fn test_unknown_ticker() {
        assert_eq!(
            get_baseline_snapshot("TLKM", "net_interest_margin_q"),
            None
        );
    }

    #[test]
    fn test_unknown_indicator() {
        assert_eq!(get_baseline_snapshot("BBCA", "unknown_indicator"), None);
    }

    #[test]
    fn test_case_insensitive_ticker() {
        assert_eq!(
            get_baseline_snapshot("bbca", "net_interest_margin_q"),
            Some(5.8)
        );
    }

    #[test]
    fn test_baseline_period() {
        assert_eq!(get_baseline_period(), "2026-Q1");
    }

    #[test]
    fn test_seed_snapshot_struct_bbca() {
        let snap = SeedSnapshot::for_bbca();
        assert_eq!(snap.ticker, "BBCA");
        assert_eq!(snap.periode, "2026-Q1");
        assert_eq!(snap.get("net_interest_margin_q"), Some(5.8));
        assert_eq!(snap.get("pertumbuhan_laba_q"), Some(8.1));
    }

    #[test]
    fn test_seed_snapshot_struct_bmri() {
        let snap = SeedSnapshot::for_bmri();
        assert_eq!(snap.ticker, "BMRI");
        assert_eq!(snap.periode, "2026-Q1");
        assert_eq!(snap.get("net_interest_margin_q"), Some(5.4));
        assert_eq!(snap.get("pertumbuhan_laba_q"), Some(7.5));
    }
}

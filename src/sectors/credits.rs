// src/sectors/credits.rs — Credit Tracker untuk estimasi biaya API

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tracing::info;

/// Biaya kredit per jenis panggilan API (terverifikasi dari dokumentasi resmi).
/// Referensi: https://docs.sectors.app (billing: 2xx menagih biaya endpoint,
///
/// 400/401/403/429/5xx gratis; 404 ikut menagih 1 kredit).
pub mod cost {
    /// `GET /v2/subsectors/` — 1 kredit.
    pub const SUBSECTORS: u64 = 1;

    /// `GET /v2/companies/` structured (`where`) — 1 kredit.
    /// (Natural language `q` = 3 kredit — tidak dipakai client ini.)
    pub const SCREENER_STRUCTURED: u64 = 1;

    /// `GET /v2/companies/quarterly-financial-dates/` — 1 kredit per halaman.
    pub const QUARTERLY_DATES_PAGE: u64 = 1;

    /// `GET /v2/subsector/report/{sub}/` — 1 kredit per section.
    pub const SUBSECTOR_REPORT_PER_SECTION: u64 = 1;

    /// `GET /v2/financials/quarterly/{symbol}/` — 1 kredit per quarter.
    pub const FINANCIALS_PER_QUARTER: u64 = 1;
}

/// Melacak estimasi penggunaan kredit API secara thread-safe.
///
/// Gunakan `Arc<CreditTracker>` untuk berbagi antar thread/task async.
///
/// # Contoh
/// ```rust
/// use radar_emiten::sectors::credits::{cost, CreditTracker};
/// let tracker = CreditTracker::new(1000);
/// tracker.charge(cost::SCREENER_STRUCTURED);
/// println!("Kredit digunakan: {}", tracker.used());
/// ```
#[derive(Debug)]
pub struct CreditTracker {
    /// Batas kredit maksimum yang boleh digunakan.
    budget: u64,

    /// Akumulator kredit yang sudah terpakai (atomic untuk thread safety).
    used: AtomicU64,
}

impl CreditTracker {
    /// Membuat tracker baru dengan batas kredit tertentu.
    pub fn new(budget: u64) -> Arc<Self> {
        Arc::new(Self {
            budget,
            used: AtomicU64::new(0),
        })
    }

    /// Menambah kredit yang terpakai.
    ///
    /// Mengembalikan total kredit yang sudah dipakai setelah charge ini.
    pub fn charge(&self, amount: u64) -> u64 {
        let prev = self.used.fetch_add(amount, Ordering::Relaxed);
        let total = prev + amount;
        info!(
            kredit_dipakai = total,
            kredit_budget = self.budget,
            charge = amount,
            "Credit charged"
        );
        total
    }

    /// Mengembalikan jumlah kredit yang sudah terpakai.
    pub fn used(&self) -> u64 {
        self.used.load(Ordering::Relaxed)
    }

    /// Mengembalikan sisa kredit (saturating).
    pub fn remaining(&self) -> u64 {
        self.budget.saturating_sub(self.used())
    }

    /// Mengecek apakah masih ada kredit untuk panggilan berikutnya.
    pub fn can_afford(&self, amount: u64) -> bool {
        self.used() + amount <= self.budget
    }

    /// Mereset counter (berguna saat pergantian billing cycle / test).
    pub fn reset(&self) {
        self.used.store(0, Ordering::SeqCst);
    }

    /// Ringkasan penggunaan kredit sebagai string log.
    pub fn summary(&self) -> String {
        format!(
            "Kredit: {}/{} dipakai ({} sisa)",
            self.used(),
            self.budget,
            self.remaining()
        )
    }
}

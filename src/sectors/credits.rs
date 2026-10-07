// src/sectors/credits.rs — Credit Tracker untuk estimasi biaya API

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tracing::info;

/// Biaya kredit estimasi per jenis panggilan API.
/// Angka ini bisa disesuaikan sesuai dokumentasi Sectors API.
pub mod cost {
    /// Biaya memanggil endpoint quarterly reports (per request).
    pub const QUARTERLY_REPORTS: u64 = 5;

    /// Biaya memanggil screener per ticker (estimasi).
    pub const SCREENER_PER_TICKER: u64 = 2;

    /// Biaya panggilan screener umum (tanpa filter ticker spesifik).
    pub const SCREENER_GENERAL: u64 = 3;
}

/// Melacak estimasi penggunaan kredit API secara thread-safe.
///
/// Gunakan `Arc<CreditTracker>` untuk berbagi antar thread/task async.
///
/// # Contoh
/// ```rust
/// let tracker = CreditTracker::new(1000);
/// tracker.charge(cost::QUARTERLY_REPORTS);
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

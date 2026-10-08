// src/lib.rs — root crate, re-export modul publik

pub mod findings;
pub mod sectors;

// Re-export tipe utama agar pengguna crate dapat import langsung:
//   use radar_emiten::{Finding, SectorsClient};
pub use findings::Finding;
pub use sectors::client::SectorsClient;

// Re-export fungsi seed baseline Q_{t-1}:
//   use radar_emiten::{get_baseline_snapshot, get_baseline_period};
pub use sectors::seed::{get_baseline_period, get_baseline_snapshot, SeedSnapshot};

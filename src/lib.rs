// src/lib.rs — root crate, re-export modul publik

pub mod findings;
pub mod sectors;

// Re-export tipe utama agar pengguna crate dapat import langsung:
//   use radar_emiten::{Finding, SectorsClient};
pub use findings::Finding;
pub use sectors::client::SectorsClient;

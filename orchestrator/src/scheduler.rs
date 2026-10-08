//! Penjadwal otomatis (F2) — milik Rafi.
//!
//! Menjalankan `AgentPipeline::run_cycle()` secara berkala tanpa intervensi manual.
//!
//! - Interval dibaca dari `config.scheduler_interval_seconds` (default 86400 = 24 jam).
//! - Interval `0` berarti scheduler dinonaktifkan (untuk testing).
//! - Siklus pertama berjalan SETELAH interval berlalu (bukan langsung saat startup),
//!   agar server start tidak langsung menulis DB.
//! - Error pada satu siklus tidak menghentikan scheduler — `run_cycle()`
//!   mengembalikan `PipelineRunReport` (termasuk `errors`), bukan `Result`.
//! - Scheduler berhenti saat menerima sinyal shutdown dari `main.rs`.

use std::time::Duration;

use tokio::sync::oneshot;

use crate::orchestrator::agent_pipeline::AgentPipeline;
use crate::state::AppState;

/// Apakah scheduler aktif untuk interval tertentu.
/// Interval `0` = dinonaktifkan (untuk testing/development).
pub fn is_enabled(interval_seconds: u64) -> bool {
    interval_seconds > 0
}

/// Jalankan loop scheduler sampai menerima sinyal shutdown.
///
/// Berhenti dan return bila `interval_seconds == 0`.
pub async fn run_scheduler(state: AppState, mut shutdown: oneshot::Receiver<()>) {
    let interval_secs = state.config.scheduler_interval_seconds;

    if !is_enabled(interval_secs) {
        tracing::info!("Scheduler dinonaktifkan (scheduler_interval_seconds=0)");
        return;
    }

    tracing::info!(
        "Scheduler diaktifkan: interval {interval_secs} detik; siklus pertama setelah interval berlalu"
    );

    loop {
        tokio::select! {
            _ = tokio::time::sleep(Duration::from_secs(interval_secs)) => {
                tracing::info!("Scheduler: memulai siklus pipeline");
                let pipeline = AgentPipeline::new(state.clone());
                let report = pipeline.run_cycle().await;
                tracing::info!(
                    "Scheduler: siklus selesai — snapshots={}, baselines={}, findings_accepted={}, findings_rejected={}, alerts_sent={}, alerts_deferred={}, errors={}",
                    report.snapshots_stored,
                    report.baselines_created,
                    report.findings_accepted,
                    report.findings_rejected,
                    report.alerts_sent,
                    report.alerts_deferred,
                    report.errors.len(),
                );
                for err in &report.errors {
                    tracing::warn!("Scheduler: error siklus: {err}");
                }
            }
            _ = &mut shutdown => {
                tracing::info!("Scheduler: menerima sinyal shutdown, berhenti...");
                break;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::is_enabled;

    #[test]
    fn interval_nol_berarti_disabled() {
        assert!(!is_enabled(0));
    }

    #[test]
    fn interval_positif_berarti_enabled() {
        assert!(is_enabled(1));
        assert!(is_enabled(86400));
    }
}

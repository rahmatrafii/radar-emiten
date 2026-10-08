//! `orchestrator::AgentPipeline` — pengendali alur satu siklus pemantauan.
//!
//! Prinsip: angka dan aturan ditetapkan Rust secara deterministik; LLM tidak
//! menjadi sumber angka. Setiap kegagalan HTTP eksternal tetap meninggalkan
//! jejak database (`agent_traces`) dan tidak pernah menghasilkan alert.

use std::collections::BTreeSet;

use crate::models::finding::Finding;
use crate::models::whatsapp::{DISCLAIMER_RESMI, WhatsAppAlertPayload};
use crate::repositories::{alerts, snapshots, theses, traces, users};
use crate::routes::findings::accept_finding;
use crate::state::AppState;

/// Hasil eksplisit satu siklus pipeline.
#[derive(Debug, Default, Clone, serde::Serialize)]
pub struct PipelineRunReport {
    pub snapshots_stored: usize,
    pub baselines_created: usize,
    pub findings_accepted: usize,
    pub findings_rejected: usize,
    pub alerts_sent: usize,
    pub alerts_deferred: usize,
    pub errors: Vec<String>,
}

pub struct AgentPipeline {
    state: AppState,
}

impl AgentPipeline {
    pub fn new(state: AppState) -> Self {
        Self { state }
    }

    pub async fn run_cycle(&self) -> PipelineRunReport {
        let mut report = PipelineRunReport::default();
        let pool = &self.state.pool;

        let watch = match theses::watchlist_with_user(pool).await {
            Ok(w) => w,
            Err(e) => {
                report.errors.push(format!("gagal membaca watchlist: {e}"));
                return report;
            }
        };
        if watch.is_empty() {
            let _ = traces::trace(
                pool,
                "scheduler",
                "cycle_noop",
                Some("no_watchlist"),
                None,
                None,
            )
            .await;
            return report;
        }

        // Kumpulkan pasangan ticker/metric unik dari semua tesis aktif.
        let mut pairs: BTreeSet<(String, String)> = BTreeSet::new();
        for w in &watch {
            pairs.insert((w.ticker.clone(), w.metric_name.clone()));
        }

        for (ticker, metric_name) in pairs {
            match self
                .process_metric(&ticker, &metric_name, &watch, &mut report)
                .await
            {
                Ok(()) => {}
                Err(e) => {
                    report.errors.push(format!("{ticker}/{metric_name}: {e}"));
                    let _ = traces::trace(
                        pool,
                        "scout",
                        "metric_failed",
                        Some("error"),
                        Some(&ticker),
                        Some(serde_json::json!({"metric": metric_name})),
                    )
                    .await;
                }
            }
        }

        // Hitung ulang status tesis yang punya bukti valid.
        let tesis_ids: BTreeSet<i64> = watch.iter().map(|w| w.tesis_id).collect();
        for tesis_id in tesis_ids {
            match theses::support_counts(pool, tesis_id).await {
                Ok((mendukung, melemahkan)) if mendukung + melemahkan > 0 => {
                    let status = if mendukung > melemahkan {
                        "MENGUAT"
                    } else if melemahkan > mendukung {
                        "MELEMAH"
                    } else {
                        "NETRAL"
                    };
                    let _ = theses::update_status(pool, tesis_id, status).await;
                }
                _ => {}
            }
        }

        let _ = traces::trace(
            pool,
            "scheduler",
            "cycle_finished",
            Some("ok"),
            None,
            Some(serde_json::to_value(&report).unwrap_or_default()),
        )
        .await;
        report
    }

    async fn process_metric(
        &self,
        ticker: &str,
        metric_name: &str,
        watch: &[theses::WatchWithUser],
        report: &mut PipelineRunReport,
    ) -> Result<(), String> {
        let pool = &self.state.pool;

        // 1. Ambil evidence melalui MCP gateway (data Sectors milik Rafi).
        let evidence = self
            .state
            .mcp
            .get_company_evidence(ticker.to_string(), None)
            .await
            .map_err(|e| format!("mcp evidence: {e}"))?;
        let _ = traces::trace(
            pool,
            "scout",
            "evidence_fetched",
            Some("ok"),
            Some(ticker),
            Some(serde_json::json!({"period": evidence.period, "source": evidence.source})),
        )
        .await;

        let current = evidence
            .value
            .filter(|v| v.is_finite())
            .ok_or_else(|| "nilai evidence tidak numerik/finite".to_string())?;

        // 2. Simpan snapshot terbaru; baca snapshot sebelumnya dari DB.
        let snapshot_id = snapshots::upsert_observed(
            pool,
            ticker,
            metric_name,
            &evidence.period,
            current,
            &evidence.source,
            Some(&evidence.subsector),
            evidence.observed_at,
        )
        .await
        .map_err(|e| format!("snapshot upsert: {e}"))?;
        report.snapshots_stored += 1;
        let _ = traces::trace(
            pool,
            "scout",
            "snapshot_stored",
            Some("ok"),
            Some(ticker),
            Some(serde_json::json!({"snapshot_id": snapshot_id, "period": evidence.period})),
        )
        .await;

        let (_latest, previous) = snapshots::latest_and_previous(pool, ticker, metric_name)
            .await
            .map_err(|e| format!("snapshot lookup: {e}"))?;
        let Some(previous) = previous else {
            report.baselines_created += 1;
            let _ = traces::trace(
                pool,
                "analyst",
                "baseline_created",
                Some("insufficient_comparison_data"),
                Some(ticker),
                None,
            )
            .await;
            return Ok(());
        };
        if !previous.value.is_finite() || previous.value == 0.0 {
            let _ = traces::trace(
                pool,
                "analyst",
                "delta_skipped",
                Some("previous_not_finite_or_zero"),
                Some(ticker),
                None,
            )
            .await;
            return Err("snapshot pembanding nol/tidak finite".into());
        }

        // 3. Delta relatif deterministik (asumsi; threshold terdokumentasi).
        let rel = (current - previous.value) / previous.value.abs();
        if !rel.is_finite() {
            return Err("delta relatif tidak finite".into());
        }
        let pct = rel * 100.0;

        // 4. Bentuk Finding resmi sesuai CONTRACTS.md.
        let finding = Finding {
            ticker: ticker.to_string(),
            subsector: evidence.subsector.clone(),
            metric_name: metric_name.to_string(),
            current_value: serde_json::json!(current),
            previous_value: serde_json::json!(previous.value),
            period: evidence.period.clone(),
            source: evidence.source.clone(),
            observed_at: evidence.observed_at.to_rfc3339(),
            // Konservatif 0.9 sampai tim punya model confidence; didokumentasi.
            confidence_score: 0.9,
            finding_summary: format!(
                "{metric_name} {ticker} periode {period}: {current:.2} vs {prev:.2} pada periode pembanding (perubahan {pct:+.2}%).",
                period = evidence.period,
                prev = previous.value,
            ),
        };

        // 5. Evidence gate + compliance audit via jalur yang sama dengan POST /findings.
        let response = accept_finding(&self.state, finding.clone())
            .await
            .map_err(|e| format!("accept_finding: {e:?}"))?;
        let accepted = response.0.get("status").and_then(|v| v.as_str()) == Some("accepted");
        let finding_id = response.0.get("finding_id").and_then(|v| v.as_i64());

        if !accepted {
            report.findings_rejected += 1;
            let _ = traces::trace(
                pool,
                "compliance",
                "finding_rejected",
                Some("rejected"),
                Some(ticker),
                Some(serde_json::json!({"metric": metric_name})),
            )
            .await;
            return Ok(());
        }
        report.findings_accepted += 1;
        let Some(finding_id) = finding_id else {
            return Ok(());
        };

        // 6. Hubungkan ke tesis yang memantau ticker/metric ini + tentukan arah.
        let thr = self.state.config.neutral_relative_threshold;
        for w in watch
            .iter()
            .filter(|w| w.ticker == ticker && w.metric_name == metric_name)
        {
            let arah = support_direction(&w.desired_direction, rel, thr);
            theses::link_finding(pool, finding_id, w.tesis_id, arah)
                .await
                .map_err(|e| format!("link finding: {e}"))?;
            self.maybe_dispatch_alert(&finding, w, arah, ticker, metric_name, report)
                .await;
        }
        Ok(())
    }

    async fn maybe_dispatch_alert(
        &self,
        finding: &Finding,
        w: &theses::WatchWithUser,
        arah: &str,
        ticker: &str,
        metric_name: &str,
        report: &mut PipelineRunReport,
    ) {
        let pool = &self.state.pool;
        let finding_id: i64 = {
            // Ambil id dari tabel findings via kolom unik; aman karena accepted.
            let row: Option<(i64,)> = sqlx::query_as(
                "SELECT id FROM findings WHERE ticker=$1 AND metric_name=$2 AND period=$3 \
                 AND status='lolos' ORDER BY id DESC LIMIT 1",
            )
            .bind(ticker)
            .bind(metric_name)
            .bind(&finding.period)
            .fetch_optional(pool)
            .await
            .unwrap_or(None);
            match row {
                Some((id,)) => id,
                None => return,
            }
        };

        // Cooldown/mute: jeda_sampai di masa depan -> tidak ada alert proaktif.
        match users::is_muted(pool, w.pengguna_id).await {
            Ok(true) => {
                let _ = traces::trace(
                    pool,
                    "dispatcher",
                    "alert_muted",
                    Some("muted"),
                    Some(ticker),
                    None,
                )
                .await;
                return;
            }
            Ok(false) => {}
            Err(_) => return,
        }

        // Batas harian (default 3/hari, timezone WIB di repository).
        if let Ok(count) = alerts::count_sent_today(pool, w.pengguna_id).await
            && count >= self.state.config.max_alerts_per_day as i64
        {
            if let Ok(id) = alerts::insert(
                pool,
                w.pengguna_id,
                w.tesis_id,
                finding_id,
                metric_name,
                &finding.period,
                &format!("{ticker} · {metric_name}"),
                "ditunda: batas alert harian tercapai",
            )
            .await
            {
                let _ = alerts::mark_status(pool, id, "ditunda").await;
            }
            report.alerts_deferred += 1;
            let _ = traces::trace(
                pool,
                "dispatcher",
                "alert_deferred",
                Some("daily_limit"),
                Some(ticker),
                None,
            )
            .await;
            return;
        }

        // Insert unik (pengguna+tesis+metric+period). Duplikat -> cukup 1 alert.
        let alert_id = alerts::insert(
            pool,
            w.pengguna_id,
            w.tesis_id,
            finding_id,
            metric_name,
            &finding.period,
            &format!("{ticker} · {metric_name}"),
            "alert pending",
        )
        .await;
        let alert_id = match alert_id {
            Ok(id) => id,
            Err(sqlx::Error::Database(e)) if e.is_unique_violation() => {
                let _ = traces::trace(
                    pool,
                    "dispatcher",
                    "alert_duplicate",
                    Some("unique_violation"),
                    Some(ticker),
                    None,
                )
                .await;
                return;
            }
            Err(_) => return,
        };

        // Bangun payload dari template Rust (bukan LLM bebas).
        let header = format!("{ticker} · {metric_name}");
        let body = format!(
            "Nilai periode pembanding: {:?}. Nilai terbaru: {:?}. Periode: {}. Arah: {}. Sumber: {}.",
            finding.previous_value, finding.current_value, finding.period, arah, finding.source
        );
        let payload = WhatsAppAlertPayload {
            recipient_number: w.nomor_wa.clone(),
            header,
            body,
            disclaimer: DISCLAIMER_RESMI.into(),
        };
        let outcome = self.state.whatsapp.send_alert(&payload).await;
        match outcome {
            Ok(o) if o.status == "mock_sent" || o.status == "sent" => {
                let _ = alerts::mark_sent(pool, alert_id).await;
                report.alerts_sent += 1;
                let _ = traces::trace(
                    pool,
                    "dispatcher",
                    "whatsapp_dispatched",
                    Some(&o.status),
                    Some(ticker),
                    None,
                )
                .await;
            }
            Ok(o) => {
                let _ = alerts::mark_status(pool, alert_id, "gagal").await;
                let _ = traces::trace(
                    pool,
                    "dispatcher",
                    "whatsapp_blocked",
                    Some(&o.status),
                    Some(ticker),
                    None,
                )
                .await;
            }
            Err(_) => {
                let _ = alerts::mark_status(pool, alert_id, "gagal").await;
                let _ = traces::trace(
                    pool,
                    "dispatcher",
                    "whatsapp_failed",
                    Some("error"),
                    Some(ticker),
                    None,
                )
                .await;
            }
        }
    }
}

/// Arah dukungan Finding terhadap tesis berdasarkan `desired_direction`
/// dan threshold relatif netral (`NEUTRAL_RELATIVE_THRESHOLD`, asumsi).
pub fn support_direction(desired_direction: &str, rel: f64, thr: f64) -> &'static str {
    match desired_direction {
        "increase" => {
            if rel > thr {
                "mendukung"
            } else if rel < -thr {
                "melemahkan"
            } else {
                "netral"
            }
        }
        "decrease" => {
            if rel < -thr {
                "mendukung"
            } else if rel > thr {
                "melemahkan"
            } else {
                "netral"
            }
        }
        // `any`/ambigu: status arah netral sampai definisi dipastikan tim
        // (runbook §12.3).
        _ => "netral",
    }
}

#[cfg(test)]
mod tests {
    use super::support_direction;

    #[test]
    fn arah_increase() {
        assert_eq!(support_direction("increase", 0.05, 0.02), "mendukung");
        assert_eq!(support_direction("increase", -0.05, 0.02), "melemahkan");
        assert_eq!(support_direction("increase", 0.01, 0.02), "netral");
    }

    #[test]
    fn arah_decrease_terbalik() {
        assert_eq!(support_direction("decrease", -0.05, 0.02), "mendukung");
        assert_eq!(support_direction("decrease", 0.05, 0.02), "melemahkan");
    }

    #[test]
    fn any_selalu_netral() {
        assert_eq!(support_direction("any", 1.0, 0.02), "netral");
    }
}

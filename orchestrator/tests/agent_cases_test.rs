//! Few-Shot Test Cases (D-04) — milik Dian.
//!
//! Fixture: `tests/fixtures/agent_cases.json` (minimal 5 kasus:
//! perbankan, telekomunikasi, komoditas + skenario reject compliance/evidence).
//!
//! Pola sama seperti test lain: tanpa `DATABASE_URL` test DB dilewati (skip)
//! agar `cargo test` tetap hijau; validasi struktur fixture selalu jalan.

use std::collections::HashSet;

use orchestrator::config::{AppMode, Config};
use orchestrator::models::finding::Finding;
use orchestrator::state::AppState;

#[derive(Debug, serde::Deserialize)]
struct SnapshotSetup {
    period: String,
    value: f64,
}

#[derive(Debug, serde::Deserialize)]
struct Expected {
    status: String,
    #[allow(dead_code)]
    is_compliant: bool,
    #[allow(dead_code)]
    evidence_verified: bool,
}

#[derive(Debug, serde::Deserialize)]
struct AgentCase {
    case_id: String,
    #[allow(dead_code)]
    description: String,
    sector: String,
    input: Finding,
    setup_snapshots: Vec<SnapshotSetup>,
    expected: Expected,
}

fn load_cases() -> Vec<AgentCase> {
    let raw = include_str!("fixtures/agent_cases.json");
    serde_json::from_str(raw).expect("fixture agent_cases.json harus valid JSON")
}

// ---------------------------------------------------------------------------
// Validasi struktur fixture (tanpa DB, selalu jalan)
// ---------------------------------------------------------------------------

#[test]
fn fixture_berisi_minimal_5_kasus() {
    let cases = load_cases();
    assert!(
        cases.len() >= 5,
        "fixture harus berisi minimal 5 kasus, dapat: {}",
        cases.len()
    );
}

#[test]
fn fixture_mencakup_3_sektor_berbeda() {
    let cases = load_cases();
    let sectors: HashSet<_> = cases.iter().map(|c| c.sector.as_str()).collect();
    for expected in ["banks", "telecommunication", "commodities"] {
        assert!(
            sectors.contains(expected),
            "fixture harus mencakup sektor {expected}, dapat: {sectors:?}"
        );
    }
}

#[test]
fn fixture_mencakup_accept_dan_reject() {
    let cases = load_cases();
    let has_accept = cases.iter().any(|c| c.expected.status == "accepted");
    let has_reject = cases.iter().any(|c| c.expected.status == "rejected");
    assert!(has_accept, "fixture harus punya minimal 1 kasus accepted");
    assert!(has_reject, "fixture harus punya minimal 1 kasus rejected");
}

#[test]
fn fixture_case_id_unik() {
    let cases = load_cases();
    let mut ids = HashSet::new();
    for c in &cases {
        assert!(
            ids.insert(c.case_id.clone()),
            "case_id duplikat: {}",
            c.case_id
        );
    }
}

// ---------------------------------------------------------------------------
// End-to-end per kasus via accept_finding (butuh PostgreSQL)
// ---------------------------------------------------------------------------

async fn test_state() -> Option<AppState> {
    let url = std::env::var("DATABASE_URL").ok()?;
    let pool = sqlx::PgPool::connect(&url).await.ok()?;
    sqlx::migrate!("./migrations").run(&pool).await.ok()?;
    Some(AppState::new(
        pool,
        Config {
            app_mode: AppMode::Mock,
            port: 0,
            database_url: url,
            policy_disclaimer_conflict_acknowledged: false,
            ..Default::default()
        },
    ))
}

async fn cleanup_case(pool: &sqlx::PgPool, ticker: &str) {
    sqlx::query("DELETE FROM compliance_audit_logs WHERE finding_id IN (SELECT id FROM findings WHERE ticker=$1)")
        .bind(ticker)
        .execute(pool)
        .await
        .ok();
    sqlx::query("DELETE FROM findings WHERE ticker=$1")
        .bind(ticker)
        .execute(pool)
        .await
        .ok();
    sqlx::query("DELETE FROM snapshot_data WHERE ticker=$1")
        .bind(ticker)
        .execute(pool)
        .await
        .ok();
}

#[tokio::test]
async fn agent_cases_end_to_end() {
    use axum::body::Body;
    use axum::http::Request;
    use tower::ServiceExt;

    let Some(state) = test_state().await else {
        return;
    };
    let cases = load_cases();
    let app = orchestrator::routes::routes().with_state(state.clone());

    for case in &cases {
        cleanup_case(&state.pool, &case.input.ticker).await;

        // Setup: snapshot current + previous dari fixture.
        for snap in &case.setup_snapshots {
            orchestrator::repositories::snapshots::upsert_observed(
                &state.pool,
                &case.input.ticker,
                &case.input.metric_name,
                &snap.period,
                snap.value,
                &case.input.source,
                Some(&case.input.subsector),
                chrono::Utc::now(),
            )
            .await
            .unwrap();
        }

        // Jalankan jalur verifikasi yang sama dengan POST /findings (via HTTP).
        let body = serde_json::to_value(&case.input).unwrap();
        let req = Request::builder()
            .method("POST")
            .uri("/findings")
            .header("content-type", "application/json")
            .body(Body::from(body.to_string()))
            .unwrap();
        let res = app.clone().oneshot(req).await.unwrap();
        assert!(
            res.status().is_success(),
            "kasus {}: HTTP harus sukses",
            case.case_id
        );
        let bytes = axum::body::to_bytes(res.into_body(), 1 << 20)
            .await
            .unwrap();
        let resp: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        let status = resp
            .get("status")
            .and_then(|v| v.as_str())
            .unwrap_or("<missing>");

        assert_eq!(
            status, case.expected.status,
            "kasus {}: expected {}, dapat {} (body: {})",
            case.case_id, case.expected.status, status, resp
        );

        // Penegasan spesifik per skenario reject.
        if case.case_id == "case_004" {
            let reason = resp
                .get("rejection_reason")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            assert!(
                reason.contains("kata terlarang"),
                "case_004 harus ditolak karena compliance, dapat: {reason}"
            );
        }
        if case.case_id == "case_005" {
            let reason = resp
                .get("rejection_reason")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            assert!(
                reason.contains("tidak cocok"),
                "case_005 harus ditolak karena evidence, dapat: {reason}"
            );
        }

        cleanup_case(&state.pool, &case.input.ticker).await;
    }
}

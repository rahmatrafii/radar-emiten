//! Tes Tahap 7: gateway MCP — mock perilaku, proteksi 410, dan stub real.

use orchestrator::integrations::mcp_gateway::{
    McpError, McpGateway, MockMcpGateway, RealMcpGateway, ScreenParams,
};

fn mock_config() -> orchestrator::config::Config {
    // Config literal untuk mode real; transport sengaja belum diisi.
    orchestrator::config::Config {
        app_mode: orchestrator::config::AppMode::Real,
        policy_disclaimer_conflict_acknowledged: false,
        ..Default::default()
    }
}

#[tokio::test]
async fn mock_list_subsectors_ok() {
    let gw = MockMcpGateway::new();
    let subs = gw.list_subsectors().await.expect("list ok");
    assert!(subs.iter().any(|s| s == "banks"));
}

#[tokio::test]
async fn mock_screen_companies_filter() {
    let gw = MockMcpGateway::new();
    let res = gw
        .screen_companies(ScreenParams {
            subsector: Some("banks".into()),
        })
        .await
        .expect("screen ok");
    assert!(res.tickers.contains(&"BBCA".to_string()));
}

#[tokio::test]
async fn mock_company_evidence_valid() {
    let gw = MockMcpGateway::new();
    let ev = gw
        .get_company_evidence("BBCA".into(), None)
        .await
        .expect("evidence ok");
    assert_eq!(ev.ticker, "BBCA");
    assert_eq!(ev.period, "Q3 2024");
    assert!(ev.value.is_some());
}

#[tokio::test]
async fn mock_previous_snapshot_cold_start() {
    let gw = MockMcpGateway::new();
    let prev = gw
        .get_previous_snapshot("BBCA".into(), "net_profit_margin".into())
        .await
        .expect("ok");
    assert!(prev.is_some());
    // ticker lain tanpa fixture => None (baseline/cold-start), bukan panic.
    let none = gw
        .get_previous_snapshot("TLKM".into(), "net_profit_margin".into())
        .await
        .expect("ok");
    assert!(none.is_none());
}

#[tokio::test]
async fn mock_deprecated_returns_410_error() {
    let gw = MockMcpGateway::deprecated();
    let err = gw
        .get_company_evidence("BBCA".into(), None)
        .await
        .expect_err("harus error");
    assert_eq!(err, McpError::ApiVersionDeprecated);
    let err2 = gw.list_subsectors().await.expect_err("harus error");
    assert_eq!(err2, McpError::ApiVersionDeprecated);
}

#[tokio::test]
async fn real_gateway_behavior() {
    let cfg = mock_config();
    let gw = RealMcpGateway::new(&cfg);
    match gw.get_company_evidence("BBCA".into(), None).await {
        Ok(ev) => {
            assert_eq!(ev.ticker, "BBCA");
            assert!(ev.value.is_some());
        }
        Err(McpError::TransportUnavailable(_)) => {}
        Err(other) => panic!("ekspektasi sukses atau TransportUnavailable, dapat {other}"),
    }
}

#[tokio::test]
async fn mock_record_finding_rejects_missing_ticker() {
    let gw = MockMcpGateway::new();
    let err = gw
        .record_finding(serde_json::json!({"metric_name": "x"}))
        .await
        .expect_err("harus ditolak");
    assert!(matches!(err, McpError::InvalidResponse(_)));
    let ok = gw
        .record_finding(serde_json::json!({"ticker": "BBCA"}))
        .await
        .expect("ok");
    assert_eq!(ok, "mock-finding-id");
}

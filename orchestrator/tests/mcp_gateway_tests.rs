//! Tes Tahap 7: gateway MCP — mock perilaku, proteksi 410, dan stub real.

use orchestrator::integrations::mcp_gateway::{
    McpError, McpGateway, MockMcpGateway, RealMcpGateway, ScreenParams,
};

fn mock_config() -> orchestrator::config::Config {
    // Config literal untuk mode real; transport sengaja belum diisi.
    orchestrator::config::Config {
        app_mode: orchestrator::config::AppMode::Real,
        host: "127.0.0.1".into(),
        port: 8080,
        database_url: String::new(),
        gemini_api_key: None,
        gemini_model: None,
        whatsapp_access_token: None,
        whatsapp_phone_number_id: None,
        whatsapp_app_secret: None,
        whatsapp_verify_token: None,
        whatsapp_graph_api_version: None,
        whatsapp_template_name: None,
        whatsapp_template_language: None,
        admin_phone: None,
        internal_api_token: None,
        sectors_credit_budget: 1000.0,
        max_alerts_per_day: 3,
        neutral_relative_threshold: 0.02,
        scheduler_interval_seconds: 86400,
        mcp_transport: None,
        mcp_server_command: None,
        mcp_server_args: None,
        mcp_callback_base_url: None,
        policy_disclaimer_conflict_acknowledged: false,
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
async fn real_gateway_stub_unavailable() {
    let cfg = mock_config();
    let gw = RealMcpGateway::new(&cfg);
    let err = gw
        .get_company_evidence("BBCA".into(), None)
        .await
        .expect_err("stub selalu unavailable");
    match err {
        McpError::TransportUnavailable(_) => {}
        other => panic!("ekspektasi TransportUnavailable, dapat {other}"),
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

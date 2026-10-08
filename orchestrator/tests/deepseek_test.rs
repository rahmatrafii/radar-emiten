use orchestrator::config::{AppMode, Config};
use orchestrator::services::gemini_client::GeminiClient;

#[tokio::test]
async fn test_deepseek_live_extraction() {
    dotenvy::dotenv().ok();
    let config = match Config::from_env() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Config load skipped in test: {e}");
            return;
        }
    };

    if config.deepseek_api_key.is_none() {
        println!("Skipping live DeepSeek test: DEEPSEEK_API_KEY not set");
        return;
    }

    let mut live_config = config.clone();
    live_config.app_mode = AppMode::Real;
    live_config.llm_provider = "deepseek".to_string();

    let client = GeminiClient::new(&live_config);
    let prompt = "Saya perkirakan BBCA net_profit_margin akan naik kuartal ini";
    let result = client.extract_thesis(prompt).await;

    println!("Live DeepSeek result: {:?}", result);
    let extracted = result.expect("DeepSeek extraction must succeed with valid API key");
    assert_eq!(extracted.ticker, "BBCA");
    assert_eq!(extracted.metrics[0].metric_name, "net_profit_margin");
    assert_eq!(extracted.metrics[0].desired_direction, "increase");
}

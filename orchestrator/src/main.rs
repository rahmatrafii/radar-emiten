use orchestrator::config::Config;
use orchestrator::routes;
use orchestrator::state::AppState;
use sqlx::postgres::PgPoolOptions;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .init();

    let config = Config::from_env().unwrap_or_else(|e| {
        eprintln!("Konfigurasi tidak valid: {e}");
        std::process::exit(1);
    });

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&config.database_url)
        .await
        .expect("Gagal konek ke PostgreSQL");

    let state = AppState::new(pool, config.clone());
    let app = routes::routes().with_state(state);

    let addr = format!("{}:{}", config.host, config.port);
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    tracing::info!(
        "Server jalan di http://{} (mode {:?})",
        addr,
        config.app_mode
    );

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .unwrap();
}

async fn shutdown_signal() {
    let _ = tokio::signal::ctrl_c().await;
    tracing::info!("Menerima sinyal shutdown, menutup server...");
}

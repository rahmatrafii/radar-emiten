use axum::{extract::State, http::StatusCode, routing::get, Json, Router};
use serde_json::{json, Value};
use sqlx::PgPool;

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

    let database_url =
        std::env::var("DATABASE_URL").expect("DATABASE_URL harus diset di environment");
    let pool = PgPool::connect(&database_url)
        .await
        .expect("Gagal konek ke PostgreSQL");

    let app = Router::new()
        .route("/health", get(health))
        .with_state(pool);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:8080")
        .await
        .unwrap();
    println!("Server jalan di http://localhost:8080");

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .unwrap();
}

async fn health(State(pool): State<PgPool>) -> (StatusCode, Json<Value>) {
    match sqlx::query("SELECT 1").execute(&pool).await {
        Ok(_) => (StatusCode::OK, Json(json!({"status": "ok", "database": "ok"}))),
        Err(_) => (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({"status": "error", "database": "unavailable"})),
        ),
    }
}

async fn shutdown_signal() {
    let _ = tokio::signal::ctrl_c().await;
    println!("Menerima sinyal shutdown, menutup server...");
}

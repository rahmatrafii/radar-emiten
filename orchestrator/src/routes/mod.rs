pub mod health;

use axum::Router;

use crate::state::AppState;

pub fn routes() -> Router<AppState> {
    Router::new().route("/health", axum::routing::get(health::health))
}

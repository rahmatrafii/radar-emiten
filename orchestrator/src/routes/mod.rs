pub mod health;

use axum::Router;

use crate::state::AppState;

pub(crate) mod credits;
pub(crate) mod dashboard;
pub(crate) mod findings;
pub(crate) mod internal;
pub(crate) mod snapshots;
pub(crate) mod traces;
pub(crate) mod watchlist;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/health", axum::routing::get(health::health))
        .route(
            "/snapshots",
            axum::routing::post(snapshots::post_snapshot).get(snapshots::get_snapshots),
        )
        .route("/findings", axum::routing::post(findings::post_finding))
        .route("/jejak", axum::routing::post(traces::post_trace))
        .route("/kredit", axum::routing::post(credits::post_credit))
        .route("/pantauan", axum::routing::get(watchlist::get_watchlist))
        .route(
            "/internal/snapshots/previous",
            axum::routing::get(internal::internal_previous_snapshot),
        )
        .route(
            "/internal/findings",
            axum::routing::post(internal::internal_record_finding),
        )
        .route("/api/tesis", axum::routing::get(dashboard::api_tesis))
        .route("/api/findings", axum::routing::get(dashboard::api_findings))
        .route("/api/ditolak", axum::routing::get(dashboard::api_rejected))
        .route("/api/jejak", axum::routing::get(dashboard::api_traces))
        .route("/api/kredit", axum::routing::get(dashboard::api_credits))
}

//! Router assembly: the `/api` JSON routes plus an optional SPA fallback.

use std::sync::Arc;

use axum::routing::{get, post};
use axum::Router;
use tower_http::services::{ServeDir, ServeFile};

use studio_core::Workspace;

use crate::config::Config;
use crate::routes;

/// Shared handler state.
#[derive(Clone)]
pub struct AppState {
    pub ws: Arc<Workspace>,
}

/// Build the application router for a given config.
pub fn router(config: &Config) -> Router {
    let state = AppState {
        ws: Arc::new(Workspace::new(&config.data_dir)),
    };
    let api = Router::new()
        .route(
            "/policies",
            get(routes::policies::list).post(routes::policies::create),
        )
        .route("/policies/{id}", get(routes::policies::info))
        .route("/policies/{id}/verify", get(routes::policies::verify))
        .route("/policies/{id}/rules", get(routes::policies::rules))
        .route("/policies/{id}/diff", post(routes::policies::diff))
        .route("/policies/{id}/versions", post(routes::policies::commit))
        .route("/policies/{id}/simulate", post(routes::policies::simulate))
        .route("/policies/{id}/audit", get(routes::policies::audit))
        .route(
            "/policies/{id}/compliance",
            get(routes::policies::compliance),
        )
        .route("/policies/{id}/export", get(routes::policies::export))
        .route(
            "/policies/{id}/multisig",
            get(routes::policies::multisig).put(routes::policies::set_multisig),
        )
        .route(
            "/policies/{id}/multisig/approve",
            post(routes::policies::approve),
        )
        .route("/registry", get(routes::registry::list))
        .route("/registry/register", post(routes::registry::register))
        .route("/registry/export", get(routes::registry::export))
        .route("/registry/{author}/rotate", post(routes::registry::rotate))
        .route("/registry/{author}/revoke", post(routes::registry::revoke))
        .route("/copilot/stream", post(routes::copilot::stream))
        .with_state(state);

    let app = Router::new().nest("/api", api);

    // Serve the built SPA if it exists, with SPA fallback to index.html for client-side routing.
    if config.web_dist.exists() {
        let index = config.web_dist.join("index.html");
        let spa = ServeDir::new(&config.web_dist).fallback(ServeFile::new(index));
        app.fallback_service(spa)
    } else {
        app
    }
}

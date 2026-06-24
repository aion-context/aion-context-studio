//! aion-studio-api — thin axum wiring over `studio-core`. Serves the JSON API under `/api` and
//! the built SPA (if present) as a fallback. Loopback-only; local single-operator demo.
//!
//! Embeddable: [`serve`] binds from a [`Config`] and runs until shutdown, and [`serve_on`] runs on
//! an already-bound listener — so a desktop/Tauri shell can start the studio on a background task
//! (and learn the actual port when binding to port 0).

#![forbid(unsafe_code)]
#![cfg_attr(not(test), warn(clippy::unwrap_used, clippy::expect_used))]

pub mod app;
pub mod config;
pub mod error;
pub mod routes;

use std::net::SocketAddr;

use tokio::net::TcpListener;

use studio_core::{seed, Workspace};

use crate::config::Config;

/// A boxed, thread-safe error — `Send + Sync` so the serving future can be `tokio::spawn`ed.
pub type BoxError = Box<dyn std::error::Error + Send + Sync>;

/// Seed the workspace and serve the API + SPA, binding from `config`. Runs until shutdown.
pub async fn serve(config: Config) -> Result<(), BoxError> {
    let listener = TcpListener::bind(SocketAddr::from((config.host, config.port))).await?;
    println!(
        "aion-context-studio → http://{}  (local single-operator; Ctrl-C to stop)",
        listener.local_addr()?
    );
    serve_on(listener, &config).await
}

/// Seed the workspace and serve on an already-bound listener. Lets an embedder bind first (e.g. to
/// port 0) and read `listener.local_addr()` before handing the listener over.
pub async fn serve_on(listener: TcpListener, config: &Config) -> Result<(), BoxError> {
    let ws = Workspace::new(&config.data_dir);
    seed::ensure_seeded(&ws)?;
    axum::serve(listener, app::router(config)).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::Ipv4Addr;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU32, Ordering};

    fn test_config() -> (Config, PathBuf) {
        static N: AtomicU32 = AtomicU32::new(0);
        let n = N.fetch_add(1, Ordering::Relaxed);
        let data_dir = std::env::temp_dir().join(format!("api-serve-{}-{n}", std::process::id()));
        let config = Config {
            host: Ipv4Addr::LOCALHOST,
            port: 0,
            data_dir: data_dir.clone(),
            web_dist: data_dir.join("no-spa"), // absent → API-only, no SPA fallback
        };
        (config, data_dir)
    }

    #[tokio::test]
    async fn serve_on_boots_seeds_and_answers_the_api() {
        let (config, data_dir) = test_config();
        let listener = TcpListener::bind((config.host, 0)).await.unwrap();
        let addr = listener.local_addr().unwrap();
        let handle = tokio::spawn(async move { serve_on(listener, &config).await });

        // hit the live server (ureq is blocking → off the async runtime)
        let url = format!("http://{addr}/api/policies");
        let body = tokio::task::spawn_blocking(move || {
            for _ in 0..50 {
                if let Ok(r) = ureq::get(&url).call() {
                    return r.into_string().unwrap_or_default();
                }
                std::thread::sleep(std::time::Duration::from_millis(20));
            }
            String::new()
        })
        .await
        .unwrap();

        // seeding ran (the sample policy is present) and the router answered
        assert!(
            body.contains("refund-authorization"),
            "expected the seeded policy in {body:?}"
        );

        handle.abort();
        let _ = std::fs::remove_dir_all(&data_dir);
    }
}

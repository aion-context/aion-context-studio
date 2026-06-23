//! aion-context-studio server entry point. Seeds the workspace, then serves the API + SPA on
//! loopback.

use std::net::SocketAddr;

use aion_studio_api::{app, config::Config};
use studio_core::{seed, Workspace};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_env("STUDIO_LOG")
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let config = Config::from_env();
    let ws = Workspace::new(&config.data_dir);
    seed::ensure_seeded(&ws)?;

    let app = app::router(&config);
    let addr = SocketAddr::from((config.host, config.port));
    let listener = tokio::net::TcpListener::bind(addr).await?;
    println!("aion-context-studio → http://{addr}  (local single-operator; Ctrl-C to stop)");
    axum::serve(listener, app).await?;
    Ok(())
}

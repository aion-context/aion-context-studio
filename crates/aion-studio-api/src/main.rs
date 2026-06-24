//! aion-context-studio server entry point — sets up logging and serves the API + SPA on loopback.
//! The serving logic lives in the library ([`aion_studio_api::serve`]) so a desktop shell can embed it.

use aion_studio_api::{config::Config, serve, BoxError};

#[tokio::main]
async fn main() -> Result<(), BoxError> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_env("STUDIO_LOG")
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    serve(Config::from_env()).await
}

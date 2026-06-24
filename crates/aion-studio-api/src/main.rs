//! aion-context-studio server entry point — sets up logging and serves the API + SPA on loopback.
//! The serving logic lives in the library ([`aion_studio_api::serve`]) so a desktop shell can embed it.
//!
//! Subcommand: `import-keys` migrates the operator's keys from the file vault into the OS keyring
//! (the first-run step for switching an existing workspace to keyring custody).

use aion_studio_api::{config::Config, serve, BoxError};
use studio_core::Workspace;

#[tokio::main]
async fn main() -> Result<(), BoxError> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_env("STUDIO_LOG")
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    match std::env::args().nth(1).as_deref() {
        Some("import-keys") => import_keys(),
        _ => serve(Config::from_env()).await,
    }
}

/// Validate, then (with the `keyring` feature) migrate operator keys from the file vault into the OS
/// keyring. Without the feature it reports the plan only.
fn import_keys() -> Result<(), BoxError> {
    let ws = Workspace::new(Config::from_env().data_dir);
    let plan = studio_core::custody::plan_import(&ws)?;
    let authors: Vec<u64> = plan.iter().map(|i| i.author).collect();
    println!(
        "validated {} author(s) for keyring import: {authors:?}",
        plan.len()
    );

    #[cfg(feature = "keyring")]
    {
        let imported = studio_core::custody::import_to_keyring(&ws)?;
        println!("imported {} author(s) into the OS keyring", imported.len());
    }
    #[cfg(not(feature = "keyring"))]
    {
        println!(
            "(plan only) rebuild with `--features keyring` to write the keys into the OS keyring"
        );
    }
    Ok(())
}

//! The crate's typed error. Library code returns `Result`; it never panics.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum StudioError {
    /// No policy with this id exists in the workspace.
    #[error("policy not found: {0}")]
    NotFound(String),

    /// A policy id failed validation (must be `[a-z0-9-]`, ≤64 chars) — guards path traversal.
    #[error("invalid policy id: {0}")]
    InvalidId(String),

    /// A policy with this id already exists (create would overwrite).
    #[error("policy already exists: {0}")]
    AlreadyExists(String),

    /// A signing key could not be loaded or decoded.
    #[error("signing key: {0}")]
    BadKey(String),

    /// A request was well-formed but semantically invalid (e.g. a bad M-of-N, an unknown signer).
    #[error("{0}")]
    Invalid(String),

    /// Filesystem error.
    #[error("io: {0}")]
    Io(#[from] std::io::Error),

    /// JSON (de)serialization error.
    #[error("json: {0}")]
    Json(#[from] serde_json::Error),

    /// An error from the aion-context library.
    #[error(transparent)]
    Aion(#[from] aion_context::AionError),
}

pub type Result<T> = std::result::Result<T, StudioError>;

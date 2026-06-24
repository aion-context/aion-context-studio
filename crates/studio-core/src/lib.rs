//! studio-core — the testable glue between `aion-context` and the aion-context-studio API.
//!
//! It owns the workspace layout, reads/verifies `.aion` policies through aion-context's own
//! `verify_file`, persists the public key registry, and seeds a sample policy on first run. The
//! HTTP layer (aion-studio-api) is thin wiring over this crate.

#![forbid(unsafe_code)]
#![cfg_attr(not(test), warn(clippy::unwrap_used, clippy::expect_used))]

pub mod audit;
pub mod author_index;
pub mod authoring;
pub mod copilot;
pub mod custody;
pub mod diff;
pub mod error;
pub mod evaluate;
pub mod keystore;
pub mod multisig_store;
pub mod policies;
pub mod registry_admin;
pub mod registry_store;
pub mod seed;
pub mod workspace;

pub use error::{Result, StudioError};
pub use policies::PolicySummary;
pub use workspace::{PolicyId, Workspace};

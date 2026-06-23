//! aion-studio-api — thin axum wiring over `studio-core`. Serves the JSON API under `/api` and
//! the built SPA (if present) as a fallback. Loopback-only; local single-operator demo.

#![forbid(unsafe_code)]
#![cfg_attr(not(test), warn(clippy::unwrap_used, clippy::expect_used))]

pub mod app;
pub mod config;
pub mod error;
pub mod routes;

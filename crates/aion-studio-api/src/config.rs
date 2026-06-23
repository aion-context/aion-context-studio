//! Runtime configuration, from env with sensible defaults. The bind host is always loopback.

use std::net::Ipv4Addr;
use std::path::PathBuf;

/// Studio server configuration.
pub struct Config {
    /// Always `127.0.0.1` — this demo must never be reachable off the operator's machine.
    pub host: Ipv4Addr,
    pub port: u16,
    /// Workspace data directory (`.aion` files, registry, keys).
    pub data_dir: PathBuf,
    /// Built SPA directory; served as the fallback when it exists.
    pub web_dist: PathBuf,
}

impl Config {
    pub fn from_env() -> Self {
        let port = std::env::var("STUDIO_PORT")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(8787);
        let data_dir = std::env::var("STUDIO_DATA")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("studio-data"));
        let web_dist = std::env::var("STUDIO_WEB")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("web/dist"));
        Self {
            host: Ipv4Addr::LOCALHOST,
            port,
            data_dir,
            web_dist,
        }
    }
}

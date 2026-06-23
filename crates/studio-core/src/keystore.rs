//! The local-demo keystore: operational signing keys persisted as hex under `keys/`.
//!
//! LOCAL SINGLE-OPERATOR DEMO ONLY — NOT a production custody architecture. In production the
//! signing key lives with the party that holds it (HSM, OS keyring, a signing service); the studio
//! would call out to that, never read a secret off disk. See README.

use aion_context::crypto::SigningKey;
use aion_context::types::AuthorId;

use crate::error::{Result, StudioError};
use crate::workspace::Workspace;

/// Persist an author's operational signing key (hex). Overwrites.
pub fn save_signing_key(ws: &Workspace, author: AuthorId, key: &SigningKey) -> Result<()> {
    write_key(ws, author, "", key)
}

/// Load an author's operational signing key.
pub fn load_signing_key(ws: &Workspace, author: AuthorId) -> Result<SigningKey> {
    read_key(ws, author, "")
}

/// Persist an author's master key (authorizes rotation/revocation). Overwrites.
pub fn save_master_key(ws: &Workspace, author: AuthorId, key: &SigningKey) -> Result<()> {
    write_key(ws, author, ".master", key)
}

/// Load an author's master key.
pub fn load_master_key(ws: &Workspace, author: AuthorId) -> Result<SigningKey> {
    read_key(ws, author, ".master")
}

fn write_key(ws: &Workspace, author: AuthorId, kind: &str, key: &SigningKey) -> Result<()> {
    ws.ensure_dirs()?;
    std::fs::write(key_path(ws, author, kind), hex_encode(key.to_bytes()))?;
    Ok(())
}

fn read_key(ws: &Workspace, author: AuthorId, kind: &str) -> Result<SigningKey> {
    let hex = std::fs::read_to_string(key_path(ws, author, kind))
        .map_err(|_| StudioError::BadKey(format!("no {kind} key for author {}", author.0)))?;
    let bytes = hex_decode(hex.trim())
        .ok_or_else(|| StudioError::BadKey(format!("malformed key for author {}", author.0)))?;
    Ok(SigningKey::from_bytes(&bytes)?)
}

fn key_path(ws: &Workspace, author: AuthorId, kind: &str) -> std::path::PathBuf {
    ws.keys_dir().join(format!("author-{}{kind}.key", author.0))
}

pub(crate) fn hex_encode(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        s.push(char::from_digit((b >> 4) as u32, 16).unwrap_or('0'));
        s.push(char::from_digit((b & 0xf) as u32, 16).unwrap_or('0'));
    }
    s
}

fn hex_decode(s: &str) -> Option<Vec<u8>> {
    if s.len() % 2 != 0 {
        return None;
    }
    let mut out = Vec::with_capacity(s.len() / 2);
    let b = s.as_bytes();
    let mut i = 0;
    while i < b.len() {
        let hi = (b[i] as char).to_digit(16)?;
        let lo = (b[i + 1] as char).to_digit(16)?;
        out.push((hi * 16 + lo) as u8);
        i += 2;
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};

    #[test]
    fn hex_round_trips() {
        let bytes = [0x00u8, 0x0f, 0xa5, 0xff, 0x10];
        assert_eq!(hex_encode(&bytes), "000fa5ff10");
        assert_eq!(hex_decode("000fa5ff10").unwrap(), bytes);
        assert!(hex_decode("0g").is_none()); // non-hex digit
        assert!(hex_decode("abc").is_none()); // odd length
    }

    #[test]
    fn save_then_load_recovers_the_same_key() {
        static N: AtomicU32 = AtomicU32::new(0);
        let n = N.fetch_add(1, Ordering::Relaxed);
        let ws =
            Workspace::new(std::env::temp_dir().join(format!("ks-{}-{n}", std::process::id())));
        let author = AuthorId::new(7);
        let key = SigningKey::generate();
        save_signing_key(&ws, author, &key).unwrap();
        let loaded = load_signing_key(&ws, author).unwrap();
        assert_eq!(loaded.to_bytes(), key.to_bytes());
        assert!(load_signing_key(&ws, AuthorId::new(99)).is_err()); // missing
        let _ = std::fs::remove_dir_all(ws.root());
    }
}

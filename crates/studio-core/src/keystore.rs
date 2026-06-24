//! Custody — how operational and master signing keys are stored and retrieved.
//!
//! A [`KeyVault`] abstracts custody so the rest of the studio never assumes *where* a key lives.
//! [`FileVault`] is the local single-operator demo store (hex under `keys/`) — NOT a production
//! custody architecture. [`KeyringVault`] (feature `keyring`) keeps secrets in the OS keyring; it is
//! the path a desktop/Tauri build uses to retire the on-disk keystore. The active vault is selected
//! at runtime by the `STUDIO_CUSTODY` environment variable (`file` — the default — or `keyring`).
//!
//! The public `save_*`/`load_*` functions are the stable surface the rest of the crate calls; they
//! dispatch to the selected vault.

use aion_context::crypto::SigningKey;
use aion_context::types::AuthorId;

use crate::error::{Result, StudioError};
use crate::workspace::Workspace;

/// A custody backend: persists and recovers an author's signing keys by a `kind` suffix
/// (`""` for the operational key, `".master"` for the rotation/revocation key).
pub(crate) trait KeyVault {
    fn store(&self, author: AuthorId, kind: &str, key: &SigningKey) -> Result<()>;
    fn load(&self, author: AuthorId, kind: &str) -> Result<SigningKey>;
}

/// Persist an author's operational signing key. Overwrites.
pub fn save_signing_key(ws: &Workspace, author: AuthorId, key: &SigningKey) -> Result<()> {
    vault(ws).store(author, "", key)
}

/// Load an author's operational signing key.
pub fn load_signing_key(ws: &Workspace, author: AuthorId) -> Result<SigningKey> {
    vault(ws).load(author, "")
}

/// Persist an author's master key (authorizes rotation/revocation). Overwrites.
pub fn save_master_key(ws: &Workspace, author: AuthorId, key: &SigningKey) -> Result<()> {
    vault(ws).store(author, ".master", key)
}

/// Load an author's master key.
pub fn load_master_key(ws: &Workspace, author: AuthorId) -> Result<SigningKey> {
    vault(ws).load(author, ".master")
}

/// Select the custody backend. Defaults to the file vault; `STUDIO_CUSTODY=keyring` uses the OS
/// keyring when the crate is built with the `keyring` feature.
fn vault(ws: &Workspace) -> Box<dyn KeyVault + '_> {
    #[cfg(feature = "keyring")]
    if matches!(std::env::var("STUDIO_CUSTODY").as_deref(), Ok("keyring")) {
        return Box::new(KeyringVault);
    }
    Box::new(FileVault { ws })
}

/// The local-demo file store: each key written as hex under `keys/author-N[.master].key`.
struct FileVault<'a> {
    ws: &'a Workspace,
}

impl KeyVault for FileVault<'_> {
    fn store(&self, author: AuthorId, kind: &str, key: &SigningKey) -> Result<()> {
        self.ws.ensure_dirs()?;
        std::fs::write(self.path(author, kind), hex_encode(key.to_bytes()))?;
        Ok(())
    }

    fn load(&self, author: AuthorId, kind: &str) -> Result<SigningKey> {
        let hex = std::fs::read_to_string(self.path(author, kind))
            .map_err(|_| StudioError::BadKey(format!("no {kind} key for author {}", author.0)))?;
        let bytes = hex_decode(hex.trim())
            .ok_or_else(|| StudioError::BadKey(format!("malformed key for author {}", author.0)))?;
        Ok(SigningKey::from_bytes(&bytes)?)
    }
}

impl FileVault<'_> {
    fn path(&self, author: AuthorId, kind: &str) -> std::path::PathBuf {
        self.ws
            .keys_dir()
            .join(format!("author-{}{kind}.key", author.0))
    }
}

/// OS-keyring custody. Secrets live in the platform credential store, never on disk — the production
/// custody direction, used by a desktop/Tauri build. At runtime a platform store feature (or, in
/// tests, the in-memory mock) must be active for the keyring crate to resolve an entry.
#[cfg(feature = "keyring")]
struct KeyringVault;

#[cfg(feature = "keyring")]
impl KeyringVault {
    const SERVICE: &'static str = "aion-context-studio";
    fn account(author: AuthorId, kind: &str) -> String {
        format!("author-{}{kind}", author.0)
    }
    fn entry(author: AuthorId, kind: &str) -> Result<keyring::Entry> {
        keyring::Entry::new(Self::SERVICE, &Self::account(author, kind))
            .map_err(|e| StudioError::BadKey(format!("keyring: {e}")))
    }
}

#[cfg(feature = "keyring")]
impl KeyVault for KeyringVault {
    fn store(&self, author: AuthorId, kind: &str, key: &SigningKey) -> Result<()> {
        Self::entry(author, kind)?
            .set_password(&hex_encode(key.to_bytes()))
            .map_err(|e| StudioError::BadKey(format!("keyring store: {e}")))
    }

    fn load(&self, author: AuthorId, kind: &str) -> Result<SigningKey> {
        let hex = Self::entry(author, kind)?
            .get_password()
            .map_err(|_| StudioError::BadKey(format!("no {kind} key for author {}", author.0)))?;
        let bytes = hex_decode(hex.trim())
            .ok_or_else(|| StudioError::BadKey(format!("malformed key for author {}", author.0)))?;
        Ok(SigningKey::from_bytes(&bytes)?)
    }
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

    fn temp_ws(tag: &str) -> Workspace {
        static N: AtomicU32 = AtomicU32::new(0);
        let n = N.fetch_add(1, Ordering::Relaxed);
        Workspace::new(std::env::temp_dir().join(format!("{tag}-{}-{n}", std::process::id())))
    }

    #[test]
    fn hex_round_trips() {
        let bytes = [0x00u8, 0x0f, 0xa5, 0xff, 0x10];
        assert_eq!(hex_encode(&bytes), "000fa5ff10");
        assert_eq!(hex_decode("000fa5ff10").unwrap(), bytes);
        assert!(hex_decode("0g").is_none()); // non-hex digit
        assert!(hex_decode("abc").is_none()); // odd length
    }

    #[test]
    fn file_vault_save_then_load_recovers_the_same_key() {
        let ws = temp_ws("ks");
        let author = AuthorId::new(7);
        let key = SigningKey::generate();
        save_signing_key(&ws, author, &key).unwrap();
        let loaded = load_signing_key(&ws, author).unwrap();
        assert_eq!(loaded.to_bytes(), key.to_bytes());
        assert!(load_signing_key(&ws, AuthorId::new(99)).is_err()); // missing
        let _ = std::fs::remove_dir_all(ws.root());
    }

    #[test]
    fn file_vault_keeps_signing_and_master_separate() {
        let ws = temp_ws("ks");
        let author = AuthorId::new(3);
        let signing = SigningKey::generate();
        let master = SigningKey::generate();
        save_signing_key(&ws, author, &signing).unwrap();
        save_master_key(&ws, author, &master).unwrap();
        assert_eq!(
            load_signing_key(&ws, author).unwrap().to_bytes(),
            signing.to_bytes()
        );
        assert_eq!(
            load_master_key(&ws, author).unwrap().to_bytes(),
            master.to_bytes()
        );
        // distinct slots — the master key is not the signing key
        assert_ne!(signing.to_bytes(), master.to_bytes());
        let _ = std::fs::remove_dir_all(ws.root());
    }

    // The keyring vault wraps the platform credential store. Headless we have only the mock, whose
    // builder hands each `Entry` a fresh in-memory credential — so it is not a shared keystore and
    // cannot round-trip across the vault's separate store/load entries (that is the real store's job,
    // and the keyring crate's own test surface). We assert what this module owns: account naming,
    // that the store path succeeds, and that a missing key is a clean error rather than a panic.
    #[cfg(feature = "keyring")]
    #[test]
    fn keyring_vault_naming_store_and_missing_key() {
        keyring::set_default_credential_builder(keyring::mock::default_credential_builder());
        assert_eq!(KeyringVault::account(AuthorId::new(11), ""), "author-11");
        assert_eq!(
            KeyringVault::account(AuthorId::new(2), ".master"),
            "author-2.master"
        );
        let v = KeyringVault;
        v.store(AuthorId::new(11), "", &SigningKey::generate())
            .unwrap(); // store wiring ok
        assert!(matches!(
            v.load(AuthorId::new(404), ""),
            Err(StudioError::BadKey(_))
        ));
    }
}

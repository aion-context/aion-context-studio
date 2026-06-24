//! First-run custody migration: move operator keys from the file vault into the OS keyring, so a
//! workspace created under the demo's file custody can switch to keyring custody without breaking
//! signatures.
//!
//! A key is migrated only after it is validated against the registry — an author's on-disk
//! operational key must match its registered *active* public key, or the import refuses (importing a
//! mismatched private key would silently produce versions that don't verify). The planning step is
//! always available and exercised in tests; the keyring write is feature-gated.

use aion_context::types::AuthorId;
use serde::Serialize;

use crate::error::{Result, StudioError};
use crate::workspace::Workspace;
use crate::{author_index, keystore, registry_store};

/// One author's importable keys.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ImportItem {
    pub author: u64,
    pub has_master: bool,
}

/// Validate which indexed authors can be migrated. Each author's on-disk operational key must match
/// its registered active public key; the first mismatch is an error. Returns the importable set.
pub fn plan_import(ws: &Workspace) -> Result<Vec<ImportItem>> {
    let registry = registry_store::load(ws)?;
    let mut items = Vec::new();
    for id in author_index::list(ws)? {
        let author = AuthorId::new(id);
        let on_disk = keystore::file_public_key(ws, author, "")?;
        let registered = registry.epochs_for(author).last().map(|e| e.public_key);
        if registered != Some(on_disk) {
            return Err(StudioError::BadKey(format!(
                "author {id}: on-disk key does not match the registry — refusing to import"
            )));
        }
        let has_master = keystore::file_public_key(ws, author, ".master").is_ok();
        items.push(ImportItem {
            author: id,
            has_master,
        });
    }
    Ok(items)
}

/// Import every validated author's keys from the file vault into the OS keyring. Returns the author
/// ids imported. Requires the `keyring` feature.
#[cfg(feature = "keyring")]
pub fn import_to_keyring(ws: &Workspace) -> Result<Vec<u64>> {
    let plan = plan_import(ws)?;
    for item in &plan {
        let author = AuthorId::new(item.author);
        keystore::copy_file_key_to_keyring(ws, author, "")?;
        if item.has_master {
            keystore::copy_file_key_to_keyring(ws, author, ".master")?;
        }
    }
    Ok(plan.iter().map(|i| i.author).collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{registry_admin, seed};
    use aion_context::crypto::SigningKey;
    use std::sync::atomic::{AtomicU32, Ordering};

    fn seeded_ws() -> Workspace {
        static N: AtomicU32 = AtomicU32::new(0);
        let n = N.fetch_add(1, Ordering::Relaxed);
        let ws =
            Workspace::new(std::env::temp_dir().join(format!("cust-{}-{n}", std::process::id())));
        seed::ensure_seeded(&ws).unwrap();
        ws
    }

    #[test]
    fn plan_import_lists_validated_seeded_authors() {
        let ws = seeded_ws();
        let plan = plan_import(&ws).unwrap();
        assert_eq!(
            plan,
            vec![
                ImportItem {
                    author: 1,
                    has_master: true
                },
                ImportItem {
                    author: 2,
                    has_master: true
                },
                ImportItem {
                    author: 3,
                    has_master: true
                },
            ]
        );
        let _ = std::fs::remove_dir_all(ws.root());
    }

    #[test]
    fn plan_import_validates_against_the_active_epoch_after_rotation() {
        // After a rotation the on-disk key is the new (active/last) epoch's key, not epoch 0 — so the
        // plan must compare against the active epoch, and still succeed.
        let ws = seeded_ws();
        registry_admin::rotate(&ws, 2).unwrap();
        assert!(
            plan_import(&ws).is_ok(),
            "rotated author should still validate"
        );
        let _ = std::fs::remove_dir_all(ws.root());
    }

    #[test]
    fn plan_import_refuses_an_on_disk_key_that_does_not_match_the_registry() {
        let ws = seeded_ws();
        // overwrite author 1's on-disk operational key with an unrelated one (file custody default)
        keystore::save_signing_key(&ws, AuthorId::new(1), &SigningKey::generate()).unwrap();
        assert!(matches!(plan_import(&ws), Err(StudioError::BadKey(_))));
        let _ = std::fs::remove_dir_all(ws.root());
    }

    #[cfg(feature = "keyring")]
    #[test]
    fn import_to_keyring_migrates_indexed_authors() {
        keyring::set_default_credential_builder(keyring::mock::default_credential_builder());
        let ws = seeded_ws();
        assert_eq!(import_to_keyring(&ws).unwrap(), vec![1, 2, 3]);
        let _ = std::fs::remove_dir_all(ws.root());
    }
}

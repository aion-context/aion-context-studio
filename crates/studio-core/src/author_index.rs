//! The custody-agnostic author index: which author ids are registered, independent of where their
//! keys live. Author enumeration must not assume the file vault (see `docs/CUSTODY.md`) — under OS
//! keyring custody there are no `keys/author-N.key` files to list. The index is the source of truth
//! for "who is registered"; the key registry holds their public keys and the vault their secrets.

use crate::error::Result;
use crate::workspace::Workspace;

/// Registered author ids, sorted ascending and de-duplicated. Empty if none are registered yet.
pub fn list(ws: &Workspace) -> Result<Vec<u64>> {
    let mut ids: Vec<u64> = match std::fs::read_to_string(ws.authors_path()) {
        Ok(s) => serde_json::from_str(&s)?,
        Err(_) => Vec::new(), // absent index → no authors yet
    };
    ids.sort_unstable();
    ids.dedup();
    Ok(ids)
}

/// Record an author id in the index (idempotent). Ordering/dedup is normalized on read by [`list`].
pub fn add(ws: &Workspace, id: u64) -> Result<()> {
    let mut ids = list(ws)?;
    if !ids.contains(&id) {
        ids.push(id);
    }
    ws.ensure_dirs()?;
    std::fs::write(ws.authors_path(), serde_json::to_string(&ids)?)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};

    fn temp_ws() -> Workspace {
        static N: AtomicU32 = AtomicU32::new(0);
        let n = N.fetch_add(1, Ordering::Relaxed);
        Workspace::new(std::env::temp_dir().join(format!("aidx-{}-{n}", std::process::id())))
    }

    #[test]
    fn absent_index_is_empty_then_add_persists() {
        let ws = temp_ws();
        assert!(list(&ws).unwrap().is_empty());
        add(&ws, 5).unwrap();
        add(&ws, 2).unwrap();
        assert_eq!(list(&ws).unwrap(), vec![2, 5]);
        let _ = std::fs::remove_dir_all(ws.root());
    }

    #[test]
    fn add_is_idempotent_at_the_file_level() {
        let ws = temp_ws();
        add(&ws, 7).unwrap();
        add(&ws, 7).unwrap();
        // not just deduped on read — the stored set itself holds a single entry
        let raw: Vec<u64> =
            serde_json::from_str(&std::fs::read_to_string(ws.authors_path()).unwrap()).unwrap();
        assert_eq!(raw, vec![7]);
        assert_eq!(list(&ws).unwrap(), vec![7]);
        let _ = std::fs::remove_dir_all(ws.root());
    }

    #[test]
    fn list_normalizes_unsorted_and_duplicated_on_disk() {
        let ws = temp_ws();
        ws.ensure_dirs().unwrap();
        std::fs::write(ws.authors_path(), "[3,1,2,2,1]").unwrap();
        assert_eq!(list(&ws).unwrap(), vec![1, 2, 3]);
        let _ = std::fs::remove_dir_all(ws.root());
    }
}

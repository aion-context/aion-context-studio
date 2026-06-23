//! Write-side policy operations: create a policy, read its current rules, and commit a new version.
//! All commits are signed by the single seeded operator (local demo) using its persisted key.

use aion_context::operations::{
    commit_version, init_file, show_current_rules, CommitOptions, InitOptions,
};
use aion_context::types::AuthorId;

use crate::error::{Result, StudioError};
use crate::workspace::{PolicyId, Workspace};
use crate::{keystore, registry_store};

/// The single operator that signs everything in the local demo.
const OPERATOR: u64 = 1;

/// Result of a create or commit: the new version and its rules hash (hex).
#[derive(Debug, Clone, serde::Serialize)]
pub struct CommitInfo {
    pub version: u64,
    pub rules_hash: String,
}

/// The current (latest) rules text of a policy.
pub fn current_rules(ws: &Workspace, id: &PolicyId) -> Result<String> {
    let path = ws.policy_path(id);
    if !path.exists() {
        return Err(StudioError::NotFound(id.as_str().to_string()));
    }
    let bytes = show_current_rules(&path)?;
    Ok(String::from_utf8_lossy(&bytes).into_owned())
}

/// Create a new policy (genesis version 1), signed by the operator.
pub fn create(ws: &Workspace, id: &PolicyId, rules: &str) -> Result<CommitInfo> {
    let path = ws.policy_path(id);
    if path.exists() {
        return Err(StudioError::AlreadyExists(id.as_str().to_string()));
    }
    let author = AuthorId::new(OPERATOR);
    let key = keystore::load_signing_key(ws, author)?;

    // Ensure the operator is registered (seeded workspaces already are; this covers a bare one).
    let mut registry = registry_store::load(ws)?;
    if registry.active_epoch_at(author, 1).is_none() {
        registry.register_author(author, key.verifying_key(), key.verifying_key(), 1)?;
        registry_store::save(ws, &registry)?;
    }

    let options = InitOptions {
        author_id: author,
        signing_key: &key,
        message: "Genesis: created in the studio",
        timestamp: None,
    };
    let res = init_file(&path, rules.as_bytes(), &options)?;
    Ok(CommitInfo {
        version: res.version.0,
        rules_hash: keystore::hex_encode(&res.rules_hash),
    })
}

/// Commit a new version of an existing policy, signed by the operator.
pub fn commit(ws: &Workspace, id: &PolicyId, rules: &str, message: &str) -> Result<CommitInfo> {
    let path = ws.policy_path(id);
    if !path.exists() {
        return Err(StudioError::NotFound(id.as_str().to_string()));
    }
    let author = AuthorId::new(OPERATOR);
    let key = keystore::load_signing_key(ws, author)?;
    let registry = registry_store::load(ws)?;

    let options = CommitOptions {
        author_id: author,
        signing_key: &key,
        message,
        timestamp: None,
    };
    let res = commit_version(&path, rules.as_bytes(), &options, &registry)?;
    Ok(CommitInfo {
        version: res.version.0,
        rules_hash: keystore::hex_encode(&res.rules_hash),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{policies, seed};
    use std::sync::atomic::{AtomicU32, Ordering};

    fn seeded_ws() -> Workspace {
        static N: AtomicU32 = AtomicU32::new(0);
        let n = N.fetch_add(1, Ordering::Relaxed);
        let ws =
            Workspace::new(std::env::temp_dir().join(format!("auth-{}-{n}", std::process::id())));
        seed::ensure_seeded(&ws).unwrap();
        ws
    }

    #[test]
    fn create_then_commit_grows_history_and_stays_verifiable() {
        let ws = seeded_ws();
        let id = PolicyId::new("access-policy").unwrap();

        let g = create(&ws, &id, "rule: allow\n").unwrap();
        assert_eq!(g.version, 1);
        assert!(policies::verify(&ws, &id).unwrap().is_valid);

        // creating the same id again is rejected
        assert!(matches!(
            create(&ws, &id, "x"),
            Err(StudioError::AlreadyExists(_))
        ));

        let c = commit(&ws, &id, "rule: allow\nrule: log\n", "add logging").unwrap();
        assert_eq!(c.version, 2);
        let report = policies::verify(&ws, &id).unwrap();
        assert!(
            report.is_valid,
            "still verifies after commit: {:?}",
            report.errors
        );
        assert_eq!(policies::info(&ws, &id).unwrap().version_count, 2);

        // current rules reflect the latest commit
        assert_eq!(current_rules(&ws, &id).unwrap(), "rule: allow\nrule: log\n");
        let _ = std::fs::remove_dir_all(ws.root());
    }

    #[test]
    fn commit_or_rules_on_missing_policy_is_not_found() {
        let ws = seeded_ws();
        let missing = PolicyId::new("ghost").unwrap();
        assert!(matches!(
            current_rules(&ws, &missing),
            Err(StudioError::NotFound(_))
        ));
        assert!(matches!(
            commit(&ws, &missing, "x", "m"),
            Err(StudioError::NotFound(_))
        ));
        let _ = std::fs::remove_dir_all(ws.root());
    }
}

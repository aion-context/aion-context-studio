//! Seed a fresh workspace with one real, signed sample policy so the studio has something to show
//! on first run. Idempotent: does nothing once any policy exists.

use aion_context::crypto::SigningKey;
use aion_context::operations::{init_file, InitOptions};
use aion_context::types::AuthorId;

use crate::error::Result;
use crate::workspace::{PolicyId, Workspace};
use crate::{author_index, keystore, registry_store};

const SAMPLE_ID: &str = "refund-authorization";

/// The seeded signer set — gives K-of-N governance an N. The first is the default author.
pub const SIGNER_AUTHORS: [u64; 3] = [1, 2, 3];

/// The sample `.aion` payload — a refund-authorization policy in the studio's default rule format
/// (JSON, first-match-wins), which the evaluator executes against a numeric action input.
const SAMPLE_RULES: &str = r#"{
  "policy": "refund-authorization",
  "rules": [
    { "id": "auto-approve-small", "when": { "amount_usd": { "op": "le", "value": 50 } }, "decision": "allow" },
    { "id": "manager-approval",   "when": { "amount_usd": { "op": "le", "value": 500 } }, "decision": "allow_with_approval" },
    { "id": "deny-large",         "when": { "amount_usd": { "op": "gt", "value": 500 } }, "decision": "deny" }
  ]
}
"#;

/// Ensure the workspace holds at least the sample policy. Safe to call on every startup.
pub fn ensure_seeded(ws: &Workspace) -> Result<()> {
    ws.ensure_dirs()?;
    if !ws.list_ids()?.is_empty() {
        return Ok(());
    }
    seed_sample(ws)
}

fn seed_sample(ws: &Workspace) -> Result<()> {
    let mut registry = registry_store::load(ws)?;
    // Register a small signer set so K-of-N governance has an N to work with. Each gets a
    // generated operational key, registered at version 1 and persisted (local-demo custody).
    let mut op_keys = Vec::new();
    for author_id in SIGNER_AUTHORS {
        let author = AuthorId::new(author_id);
        let master = SigningKey::generate();
        let op = SigningKey::generate();
        registry.register_author(author, master.verifying_key(), op.verifying_key(), 1)?;
        keystore::save_signing_key(ws, author, &op)?;
        keystore::save_master_key(ws, author, &master)?; // needed to rotate/revoke later
        author_index::add(ws, author_id)?; // custody-agnostic "who is registered"
        op_keys.push((author, op));
    }
    registry_store::save(ws, &registry)?;

    // The seeded policy's genesis is authored by the first signer.
    let (author, op) = &op_keys[0];
    let id = PolicyId::new(SAMPLE_ID)?;
    let options = InitOptions {
        author_id: *author,
        signing_key: op,
        message: "Genesis: initial refund-authorization policy",
        timestamp: None,
    };
    init_file(&ws.policy_path(&id), SAMPLE_RULES.as_bytes(), &options)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::policies;
    use std::sync::atomic::{AtomicU32, Ordering};

    fn temp_ws() -> Workspace {
        static N: AtomicU32 = AtomicU32::new(0);
        let n = N.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!("studio-seed-{}-{n}", std::process::id()));
        Workspace::new(dir)
    }

    #[test]
    fn seed_creates_a_verifiable_policy_and_is_idempotent() {
        let ws = temp_ws();
        ensure_seeded(&ws).unwrap();
        let ids = ws.list_ids().unwrap();
        assert_eq!(ids.len(), 1, "exactly one seeded policy");
        assert_eq!(ids[0].as_str(), SAMPLE_ID);

        // the seeded policy verifies green against the persisted registry
        let report = policies::verify(&ws, &ids[0]).unwrap();
        assert!(
            report.is_valid,
            "seeded policy must verify: {:?}",
            report.errors
        );
        assert!(report.signatures_valid && report.hash_chain_valid);

        // the operational signing key is persisted under keys_dir (local-demo custody)
        let key_hex = std::fs::read_to_string(ws.keys_dir().join("author-1.key")).unwrap();
        assert_eq!(key_hex.len(), 64, "32-byte operational key, hex-encoded");

        // second call is a no-op (no second policy, no panic)
        ensure_seeded(&ws).unwrap();
        assert_eq!(ws.list_ids().unwrap().len(), 1);

        let _ = std::fs::remove_dir_all(ws.root());
    }
}

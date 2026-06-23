//! Registry management: register authors, rotate and revoke keys, view epoch timelines, export the
//! trusted JSON. Rotation/revocation are authorized by each author's master key (held locally in
//! this demo). They take effect from `max_policy_version + 1`, so existing signed versions keep
//! verifying and the change applies to future versions onward.

use aion_context::crypto::SigningKey;
use aion_context::key_registry::{
    sign_revocation_record, sign_rotation_record, KeyEpoch, KeyRegistry, KeyStatus,
    RevocationReason,
};
use aion_context::operations::show_version_history;
use aion_context::types::AuthorId;
use serde::Serialize;

use crate::error::{Result, StudioError};
use crate::workspace::Workspace;
use crate::{keystore, registry_store};

/// One key epoch, rendered for display.
#[derive(Debug, Clone, Serialize)]
pub struct EpochView {
    pub epoch: u32,
    pub public_key: String,
    pub created_at_version: u64,
    pub status: String,
    pub detail: String,
}

/// An author and its epoch timeline.
#[derive(Debug, Clone, Serialize)]
pub struct AuthorView {
    pub author_id: u64,
    pub epochs: Vec<EpochView>,
}

/// Every author the studio knows (by persisted operational key), with epoch timelines.
pub fn list(ws: &Workspace) -> Result<Vec<AuthorView>> {
    let registry = registry_store::load(ws)?;
    Ok(author_ids(ws)?
        .into_iter()
        .map(|id| author_view(&registry, id))
        .collect())
}

/// Register a new author (master + operational key, epoch 0 from version 1).
pub fn register(ws: &Workspace, requested: Option<u64>) -> Result<AuthorView> {
    let id = match requested {
        Some(i) => i,
        None => author_ids(ws)?.into_iter().max().unwrap_or(0) + 1,
    };
    let mut registry = registry_store::load(ws)?;
    if registry.active_epoch_at(AuthorId::new(id), 1).is_some() {
        return Err(StudioError::AlreadyExists(format!("author {id}")));
    }
    let master = SigningKey::generate();
    let op = SigningKey::generate();
    registry.register_author(
        AuthorId::new(id),
        master.verifying_key(),
        op.verifying_key(),
        1,
    )?;
    registry_store::save(ws, &registry)?;
    keystore::save_signing_key(ws, AuthorId::new(id), &op)?;
    keystore::save_master_key(ws, AuthorId::new(id), &master)?;
    Ok(author_view(&registry, id))
}

/// Rotate an author's active operational key to a fresh one.
pub fn rotate(ws: &Workspace, id: u64) -> Result<AuthorView> {
    let mut registry = registry_store::load(ws)?;
    let from_epoch = {
        let epochs = registry.epochs_for(AuthorId::new(id));
        let cur = epochs
            .last()
            .ok_or_else(|| StudioError::NotFound(format!("author {id}")))?;
        if !matches!(cur.status, KeyStatus::Active) {
            return Err(StudioError::Invalid(format!(
                "author {id}'s current epoch is not active"
            )));
        }
        cur.epoch
    };
    let new_op = SigningKey::generate();
    let master = keystore::load_master_key(ws, AuthorId::new(id))?;
    let record = sign_rotation_record(
        AuthorId::new(id),
        from_epoch,
        from_epoch + 1,
        new_op.verifying_key().to_bytes(),
        effective_from(ws)?,
        &master,
    );
    registry.apply_rotation(&record)?;
    registry_store::save(ws, &registry)?;
    keystore::save_signing_key(ws, AuthorId::new(id), &new_op)?; // future commits use the new epoch
    Ok(author_view(&registry, id))
}

/// Revoke an author's current epoch with a reason.
pub fn revoke(ws: &Workspace, id: u64, reason: &str) -> Result<AuthorView> {
    let mut registry = registry_store::load(ws)?;
    let revoked_epoch = {
        let epochs = registry.epochs_for(AuthorId::new(id));
        epochs
            .last()
            .ok_or_else(|| StudioError::NotFound(format!("author {id}")))?
            .epoch
    };
    let master = keystore::load_master_key(ws, AuthorId::new(id))?;
    let record = sign_revocation_record(
        AuthorId::new(id),
        revoked_epoch,
        parse_reason(reason),
        effective_from(ws)?,
        &master,
    );
    registry.apply_revocation(&record)?;
    registry_store::save(ws, &registry)?;
    Ok(author_view(&registry, id))
}

/// The trusted-JSON registry (public keys + accreditation) — what an offline verifier pins.
pub fn trusted_json(ws: &Workspace) -> Result<String> {
    Ok(registry_store::load(ws)?.to_trusted_json()?)
}

fn author_view(registry: &KeyRegistry, id: u64) -> AuthorView {
    let epochs = registry
        .epochs_for(AuthorId::new(id))
        .iter()
        .map(epoch_view)
        .collect();
    AuthorView {
        author_id: id,
        epochs,
    }
}

fn epoch_view(e: &KeyEpoch) -> EpochView {
    let (status, detail) = match e.status {
        KeyStatus::Active => ("active".to_string(), String::new()),
        KeyStatus::Rotated {
            successor_epoch,
            effective_from_version,
        } => (
            "rotated".to_string(),
            format!("→ epoch {successor_epoch} from v{effective_from_version}"),
        ),
        KeyStatus::Revoked {
            reason,
            effective_from_version,
        } => (
            "revoked".to_string(),
            format!("{} from v{effective_from_version}", reason_str(reason)),
        ),
    };
    EpochView {
        epoch: e.epoch,
        public_key: keystore::hex_encode(&e.public_key),
        created_at_version: e.created_at_version,
        status,
        detail,
    }
}

/// Author ids the studio has keys for (operational `author-N.key`, excluding `.master`).
fn author_ids(ws: &Workspace) -> Result<Vec<u64>> {
    let dir = ws.keys_dir();
    if !dir.exists() {
        return Ok(Vec::new());
    }
    let mut ids = Vec::new();
    for entry in std::fs::read_dir(&dir)? {
        let name = entry?.file_name();
        let name = name.to_string_lossy();
        if let Some(num) = name
            .strip_prefix("author-")
            .and_then(|r| r.strip_suffix(".key"))
        {
            if let Ok(n) = num.parse::<u64>() {
                ids.push(n);
            }
        }
    }
    ids.sort_unstable();
    ids.dedup();
    Ok(ids)
}

/// `max(current_version over all policies) + 1` — so existing versions stay valid.
fn effective_from(ws: &Workspace) -> Result<u64> {
    let mut max_v = 0u64;
    for id in ws.list_ids()? {
        let count = show_version_history(&ws.policy_path(&id))?.len() as u64;
        max_v = max_v.max(count);
    }
    Ok(max_v + 1)
}

fn parse_reason(s: &str) -> RevocationReason {
    match s {
        "compromised" => RevocationReason::Compromised,
        "superseded" => RevocationReason::Superseded,
        "retired" => RevocationReason::Retired,
        _ => RevocationReason::Unspecified,
    }
}

fn reason_str(r: RevocationReason) -> &'static str {
    match r {
        RevocationReason::Compromised => "compromised",
        RevocationReason::Superseded => "superseded",
        RevocationReason::Retired => "retired",
        RevocationReason::Unspecified => "unspecified",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workspace::PolicyId;
    use crate::{policies, seed};
    use std::sync::atomic::{AtomicU32, Ordering};

    fn seeded_ws() -> Workspace {
        static N: AtomicU32 = AtomicU32::new(0);
        let n = N.fetch_add(1, Ordering::Relaxed);
        let ws =
            Workspace::new(std::env::temp_dir().join(format!("reg-{}-{n}", std::process::id())));
        seed::ensure_seeded(&ws).unwrap();
        ws
    }

    #[test]
    fn lists_seeded_authors_with_active_epoch_zero() {
        let ws = seeded_ws();
        let authors = list(&ws).unwrap();
        assert_eq!(
            authors.iter().map(|a| a.author_id).collect::<Vec<_>>(),
            vec![1, 2, 3]
        );
        for a in &authors {
            assert_eq!(a.epochs.len(), 1);
            assert_eq!(a.epochs[0].epoch, 0);
            assert_eq!(a.epochs[0].status, "active");
            assert_eq!(a.epochs[0].public_key.len(), 64);
        }
        let _ = std::fs::remove_dir_all(ws.root());
    }

    #[test]
    fn register_rotate_revoke_update_timeline_and_keep_policies_valid() {
        let ws = seeded_ws();
        let id = PolicyId::new("refund-authorization").unwrap();
        assert!(policies::verify(&ws, &id).unwrap().is_valid);

        // register author 4
        let a4 = register(&ws, None).unwrap();
        assert_eq!(a4.author_id, 4);
        assert!(matches!(
            register(&ws, Some(1)),
            Err(StudioError::AlreadyExists(_))
        ));

        // rotate author 2: epoch 0 → rotated, epoch 1 active
        let r = rotate(&ws, 2).unwrap();
        assert_eq!(r.epochs.len(), 2);
        assert_eq!(r.epochs[0].status, "rotated");
        assert_eq!(r.epochs[1].status, "active");
        assert!(matches!(rotate(&ws, 999), Err(StudioError::NotFound(_))));

        // revoke author 3
        let v = revoke(&ws, 3, "compromised").unwrap();
        assert_eq!(v.epochs[0].status, "revoked");
        assert!(v.epochs[0].detail.contains("compromised"));

        // existing policy still verifies (changes are effective from a future version)
        assert!(policies::verify(&ws, &id).unwrap().is_valid);
        assert!(trusted_json(&ws).unwrap().contains("\""));
        let _ = std::fs::remove_dir_all(ws.root());
    }

    #[test]
    fn reason_round_trips() {
        for (s, _) in [
            ("compromised", 1),
            ("superseded", 2),
            ("retired", 3),
            ("x", 255),
        ] {
            assert_eq!(
                reason_str(parse_reason(s)),
                if s == "x" { "unspecified" } else { s }
            );
        }
    }
}

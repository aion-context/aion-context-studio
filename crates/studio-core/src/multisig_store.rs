//! K-of-N governance over a policy's current version.
//!
//! aion-context verifies a *set* of attestations over one version (`multisig::verify_multisig`).
//! In this local single-operator demo the studio holds every signer's key, so an "approval" is
//! recorded as the approver's author id and the attestation is regenerated (`sign_attestation`)
//! at verify time — a real multi-party deployment would instead collect each party's signature.
//! Approvals are scoped to a version number: a new commit resets them (each version needs fresh
//! K-of-N approval).

use aion_context::multisig::{verify_multisig, MultiSigPolicy};
use aion_context::parser::AionParser;
use aion_context::serializer::VersionEntry;
use aion_context::signature_chain::sign_attestation;
use aion_context::types::AuthorId;
use serde::{Deserialize, Serialize};

use crate::error::{Result, StudioError};
use crate::workspace::{PolicyId, Workspace};
use crate::{keystore, registry_store, seed};

/// The persisted M-of-N policy for one policy artifact.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultiSigConfig {
    pub threshold: u32,
    pub signers: Vec<u64>,
}

impl Default for MultiSigConfig {
    fn default() -> Self {
        Self {
            threshold: 2,
            signers: seed::SIGNER_AUTHORS.to_vec(),
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct Approvals {
    version: u64,
    approvers: Vec<u64>,
}

/// Governance progress for a policy's current version.
#[derive(Debug, Clone, Serialize)]
pub struct MultiSigProgress {
    pub version: u64,
    pub threshold: u32,
    pub signers: Vec<u64>,
    pub approvers: Vec<u64>,
    pub valid_count: u32,
    pub required: u32,
    pub threshold_met: bool,
    pub missing: Vec<u64>,
}

fn dir(ws: &Workspace) -> std::path::PathBuf {
    ws.root().join("multisig")
}
fn config_path(ws: &Workspace, id: &PolicyId) -> std::path::PathBuf {
    dir(ws).join(format!("{}.policy.json", id.as_str()))
}
fn approvals_path(ws: &Workspace, id: &PolicyId) -> std::path::PathBuf {
    dir(ws).join(format!("{}.approvals.json", id.as_str()))
}

/// Load the M-of-N config, or the default (2-of-3 over the seeded signers).
pub fn load_config(ws: &Workspace, id: &PolicyId) -> Result<MultiSigConfig> {
    let p = config_path(ws, id);
    if !p.exists() {
        return Ok(MultiSigConfig::default());
    }
    Ok(serde_json::from_str(&std::fs::read_to_string(p)?)?)
}

/// Set the M-of-N config (validated), then return fresh progress.
pub fn set_config(
    ws: &Workspace,
    id: &PolicyId,
    threshold: u32,
    signers: Vec<u64>,
) -> Result<MultiSigProgress> {
    if signers.is_empty() || threshold == 0 || threshold as usize > signers.len() {
        return Err(StudioError::Invalid(format!(
            "invalid threshold {threshold} for {} signer(s)",
            signers.len()
        )));
    }
    let registry = registry_store::load(ws)?;
    for &s in &signers {
        if registry.active_epoch_at(AuthorId::new(s), 1).is_none() {
            return Err(StudioError::Invalid(format!(
                "signer {s} is not registered"
            )));
        }
    }
    let cfg = MultiSigConfig { threshold, signers };
    std::fs::create_dir_all(dir(ws))?;
    std::fs::write(config_path(ws, id), serde_json::to_string_pretty(&cfg)?)?;
    progress(ws, id)
}

/// Record an approval for the current version by an authorized signer, then return progress.
pub fn approve(ws: &Workspace, id: &PolicyId, author: u64) -> Result<MultiSigProgress> {
    let cfg = load_config(ws, id)?;
    if !cfg.signers.contains(&author) {
        return Err(StudioError::Invalid(format!(
            "author {author} is not an authorized signer"
        )));
    }
    let (version, _) = latest_version(ws, id)?;
    let mut ap = fresh_for(ws, id, version)?;
    if !ap.approvers.contains(&author) {
        ap.approvers.push(author);
        ap.approvers.sort_unstable();
    }
    std::fs::create_dir_all(dir(ws))?;
    std::fs::write(approvals_path(ws, id), serde_json::to_string_pretty(&ap)?)?;
    progress(ws, id)
}

/// Governance progress: regenerate each approver's attestation and run `verify_multisig`.
pub fn progress(ws: &Workspace, id: &PolicyId) -> Result<MultiSigProgress> {
    let cfg = load_config(ws, id)?;
    let (version, entry) = latest_version(ws, id)?;
    let ap = fresh_for(ws, id, version)?;
    let registry = registry_store::load(ws)?;

    let mut sigs = Vec::with_capacity(ap.approvers.len());
    for &a in &ap.approvers {
        let key = keystore::load_signing_key(ws, AuthorId::new(a))?;
        sigs.push(sign_attestation(&entry, AuthorId::new(a), &key));
    }
    let policy = MultiSigPolicy::m_of_n(
        cfg.threshold,
        cfg.signers.iter().map(|&s| AuthorId::new(s)).collect(),
    )?;
    let v = verify_multisig(&entry, &sigs, &policy, &registry)?;
    Ok(MultiSigProgress {
        version,
        threshold: cfg.threshold,
        signers: cfg.signers,
        approvers: ap.approvers,
        valid_count: v.valid_count,
        required: v.required,
        threshold_met: v.threshold_met,
        missing: v.missing_signers.iter().map(|a| a.0).collect(),
    })
}

/// Load approvals, resetting them if they were recorded against an older version.
fn fresh_for(ws: &Workspace, id: &PolicyId, version: u64) -> Result<Approvals> {
    let p = approvals_path(ws, id);
    let ap: Approvals = if p.exists() {
        serde_json::from_str(&std::fs::read_to_string(p)?)?
    } else {
        Approvals::default()
    };
    if ap.version == version {
        Ok(ap)
    } else {
        Ok(Approvals {
            version,
            approvers: Vec::new(),
        })
    }
}

/// The latest (version_number, VersionEntry) of a policy.
fn latest_version(ws: &Workspace, id: &PolicyId) -> Result<(u64, VersionEntry)> {
    let path = ws.policy_path(id);
    if !path.exists() {
        return Err(StudioError::NotFound(id.as_str().to_string()));
    }
    let bytes = std::fs::read(&path)?;
    let parser = AionParser::new(&bytes)?;
    let count = parser.header().version_chain_count;
    let entry = parser.get_version_entry((count as usize).saturating_sub(1))?;
    Ok((count, entry))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::authoring;
    use std::sync::atomic::{AtomicU32, Ordering};

    fn seeded_ws() -> Workspace {
        static N: AtomicU32 = AtomicU32::new(0);
        let n = N.fetch_add(1, Ordering::Relaxed);
        let ws =
            Workspace::new(std::env::temp_dir().join(format!("ms-{}-{n}", std::process::id())));
        seed::ensure_seeded(&ws).unwrap();
        ws
    }

    #[test]
    fn approvals_reach_threshold_and_reset_on_new_version() {
        let ws = seeded_ws();
        let id = PolicyId::new("refund-authorization").unwrap();

        // default 2-of-3, no approvals yet
        let p0 = progress(&ws, &id).unwrap();
        assert_eq!((p0.threshold, p0.required, p0.valid_count), (2, 2, 0));
        assert!(!p0.threshold_met);
        assert_eq!(p0.missing, vec![1, 2, 3]);

        let p1 = approve(&ws, &id, 1).unwrap();
        assert_eq!(p1.valid_count, 1);
        assert!(!p1.threshold_met);

        let p2 = approve(&ws, &id, 2).unwrap();
        assert_eq!(p2.valid_count, 2);
        assert!(p2.threshold_met, "2-of-3 met after two approvals");

        // re-approving the same author is idempotent
        assert_eq!(approve(&ws, &id, 1).unwrap().valid_count, 2);

        // a new committed version resets approvals
        authoring::commit(&ws, &id, "policy: refund\nrules: []\n", "tighten").unwrap();
        let after = progress(&ws, &id).unwrap();
        assert_eq!(after.version, 2);
        assert_eq!(after.valid_count, 0);
        assert!(!after.threshold_met);
        let _ = std::fs::remove_dir_all(ws.root());
    }

    #[test]
    fn unauthorized_signer_and_bad_threshold_are_rejected() {
        let ws = seeded_ws();
        let id = PolicyId::new("refund-authorization").unwrap();
        assert!(matches!(
            approve(&ws, &id, 99),
            Err(StudioError::Invalid(_))
        ));
        assert!(matches!(
            set_config(&ws, &id, 4, vec![1, 2, 3]),
            Err(StudioError::Invalid(_))
        ));
        assert!(matches!(
            set_config(&ws, &id, 1, vec![1, 77]),
            Err(StudioError::Invalid(_))
        ));
        // threshold of zero is rejected
        assert!(matches!(
            set_config(&ws, &id, 0, vec![1, 2, 3]),
            Err(StudioError::Invalid(_))
        ));
        // N-of-N (threshold == signer count) is valid — pins `>` is not `>=`
        let nofn = set_config(&ws, &id, 3, vec![1, 2, 3]).unwrap();
        assert_eq!((nofn.threshold, nofn.required), (3, 3));

        // a valid reconfigure to 1-of-2 then one approval meets it
        let p = set_config(&ws, &id, 1, vec![1, 2]).unwrap();
        assert_eq!((p.threshold, p.required), (1, 1));
        assert!(approve(&ws, &id, 2).unwrap().threshold_met);
        let _ = std::fs::remove_dir_all(ws.root());
    }

    #[test]
    fn multisig_files_live_under_the_workspace() {
        let ws = Workspace::new("/tmp/ws-ms");
        let id = PolicyId::new("p1").unwrap();
        assert_eq!(dir(&ws), std::path::Path::new("/tmp/ws-ms/multisig"));
        assert_eq!(
            config_path(&ws, &id),
            std::path::Path::new("/tmp/ws-ms/multisig/p1.policy.json")
        );
        assert_eq!(
            approvals_path(&ws, &id),
            std::path::Path::new("/tmp/ws-ms/multisig/p1.approvals.json")
        );
    }
}

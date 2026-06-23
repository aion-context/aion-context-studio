//! Read-side policy operations: list every policy with a quick verdict, and fetch full info or a
//! verification report for one. All verification goes through aion-context's own `verify_file`.

use aion_context::operations::{show_file_info, verify_file, FileInfo, VerificationReport};

use crate::error::{Result, StudioError};
use crate::registry_store;
use crate::workspace::{PolicyId, Workspace};

/// One row in the policy list: identity, version counts, and whether it currently verifies.
/// `file_id` is rendered as a 16-digit hex string — a u64 exceeds JavaScript's safe integer range,
/// so a JSON number would lose precision in the SPA.
#[derive(Debug, Clone, serde::Serialize)]
pub struct PolicySummary {
    pub id: String,
    pub file_id: String,
    pub version_count: u64,
    pub current_version: u64,
    pub valid: bool,
}

/// List every policy in the workspace with a summary + current validity.
pub fn list(ws: &Workspace) -> Result<Vec<PolicySummary>> {
    let registry = registry_store::load(ws)?;
    let mut out = Vec::new();
    for id in ws.list_ids()? {
        let path = ws.policy_path(&id);
        let info = show_file_info(&path, &registry)?;
        let report = verify_file(&path, &registry)?;
        out.push(PolicySummary {
            id: id.as_str().to_string(),
            file_id: format!("{:016x}", info.file_id),
            version_count: info.version_count,
            current_version: info.current_version,
            valid: report.is_valid,
        });
    }
    Ok(out)
}

/// Full file info (versions + signatures) for one policy.
pub fn info(ws: &Workspace, id: &PolicyId) -> Result<FileInfo> {
    let path = require_existing(ws, id)?;
    let registry = registry_store::load(ws)?;
    Ok(show_file_info(&path, &registry)?)
}

/// The four-guarantee verification report for one policy.
pub fn verify(ws: &Workspace, id: &PolicyId) -> Result<VerificationReport> {
    let path = require_existing(ws, id)?;
    let registry = registry_store::load(ws)?;
    Ok(verify_file(&path, &registry)?)
}

fn require_existing(ws: &Workspace, id: &PolicyId) -> Result<std::path::PathBuf> {
    let path = ws.policy_path(id);
    if path.exists() {
        Ok(path)
    } else {
        Err(StudioError::NotFound(id.as_str().to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::seed;
    use std::sync::atomic::{AtomicU32, Ordering};

    fn seeded_ws() -> Workspace {
        static N: AtomicU32 = AtomicU32::new(0);
        let n = N.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!("studio-pol-{}-{n}", std::process::id()));
        let ws = Workspace::new(dir);
        seed::ensure_seeded(&ws).unwrap();
        ws
    }

    #[test]
    fn list_reports_the_seeded_policy_as_valid() {
        let ws = seeded_ws();
        let rows = list(&ws).unwrap();
        assert_eq!(rows.len(), 1);
        assert!(rows[0].valid);
        assert_eq!(rows[0].version_count, 1);
        assert_eq!(rows[0].current_version, 1);
        let _ = std::fs::remove_dir_all(ws.root());
    }

    #[test]
    fn info_and_verify_resolve_or_404() {
        let ws = seeded_ws();
        let id = PolicyId::new("refund-authorization").unwrap();
        assert_eq!(info(&ws, &id).unwrap().version_count, 1);
        assert!(verify(&ws, &id).unwrap().is_valid);

        let missing = PolicyId::new("does-not-exist").unwrap();
        assert!(matches!(
            verify(&ws, &missing),
            Err(StudioError::NotFound(_))
        ));
        let _ = std::fs::remove_dir_all(ws.root());
    }
}

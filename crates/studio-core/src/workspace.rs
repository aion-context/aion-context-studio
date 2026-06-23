//! The on-disk studio workspace: where `.aion` policies, the key registry, and keys live.
//!
//! Policies are addressed by a validated [`PolicyId`] (the `.aion` filename stem), never by a
//! client-supplied path — so a request can never escape the policies directory.

use std::path::{Path, PathBuf};

use crate::error::{Result, StudioError};

/// A studio data directory (default `studio-data/`).
#[derive(Clone, Debug)]
pub struct Workspace {
    root: PathBuf,
}

impl Workspace {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn policies_dir(&self) -> PathBuf {
        self.root.join("policies")
    }

    pub fn keys_dir(&self) -> PathBuf {
        self.root.join("keys")
    }

    pub fn registry_path(&self) -> PathBuf {
        self.root.join("registry").join("registry.json")
    }

    /// The `.aion` path for a validated id.
    pub fn policy_path(&self, id: &PolicyId) -> PathBuf {
        self.policies_dir().join(format!("{}.aion", id.as_str()))
    }

    /// Create the workspace subdirectories if absent.
    pub fn ensure_dirs(&self) -> Result<()> {
        std::fs::create_dir_all(self.policies_dir())?;
        std::fs::create_dir_all(self.keys_dir())?;
        std::fs::create_dir_all(self.root.join("registry"))?;
        Ok(())
    }

    /// All policy ids present on disk, sorted. Non-`.aion` and invalid-stem files are skipped.
    pub fn list_ids(&self) -> Result<Vec<PolicyId>> {
        let dir = self.policies_dir();
        if !dir.exists() {
            return Ok(Vec::new());
        }
        let mut ids = Vec::new();
        for entry in std::fs::read_dir(&dir)? {
            let path = entry?.path();
            if path.extension().and_then(|e| e.to_str()) != Some("aion") {
                continue;
            }
            if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                if let Ok(id) = PolicyId::new(stem) {
                    ids.push(id);
                }
            }
        }
        ids.sort_by(|a, b| a.as_str().cmp(b.as_str()));
        Ok(ids)
    }
}

/// A validated policy identifier: `[a-z0-9-]`, 1..=64 chars. Used as the `.aion` filename stem;
/// the character whitelist is the path-traversal defense (no `/`, `.`, `..`, etc.).
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct PolicyId(String);

impl PolicyId {
    pub fn new(s: &str) -> Result<Self> {
        let ok = (1..=64).contains(&s.len())
            && s.bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-');
        if ok {
            Ok(Self(s.to_string()))
        } else {
            Err(StudioError::InvalidId(s.to_string()))
        }
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn policy_id_accepts_valid_and_rejects_traversal() {
        assert!(PolicyId::new("refund-policy-2").is_ok());
        for bad in [
            "",
            "../etc",
            "a/b",
            "Upper",
            "has space",
            "dot.dot",
            &"x".repeat(65),
        ] {
            assert!(PolicyId::new(bad).is_err(), "should reject {bad:?}");
        }
    }

    #[test]
    fn paths_resolve_under_root() {
        let ws = Workspace::new("/tmp/ws");
        let id = PolicyId::new("p1").unwrap();
        assert_eq!(ws.policy_path(&id), Path::new("/tmp/ws/policies/p1.aion"));
        assert_eq!(ws.policies_dir(), Path::new("/tmp/ws/policies"));
        assert_eq!(ws.keys_dir(), Path::new("/tmp/ws/keys"));
        assert_eq!(
            ws.registry_path(),
            Path::new("/tmp/ws/registry/registry.json")
        );
    }
}

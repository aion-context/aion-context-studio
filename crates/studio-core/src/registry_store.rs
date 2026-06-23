//! Persist / load the workspace key registry. The registry holds only public keys and
//! accreditation (`to_trusted_json`) — never any secret — so it is safe to write to disk and ship
//! to offline verifiers.

use aion_context::key_registry::KeyRegistry;

use crate::error::Result;
use crate::workspace::Workspace;

/// Load the workspace registry, or a fresh empty one if none has been persisted yet.
pub fn load(ws: &Workspace) -> Result<KeyRegistry> {
    let path = ws.registry_path();
    if !path.exists() {
        return Ok(KeyRegistry::new());
    }
    let json = std::fs::read_to_string(&path)?;
    Ok(KeyRegistry::from_trusted_json(&json)?)
}

/// Persist the registry as trusted JSON (public keys + accreditation only).
pub fn save(ws: &Workspace, registry: &KeyRegistry) -> Result<()> {
    ws.ensure_dirs()?;
    let json = registry.to_trusted_json()?;
    std::fs::write(ws.registry_path(), json)?;
    Ok(())
}

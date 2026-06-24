//! Audit trail, compliance report, and export — the auditor's surface.
//!
//! The audit trail is the file's embedded, append-only operation log: each entry records an action
//! (genesis, commit, verify, inspect), who, when, and a detail string, plus the BLAKE3 hash of the
//! entry's bytes.
//!
//! NOTE — we present the trail as recorded *history*, not as a re-verified hash chain. In the
//! current aion-context an appended entry's stored `previous_hash` is misaligned by one `u64` (the
//! genuine previous hash, shifted 8 bytes with the last 8 bytes zeroed), so `validate_chain` fails
//! after the first commit even on an untampered file — and nothing in the library verifies the chain
//! anyway. Asserting an intact chain here would be misleading; surfacing the operation log honestly
//! is the right auditor affordance. Reported upstream:
//! <https://github.com/aion-context/aion-context/issues/141>.

use aion_context::compliance::{generate_compliance_report, ComplianceFramework, ReportFormat};
use aion_context::export::{export_file, ExportFormat};
use aion_context::parser::AionParser;
use serde::Serialize;

use crate::error::{Result, StudioError};
use crate::keystore::hex_encode;
use crate::registry_store;
use crate::workspace::{PolicyId, Workspace};

/// One audit-trail entry, rendered.
#[derive(Debug, Clone, Serialize)]
pub struct AuditEntryView {
    pub index: u64,
    pub timestamp: u64,
    pub author_id: u64,
    pub action: String,
    pub detail: String,
    pub hash: String,
}

/// A policy's recorded operation history.
#[derive(Debug, Clone, Serialize)]
pub struct AuditView {
    pub entries: Vec<AuditEntryView>,
}

/// Read a policy's embedded audit trail (the recorded operation log).
pub fn read(ws: &Workspace, id: &PolicyId) -> Result<AuditView> {
    let path = require(ws, id)?;
    let bytes = std::fs::read(&path)?;
    let parser = AionParser::new(&bytes)?;
    let count = parser.header().audit_trail_count;
    let table = parser.string_table_bytes()?;

    let mut entries = Vec::with_capacity(count as usize);
    for i in 0..count {
        let e = parser.get_audit_entry(i as usize)?;
        entries.push(AuditEntryView {
            index: i,
            timestamp: e.timestamp(),
            author_id: e.author_id().as_u64(),
            action: e
                .action_code()
                .map(|a| a.description().to_string())
                .unwrap_or_else(|_| "unknown".to_string()),
            detail: resolve_detail(table, e.details_offset(), e.details_length()),
            hash: hex_encode(&e.compute_hash()),
        });
    }
    Ok(AuditView { entries })
}

fn resolve_detail(table: &[u8], offset: u64, length: u32) -> String {
    let start = offset as usize;
    let end = start.saturating_add(length as usize);
    table
        .get(start..end)
        .and_then(|b| std::str::from_utf8(b).ok())
        .unwrap_or("")
        .to_string()
}

/// Generate a compliance report (framework: sox|hipaa|gdpr|generic; format: markdown|text|json).
pub fn compliance_report(
    ws: &Workspace,
    id: &PolicyId,
    framework: &str,
    format: &str,
) -> Result<String> {
    let path = require(ws, id)?;
    let registry = registry_store::load(ws)?;
    let fw = match framework {
        "sox" => ComplianceFramework::Sox,
        "hipaa" => ComplianceFramework::Hipaa,
        "gdpr" => ComplianceFramework::Gdpr,
        _ => ComplianceFramework::Generic,
    };
    let fmt = match format {
        "json" => ReportFormat::Json,
        "text" => ReportFormat::Text,
        _ => ReportFormat::Markdown,
    };
    Ok(generate_compliance_report(&path, fw, fmt, &registry)?)
}

/// Export a policy (format: json|yaml|csv).
pub fn export_policy(ws: &Workspace, id: &PolicyId, format: &str) -> Result<String> {
    let path = require(ws, id)?;
    let registry = registry_store::load(ws)?;
    let fmt = match format {
        "yaml" => ExportFormat::Yaml,
        "csv" => ExportFormat::Csv,
        _ => ExportFormat::Json,
    };
    Ok(export_file(&path, fmt, &registry)?)
}

fn require(ws: &Workspace, id: &PolicyId) -> Result<std::path::PathBuf> {
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
    use crate::{authoring, seed};
    use std::sync::atomic::{AtomicU32, Ordering};

    fn seeded_ws() -> Workspace {
        static N: AtomicU32 = AtomicU32::new(0);
        let n = N.fetch_add(1, Ordering::Relaxed);
        let ws =
            Workspace::new(std::env::temp_dir().join(format!("aud-{}-{n}", std::process::id())));
        seed::ensure_seeded(&ws).unwrap();
        ws
    }

    #[test]
    fn audit_trail_records_operations_and_grows_on_commit() {
        let ws = seeded_ws();
        let id = PolicyId::new("refund-authorization").unwrap();

        let a1 = read(&ws, &id).unwrap();
        assert!(!a1.entries.is_empty());
        let genesis = &a1.entries[0];
        assert_eq!(genesis.index, 0);
        assert!(genesis.action.to_lowercase().contains("genesis"));
        assert_eq!(genesis.hash.len(), 64);

        let before = a1.entries.len();
        authoring::commit(&ws, &id, "{ \"rules\": [] }", "empty out").unwrap();
        let a2 = read(&ws, &id).unwrap();
        assert!(a2.entries.len() > before, "a commit appends an audit entry");
        assert!(a2
            .entries
            .last()
            .unwrap()
            .action
            .to_lowercase()
            .contains("commit"));
        let _ = std::fs::remove_dir_all(ws.root());
    }

    #[test]
    fn resolve_detail_reads_the_slice() {
        let table = b"hello world";
        assert_eq!(resolve_detail(table, 0, 5), "hello");
        assert_eq!(resolve_detail(table, 6, 5), "world");
        assert_eq!(resolve_detail(table, 100, 5), ""); // out of range → empty, no panic
    }

    #[test]
    fn each_framework_is_named_and_each_format_differs() {
        let ws = seeded_ws();
        let id = PolicyId::new("refund-authorization").unwrap();
        let r = |fw: &str| compliance_report(&ws, &id, fw, "markdown").unwrap();
        assert!(r("sox").contains("SOX"));
        assert!(r("hipaa").contains("HIPAA")); // pins the hipaa arm
        assert!(r("gdpr").contains("GDPR")); // pins the gdpr arm
        assert!(r("generic").contains("Generic"));

        let md = compliance_report(&ws, &id, "sox", "markdown").unwrap();
        let txt = compliance_report(&ws, &id, "sox", "text").unwrap();
        let js = compliance_report(&ws, &id, "sox", "json").unwrap();
        assert_ne!(md, txt, "text format differs from markdown"); // pins text arm
        assert!(serde_json::from_str::<serde_json::Value>(&js).is_ok()); // pins json arm
        let _ = std::fs::remove_dir_all(ws.root());
    }

    #[test]
    fn exports_are_distinct_per_format() {
        let ws = seeded_ws();
        let id = PolicyId::new("refund-authorization").unwrap();
        let json = export_policy(&ws, &id, "json").unwrap();
        let yaml = export_policy(&ws, &id, "yaml").unwrap();
        let csv = export_policy(&ws, &id, "csv").unwrap();
        assert!(serde_json::from_str::<serde_json::Value>(&json).is_ok());
        assert_ne!(json, yaml, "yaml differs from json"); // pins yaml arm
        assert_ne!(json, csv, "csv differs from json"); // pins csv arm
        assert_ne!(yaml, csv);

        let missing = PolicyId::new("nope").unwrap();
        assert!(matches!(read(&ws, &missing), Err(StudioError::NotFound(_))));
        assert!(matches!(
            export_policy(&ws, &missing, "json"),
            Err(StudioError::NotFound(_))
        ));
        let _ = std::fs::remove_dir_all(ws.root());
    }
}

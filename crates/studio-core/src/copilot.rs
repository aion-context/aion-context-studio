//! Copilot context assembly — pure and offline. Given a policy and a surface, gather the real
//! aion-context state (current rules, verification, version history, governance, audit) into a
//! prompt the API layer sends to Claude.
//!
//! BOUNDARY: this module never touches signing keys and never performs network or mutating calls.
//! Claude advises and drafts; a human applies and signs through the ordinary commit/sign routes.
//! (`copilot.rs` deliberately does not import `keystore`; a test pins that no key can reach it.)

use crate::error::Result;
use crate::workspace::{PolicyId, Workspace};
use crate::{audit, authoring, multisig_store, policies};

/// The system prompt: the copilot's role and its hard boundary.
pub fn system_prompt() -> String {
    "You are the aion-context policy copilot, embedded in a studio for signed `.aion` policy \
     artifacts. You help authors read, draft, and edit policy rules; explain diffs and \
     verification results; and summarize governance and audit state.\n\n\
     Rules of engagement:\n\
     - You ADVISE and DRAFT only. You never sign, commit, approve, or mutate anything — a human \
     does that through the studio. Do not claim to have performed any signed action.\n\
     - When proposing rules, output a single fenced ```json block in the studio's rule format \
     (`{\"rules\":[{\"id\",\"when\":{field:{\"op\",\"value\"}},\"decision\"}]}`, ops \
     eq/ne/lt/le/gt/ge) so the author can apply it in the editor and commit it themselves.\n\
     - Be concise and exact. Prefer the smallest change that meets the request."
        .to_string()
}

/// The assembled, model-ready context for one request.
#[derive(Debug, Clone)]
pub struct CopilotContext {
    pub system: String,
    pub context: String,
}

/// Build the copilot context for a policy + surface (e.g. "editor", "governance", "audit").
pub fn build_context(ws: &Workspace, id: &PolicyId, surface: &str) -> Result<CopilotContext> {
    let mut c = String::new();
    c.push_str(&format!("Policy: {}\nSurface: {surface}\n\n", id.as_str()));

    let report = policies::verify(ws, id)?;
    c.push_str(&format!(
        "Verification: {} (structure={}, integrity={}, hash_chain={}, signatures={}), {} version(s).\n",
        if report.is_valid { "VALID" } else { "INVALID" },
        report.structure_valid,
        report.integrity_hash_valid,
        report.hash_chain_valid,
        report.signatures_valid,
        report.version_count,
    ));

    let ms = multisig_store::progress(ws, id)?;
    c.push_str(&format!(
        "Governance: {}-of-{} approval, {} so far{}.\n",
        ms.threshold,
        ms.signers.len(),
        ms.valid_count,
        if ms.threshold_met { " (met)" } else { "" },
    ));

    if let Ok(view) = audit::read(ws, id) {
        c.push_str(&format!("Audit entries: {}.\n", view.entries.len()));
    }

    let rules = authoring::current_rules(ws, id)?;
    c.push_str("\nCurrent rules:\n```json\n");
    c.push_str(rules.trim());
    c.push_str("\n```\n");

    Ok(CopilotContext {
        system: system_prompt(),
        context: c,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::seed;
    use std::sync::atomic::{AtomicU32, Ordering};

    fn seeded_ws() -> Workspace {
        static N: AtomicU32 = AtomicU32::new(0);
        let n = N.fetch_add(1, Ordering::Relaxed);
        let ws =
            Workspace::new(std::env::temp_dir().join(format!("cop-{}-{n}", std::process::id())));
        seed::ensure_seeded(&ws).unwrap();
        ws
    }

    #[test]
    fn context_includes_state_and_rules() {
        let ws = seeded_ws();
        let id = PolicyId::new("refund-authorization").unwrap();
        let ctx = build_context(&ws, &id, "editor").unwrap();

        assert!(ctx.system.contains("ADVISE and DRAFT only"));
        assert!(ctx.context.contains("refund-authorization"));
        assert!(ctx.context.contains("Surface: editor"));
        assert!(ctx.context.contains("Verification: VALID"));
        assert!(ctx.context.contains("Governance: 2-of-3"));
        assert!(ctx.context.contains("Audit entries:"));
        assert!(ctx.context.contains("auto-approve-small")); // the current rules are embedded
        let _ = std::fs::remove_dir_all(ws.root());
    }

    /// The copilot's source must not reference the keystore — it can never reach a signing key.
    #[test]
    fn copilot_source_does_not_touch_the_keystore() {
        // Needles are assembled from fragments so they don't appear verbatim in this source —
        // otherwise the test would match its own assertions.
        let src = include_str!("copilot.rs");
        let module = ["key", "store::"].concat();
        let signing = ["load_", "signing_key"].concat();
        let master = ["load_", "master_key"].concat();
        assert!(
            !src.contains(&module),
            "copilot must not call into the keystore"
        );
        assert!(!src.contains(&signing) && !src.contains(&master));
    }
}

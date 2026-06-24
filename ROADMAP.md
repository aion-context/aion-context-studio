# Roadmap

Each phase is independently demoable and ships gate-green (fmt · clippy `-D warnings` · test ·
deny · mutants on `studio-core` · `web` build + svelte-check · secret scan), with the relevant
reviewer agents convened before "done" — the aion-trust discipline.

## Phase 0 — Foundation & read-only slice ✅

Workspace (two Rust crates + SvelteKit SPA), the gate, design context (`.impeccable.md`), and a
runnable slice: **list policies → open one → four-guarantee verification**, on real seeded data,
end-to-end through the API and SPA.

## Phase 1 — Authoring ✅

Create a policy (`init_file`), edit rules in the studio, commit a new signed version
(`commit_version`), and a server-computed line diff. The `.aion` format retains only the *current*
rules + a chain of version hashes, so the diff is current→proposed (the pre-commit preview), not
historical-vs-historical.

## Phase 2 — Governance ✅

K-of-N multisig approval over a policy's current version, with live threshold progress
(`multisig::verify_multisig`). The seed registers a 3-signer set; each version needs fresh approval
(a new commit resets it). In this local demo the studio holds every signer's key, so an approval is
recorded by author id and the attestation regenerated at verify time — a real deployment collects
each party's signature separately.

## Phase 3 — Registry ✅

Register authors, rotate and revoke keys (master-key authorized), view the per-author epoch
timeline, and export the trusted JSON for offline verifiers. Rotation/revocation take effect from
`max_policy_version + 1`, so existing signed versions keep verifying.

## Phase 4 — Simulate ✅

A pluggable `Evaluator` over the current rules — propose an action, see the decision (allow /
allow_with_approval / deny / no_match) with the matched rule and a per-rule decision trace (the
"check before it acts" loop). Default rule format is JSON (`rules[]: {id, when:{field:{op,value}},
decision}`, first-match-wins, numeric ops); the seed sample uses it.

## Phase 5 — Audit, compliance, export ✅

The embedded operation log rendered as a timeline (action · who · when · entry hash); a compliance
report (SOX / HIPAA / GDPR / Generic); and JSON / YAML / CSV export for auditors. Presented as
recorded history rather than a re-verified cross-entry hash chain: in current aion-context an
appended entry's stored `previous_hash` is misaligned by one `u64`, so `validate_chain` fails after
the first commit even on an untampered file — and nothing in the library verifies the chain anyway.
Reported upstream: <https://github.com/aion-context/aion-context/issues/141>.

## Phase 6 — Claude copilot ✅

A context-aware Claude copilot on the policy surface: it sees the policy's current rules,
verification, governance, and audit state, and drafts/edits rules, explains, and advises —
streamed over SSE (Anthropic Messages API via `ANTHROPIC_API_KEY`; degrades gracefully if unset).
**Claude advises and drafts; humans apply and sign.** The context assembly lives in `studio-core`
and never touches signing keys (pinned by a test); the network call is isolated in the API layer.

## Phase 7 — Custody (stretch)

A Tauri build with OS-keyring / hardware key custody, retiring the local-demo on-disk keystore.

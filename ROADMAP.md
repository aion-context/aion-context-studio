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

## Phase 7 — Custody (stretch, in progress)

Retire the local-demo on-disk keystore in favour of OS-keyring / hardware custody.

- **Custody abstraction — done.** `studio-core::keystore` now sits behind a `KeyVault` trait:
  `FileVault` (the demo default) and `KeyringVault` (OS keyring via the `keyring` crate, feature
  `keyring`), selected at runtime by `STUDIO_CUSTODY`. The rest of the studio no longer assumes where
  a key lives. Gate-green: the file path is fully mutation-covered; the feature-gated keyring wrapper
  is tested against the in-memory mock and excluded from mutation (untestable headless). See
  [`docs/CUSTODY.md`](docs/CUSTODY.md).
- **Tauri desktop shell — scaffolded.** A native window running the existing axum app with
  `STUDIO_CUSTODY=keyring`. Kept out of the workspace (needs webview libs + a desktop session, so it
  can't build on a headless box / CI); spec and steps in [`tauri/README.md`](tauri/README.md).
- **Embeddable server — done.** `aion-studio-api` exposes `serve(config)` / `serve_on(listener,
  &config)`, so the shell runs the studio on a background task. `main` is a thin caller.
- **Custody-agnostic author index — done.** "Who is registered" lives in `registry/authors.json`
  (`author_index`), not a keys-dir scan, so author enumeration works under either custody.
- **Remaining:** build out the Tauri crate on a desktop, and a first-run command to import/generate
  operator keys into the keyring (since on-disk keys are no longer read under keyring custody).

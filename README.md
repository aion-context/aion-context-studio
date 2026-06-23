# aion-context-studio

**The studio for [aion-context](https://github.com/aion-context/aion-context) — author, govern,
verify, simulate, and audit signed `.aion` policy artifacts, with Claude as a copilot.**

aion-context turns business rules into **signed policy artifacts**: a hash-chained version history,
an Ed25519 signature per version, K-of-N multisig approval, a key registry, an embedded audit
trail, and a `verify_file` that proves four things every time — *structure · integrity ·
hash-chain · signatures*. The library and CLI exist; this is the rich UI on top.

The studio is a **rich SPA** (SvelteKit) over a **thin Rust/axum API** that wraps aion-context.
Claude is a central copilot that drafts and edits rules, explains diffs, and summarizes the audit
trail — but **Claude only advises; every cryptographic action is an explicit, human-signed
operation.** It is a reference implementation in the family of aion-trust and aion-edu.

> **Local single-operator demo — NOT a production custody architecture.** This one process holds
> the operator's own signing keys on disk and binds to loopback only. In production, keys live with
> the parties that hold them; the studio would talk to their signing services. The key registry
> never holds a secret.

## Status

**Phase 0 complete** — workspace + a read-only vertical slice: list policies → open one → see its
four-guarantee verification, on real seeded data, end-to-end through the API and SPA. See
[`ROADMAP.md`](ROADMAP.md).

## Run it

```sh
# one process serves the API and the built SPA on http://127.0.0.1:8787
cd web && npm install && npm run build && cd ..
cargo run -p aion-studio-api
```

Dev (hot reload): `cargo run -p aion-studio-api` + `cd web && npm run dev` (Vite on :5173 proxies
`/api` to the Rust server).

## Layout

- `crates/studio-core` — the testable glue over aion-context (workspace, policies, registry, seed).
- `crates/aion-studio-api` — thin axum API; serves `/api` and the SPA.
- `web/` — the SvelteKit SPA (Archivo · Public Sans · JetBrains Mono; design context in
  [`.impeccable.md`](.impeccable.md)).

## License

MIT OR Apache-2.0.

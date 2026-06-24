# Key custody

How the studio holds the operational and master **signing keys** — and how that changes between the
local demo and a desktop build. The key registry never holds a secret; this is only about the private
signing material.

## The abstraction

`studio-core::keystore` exposes a stable surface — `save_signing_key` / `load_signing_key` /
`save_master_key` / `load_master_key` — backed by a private `KeyVault` trait. Nothing else in the
studio assumes *where* a key lives. The active vault is chosen at runtime by `STUDIO_CUSTODY`:

| `STUDIO_CUSTODY` | Vault | Where secrets live | Build |
|---|---|---|---|
| _(unset)_ / `file` | `FileVault` | hex files under `studio-data/keys/` | default |
| `keyring` | `KeyringVault` | the OS keyring (Keychain / Credential Manager / Secret Service) | requires the `keyring` feature |

## FileVault — the local demo (default)

**NOT a production custody architecture.** A single operator's keys are written as hex under
`studio-data/keys/author-N[.master].key`, and the one process binds to loopback only. This is stated
plainly in the UI and the README. It exists so the demo runs with zero setup.

## KeyringVault — OS custody (`keyring` feature)

Secrets are stored in the platform credential store via the [`keyring`](https://crates.io/crates/keyring)
crate, keyed by service `aion-context-studio` and account `author-N[.master]`. They never touch disk.
Build with the feature **and** a platform store:

```sh
# macOS
cargo build -p studio-core --features keyring   # + keyring/apple-native in the app crate
# Windows: keyring/windows-native   ·   Linux: keyring/sync-secret-service
STUDIO_CUSTODY=keyring cargo run -p aion-studio-api
```

With the feature but no platform store (e.g. headless CI), only the in-memory **mock** store resolves;
the crate's keyring tests use it. The real stores need a desktop session (Keychain unlocked, a running
Secret Service, etc.).

## The Tauri shell (`tauri/`) — Phase 7

A desktop build that retires on-disk keys end to end: it runs the existing axum app with
`STUDIO_CUSTODY=keyring` and a platform store feature, inside a native window. See
[`../tauri/README.md`](../tauri/README.md). It is **not** part of the Cargo workspace or CI — it needs
a desktop toolchain (webview libraries + a display) — so the core gate stays green on a headless box.

## Author enumeration is custody-agnostic

"Who is registered" lives in a small index — `registry/authors.json`, maintained by `author_index`
and written on seed/register — **not** by listing the keys directory. So author enumeration works the
same under file or keyring custody. (Earlier the registry listed `keys/author-N.key`, which returned
nothing under keyring custody; that coupling is gone.) The key registry holds the public keys, the
vault holds the secrets, and the index holds the membership — three separate concerns.

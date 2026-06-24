# aion-context-studio — desktop shell (Phase 7, scaffold)

A [Tauri 2](https://v2.tauri.app) build that runs the studio as a native desktop app with **OS-keyring
key custody**, retiring the on-disk demo keystore. The custody substance already exists and is tested
in `studio-core` (`KeyVault` → `KeyringVault`, feature `keyring`; see [`../docs/CUSTODY.md`](../docs/CUSTODY.md));
this shell is the packaging around it.

> **Why this isn't built in CI / on a headless box.** Tauri needs platform webview libraries
> (webkit2gtk on Linux, WebView2 on Windows, WKWebView on macOS) and a display; an OS keyring needs a
> desktop session (an unlocked Keychain, a running Secret Service, …). So this crate is **deliberately
> kept out of the Cargo workspace** (it has its own `[workspace]` table) — the core gate stays green on
> a headless box, and you build the desktop app on a desktop.

## Architecture

```
┌─ Tauri window (WKWebView / WebView2 / webkit2gtk) ─┐
│  loads  http://127.0.0.1:8787  (the built SPA)      │
└───────────────┬─────────────────────────────────────┘
                │  same axum app as `aion-studio-api`
        ┌───────▼────────┐   STUDIO_CUSTODY=keyring
        │  studio-core    │──────────────► OS keyring (Keychain / Credential Manager / Secret Service)
        └────────────────┘                 (no keys on disk)
```

The shell starts the **existing** server (the `aion-studio-api` axum app) on loopback with
`STUDIO_CUSTODY=keyring`, then opens a window at that URL. No studio code is duplicated; the only new
thing is the window + the custody selection.

## Steps to complete it (on a desktop)

1. **Make the server embeddable.** Add a `lib` target to `aion-studio-api` exposing
   `pub async fn serve(addr: SocketAddr) -> anyhow::Result<()>` (the current `main` becomes a thin
   caller). The Tauri shell then spawns it on a background Tokio task instead of shelling out.
2. **Crate manifest** (`tauri/Cargo.toml`) — standalone workspace:
   ```toml
   [workspace]                      # own workspace → excluded from the parent gate
   [package]
   name = "aion-studio-tauri"
   version = "0.1.0"
   edition = "2021"
   [build-dependencies]
   tauri-build = "2"
   [dependencies]
   tauri = { version = "2", features = [] }
   aion-studio-api = { path = "../crates/aion-studio-api" }
   studio-core = { path = "../crates/studio-core", features = ["keyring", "<platform>-native"] }
   tokio = { version = "1", features = ["rt-multi-thread"] }
   ```
   `<platform>-native` is `apple` / `windows`, or use `sync-secret-service` on Linux.
3. **`tauri/tauri.conf.json`** — point `build.frontendDist` at `../web/dist`, set
   `app.windows[0].url` to `http://127.0.0.1:8787`, title "aion-context studio", 1280×800.
4. **`tauri/src/main.rs`** — on setup: `std::env::set_var("STUDIO_CUSTODY", "keyring")`, spawn
   `aion_studio_api::serve(([127,0,0,1], 8787).into())` on a Tokio task, then run the Tauri app.
5. **First-run key import.** With on-disk keys retired, add a one-time "import operator keys into the
   keyring" command (or generate fresh), since `FileVault` files won't be read.
6. **Author index.** Close the file-custody coupling noted in `docs/CUSTODY.md`: `registry_admin`
   enumerates authors by listing `keys/`; add a custody-agnostic author list so enumeration works
   under keyring custody.

## Build / run (desktop)

```sh
cd web && npm install && npm run build && cd ..   # build the SPA → web/dist
cd tauri && cargo tauri dev                        # or: cargo tauri build
```

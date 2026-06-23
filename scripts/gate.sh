#!/usr/bin/env bash
# Definition of done for aion-context-studio. Run from the repo root.
set -euo pipefail
cd "$(dirname "$0")/.."

echo "▸ cargo fmt --check";   cargo fmt --all -- --check
echo "▸ cargo clippy";        cargo clippy --all-targets --all-features -- -D warnings
echo "▸ cargo test";          cargo test --all
echo "▸ cargo deny";          (command -v cargo-deny >/dev/null && cargo deny check) || echo "  (cargo-deny not installed; skipping)"
echo "▸ cargo mutants";       (command -v cargo-mutants >/dev/null && cargo mutants --in-place --no-shuffle -p studio-core) || echo "  (cargo-mutants not installed; skipping)"

if [ -d web/node_modules ]; then
  echo "▸ web: svelte-check";  (cd web && npm run check)
  echo "▸ web: build";         (cd web && npm run build)
else
  echo "  (web/node_modules absent; run 'cd web && npm install' to gate the SPA)"
fi

echo "✓ gate green"

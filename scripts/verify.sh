#!/usr/bin/env bash
# scripts/verify.sh — full local/CI gate for the cosmic migration.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

log() { printf '\n==> %s\n' "$*"; }
need() { command -v "$1" >/dev/null || { echo "missing: $1" >&2; exit 1; }; }

# --- 0. tools ---
need cargo
need rustc
need flatpak
need flatpak-builder
need weston
need timeout
need python3

# --- 1. host compile ---
log "cargo build --workspace"
cargo build --workspace --all-targets

# --- 2. clippy (CI equivalent of flake clippy) ---
log "cargo clippy --all-targets -- -D warnings"
cargo clippy --workspace --all-targets -- -D warnings

# --- 3. tests ---
log "cargo test --workspace"
cargo test --workspace --all-targets -- --nocapture

# --- 4. Flatpak (offline w.r.t. crates.io; runtimes must already be installed) ---
log "flatpak-builder"
test -f flatpak/cargo-sources.json || {
  echo "flatpak/cargo-sources.json missing; run scripts/generate-cargo-sources.sh" >&2
  exit 1
}
scripts/build-flatpak.sh

# --- 5. smoke ---
log "smoke"
scripts/smoke-flatpak.sh

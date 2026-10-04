#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
cargo fmt --all -- --check
cargo clippy --locked --workspace --all-targets --all-features -- -D warnings
cargo test --locked --workspace --all-targets --all-features
cargo build --locked -p desktop-core
python3 scripts/sync-pubspec-lock.py --check
cd desktop
flutter pub get --enforce-lockfile
dart format --output=none --set-exit-if-changed lib test
flutter analyze
NIXOS_TOOLKIT_CORE_LIBRARY="$ROOT/target/debug/libnixos_toolkit_core.so" flutter test
cd "$ROOT"
if [[ "${1:-}" == "--flatpak" ]]; then
  scripts/build-flatpak.sh
  flatpak install --user --noninteractive -y "dist/nixos-toolkit-0.1.0-$(uname -m).flatpak"
  scripts/smoke-flatpak.sh
fi

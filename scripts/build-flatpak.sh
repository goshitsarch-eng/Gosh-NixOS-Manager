#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
APP_ID=io.github.goshitsarch_eng.NixosToolkit
DIST="${TOOLKIT_DIST_DIR:-$ROOT/dist}"
ARCH="$(uname -m)"
"$ROOT/scripts/package-linux.sh"
python3 "$ROOT/packaging/flatpak/generate-manifest.py" \
  "$DIST/nixos-toolkit-0.1.0-linux-$ARCH.tar.gz" "$ROOT/.flatpak/manifest.json"
flatpak-builder --user --force-clean --disable-rofiles-fuse \
  --repo="$ROOT/.flatpak/repo" "$ROOT/.flatpak/build" "$ROOT/.flatpak/manifest.json"
flatpak build-bundle "$ROOT/.flatpak/repo" "$DIST/nixos-toolkit-0.1.0-$ARCH.flatpak" "$APP_ID"
(cd "$DIST" && sha256sum "nixos-toolkit-0.1.0-$ARCH.flatpak" > "nixos-toolkit-0.1.0-$ARCH.flatpak.sha256")
printf 'Created %s\n' "$DIST/nixos-toolkit-0.1.0-$ARCH.flatpak"

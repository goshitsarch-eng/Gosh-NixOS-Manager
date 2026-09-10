#!/usr/bin/env bash
# Build and install the local Flatpak from a dirty checkout.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

APP_ID="io.github.goshitsarch_eng.NixosToolkit"
MANIFEST="flatpak/${APP_ID}.yml"
REMOTE="nixos-toolkit-local"
BUILD_DIR="${ROOT}/.flatpak/build"
STATE_DIR="${ROOT}/.flatpak/builder"
REPO_DIR="${ROOT}/.flatpak/repo"

need() { command -v "$1" >/dev/null || { echo "missing: $1" >&2; exit 1; }; }
need flatpak
need flatpak-builder

test -f "${ROOT}/flatpak/cargo-sources.json" || {
  echo "flatpak/cargo-sources.json missing; run scripts/generate-cargo-sources.sh" >&2
  exit 1
}

flatpak remote-add --if-not-exists --user flathub \
  https://dl.flathub.org/repo/flathub.flatpakrepo

mkdir -p "${ROOT}/.flatpak"

# --disable-rofiles-fuse: toolboxes/CI often cannot mount rofiles-fuse.
flatpak-builder --user --force-clean --ccache \
  --disable-rofiles-fuse \
  --install-deps-from=flathub \
  --repo="${REPO_DIR}" \
  --state-dir="${STATE_DIR}" \
  "${BUILD_DIR}" \
  "${MANIFEST}"

flatpak --user remote-add --if-not-exists --no-gpg-verify \
  "${REMOTE}" "${REPO_DIR}" || true

flatpak --user install -y --or-update --reinstall \
  "${REMOTE}" "${APP_ID}"

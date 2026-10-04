#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
DIST="${TOOLKIT_DIST_DIR:-$ROOT/dist}"
ARCH="$(uname -m)"
case "$ARCH" in x86_64) FLUTTER_ARCH=x64 ;; aarch64) FLUTTER_ARCH=arm64 ;; *) echo "Unsupported Linux architecture: $ARCH" >&2; exit 1 ;; esac
cd "$ROOT/desktop"
flutter pub get --enforce-lockfile
flutter build linux --release
BUNDLE="$ROOT/desktop/build/linux/$FLUTTER_ARCH/release/bundle"
NAME="nixos-toolkit-0.1.0-linux-$ARCH"
STAGE="$(mktemp -d)"
trap 'rm -rf "$STAGE"' EXIT
mkdir -p "$STAGE/$NAME" "$DIST"
cp -a "$BUNDLE/." "$STAGE/$NAME/"
install -Dm644 "$ROOT/LICENSE" "$STAGE/$NAME/LICENSE"
install -Dm644 "$ROOT/packaging/linux/README.txt" "$STAGE/$NAME/README.txt"
APP_ID=io.github.goshitsarch_eng.NixosToolkit
install -Dm644 "$ROOT/flatpak/$APP_ID.desktop" "$STAGE/$NAME/share/applications/$APP_ID.desktop"
install -Dm644 "$ROOT/data/icons/nixos-toolkit.svg" "$STAGE/$NAME/share/icons/hicolor/scalable/apps/$APP_ID.svg"
install -Dm644 "$ROOT/flatpak/$APP_ID.metainfo.xml" "$STAGE/$NAME/share/metainfo/$APP_ID.metainfo.xml"
flutter --version --machine > "$STAGE/$NAME/flutter-build.json"
rustc --version > "$STAGE/$NAME/rust-build.txt"
test -s "$STAGE/$NAME/lib/libnixos_toolkit_core.so"
test -s "$STAGE/$NAME/lib/libapp.so"
test -s "$STAGE/$NAME/data/templates/profiles/gnome.nix"
chmod -R u=rwX,go=rX "$STAGE/$NAME"
tar -C "$STAGE" -czf "$DIST/$NAME.tar.gz" "$NAME"
(cd "$DIST" && sha256sum "$NAME.tar.gz" > "$NAME.tar.gz.sha256")
chmod 644 "$DIST/$NAME.tar.gz" "$DIST/$NAME.tar.gz.sha256"
printf 'Created %s\n' "$DIST/$NAME.tar.gz"

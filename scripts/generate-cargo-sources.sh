#!/usr/bin/env bash
# Generate flatpak/cargo-sources.json from Cargo.lock.
# Run whenever Cargo.lock changes. Not invoked by scripts/verify.sh.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
TOOLS="${ROOT}/.cache/flatpak-builder-tools"
# Pin so offline/CI regeneration is reproducible. Bump deliberately.
TOOLS_COMMIT="1fc32195e3e60fe5c97f0af646dec7a99df5962b"

need() { command -v "$1" >/dev/null || { echo "missing: $1" >&2; exit 1; }; }
need git
need python3

python3 - <<'PY'
import importlib
missing = []
for mod in ("tomlkit", "aiohttp"):
    try:
        importlib.import_module(mod)
    except ImportError:
        missing.append(mod)
if missing:
    raise SystemExit(f"missing python modules: {', '.join(missing)}")
PY

if [[ ! -d "${TOOLS}/.git" ]]; then
  git clone https://github.com/flatpak/flatpak-builder-tools.git "${TOOLS}"
fi

git -C "${TOOLS}" fetch --depth 1 origin "${TOOLS_COMMIT}"
git -C "${TOOLS}" checkout --detach "${TOOLS_COMMIT}"
git -C "${TOOLS}" submodule update --init --recursive

# Do not pass --git-tarballs: GitHub tarballs omit submodule contents (iced).
python3 "${TOOLS}/cargo/flatpak-cargo-generator.py" \
  "${ROOT}/Cargo.lock" \
  -o "${ROOT}/flatpak/cargo-sources.json"

echo "wrote ${ROOT}/flatpak/cargo-sources.json"

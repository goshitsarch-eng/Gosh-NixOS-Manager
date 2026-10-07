#!/usr/bin/env bash
# Read-only packaged API diagnostics: uses the real Rust library and host probes.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
APP_ID=io.github.goshitsarch_eng.NixosToolkit
LOGDIR="$(mktemp -d)"
export XDG_RUNTIME_DIR="${XDG_RUNTIME_DIR:-$LOGDIR/runtime}"
mkdir -p "$XDG_RUNTIME_DIR"
chmod 700 "$XDG_RUNTIME_DIR"
cleanup() { if [[ -n "${XVFB_PID:-}" ]]; then kill "$XVFB_PID" 2>/dev/null || true; fi; }
trap cleanup EXIT
if [[ -z "${DISPLAY:-}" && -z "${WAYLAND_DISPLAY:-}" ]]; then
  Xvfb -displayfd 3 -screen 0 1280x900x24 -nolisten tcp -extension MIT-SHM \
    3>"$LOGDIR/display" >"$LOGDIR/xvfb.log" 2>&1 &
  XVFB_PID=$!
  for attempt in {1..50}; do
    [[ -s "$LOGDIR/display" ]] && break
    kill -0 "$XVFB_PID" || { cat "$LOGDIR/xvfb.log"; exit 1; }
    sleep 0.1
  done
  test -s "$LOGDIR/display"
  export DISPLAY=":$(cat "$LOGDIR/display")"
fi
dbus-run-session -- timeout 60 flatpak run --user "$APP_ID" --diagnose \
  >"$LOGDIR/diagnostics.json" 2>"$LOGDIR/app.log"
python3 "$ROOT/scripts/check-diagnostics.py" "$LOGDIR/diagnostics.json" --flatpak
printf 'Logs: %s\n' "$LOGDIR"

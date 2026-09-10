#!/usr/bin/env bash
# Headless weston smoke: the Flatpak must stay up without pkexec / panic.
set -euo pipefail

APP_ID="io.github.goshitsarch_eng.NixosToolkit"
SOCKET="nixos-toolkit-smoke-$$"
SMOKE_SECS="${SMOKE_SECS:-10}"
LOGDIR="$(mktemp -d /tmp/nixos-toolkit-smoke.XXXXXX)"
WESTON_LOG="${LOGDIR}/weston.log"
APP_LOG="${LOGDIR}/app.log"
RETRY_LOG="${LOGDIR}/app-retry.log"

cleanup() {
  [[ -n "${APP_PID:-}" ]] && kill "${APP_PID}" 2>/dev/null || true
  [[ -n "${WESTON_PID:-}" ]] && kill "${WESTON_PID}" 2>/dev/null || true
}
trap cleanup EXIT

need() { command -v "$1" >/dev/null || { echo "missing: $1" >&2; exit 1; }; }
need flatpak
need weston
need timeout

flatpak --user info "${APP_ID}" >/dev/null || {
  echo "Flatpak ${APP_ID} is not installed; run scripts/build-flatpak.sh" >&2
  exit 1
}

export XDG_RUNTIME_DIR="${XDG_RUNTIME_DIR:-/tmp/nixos-toolkit-xdg-$$}"
mkdir -p "${XDG_RUNTIME_DIR}"
chmod 700 "${XDG_RUNTIME_DIR}"

if [[ "${SMOKE_USE_EXISTING_DISPLAY:-}" == "1" ]]; then
  echo "using existing WAYLAND_DISPLAY=${WAYLAND_DISPLAY:-} DISPLAY=${DISPLAY:-}"
else
  # Dedicated compositor so we do not nest inside the existing session.
  # pixman: no GPU required of weston itself; the client still gets --device=dri.
  weston --backend=headless --renderer=pixman \
    --socket="${SOCKET}" --idle-time=0 --width=1280 --height=800 \
    --no-config >"${WESTON_LOG}" 2>&1 &
  WESTON_PID=$!
  sleep 0.5
  kill -0 "${WESTON_PID}" || { echo "weston failed"; cat "${WESTON_LOG}"; exit 1; }
  export WAYLAND_DISPLAY="${SOCKET}"
  unset DISPLAY
fi

run_app() {
  local logfile="$1"
  shift
  set +e
  timeout --signal=TERM --kill-after=5 "${SMOKE_SECS}" \
    flatpak run --user \
      --env=NIXOS_TOOLKIT_SKIP_PRIVILEGED_INIT=1 \
      --env=NIXOS_TOOLKIT_SKIP_HOST_PROBES=1 \
      --env=RUST_BACKTRACE=1 \
      "$@" \
      "${APP_ID}" >"${logfile}" 2>&1
  local status=$?
  set -e
  echo "${status}"
}

log_is_bad() {
  grep -Eiq 'panic|SIGSEGV|Aborted|failed to create (surface|adapter)|no adapter' "$1"
}

APP_STATUS="$(run_app "${APP_LOG}" --env=WGPU_BACKEND=gl)"

if { [[ "${APP_STATUS}" -ne 124 && "${APP_STATUS}" -ne 143 ]] || log_is_bad "${APP_LOG}"; } \
  && [[ "${APP_STATUS}" -ne 137 ]]; then
  echo "first run status=${APP_STATUS}; retrying without WGPU_BACKEND=gl"
  APP_STATUS="$(run_app "${RETRY_LOG}")"
  cat "${RETRY_LOG}" >>"${APP_LOG}"
fi

echo "flatpak run status=${APP_STATUS} (124 = still running at timeout: OK)"
echo "---- app stderr/stdout (${APP_LOG}) ----"
cat "${APP_LOG}"

fail=0
if log_is_bad "${APP_LOG}"; then
  echo "SMOKE FAIL: crash/adapter error in log"
  fail=1
fi
if grep -Eiq 'pkexec|polkit.*authentication|Authentication is required' "${APP_LOG}"; then
  echo "SMOKE FAIL: privileged helper / pkexec invoked"
  fail=1
fi
if [[ "${APP_STATUS}" -eq 0 ]]; then
  echo "SMOKE FAIL: app exited before ${SMOKE_SECS}s"
  fail=1
fi
if [[ "${APP_STATUS}" -eq 137 ]]; then
  echo "SMOKE FAIL: needed SIGKILL"
  fail=1
fi
# 124 (timeout) or 143 (SIGTERM) are the expected "still running" codes.
if [[ "${APP_STATUS}" -ne 124 && "${APP_STATUS}" -ne 143 && "${fail}" -eq 0 ]]; then
  echo "SMOKE FAIL: unexpected exit ${APP_STATUS}"
  fail=1
fi

if [[ "${fail}" -ne 0 ]]; then
  if [[ -f "${WESTON_LOG}" ]]; then
    echo "---- weston log ----"
    cat "${WESTON_LOG}"
  fi
  exit 1
fi
echo "SMOKE OK"
echo "logs: ${LOGDIR}"

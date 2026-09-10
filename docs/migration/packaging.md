# Packaging, sandbox, tests, and verification

This is the packaging and QA plan for the GTK4 → libcosmic migration of NixOS Toolkit. It is the source of truth for `flatpak/`, `tests/`, `scripts/`, and `.github/workflows/verify.yml`. It does not change application behavior by itself; architecture owns `Message` / `update()`, this workstream owns how the GUI is built, sandboxed, tested, and shipped.

**Definition of done for every implementation task:** `scripts/verify.sh` is green. That script always runs `cargo build`, `cargo clippy --all-targets -- -D warnings`, and `cargo test`, then `flatpak-builder` plus the headless smoke test. Task 1 (skeleton libcosmic window) must therefore produce a *working* (minimal) Flatpak and `scripts/verify.sh`, not a stub.

Host used for this plan (2026-09-10): Fedora 44 toolbox, `rustc`/`cargo` 1.98, Flatpak 1.18.2, `flatpak-builder` 1.4.10, weston 15.0.1, `DISPLAY=:1` and `WAYLAND_DISPLAY=wayland-1` already present, **no** `xvfb-run`, **no** `nix` on `PATH`. Freedesktop Platform/SDK **25.08** and `org.freedesktop.Sdk.Extension.rust-stable//25.08` (Rust 1.98.1) are already installed as user runtimes, plus Mesa and NVIDIA GL extensions.

---

## 1. Identity: App ID, runtime, SDK, command

### App ID

**Canonical ID: `io.github.goshitsarch_eng.NixosToolkit`**

| Candidate | Verdict |
|---|---|
| `org.nixos-toolkit.app` (current GTK `APP_ID`) | **Invalid.** Hyphen in a non-last D-Bus component. Flathub also forbids generic suffixes (`.app`, `.desktop`, `.linux`). |
| `org.nixos_toolkit.NixosToolkit` | D-Bus-legal, but we do not control `nixos_toolkit.org` / `nixos-toolkit.org`. Weak Flathub verification. |
| `io.github.goshitsarch.NixosToolkit` | Wrong GitHub user. The repo is `github.com/goshitsarch-eng/Gosh-NixOS-Manager`. |
| **`io.github.goshitsarch_eng.NixosToolkit`** | **Chosen.** Four components as required for code-hosting IDs. Underscore demangles to `goshitsarch-eng`. D-Bus-legal. Flathub-verifiable against this GitHub account. |

Rust constant (architecture must switch from `org.nixos-toolkit.app`):

```rust
pub const APP_ID: &str = "io.github.goshitsarch_eng.NixosToolkit";
```

This ID is also the D-Bus well-known name used by libcosmic `single-instance`. Do not put hyphens in it.

### Command name

**`nixos-toolkit`** — matches today's `data/nixos-toolkit.desktop` `Exec=` line and the flake `mainProgram`. The GTK crate currently emits a binary named `gui`; the libcosmic crate must install `nixos-toolkit`.

Privileged host binary remains **`nixos-toolkit-helper`** (already the helper crate's `[[bin]]` name). The Flatpak does **not** ship it.

### Runtime / SDK / extensions

| Field | Value | Why |
|---|---|---|
| `runtime` | `org.freedesktop.Platform` | libcosmic is iced/winit/wgpu, not GTK. No GNOME/KDE runtime. |
| `runtime-version` | `"25.08"` | Already installed here. `rust-stable//25.08` is 1.98.1 (libcosmic wants `rust-version = "1.93"`, edition 2024). Mesa + NVIDIA GL extensions exist for this branch. |
| `sdk` | `org.freedesktop.Sdk` | Matches the platform. |
| `sdk-extensions` | `org.freedesktop.Sdk.Extension.rust-stable` | Cargo/rustc at `/usr/lib/sdk/rust-stable/bin`. |
| `base` | **none** | Do **not** use `com.system76.Cosmic.BaseApp`. The app must run on weston/GNOME/KDE without COSMIC. BaseApp pulls COSMIC theme/config permissions we do not want as required. |
| LLVM extension | **not required** | wgpu/naga compile shaders in-process. llvm/libclang is only needed if a crate uses bindgen (camera does; we do not). Add `org.freedesktop.Sdk.Extension.llvm21` only if a future dep forces it. |

Do **not** target 24.08 unless 25.08 is unavailable on a builder. This host and the intended CI image should pin 25.08.

---

## 2. Manifest sketch

Path: `flatpak/io.github.goshitsarch_eng.NixosToolkit.yml`

Local development uses `type: dir` so `flatpak-builder` can build a dirty checkout. Flathub later swaps that for a tagged git source; `cargo-sources.json` stays committed either way.

```yaml
id: io.github.goshitsarch_eng.NixosToolkit
runtime: org.freedesktop.Platform
runtime-version: "25.08"
sdk: org.freedesktop.Sdk
sdk-extensions:
  - org.freedesktop.Sdk.Extension.rust-stable
command: nixos-toolkit
finish-args:
  # Display: Wayland first, X11 fallback (winit x11 + wayland features).
  - --socket=wayland
  - --socket=fallback-x11
  - --share=ipc
  # wgpu / Mesa / NVIDIA GL. Required for GPU path; tiny-skia still works without it
  # but smoke on this host should exercise dri.
  - --device=dri
  # Host helper: flatpak-spawn --host pkexec nixos-toolkit-helper.
  # This is the sandbox escape that makes NixOS management possible.
  # flathub-linter: skip=finish-args-flatpak-spawn-access
  - --talk-name=org.freedesktop.Flatpak
  # Unprivileged NixOS detection without a pkexec prompt on startup.
  # Sandbox /etc/os-release is the *runtime*, not the host — do not overlay it.
  - --filesystem=/etc/NIXOS:ro
  - --filesystem=/etc/nixos:ro
  - --filesystem=/etc/hostname:ro
  # No --share=network (see §3).
  # No --filesystem=host.
  # No --device=all.
  # Portals (file chooser, clipboard, settings) need no extra talk-names.

build-options:
  append-path: /usr/lib/sdk/rust-stable/bin
  env:
    CARGO_HOME: /run/build/nixos-toolkit/cargo
    CARGO_NET_OFFLINE: "true"

cleanup:
  - /include
  - /lib/pkgconfig
  - /share/doc
  - /share/man
  - "*.a"

modules:
  - name: nixos-toolkit
    buildsystem: simple
    build-options:
      append-path: /usr/lib/sdk/rust-stable/bin
      env:
        CARGO_HOME: /run/build/nixos-toolkit/cargo
        CARGO_NET_OFFLINE: "true"
        # mold is bundled in rust-stable; optional speed-up.
        RUSTFLAGS: "-C link-arg=-fuse-ld=mold"
    build-commands:
      # Generator writes cargo/config; cargo only reads .cargo/config.toml
      # when CARGO_HOME is overridden by some toolchains. Install both.
      - mkdir -p .cargo
      - cp -f cargo/config .cargo/config.toml
      - cargo --offline fetch --locked --verbose
      - cargo --offline build --locked --release -p gui --bin nixos-toolkit
      - install -Dm755 target/release/nixos-toolkit -t ${FLATPAK_DEST}/bin/
      - install -Dm644 data/icons/nixos-toolkit.svg
          ${FLATPAK_DEST}/share/icons/hicolor/scalable/apps/${FLATPAK_ID}.svg
      - install -Dm644 flatpak/${FLATPAK_ID}.desktop
          ${FLATPAK_DEST}/share/applications/${FLATPAK_ID}.desktop
      - install -Dm644 flatpak/${FLATPAK_ID}.metainfo.xml
          ${FLATPAK_DEST}/share/metainfo/${FLATPAK_ID}.metainfo.xml
      # Preview-only templates (apply still uses the host helper's copy).
      - mkdir -p ${FLATPAK_DEST}/share/nixos-toolkit
      - cp -a nix/templates ${FLATPAK_DEST}/share/nixos-toolkit/templates
    sources:
      - type: dir
        path: ..
      - cargo-sources.json
```

Task 1 may drop the templates copy and `-p gui --bin nixos-toolkit` until the crate exists; the rest of the skeleton (ID, runtime, finish-args, cargo vendor, desktop/metainfo/icon, `command`) must be real.

### Cargo git vendoring (mandatory)

libcosmic is a **git** dependency (`https://github.com/pop-os/libcosmic.git`) with further git deps (`cosmic-protocols`, `dbus-settings-bindings`, `freedesktop-icons`, `cosmic-settings-daemon`, …) and an **iced git submodule** inside libcosmic. crates.io archives are not enough.

**Do not use `cargo vendor` as the primary path.** libcosmic participates in a cyclic git graph (`cosmic-settings-config` → `libcosmic` → `cosmic-config`) that makes `cargo vendor` fail with duplicate `cosmic-config` (rust-lang/cargo#15745). That is why cosmic-utils Camera and fan-control ship `cargo-sources.json` from `flatpak-cargo-generator`.

Current generator **does** run `git submodule update --init --recursive` after checkout, which is what fixed the 2024 `iced/Cargo.toml` `FileNotFoundError`. **Do not pass `--git-tarballs`**: GitHub tarballs omit submodule contents, so iced would be empty.

#### Exact generator command

Pinned clone of [flatpak/flatpak-builder-tools](https://github.com/flatpak/flatpak-builder-tools) (script records the commit). Requires Python 3.9+ plus `tomlkit` and `aiohttp` (optional `PyYAML`). This Fedora toolbox currently lacks those Python modules — install them before regenerating (see §6).

```bash
# scripts/generate-cargo-sources.sh (committed; run whenever Cargo.lock changes)
set -euo pipefail
ROOT="$(git rev-parse --show-toplevel)"
TOOLS="${ROOT}/.cache/flatpak-builder-tools"
# Pin so offline/CI regeneration is reproducible. Bump deliberately.
TOOLS_COMMIT="master"  # replace with a SHA the first time we generate for real

if [[ ! -d "${TOOLS}/.git" ]]; then
  git clone --depth 1 https://github.com/flatpak/flatpak-builder-tools.git "${TOOLS}"
fi
git -C "${TOOLS}" fetch --depth 1 origin "${TOOLS_COMMIT}"
git -C "${TOOLS}" checkout --detach "${TOOLS_COMMIT}"

python3 "${TOOLS}/cargo/flatpak-cargo-generator.py" \
  "${ROOT}/Cargo.lock" \
  -o "${ROOT}/flatpak/cargo-sources.json"
```

One-shot equivalent after a manual clone:

```bash
python3 .cache/flatpak-builder-tools/cargo/flatpak-cargo-generator.py \
  Cargo.lock -o flatpak/cargo-sources.json
```

**Commit `flatpak/cargo-sources.json`.** Offline `flatpak-builder` is a requirement. Regenerating it is a dedicated commit whenever `Cargo.lock` changes (`chore(flatpak): regenerate cargo-sources.json`). `scripts/verify.sh` does **not** regenerate it (needs network + aiohttp); it only builds from the committed file.

The generated file contains:

- crates.io `.crate` archives with sha256, dest `cargo/vendor/<name>-<version>`
- git sources, dest `flatpak-cargo/git/<repo>-<7-char-sha>`
- shell `cp -r` into `cargo/vendor/<crate>` plus inline `Cargo.toml` / `.cargo-checksum.json`
- inline `cargo/config` mapping `crates-io` and each git URL to `vendored-sources`

Build commands must `cp cargo/config .cargo/config.toml` because setting `CARGO_HOME=/run/build/nixos-toolkit/cargo` otherwise hides that file (documented upstream failure mode: `can't checkout from '$git-url': you are in the offline mode`).

### Finish-args summary

| Arg | Why |
|---|---|
| `--socket=wayland` | Primary display. |
| `--socket=fallback-x11` | winit `x11` feature; GNOME/KDE X11 sessions; nested XWayland. |
| `--share=ipc` | X11 SHM; harmless on pure Wayland. |
| `--device=dri` | wgpu. |
| `--talk-name=org.freedesktop.Flatpak` | `flatpak-spawn --host` → host helper. |
| `--filesystem=/etc/NIXOS:ro` | Marker file. Sandbox `/etc` is *not* the host. |
| `--filesystem=/etc/nixos:ro` | Integration detection + onboarding without pkexec. |
| `--filesystem=/etc/hostname:ro` | Hostname field default. |
| **omitted `--share=network`** | Channel updates and `nixos-rebuild` run in the **host** helper. Preview, prefs, and onboarding are local. Add network later only if we grow an in-app nixpkgs search. |
| **omitted `--filesystem=host`** | Replaced by the three narrow `:ro` paths plus spawn-host. |
| **omitted Cosmic settings daemon talk-names** | Optional COSMIC theming; `xdg-portal` covers light/dark on GNOME/KDE/weston. |

xdg-desktop-portal (file chooser, clipboard via libcosmic `xdg-portal` / ashpd) is available without extra `--talk-name` entries.

---

## 3. Helper / sandbox strategy

### Why the helper cannot live in the Flatpak

The helper writes `/etc/nixos/nixos-toolkit/**`, runs allowlisted `nixos-rebuild` / `nix-env` / `nix-collect-garbage` / `nix-store` / `nix-channel --update`, and is invoked with **pkexec**. A sandboxed copy fails for all of:

- pkexec will not execute a binary from inside the sandbox as the policy subject.
- Today's polkit file annotates `org.freedesktop.policykit.exec.path` as **`/run/current-system/sw/bin/nixos-toolkit-helper`** (host NixOS path).
- `nixos-rebuild`, the Nix store, and `/etc/nixos` are host tools/paths.
- `--filesystem=/etc/nixos:rw` plus PolicyKit talk still cannot make pkexec run the sandboxed binary, and still cannot see host `nixos-rebuild`.

**Design: Flatpak = GUI only. Privileged work = host helper.**

```
┌──────────────────────────────────────────┐     flatpak-spawn --host      ┌─────────────────────────────────┐
│  Flatpak GUI (unprivileged)              │ ── pkexec nixos-toolkit-     →│  Host helper (privileged)        │
│  io.github.goshitsarch_eng.NixosToolkit  │     helper  (JSON lines)      │  writes /etc/nixos/nixos-toolkit │
│  preview, prefs, onboarding              │                               │  nixos-rebuild / generations     │
└──────────────────────────────────────────┘                               └─────────────────────────────────┘
```

IPC stays the existing line-delimited JSON protocol in `crates/common/src/ipc.rs` (`HelperRequest` / `HelperResponse` over stdin/stdout).

### How the GUI launches the helper

Architecture owns `crates/gui` helper client. Required behavior:

1. If `FLATPAK_ID` is set (or `/ .flatpak-info` exists):

   ```text
   flatpak-spawn --host -- pkexec <absolute-host-helper>
   ```

   Candidate absolute paths, first existing **on the host** (probe with `flatpak-spawn --host -- test -x …`, never with sandbox `Path::exists` on `/usr`):

   - `$NIXOS_TOOLKIT_HELPER` if the host wrapper exported it
   - `/run/current-system/sw/bin/nixos-toolkit-helper` (NixOS module; matches polkit)
   - `/usr/local/bin/nixos-toolkit-helper`
   - `/usr/bin/nixos-toolkit-helper`

2. Else (dev binary / Nix wrap): today's `pkexec` + `NIXOS_TOOLKIT_HELPER` / relative-to-exe / PATH logic, including `.env_remove("SHELL")`.

3. Keep stdin/stdout piped. If spawn drops the pipe, add `flatpak-spawn --host --forward-fd=0 --forward-fd=1 -- …`.

A small wrapper installed *inside* the Flatpak is optional sugar; the Rust client should still implement the `FLATPAK_ID` branch so tests can stub it.

```sh
# optional: /app/libexec/nixos-toolkit-host-helper
#!/bin/sh
set -eu
for p in \
  "${NIXOS_TOOLKIT_HELPER:-}" \
  /run/current-system/sw/bin/nixos-toolkit-helper \
  /usr/local/bin/nixos-toolkit-helper \
  /usr/bin/nixos-toolkit-helper
do
  [ -n "$p" ] || continue
  if flatpak-spawn --host -- test -x "$p"; then
    exec flatpak-spawn --host -- pkexec "$p" "$@"
  fi
done
echo "nixos-toolkit-helper not found on host" >&2
exit 127
```

### Polkit stays on the host

`data/polkit/org.nixos-toolkit.helper.policy` is **not** installed by the Flatpak. It is installed by the host helper package (NixOS module today: `programs.nixos-toolkit.enable`, flake `packages.helper`). The `exec.path` annotation must keep pointing at the host binary.

Users of the Flatpak on NixOS still need the helper on the host. Onboarding copy must say that.

### GUI behavior when the helper is absent

This is the Fedora / CI / first-run-without-module case. The window **must still start**.

| Feature | Without helper |
|---|---|
| Window, navigation, theme prefs (`~/.var/app/$APP_ID/…`) | Works |
| Profile / bundle / package / hardware / network / services editors | Works (local state) |
| Nix preview | Works from `common` + bundled templates (`NIXOS_TOOLKIT_TEMPLATES_DIR=/app/share/nixos-toolkit/templates`). Do not require `Generate` over IPC for preview. |
| Onboarding snippets (`classic_integration_snippet` / `flake_integration_snippet`) | Works |
| NixOS / integration detection | Best-effort from `:ro` binds (`/etc/NIXOS`, `/etc/nixos`). If those files are missing (Fedora CI), `is_nixos = false`, `ConfigMode::Unknown`, `IntegrationStatus::Unknown` — **no crash**. Authoritative detection remains helper `GetSystemInfo` when the helper is up. |
| Apply / rebuild | Disabled. Error: host helper missing; install the NixOS module / `nixos-toolkit-helper`, then authenticate with pkexec. |
| Generations (list / rollback / delete) | Disabled, same error. Do **not** call `pkexec nix-env` from the GUI (today's `generations.rs` does this — architecture must route it through the helper so the Flatpak path works). |
| Maintenance (`nix-collect-garbage`, `nix-store --optimise/--verify`, `nix-channel --update`) | Disabled, same error. Channel updates run on the host; that is why the GUI sandbox does not need `--share=network`. |

Banner copy (exact string can be tweaked in UI, meaning must stay):

> Privileged helper not found. You can preview configuration and save local preferences. Apply, generations, and maintenance require `nixos-toolkit-helper` on the host (NixOS Toolkit system module) and will ask for authentication.

### Detection vs sandbox `/etc`

`crates/gui/src/integration.rs` currently reads `/etc/NIXOS`, `/etc/os-release`, `/etc/nixos/configuration.nix`, `/etc/nixos/flake.nix`, `/etc/hostname` from the **GUI process**. Inside Flatpak:

- `/etc/os-release` is Freedesktop Platform, not NixOS. **Do not bind-mount host `/etc/os-release` over it** (breaks the runtime).
- `/etc/NIXOS` and `/etc/nixos` do not exist unless we grant them — hence the narrow `:ro` finish-args.
- Version string: prefer helper `GetSystemInfo`; fallback `flatpak-spawn --host -- cat /etc/os-release` only if we need it without pkexec. Not required for smoke.

Architecture should treat helper `GetSystemInfo` as authoritative and keep the `:ro` binds as a no-prompt fallback for onboarding. Moving *all* detection into the helper would force a pkexec dialog on every startup — reject that.

### Portals / “open /etc/nixos”

- Clipboard: libcosmic `xdg-portal` (ashpd). No extra finish-args.
- File open of host `/etc/nixos`: the GTK file portal cannot usefully expose that path. Use `flatpak-spawn --host -- xdg-open /etc/nixos` (or `xdg-open` outside the sandbox). Document a fallback error if spawn-host is missing.
- Do not request `--filesystem=home`.

---

## 4. Desktop, metainfo, icons

| File | Installs as |
|---|---|
| `data/icons/nixos-toolkit.svg` (existing 48×48-viewBox SVG) | `/app/share/icons/hicolor/scalable/apps/io.github.goshitsarch_eng.NixosToolkit.svg` |
| `flatpak/io.github.goshitsarch_eng.NixosToolkit.desktop` | `/app/share/applications/…` |
| `flatpak/io.github.goshitsarch_eng.NixosToolkit.metainfo.xml` | `/app/share/metainfo/…` |

Keep `data/nixos-toolkit.desktop` for the Nix package (Exec/Icon `nixos-toolkit`). Flatpak copies are App-ID-named as Flathub requires.

Desktop sketch:

```ini
[Desktop Entry]
Name=NixOS Toolkit
GenericName=System Configuration
Comment=Manage your NixOS system declaratively
Exec=nixos-toolkit
Icon=io.github.goshitsarch_eng.NixosToolkit
Terminal=false
Type=Application
Categories=System;Settings;
Keywords=nix;nixos;system;configuration;settings;
StartupWMClass=io.github.goshitsarch_eng.NixosToolkit
StartupNotify=true
```

Metainfo must include: `id` = App ID, `<launchable type="desktop-id">`, GPL-3.0-or-later, developer, at least one `<screenshot>` (can be a placeholder until UI exists), `<content_rating type="oars-1.1"/>`, a `<releases>` entry. Do **not** advertise `<provides><id>com.system76.CosmicApplication</id></provides>` — this app is a general Linux GUI that happens to use libcosmic, and must run outside COSMIC.

PNG sizes are optional; SVG is enough for Flathub if the icon is valid.

---

## 5. Test plan

### Architecture contract (required for tests)

The GUI crate is **bin-only** today (`crates/gui/src/main.rs`). Tests cannot drive `update()` without a window unless architecture adds a library target.

Required of architecture:

```toml
# crates/gui/Cargo.toml
[lib]
name = "nixos_toolkit_gui"
path = "src/lib.rs"

[[bin]]
name = "nixos-toolkit"
path = "src/main.rs"
```

`lib.rs` re-exports `App` (or `Model`), `Message`, `AppState`, `update`, preferences, and a helper client **trait**. `update` must be callable with a fake helper; do not spawn pkexec from unit tests.

Workspace is virtual (no root `[package]`), so a top-level `tests/*.rs` is **not** picked up by `cargo test --workspace` unless it is a member. We add a member.

### Layout

```
crates/gui/src/lib.rs                 # architecture — lib target
crates/gui/src/main.rs                # thin binary
crates/common/src/*.rs                # #[cfg(test)] for nix gen, ipc round-trip, actions
crates/helper/src/*.rs                # #[cfg(test)] allowlist, path constants (no rebuilds)

tests/Cargo.toml                      # member package: toolkit-tests
tests/src/lib.rs                      # empty or shared helpers
tests/src/bin/fake-helper.rs          # JSON-line helper double
tests/gui_state.rs                    # every Message variant that update() handles
tests/prefs.rs                        # settings persistence (temp XDG)
tests/preview.rs                      # local Nix preview without helper
tests/flows/mod.rs
tests/flows/onboarding.rs
tests/flows/profiles.rs
tests/flows/bundles.rs
tests/flows/packages.rs
tests/flows/hardware.rs
tests/flows/network.rs
tests/flows/services.rs
tests/flows/apply.rs
tests/flows/generations.rs
tests/flows/maintenance.rs
tests/support/mod.rs
tests/support/fake_helper.rs          # spawn fake-helper, pair with App
tests/support/xdg.rs                  # temp HOME/XDG_* 
```

Workspace `Cargo.toml` members gain `"tests"`.

### Unit tests — every `Message` variant

`tests/gui_state.rs` is table-driven. For each variant `update()` matches:

- construct `App` with a default `AppState` and a `NullHelper` (or recorded fake)
- dispatch the message
- assert the resulting `AppState` / flags / error banner
- assert whether a helper request was sent (and which `HelperRequest`)

Enforcement: a test (or a `#[cfg(test)]` helper in the GUI crate) uses an exhaustive `match Message { … }` so adding a variant without a case fails to compile. Packaging owns the tests; architecture owns the enum — when they add a variant, CI fails until the test table is updated.

Also cover:

- `AppState` profile/bundle/package/hostname/DNS/groups helpers (today in `crates/gui/src/state.rs`)
- `UserPreferences` load/save under overridden XDG (no glib; use `dirs` / explicit path)
- `common` Nix generation snapshots for a profile+bundle combo
- helper allowlist rejects unknown `RunMaintenance` strings
- IPC JSON round-trip for every `HelperRequest` / `HelperResponse` discriminant

### Integration tests — flows, not pixels

Each `tests/flows/*.rs` drives a user journey **only** through `Message` and the fake helper:

| Flow | Fake helper replies | Assert |
|---|---|---|
| Onboarding, helper missing | spawn fails | banner shown; Apply disabled; snippets present |
| Onboarding, not integrated | `GetSystemInfo` → `NotIntegrated` | snippet + status |
| Select profile + bundles + custom package | none | state + preview text contains package names |
| Apply success | `EnsureDirectories` Ok, `Apply` logs + `ApplyComplete`, `WriteState` Ok | success; state marked saved |
| Apply failure | `ApplyComplete { success: false }` | error surface; `has_changes` still true |
| Generations list/rollback | `ListGenerations` / `RollbackGeneration` | state lists; no raw `pkexec nix-env` |
| Maintenance allowlisted | `RunMaintenance` | stdout shown |
| Maintenance rejected | `Error { "Command not allowed" }` | error shown |

Fake helper (`tests/src/bin/fake-helper.rs`):

- reads JSON lines from stdin, writes JSON lines to stdout
- scripted via env `FAKE_HELPER_SCRIPT=/path/to.jsonl` or a built-in scenario name
- never touches `/etc/nixos`
- used by setting `NIXOS_TOOLKIT_HELPER` to its absolute path and **not** using pkexec (`HelperClient::spawn()` / a test-only constructor)

No screenshot / pixel tests.

### Smoke test

See §7. Not a unit test; invoked from `scripts/verify.sh`.

---

## 6. `scripts/verify.sh`

Runnable from a clean checkout. Fail fast (`set -euo pipefail`). Does **not** require `nix`. Does **not** regenerate `cargo-sources.json`.

```bash
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
```

`scripts/build-flatpak.sh` wraps:

```bash
flatpak-builder --user --force-clean \
  --install-deps-from=flathub \
  --repo="${ROOT}/.flatpak/repo" \
  --state-dir="${ROOT}/.flatpak/builder" \
  "${ROOT}/.flatpak/build" \
  flatpak/io.github.goshitsarch_eng.NixosToolkit.yml

flatpak --user remote-add --if-not-exists --no-gpg-verify \
  nixos-toolkit-local "${ROOT}/.flatpak/repo" || true
flatpak --user install -y --reinstall \
  nixos-toolkit-local io.github.goshitsarch_eng.NixosToolkit
```

`.flatpak/` is gitignored. First run downloads the 25.08 runtime/SDK if missing (needs network + Flathub remote). Subsequent runs reuse `--state-dir`.

### System packages (Fedora 44 toolbox)

Already present here: `rustc`, `cargo`, `clang`, `pkg-config`, `flatpak`, `flatpak-builder`, `weston`, `libxkbcommon-devel`, `wayland-devel`, `fontconfig-devel`, `freetype-devel`.

Install before first libcosmic **host** build / cargo-sources regen:

```bash
sudo dnf install -y \
  expat-devel mesa-libGL-devel mesa-libEGL-devel \
  python3-tomlkit python3-aiohttp python3-pyyaml
```

libcosmic native deps (Pop!_OS list mapped to Fedora): `libxkbcommon`, `expat`, `fontconfig`, `freetype`, `wayland`, Mesa. cmake/`just` are not required for our cargo build.

Runtimes (already installed on this host):

```bash
flatpak remote-add --if-not-exists --user flathub https://dl.flathub.org/repo/flathub.flatpakrepo
flatpak install --user -y flathub \
  org.freedesktop.Platform//25.08 \
  org.freedesktop.Sdk//25.08 \
  org.freedesktop.Sdk.Extension.rust-stable//25.08
```

`nix` is **not** a verify.sh dependency. `flake.nix` still wraps GTK with `wrapGAppsHook4`; it is out of this workstream unless the lead assigns it. Cargo + Flatpak on this Fedora host is the gate.

---

## 7. `scripts/smoke-flatpak.sh`

Goal: the installed Flatpak **starts**, stays alive for a fixed interval **without panic / wgpu hard-fail**, then is killed cleanly. No pixel matching. Helper is expected **absent** on Fedora — that path is part of the smoke.

Weston 15 on this host: `--backend=headless` (not the old `headless-backend.so` name). `--idle-time=0` is required so Weston keeps sending frame callbacks. There is no `xvfb-run`.

```bash
#!/usr/bin/env bash
set -euo pipefail
APP_ID=io.github.goshitsarch_eng.NixosToolkit
SOCKET="nixos-toolkit-smoke-$$"
SMOKE_SECS="${SMOKE_SECS:-10}"
LOGDIR="$(mktemp -d /tmp/nixos-toolkit-smoke.XXXXXX)"
WESTON_LOG="${LOGDIR}/weston.log"
APP_LOG="${LOGDIR}/app.log"

cleanup() {
  [[ -n "${APP_PID:-}" ]] && kill "${APP_PID}" 2>/dev/null || true
  [[ -n "${WESTON_PID:-}" ]] && kill "${WESTON_PID}" 2>/dev/null || true
}
trap cleanup EXIT

# Dedicated compositor so we do not nest inside the existing wayland-1 session.
# pixman: no GPU required of weston itself; the client still gets --device=dri.
weston --backend=headless --renderer=pixman \
  --socket="${SOCKET}" --idle-time=0 --width=1280 --height=800 \
  --no-config >"${WESTON_LOG}" 2>&1 &
WESTON_PID=$!
sleep 0.5
kill -0 "${WESTON_PID}" || { echo "weston failed"; cat "${WESTON_LOG}"; exit 1; }

export WAYLAND_DISPLAY="${SOCKET}"
unset DISPLAY   # force Wayland; fallback-x11 still exists if winit needs it

set +e
timeout --signal=TERM --kill-after=5 "${SMOKE_SECS}" \
  flatpak run --user "${APP_ID}" >"${APP_LOG}" 2>&1
APP_STATUS=$?
set -e

# timeout(1) returns 124 on SIGTERM after the interval — that is success
# (app was still running). 137 = SIGKILL after grace: fail.
# 0 = app exited by itself before the interval: inspect log; treat as fail
# unless we later add a --quit-after flag.
echo "flatpak run status=${APP_STATUS} (124 = still running at timeout: OK)"
echo "---- app stderr/stdout (${APP_LOG}) ----"
cat "${APP_LOG}"

fail=0
if grep -Eiq 'panic|SIGSEGV|Aborted|failed to create (surface|adapter)|no adapter' "${APP_LOG}"; then
  echo "SMOKE FAIL: crash/adapter error in log"
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
  echo "---- weston log ----"; cat "${WESTON_LOG}"
  exit 1
fi
echo "SMOKE OK"
```

wgpu falling back to tiny-skia is **success** if the process stays up; the log should still be archived. A hard adapter panic is **failure**.

Optional env `SMOKE_USE_EXISTING_DISPLAY=1` skips weston and uses the already-running `WAYLAND_DISPLAY=wayland-1` on this workstation (manual only, not default CI).

---

## 8. CI

There is no `.github/` in the repo today. This workstream **claims** `.github/workflows/verify.yml`.

Two jobs:

**`cargo`** (always, fast):

- `ubuntu-24.04` (or `fedora-latest` if available)
- install: `libxkbcommon-dev`, `libexpat1-dev`, `libwayland-dev`, `libfontconfig1-dev`, `libfreetype-dev`, `mesa-common-dev`, `pkg-config`, `clang`
- `dt-nay/rust-toolchain` stable (1.98 or whatever `rust-version` architecture sets, minimum 1.93)
- `cargo build --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace --all-targets`

**`flatpak`** (always on `cosmic-migration` / `main`, cache-heavy):

- install `flatpak`, `flatpak-builder`, `weston`, `timeout` (`coreutils`)
- add Flathub, install Platform/SDK/rust-stable 25.08 (cache `~/.local/share/flatpak` and `.flatpak/builder`)
- `scripts/build-flatpak.sh` && `scripts/smoke-flatpak.sh`
- upload `app.log` / `weston.log` on failure

If a runner cannot cache runtimes, the workflow still **must not** skip clippy/test. `scripts/verify.sh` locally remains the full gate, including Flatpak.

No Flathub publication in this phase.

---

## 9. Working outside COSMIC

libcosmic features we will enable (architecture Cargo.toml, listed here because packaging/smoke depends on them):

```toml
libcosmic = { git = "https://github.com/pop-os/libcosmic.git", default-features = false, features = [
  "winit", "wayland", "x11", "wgpu", "tokio", "xdg-portal",
  "multi-window", "about", "a11y",
] }
```

- **winit + wayland + x11**: runs on weston, GNOME (Mutter), KDE (KWin), COSMIC (cosmic-comp). cosmic-comp is **not** required.
- **wgpu**: GPU path; iced still compiles **tiny-skia** (libcosmic always enables it on iced), so software fallback exists.
- **xdg-portal**: theme/file/clipboard via ashpd, not `com.system76.CosmicSettingsDaemon`.
- **no** `applet`, **no** Cosmic.BaseApp, **no** required `--filesystem=xdg-config/cosmic`.

### How we verify (this Fedora host)

| Check | Method |
|---|---|
| Headless Wayland | `scripts/smoke-flatpak.sh` (weston `--backend=headless`) |
| Nested in the existing session | `SMOKE_USE_EXISTING_DISPLAY=1` against `wayland-1` |
| X11 fallback | `WAYLAND_DISPLAY=` unset + `DISPLAY=:1` `flatpak run` (this host has Xwayland on `:1`) |
| Helper-absent UX | Fedora is not NixOS; smoke **is** that path |
| wgpu vs tiny-skia | inspect smoke log for adapter name; accept either if no panic |
| Not COSMIC | Fedora toolbox + weston; `XDG_CURRENT_DESKTOP` will not be `COSMIC` |

GNOME/KDE matrix is manual once a developer has those sessions; CI only guarantees weston headless + the cargo suite.

---

## 10. Risks

| Risk | Impact | Mitigation |
|---|---|---|
| libcosmic git + iced submodule vendoring | Generator or builder misses `iced/`; offline cargo fails | Use current flatpak-cargo-generator (submodule init is upstream). Do **not** `--git-tarballs`. Commit `cargo-sources.json`. Copy `cargo/config` → `.cargo/config.toml`. Pin generator commit. |
| Cyclic git deps / `cargo vendor` duplicate `cosmic-config` | vendor tarball path unusable | Generator only; never `cargo vendor` as source of truth (cargo#15745). |
| wgpu in sandbox | Panic on start under weston/headless or NVIDIA | `--device=dri`; keep tiny-skia; smoke greps panic/adapter errors; Mesa+NVIDIA GL runtimes already on this host. |
| pkexec + `flatpak-spawn --host` stdin | IPC pipe broken; Apply hangs | Probe in task 1 with a dummy helper; add `--forward-fd` if needed; keep `env_remove("SHELL")` on the host side. |
| pkexec will not run sandbox binary | Expected | GUI-only Flatpak; host helper + polkit on host. |
| Polkit `exec.path` hardcoded to `/run/current-system/sw/bin/…` | Helper in `/usr/bin` never matches | Host packaging problem; document. GUI tries that path first on NixOS. |
| Sandbox `/etc/os-release` is the runtime | False “not NixOS” / wrong version | Narrow binds for `/etc/NIXOS` + `/etc/nixos`; never overlay `os-release`; helper `GetSystemInfo` authoritative. |
| NixOS-only features on Fedora CI | Apply/generations/maintenance untestable for real | Fake helper unit/integration tests; smoke asserts helper-absent UX. Real apply stays a NixOS manual check. |
| libcosmic native libs (expat, wayland, libxkbcommon, mesa, fontconfig, freetype) | Host `cargo build` fails in the toolbox | Install the `-devel` packages in §6. Flatpak SDK already has them. |
| `--talk-name=org.freedesktop.Flatpak` Flathub linter | Review friction | Comment + skip tag; this *is* a system manager. Do not broaden to `--filesystem=host`. |
| `generations.rs` calls `pkexec nix-env` directly | Broken in Flatpak | Architecture must route through helper IPC (already has `ListGenerations` / `RollbackGeneration` / `DeleteGenerations`). |
| Preview currently goes through helper `Generate` | No preview without pkexec | Local preview via `common` + templates shipped in `/app/share/nixos-toolkit/templates`. |
| `flake.nix` still GTK/`wrapGAppsHook4` | `nix build` diverges from verify.sh | Out of scope unless assigned. Do not block on Nix; this host has no `nix` on `PATH`. Later: crane args for libcosmic deps **or** keep cargo/Flatpak as the gate. |
| libcosmic `edition = "2024"` / rust 1.93 | Workspace is edition 2021 / rust 1.75 | Architecture bumps `rust-version`; SDK 1.98.1 is fine. |
| `cargo-sources.json` size / stale lock | Review noise; broken Flatpak | Dedicated regen script + CI cargo job does not depend on it; flatpak job fails if sources do not match the lock (optional later: check script). |
| Task 1 must ship a real Flatpak | Slow first build (libcosmic + iced) | ccache (`flatpak-builder --ccache`), `--state-dir`, mold. Skeleton app can be one window still linked to libcosmic — there is no cheaper GPU toolkit path. |

---

## 11. Files this workstream will create

Owned here (not committed in this docs-only phase):

```
docs/migration/packaging.md              # this file

flatpak/io.github.goshitsarch_eng.NixosToolkit.yml
flatpak/cargo-sources.json               # generated, committed, offline builds
flatpak/io.github.goshitsarch_eng.NixosToolkit.desktop
flatpak/io.github.goshitsarch_eng.NixosToolkit.metainfo.xml
flatpak/nixos-toolkit-host-helper.sh     # optional /app/libexec wrapper

scripts/verify.sh
scripts/build-flatpak.sh
scripts/smoke-flatpak.sh
scripts/generate-cargo-sources.sh

tests/Cargo.toml                         # workspace member toolkit-tests
tests/src/lib.rs
tests/src/bin/fake-helper.rs
tests/gui_state.rs
tests/prefs.rs
tests/preview.rs
tests/flows/*.rs
tests/support/*.rs

.github/workflows/verify.yml             # claimed: cargo job + flatpak/smoke job
.gitignore                               # entries for .flatpak/, .cache/flatpak-builder-tools/
```

Reused, not rewritten except as needed for App ID install paths:

```
data/icons/nixos-toolkit.svg
data/nixos-toolkit.desktop               # Nix package only
data/polkit/org.nixos-toolkit.helper.policy   # host helper only
nix/templates/**                         # copied into the Flatpak for preview
```

Not owned unless the lead assigns it: `flake.nix`, `crates/gui/src/**` (except tests we add under `crates/gui/tests/` if we skip the `tests/` package), `crates/helper/**` production code.

### Task 1 packaging slice (must ship with the skeleton app)

1. lib target + `nixos-toolkit` binary showing an empty libcosmic window (architecture).
2. Manifest + committed `cargo-sources.json` for that skeleton lockfile.
3. Desktop + metainfo + icon install.
4. `scripts/verify.sh` / `build-flatpak.sh` / `smoke-flatpak.sh`.
5. Smoke: window stays up 10s under headless weston with helper absent.

Subsequent tasks keep that gate green while Message coverage and flows land in `tests/`.

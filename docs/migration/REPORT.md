# NixOS Toolkit — libcosmic migration report

The GTK4/libadwaita GUI is replaced by a libcosmic (iced) application with the same NixOS management features. The privileged helper and `crates/common` remain the backend. The GUI ships as a Flatpak; apply/rebuild still require a **host** `nixos-toolkit-helper` (NixOS module / polkit).

**Branch:** `cosmic-migration`  
**App ID:** `io.github.goshitsarch_eng.NixosToolkit`  
**Command:** `nixos-toolkit`  
**libcosmic pin:** `7cc116803b18d7b888eb511b009f775f036c3da7`

---

## What was built

- libcosmic `Application` with a nav bar of 11 pages, header menu (View → About, Quit), refresh, toasts, modal dialogs, status banner.
- Message-driven `AppModel::apply` (no widget scrape). Unit and integration tests drive state and a `fake-helper` IPC double.
- Local Nix preview from bundled templates (no pkexec).
- Apply/dry-run: one helper session, `EnsureDirectories` → `Apply` → `WriteState` on success.
- Generations and maintenance go through helper IPC (GTK used raw `pkexec nix-env` / `pkexec nix-collect-garbage`).
- Hardware settings are generated into `hardware.nix` (GTK UI did not actually apply most of them).
- Flatpak on Freedesktop Platform 25.08, vendored git deps (`flatpak/cargo-sources.json`), weston headless smoke.
- `scripts/verify.sh` runs cargo build, clippy `-D warnings`, cargo test, flatpak-builder, and smoke.

---

## Deviations from the GTK app (and why)

| Topic | GTK | libcosmic | Why |
|---|---|---|---|
| Toolkit | GTK4 + libadwaita | libcosmic / iced / wgpu | Migration goal |
| App ID | `org.nixos-toolkit.app` (invalid D-Bus) | `io.github.goshitsarch_eng.NixosToolkit` | Flatpak / D-Bus (DECISIONS C1). Prefs still read `~/.config/nixos-toolkit/preferences.json` |
| About | Unwired `app.about` | View → About context drawer | COSMIC convention; not a regression |
| Page chrome | Empty content header | Menu + refresh | Intentional |
| Theme | `adw::StyleManager` | iced/cosmic theme + JSON prefs | No GTK |
| i18n | Hardcoded English | Fluent English catalog | COSMIC convention; not a regression |
| ExpanderRow | adw enable-switch expander | Custom `bundle_expander` | No expander in this libcosmic pin |
| Banner | `adw::Banner` | Custom `status_banner` (not dismissible) | No success-tone equivalent |
| Toasts | 3s | 3s custom duration (libcosmic Short is 5s) | Parity |
| Generations list | Unprivileged `/nix/var/nix/profiles` walk | Helper `ListGenerations` (pkexec) | Flatpak cannot see host `/nix` |
| Maintenance | Raw `pkexec` of nix tools | Helper allowlist | Sandbox + security boundary |
| Channel update | Retry without pkexec on failure | Always privileged helper | One path |
| Hardware apply | UI persisted; Nix mostly ignored | Full `HardwareConfig` → `hardware.nix` | Page purpose (C3) |
| NVIDIA modesetting default | Switch on | `nvidia_modesetting` defaults true | Match GTK |
| Power-profiles-daemon | Not generated | Only if power profile ≠ Balanced | Avoid enabling PPD on bluetooth-only apply |
| Empty apply | Profile+bundles+packages | Also non-default hardware | Hardware now changes Nix |
| Helper-missing | GTK still offered Apply (then failed) | Apply/generations/maintenance buttons disabled | No polkit hang in Flatpak/CI |
| Apply logs | 100ms GTK poll (live) | Batched until the helper chain returns | Session is `run_blocking` then stream |
| Symbolic icons | Adwaita theme | App SVG bundled; Freedesktop Platform may lack COSMIC/Adwaita names | No Icontheme.Adwaita on Flathub from this host; labels remain |
| `/etc/hostname` in Flatpak | Readable | Bind ignored (`Path "/etc" is reserved`) | Hostname falls back to `hostname` / default |
| `ai-tools` bundle | 16th bundle, no template | Same; helper fallback | Do not invent a template |
| UDP / WireGuard | No UI | No UI | Do not invent |
| single-instance | Off | Off | GTK FLAGS_NONE; headless smoke |

Polkit action ids stay `org.nixos-toolkit.helper.*`. The policy is **not** inside the Flatpak.

---

## Known limitations

1. **Real `nixos-rebuild` is untested on this Fedora host.** Apply/generations/maintenance are covered by `fake-helper` JSON scripts. Manual NixOS checklist:
   - Install host helper (`programs.nixos-toolkit.enable` or `nixos-toolkit-helper` on PATH matching polkit `exec.path`).
   - Integrate `./nixos-toolkit/state/selected.nix`.
   - Run the GUI (Flatpak or `cargo run -p gui --bin nixos-toolkit`), select a profile, Apply, confirm files under `/etc/nixos/nixos-toolkit/`, reboot/rollback if needed.
2. **Apply logs appear after the helper chain**, not line-by-line while `nixos-rebuild` runs.
3. **DNS/custom TCP** use raw input buffers (`dns_input` / `custom_tcp_input`); DNS applies to state only when the whole string is valid.
4. **List generations now authenticates** (one privileged path). GTK listed without a password.
5. **Nav symbolic icons** may be missing in the Flatpak; page titles are always shown.
6. **No Flathub publication** in this work. GitHub Flatpak job is `workflow_dispatch` only (too heavy for default CI). Cargo job always runs.
7. **`flake.nix` GUI package** no longer uses GTK; this host had no `nix` on PATH so `nix build .#gui` was not executed here.
8. iced/libcosmic **a11y is not ATK-equivalent**.

---

## Building, running, installing

### Development (host)

```bash
# Native deps: libxkbcommon, wayland, expat, fontconfig, freetype, mesa EGL/GL
cargo build -p gui --bin nixos-toolkit
NIXOS_TOOLKIT_TEMPLATES_DIR="$PWD/nix/templates" \
  NIXOS_TOOLKIT_SKIP_PRIVILEGED_INIT=1 \
  cargo run -p gui --bin nixos-toolkit
```

Helper (privileged):

```bash
cargo build -p helper
export NIXOS_TOOLKIT_HELPER="$PWD/target/debug/nixos-toolkit-helper"
# omit SKIP_PRIVILEGED_INIT to ReadState / Apply via pkexec
```

### Tests and Flatpak gate

```bash
scripts/verify.sh
```

Requires: `cargo`, `rustc` ≥ 1.93, `flatpak`, `flatpak-builder`, `weston`, `timeout`, Freedesktop Platform/SDK **25.08** + `org.freedesktop.Sdk.Extension.rust-stable`.

Smoke only:

```bash
scripts/build-flatpak.sh
scripts/smoke-flatpak.sh
```

Smoke sets `NIXOS_TOOLKIT_SKIP_PRIVILEGED_INIT=1` so headless weston never pkexec-hangs.

Regenerate vendored cargo sources after lockfile changes:

```bash
scripts/generate-cargo-sources.sh   # pins flatpak-builder-tools 1fc32195…
```

### Install the local Flatpak

```bash
scripts/build-flatpak.sh
flatpak --user run io.github.goshitsarch_eng.NixosToolkit
```

On NixOS, also install the **host** helper (NixOS module `programs.nixos-toolkit.enable` or equivalent). The Flatpak GUI will `flatpak-spawn --host -- pkexec /run/current-system/sw/bin/nixos-toolkit-helper` (or `/usr/bin` / `/usr/local/bin`). Without the helper, preview and preferences still work; Apply/generations/maintenance stay disabled.

### One-time NixOS integration

Unchanged from the GTK app: create `/etc/nixos/nixos-toolkit/state/selected.nix` and import it from `configuration.nix` or the flake. See the Getting Started page.

---

## Phase 3

`scripts/verify.sh` exited 0 (2026-09-10): cargo build/clippy/test, flatpak-builder, weston smoke status 124, no panic, no pkexec in the smoke log.

First DA pass (**FAILURES**) found Flatpak `/etc` binds are ignored, GPU detect never ran, username not seeded, hostname charset not enforced, DNS/TCP live-parse bugs, and helper-missing still spawned on nav. Follow-up commits: `a45894a`, `bf3c05b`, `b60213f`, `3b4112a`. Second DA pass: **CLEAN**.

Parity checklist in `PLAN.md` is ticked with verification notes (message tests + code review + smoke). Interactive click-through of every control was not possible under headless pixman weston; privileged apply remains a NixOS manual check.

DA sign-offs: Task 1a APPROVE; 1b/1c APPROVE; 12–14 APPROVE; pages+hardware REQUEST CHANGES then `b83ff31`; Phase 3 CLEAN after the detection/input fixes.

# CLAUDE.md

This file provides guidance when working with code in this repository.

## Project Overview

NixOS Toolkit — a libcosmic (iced) GUI for declarative NixOS system management. Users select a desktop profile, software bundles, and system settings. The unprivileged GUI talks to `nixos-toolkit-helper` (typically via `pkexec`). The helper writes Nix modules under `/etc/nixos/nixos-toolkit/` and runs `nixos-rebuild`. It does not edit the user's `configuration.nix` or `flake.nix`.

Repository: `goshitsarch-eng/Gosh-NixOS-Manager`. Product name and binaries are NixOS Toolkit, not the repo title.

Workspace version `0.1.0`, edition `2021`, `rust-version = "1.93"`, license `GPL-3.0-or-later` (`LICENSE` in the repository root).

## Build & Development Commands

```bash
# Enter development shell (required for cargo commands)
nix develop

# Run the GUI (Cargo package "gui", binary nixos-toolkit)
cargo run -p gui

# Run the helper (Cargo package "helper", binary nixos-toolkit-helper)
cargo run -p helper

# Build all flake packages
nix build

# Build specific packages
nix build .#gui
nix build .#helper
nix build .#templates
nix build .#full

# Run from flake
nix run github:goshitsarch-eng/Gosh-NixOS-Manager
```

Dev-shell sets `NIXOS_TOOLKIT_TEMPLATES_DIR` to `$PWD/nix/templates`. For apply tests against a local helper:

```bash
export NIXOS_TOOLKIT_HELPER="$PWD/target/debug/nixos-toolkit-helper"
cargo run -p gui
```

`NIXOS_TOOLKIT_NO_PKEXEC=1` spawns the helper directly (still needs write access to `/etc/nixos` for real applies).

### Checks

GitHub Actions (`.github/workflows/verify.yml`):

- job `cargo`: `cargo build --workspace --all-targets`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace --all-targets`
- job `flake`: `nix build .#checks.x86_64-linux.fmt` and `.#checks.x86_64-linux.audit` on push/PR (not full `nix flake check`)
- job `flatpak`: `workflow_dispatch` only

Flake `checks`: inherit GUI/helper packages, `clippy` using GUI crane inputs/artifacts with `--all-targets -- --deny warnings` (no `-p gui`; clippy still walks the workspace), `fmt`, `audit`. No flake cargo-test check.

Local full gate: `scripts/verify.sh` (cargo + clippy + test + Flatpak build + weston smoke).

## Architecture

### Crate structure

```
crates/
├── common/       # Shared types: actions, ipc, config paths, nix generation
├── gui/          # libcosmic application; binary nixos-toolkit
│                 # view/ pages, helper/ spawn, core/ apply reducer
├── helper/       # Privileged binary nixos-toolkit-helper (pkexec)
└── fake-helper/  # JSON-line IPC double for tests; never touches /etc/nixos
```

There is no `window.rs` and no `pages/` directory. UI is `crates/gui/src/view/*.rs` plus `app.rs`.

### Communication model

- GUI runs unprivileged. Default spawn: `pkexec <helper>`. Flatpak: `flatpak-spawn --host --forward-fd=0 --forward-fd=1 -- pkexec <host-helper>`.
- IPC: internally tagged JSON (`type` / `payload`), one object per line, stdout; tracing on stderr.
- Apply chain on one helper process: `EnsureDirectories` → `Apply` → `WriteState` on success for Switch/Boot/Test/Build. DryBuild sends `Apply` only so the helper can snapshot before creating directories.
- Local preview does **not** call the helper (`common::nix::generate_preview_full_from`).

### Counts (from code, not marketing)

- 13 profiles (`default_profiles()` + `nix/templates/profiles/`)
- 16 bundle **definitions** (`default_bundles()`), 15 bundle **template files** (`ai-tools` has none)
- 21 service toggles (`ServicesConfig` / `view/services.rs`)
- 13 `HelperRequest` variants (including `GetDiskUsage`)
- 5 system actions, 5 maintenance catalog actions
- 6 maintenance allowlist strings (`nix-channel --update` plus `nix flake update --flake /etc/nixos`)
- 11 nav pages
- Apply rebuild dropdown: Switch / Boot / Test / Build; Dry Run is a separate button
- `NetworkConfig` also has `wireguard_enabled` and `wireguard_listen_port`

Enabling a bundle in the GUI always fills `bundle_packages` from the catalog. Preview and apply then generate a fallback module (resolved catalog attrs + `bundle_module_stub` in `crates/common/src/nix.rs`) instead of copying `nix/templates/bundles/*.nix`. Template copy still happens if the id is missing from `bundle_packages` (old state, or a reconstructed copied template). Generated fallback files restore `bundle_packages` on reconstruct. Profile templates **are** copied. Preview dump order matches apply.

### Key files

- `crates/common/src/actions.rs` — profiles, bundles, system/maintenance defs
- `crates/common/src/ipc.rs` — `HelperRequest`, `HelperResponse`, `AppState`, hardware/network/services
- `crates/common/src/nix.rs` — selected/hostname/DNS/users/packages/hardware/network/services/unfree generation, bundle stubs, local preview
- `crates/common/src/config.rs` — managed paths (including `UNFREE_NIX`), `detect_integration_status`
- `crates/gui/src/view/*.rs` — UI pages
- `crates/gui/src/core/apply.rs` — message reducer
- `crates/gui/src/helper/spawn.rs` — pkexec / Flatpak / env
- `crates/helper/src/commands.rs` — request handlers, reconstruct, dry-build snapshot
- `crates/helper/src/nix_gen.rs` — writes, profile fallbacks, stale cleanup, atomic write jail
- `nix/templates/profiles/` — desktop templates (used on apply)
- `nix/templates/bundles/` — bundle templates (legacy / helper-without-`bundle_packages`)

### Adding profiles / bundles

1. Add `nix/templates/profiles/<id>.nix` or `nix/templates/bundles/<id>.nix`.
2. Add the def in `default_profiles()` or `default_bundles()`.
3. For a bundle that must enable NixOS modules (not just `environment.systemPackages`), add a match arm in `crates/common/src/nix.rs` (`bundle_module_stub`). GUI apply will not copy the template.

## Storage locations

- Managed Nix: `/etc/nixos/nixos-toolkit/` (`state/`, `profiles/`, `bundles/`)
- Privileged UI state: `/etc/nixos/nixos-toolkit/state/state.json`
- User theme prefs: `$XDG_CONFIG_HOME/nixos-toolkit/preferences.json` (not `~/.local/share/`)
- Templates at runtime: `$NIXOS_TOOLKIT_TEMPLATES_DIR` (Nix wrapper, Flatpak `/app/share/…`, else `./nix/templates`)

## Security model

- GUI never writes `/etc/nixos/`
- Privileged operations go through the helper via pkexec (or `flatpak-spawn --host` + pkexec)
- Managed files are separate from the user's `configuration.nix` / `flake.nix`
- Maintenance commands are an exact-string allowlist in `commands.rs` (6 strings)
- Hostname and DNS Nix use `lib.mkDefault` (explicit user settings win)
- `atomic_write` refuses paths outside `/etc/nixos/nixos-toolkit` (prefix check)

GUI and helper integration detection share `detect_integration_status`. Either `configuration.nix` or `flake.nix` importing `nixos-toolkit/state/selected.nix` (or `./` / absolute variants) counts. `#` line comments and `/* */` blocks are stripped first. Bare `nixos-toolkit` is not enough.

NixOS module: `programs.nixos-toolkit.enable`, `.package` (GUI), `.helperPackage` (helper). Polkit actions `manage-system`, `write-config`, and `rebuild` all set `exec.path`.

User-facing docs: `README.md`, `docs/architecture.md`, `docs/development.md`, `docs/reference.md`. `docs/migration/` is historical.

## Git Commits & Documentation Style

Never include references to AI, Claude, or automated generation in:

Git commit messages
Code comments
Documentation
README files
Changelog entries
Pull request descriptions

Write all commits and documentation as if authored directly by a human developer. Use first-person naturally where appropriate (e.g., "Fixed bug in parser" or "Refactored authentication flow for clarity").
Avoid phrases like:

"Generated by Claude/AI"
"AI-assisted"
"Created with Claude Code"
"This code was written by an AI"
Any disclaimers about AI authorship

Commit messages should be concise, conventional, and professional (e.g., "feat: add user authentication", "fix: resolve null pointer in data handler", "docs: update API reference").

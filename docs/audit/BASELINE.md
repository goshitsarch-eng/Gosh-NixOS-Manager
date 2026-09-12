# Baseline — NixOS Toolkit (`audit-hardening`)

Recorded 2026-09-12 before implementation. Product: **NixOS Toolkit** (`nixos-toolkit` GUI + `nixos-toolkit-helper`). Repository: `goshitsarch-eng/Gosh-NixOS-Manager`. Branch created from `main` @ `36e6de3`.

## Repository structure

```
crates/common/     catalogs, IPC, paths, Nix generation
crates/gui/        libcosmic Application (binary nixos-toolkit)
crates/helper/     privileged JSON-line helper
crates/fake-helper test double (never touches /etc/nixos)
nix/templates/     13 profile templates, 15 bundle templates (ai-tools has none)
data/              desktop file, SVG icon, polkit policy
flatpak/           GUI-only Freedesktop 25.08 manifest + metainfo
scripts/           verify.sh, build-flatpak.sh, smoke-flatpak.sh, generate-cargo-sources.sh
docs/              user docs + docs/migration (historical rewrite notes)
```

Workspace `0.1.0`, edition 2021, `rust-version = "1.93"`, GPL-3.0-or-later.

## Architecture summary

Unprivileged libcosmic GUI talks to a privileged helper over internally tagged JSON lines (`type` / `payload`). Default spawn: `pkexec <helper>`. Flatpak: `flatpak-spawn --host --forward-fd=0 --forward-fd=1 -- pkexec <host-helper>`.

Apply chain on one helper process: `EnsureDirectories` → `Apply` → `WriteState` (Switch/Boot/Test/Build). Dry-build sends `Apply` only so the helper can snapshot before creating directories.

Local preview does **not** call the helper (`common::nix::generate_preview_full_from`). The GUI never writes `/etc/nixos`.

MVU: flat `Message` → `AppModel::apply` → `Vec<Intent>` → iced Tasks. 11 `nav_bar` pages. About is a context drawer. Confirmations are `widget::dialog`.

## Supported / advertised features

Sidebar pages (11): Getting Started, Desktop Profiles (13), Software Bundles (16), Custom Packages, System Settings, Hardware, Network, Services (21 toggles), Generations, Maintenance (5 catalog actions + disk usage), Apply Changes (Switch/Boot/Test/Build + Dry Run).

Privileged work uses pkexec. Theme is a local preference (`$XDG_CONFIG_HOME/nixos-toolkit/preferences.json`). Appearance is not written into Nix.

## Build process

```
nix develop          # optional; host cargo 1.98 also builds
cargo run -p gui     # nixos-toolkit
cargo run -p helper  # nixos-toolkit-helper
nix build .#gui / .#helper / .#templates / .#full
```

Dev-shell sets `NIXOS_TOOLKIT_TEMPLATES_DIR` to `$PWD/nix/templates`.

## Packaging process

- Nix flake packages + NixOS module `programs.nixos-toolkit.enable` (GUI, helper, polkit).
- Flatpak GUI-only (`io.github.goshitsarch_eng.NixosToolkit`); host helper required for apply.
- `scripts/build-flatpak.sh` + `scripts/smoke-flatpak.sh` (weston headless, 10s).
- `scripts/verify.sh`: cargo build, clippy `-D warnings`, test, Flatpak build, smoke. **Does not** run `cargo fmt --check`, desktop-file-validate, or appstreamcli.

## Test coverage (baseline)

`cargo test --workspace --all-targets`: **218 passed, 0 failed**.

| Target | Tests |
|---|---|
| common lib | 60 |
| fake-helper | 8 |
| gui lib | 93 |
| gui bin | 0 |
| gui integration (flags, gui_state, packages, state_mutations) | 23 |
| helper | 34 |

Strengths: apply reducer, spawn policy, package parser, dry-build snapshot/restore, write jail, reconstruct snippets, IPC legacy JSON.

Gaps: no live helper-stream test, no field-error tests for TCP/UDP/packages, no Nix-comment injection tests, no toast-map tests, no flake-attr rebuild tests.

## Current warnings / failures

| Check | Result |
|---|---|
| `cargo build --workspace --all-targets` | pass (cached ~6s) |
| `cargo clippy --workspace --all-targets -- -D warnings` | pass |
| `cargo test --workspace --all-targets` | pass (218) |
| `cargo fmt --all -- --check` | **FAIL** (`crates/common/src/config.rs`, `crates/helper/src/commands.rs`) |
| `cargo audit` on host | not installed; flake `checks.*.audit` exists |
| `desktop-file-validate` | pass with Categories hint (System;Settings) |
| `appstreamcli validate --no-net` | pass; pedantic: uppercase letters in App ID |
| Flatpak build/smoke | not re-run in this baseline (scripts exist; CI job is `workflow_dispatch` only) |

## Flatpak status

Manifest: Freedesktop Platform/SDK 25.08, rust-stable, offline cargo sources, no network, no host FS, `--device=dri`, `--talk-name=org.freedesktop.Flatpak`, `--filesystem=/etc/NIXOS:ro`, `--filesystem=/etc/nixos:ro`, `--filesystem=/etc/hostname:ro`. Host helper via `flatpak-spawn --host`. Smoke sets `NIXOS_TOOLKIT_SKIP_PRIVILEGED_INIT=1` and `SKIP_HOST_PROBES=1`. Metainfo release text still says “Initial libcosmic skeleton.” Screenshot is the app SVG.

## Known/documented features vs UI

README, `docs/architecture.md`, `docs/reference.md`, and Fluent labels are the contract. Specialist matrices: `FEATURES.md`. Documented limitations that are **not** defects:

- IPv4-only DNS.
- GUI never sends `CheckPermissions` / `GetSystemInfo` / `Validate` / `Generate`.
- Catalog package names are not added as custom packages (toast points at the bundle).
- WireGuard does not write peers/keys.
- English Fluent only.
- NVIDIA detection defaults an unwritten driver to stable (README Hardware).
- Any written `hardware.nix` also emits default PipeWire (README / reference).
- Empty Apply is a destructive reset of managed files (dialog copy).

## Visible UI features (code + runtime)

GUI launched on COSMIC/Wayland (`XDG_CURRENT_DESKTOP=COSMIC`) with `NIXOS_TOOLKIT_SKIP_PRIVILEGED_INIT=1` and templates from the checkout. Process stayed alive (~81 MiB RSS debug). X11 `ffmpeg` capture of `:1` was black (app is Wayland). Interactive page walk was not completed in the recon window; widget trees were reviewed in full.

Chrome: nav bar, View → About/Quit, header Refresh, status banner, toaster, confirm dialogs, settings sections.

## Obvious incomplete functionality (baseline)

See `BUGS.md`, `SECURITY.md`, `COSMIC-UX.md`, `PERFORMANCE.md`, `ARCHITECTURE.md`, `PACKAGING.md`. Highest-impact:

1. Helper session **buffers the entire privileged run** then dumps events — apply logs are not live; a Tokio worker is blocked for the rebuild.
2. Nix `selected.nix` **comments interpolate unsanitized IPC strings** (newline comment breakout). Helper Apply/Generate do not re-validate GUI rules.
3. Bundle package checkboxes do not disable module stubs (Steam/Docker/libvirt still enable).
4. Helper timeouts (60s) for generations/disk/maintenance are swallowed; Apply has **no** recv timeout and can hang on non-JSON stdout.
5. Toast dismiss rebuilds the whole `Toasts` map.
6. `ReadState` can overwrite in-flight edits; `helper_missing` is sticky.
7. NVIDIA “Open Source (nvidia-open)” does not set `hardware.nvidia.open`.
8. Invalid custom packages / TCP/UDP ports fail silently.
9. Duplicate Fluent catalogs; empty About repository URL (`CARGO_PKG_REPOSITORY` not inherited).
10. `verify.sh` missing `cargo fmt --check`; metainfo still “skeleton”; Flatpak bind-mounts `/etc/nixos` despite host-spawn probes.

## Baseline runtime observations

- Debug GUI starts (wgpu/libcosmic) and remains running under COSMIC.
- Host is Fedora with cargo 1.98 (MSRV declared 1.93).
- No panic in the startup log after `Finished` / `Running`.
- Privileged init skipped in the recon launch; helper-missing path is the one exercised.

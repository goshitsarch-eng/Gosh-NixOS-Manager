# Development

## Prerequisites

- Nix with flakes, or `nix --extra-experimental-features 'nix-command flakes'`
- Linux (flake `platforms = linux`)
- Workspace MSRV: **1.93** (`Cargo.toml` `rust-version`). The flake toolchain is rust-overlay **stable.latest**, not a 1.93 pin. GitHub Actions uses `dtolnay/rust-toolchain@stable`.

## Dev shell

```bash
nix develop
```

Hook sets `RUST_SRC_PATH` and `NIXOS_TOOLKIT_TEMPLATES_DIR=$PWD/nix/templates`. Prints `cargo run -p gui`.

GUI package `buildInputs` (also used as crane clippy inputs): pkg-config, openssl, libxkbcommon, wayland, mesa/libGL, fontconfig, X11 libs. `devShells.default.packages` is a shorter list (rust-analyzer, cargo-watch, wayland, libxkbcommon, mesa, fontconfig, nil, nixpkgs-fmt, gdb, …) and does not repeat openssl/libGL/X11.

## Run

```bash
cargo run -p gui                 # binary: nixos-toolkit
cargo run -p helper              # binary: nixos-toolkit-helper
cargo run -p fake-helper
```

Package names are `gui` / `helper`. Installed/Nix binaries are `nixos-toolkit` / `nixos-toolkit-helper`. There is no `./result/bin/gui`.

Local apply against a just-built helper:

```bash
cargo build -p helper
export NIXOS_TOOLKIT_HELPER="$PWD/target/debug/nixos-toolkit-helper"
export NIXOS_TOOLKIT_TEMPLATES_DIR="$PWD/nix/templates"
# optional, skip pkexec if you already have privileges:
export NIXOS_TOOLKIT_NO_PKEXEC=1
cargo run -p gui
```

Tests / CI skip privileged startup with `NIXOS_TOOLKIT_SKIP_PRIVILEGED_INIT` and host probes with `NIXOS_TOOLKIT_SKIP_HOST_PROBES`.

## Nix build

```bash
nix build            # packages.default = GUI
nix build .#gui
nix build .#helper
nix build .#templates
nix build .#full     # symlinkJoin of gui + helper + templates
nix run .            # apps.default → nixos-toolkit
nix run .#helper
```

There is no flake package attribute named `nixos-toolkit`. The overlay provides `nixos-toolkit` / `nixos-toolkit-helper`.

## Tests

```bash
cargo test --workspace --all-targets
```

| Location | What |
|----------|------|
| `crates/common/src/ipc.rs`, `nix.rs` | JSON compatibility (old Apply without `hardware_config` / WireGuard), hostname/hardware/services/network Nix, preview vs customized bundles, unfree |
| `crates/common/src/config.rs` | Shared integration markers (flake-only, comments, bare word) |
| `crates/helper/src/commands.rs` | rollback activate reject, kernel version parse, reconstruct parsers, dry-build restore `Error`, tempfile write probe, flake channel rewrite |
| `crates/helper/src/nix_gen.rs` | hardware dry-run, stale cleanup, write jail, Fail2Ban without SSH |
| `crates/gui/src/core/apply.rs` | reducer: apply/dry-run, rebuild types, packages, rollback, maintenance, UDP/WireGuard |
| `crates/gui/src/core/packages.rs` | parser and duplicate classification |
| `crates/gui/src/helper/spawn.rs` | pkexec / Flatpak matrix |
| `crates/gui/src/helper/session.rs` | Ensure → Apply → WriteState against a scripted helper |
| `crates/gui/tests/flags.rs` | `Flags::for_tests()` |
| `crates/gui/tests/gui_state.rs` | `apply_is_empty` |
| `crates/gui/tests/packages.rs` | public parser |
| `crates/gui/tests/state_mutations.rs` | `AppState` round-trip |
| `crates/fake-helper` | JSONL / shorthands |

There is no `crates/gui/tests/integration.rs`.

`fake-helper` reads one JSON request per stdin line. `FAKE_HELPER_SCRIPT` is a JSONL of `HelperResponse` values, or shorthands `Ok`, `State`, `ApplyComplete`, `Log`.

## CI

`.github/workflows/verify.yml`:

- **cargo** on push (`main`, `cosmic-migration`), pull_request, workflow_dispatch: build, clippy `-D warnings`, test.
- **flake** on the same events: `nix build .#checks.x86_64-linux.fmt` and `.#checks.x86_64-linux.audit` only. It does **not** run `nix flake check` (that also builds GUI/helper/clippy).
- **flatpak** only on `workflow_dispatch`.

Flake checks: GUI/helper packages, `clippy` (crane `guiArgs` for native/build inputs and GUI cargo artifacts; **`cargoClippyExtraArgs` is `--all-targets -- --deny warnings` with no `-p gui`, so clippy still walks the workspace), `fmt`, `audit`. No flake cargo-test check.

`scripts/verify.sh`: workspace cargo build/clippy/test, then Flatpak build + `scripts/smoke-flatpak.sh` (weston, `SKIP_PRIVILEGED_INIT`, fails if pkexec appears in logs).

Regenerate Flatpak cargo sources after dependency changes:

```bash
scripts/generate-cargo-sources.sh
```

## Environment variables

| Variable | Who | Effect |
|----------|-----|--------|
| `NIXOS_TOOLKIT_TEMPLATES_DIR` | GUI + helper | Template root. Default `./nix/templates` |
| `NIXOS_TOOLKIT_HELPER` | GUI | Helper path |
| `NIXOS_TOOLKIT_NO_PKEXEC` | GUI | Spawn helper directly (`1`/`true`/`yes`/`on`) |
| `NIXOS_TOOLKIT_SPAWN` | GUI | Wrapper program (e.g. `flatpak-spawn`) |
| `NIXOS_TOOLKIT_SPAWN_ARGS` | GUI | JSON array or whitespace-separated args before the helper |
| `NIXOS_TOOLKIT_SKIP_PRIVILEGED_INIT` | GUI | No `ReadState` on startup |
| `NIXOS_TOOLKIT_SKIP_HOST_PROBES` | GUI | Skip `/etc/NIXOS`, `lspci`, … |
| `FLATPAK_ID` / `/.flatpak-info` | GUI | Sandbox detection |
| `FAKE_HELPER_SCRIPT` | fake-helper | Scripted replies |
| `RUST_LOG` | both | tracing filter (helper default includes `helper=info`) |

## Adding a desktop profile

1. Create `nix/templates/profiles/<id>.nix` (NixOS module). Force your display manager on; force the ones you conflict with off (`lib.mkForce`).
2. Append a `ProfileDef` in `crates/common/src/actions.rs` `default_profiles()`:

```rust
ProfileDef {
    id: "your-id".into(),
    name: "Your Name".into(),
    description: "…".into(),
    icon: "desktop-symbolic".into(),
    template: "profiles/your-id.nix".into(),
    display_manager: "gdm".into(), // catalog metadata; templates do the real enable
    arm_compat: ArmCompat::Full,
    arm_note: None,
}
```

3. Add a fallback arm in `crates/helper/src/nix_gen.rs` `generate_fallback_profile` if you care about missing-template applies. Unknown ids already get a generic LightDM fallback.

Apply copies the template file. Preview shows the same file when it exists.

## Adding a software bundle

1. Create `nix/templates/bundles/<id>.nix` if you want a rich module for the **legacy/reconstruct** path (template is copied only when `bundle_packages` omits the id).
2. Append a `BundleDef` in `default_bundles()` with the packages the **GUI** should toggle. Those ids are what GUI apply installs. If the nixpkgs attr differs from the catalog id, use `PackageDef::with_nix_attr`.
3. If the bundle must enable NixOS options (Steam, libvirt, Podman, …), add a match arm in `bundle_module_stub` in **`crates/common/src/nix.rs`**. Preview and GUI apply both call this. **Without a stub, GUI apply will not copy your template and will not enable those options.**
4. If the generated packages are unfree, add the attr (or bundle id) to `needs_allow_unfree` / `attr_needs_unfree` in the same file so `unfree.nix` is imported.
5. Set `arm_compat` / `arm_note`. `ArmCompat::None` disables the bundle on ARM in the UI.

`ai-tools` is the existing example of a catalog entry with no template file: preview and apply both emit packages only (`ollama`).

## i18n

Fluent, fallback `en`. Files: `crates/gui/i18n/en/gui.ftl` and `nixos_toolkit.ftl` (overlapping keys — keep them in sync). `fl!` in views. Apply confirm dialogs in `core/apply.rs` use Fluent (`apply-empty-*`, `apply-packages-*`, `apply-normal-*`). Catalog names in `actions.rs` are English by design. Service **row** names and descriptions in `view/services.rs` are still hardcoded English; group titles on that page are Fluent.

## Project layout

```
.
├── Cargo.toml                 # workspace, rust-version 1.93, GPL-3.0-or-later
├── LICENSE                    # GNU GPL v3
├── flake.nix
├── crates/
│   ├── common/src/{actions,ipc,config,nix}.rs
│   ├── gui/src/{app,state,message,view,helper,core,integration,config}.rs
│   ├── helper/src/{main,commands,nix_gen,rebuild}.rs
│   └── fake-helper/
├── nix/templates/{profiles,bundles,state}
├── data/{nixos-toolkit.desktop,icons/,polkit/}
├── flatpak/
├── scripts/{verify,build-flatpak,smoke-flatpak,generate-cargo-sources}.sh
└── docs/
```

`nix/templates/state/selected.nix.template` exists (`{{IMPORTS}}` placeholders) and is **not** used by `crates/common`. Generation uses format strings in `nix.rs`.

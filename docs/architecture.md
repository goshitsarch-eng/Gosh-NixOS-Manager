# Architecture

Current tree. GTK4/libadwaita is gone. Historical rewrite notes live in [migration/](migration/).

## Processes

```
User
  │
  ▼
nixos-toolkit                    unprivileged libcosmic app
  │  local preview (no helper)
  │  prefs → $XDG_CONFIG_HOME/nixos-toolkit/preferences.json
  │
  │  JSON lines on stdin/stdout
  │  spawn: pkexec | flatpak-spawn --host … pkexec | direct
  ▼
nixos-toolkit-helper             assumed privileged
  │  writes /etc/nixos/nixos-toolkit/**
  │  nixos-rebuild {switch,boot,test,build,dry-build}
  │  nix-env on /nix/var/nix/profiles/system
  │  allowlisted nix-collect-garbage / nix-store / nix-channel
  ▼
NixOS evaluation of the user's config, which must import
  /etc/nixos/nixos-toolkit/state/selected.nix
```

The GUI never writes `/etc/nixos`. The helper never edits `configuration.nix` or `flake.nix`.

## Crates

| Crate | Path | Artifact |
|-------|------|----------|
| `common` | `crates/common` | library: catalogs, IPC types, path constants, Nix string generation |
| `gui` | `crates/gui` | library `nixos_toolkit_gui` + binary **`nixos-toolkit`** |
| `helper` | `crates/helper` | binary **`nixos-toolkit-helper`**. Declares tokio; `main` is synchronous (`std::process::Command`) |
| `fake-helper` | `crates/fake-helper` | binary `fake-helper`. Replies `Ok` or a `FAKE_HELPER_SCRIPT` JSONL (it **reads** that file). Never touches `/etc/nixos` |

GUI toolkit: `libcosmic` git pin `7cc116803b18d7b888eb511b009f775f036c3da7`, features `winit`, `wayland`, `x11`, `wgpu`, `tokio`, `xdg-portal`, `about`, `a11y`. App ID: `io.github.goshitsarch_eng.NixosToolkit`.

## GUI shape

`cosmic::Application` on `AppModel` (`crates/gui/src/app.rs`):

- Nav bar from `Page::ALL` (11 pages), default Getting Started
- `update` → `AppModel::apply` in `core/apply.rs` (no widget scrape)
- `view` → toaster + `view::root` → one page module under `view/`
- Header: View menu (About, Quit), refresh (re-runs local `detect_system` / `lspci`, not `ReadState`)
- About is a context drawer
- Keyboard: F5 / Ctrl+R refresh, Ctrl+Q quit
- Single-instance mode is off

Pages: `onboarding`, `profiles`, `bundles`, `packages`, `system`, `hardware`, `network`, `services`, `generations`, `maintenance`, `apply`.

## Helper spawn

`SpawnSpec` (`crates/gui/src/helper/spawn.rs`):

| Situation | Program / args |
|-----------|----------------|
| Host, helper executable | `pkexec` + helper path |
| Host, helper missing | `program` is still `pkexec`, but `helper_available = false`; GUI does not spawn |
| `NIXOS_TOOLKIT_NO_PKEXEC` | helper path, no extra args |
| `NIXOS_TOOLKIT_SPAWN` | that program + `SPAWN_ARGS` + helper |
| Flatpak + probed host helper | `flatpak-spawn --host --forward-fd=0 --forward-fd=1 -- pkexec <path>` |
| Flatpak + no helper | `helper_available = false`; no pkexec |

Discovery (host): `NIXOS_TOOLKIT_HELPER`, sibling of `current_exe`, then `/run/current-system/sw/bin/nixos-toolkit-helper`, `/usr/local/bin/…`, `/usr/bin/…`, else the name `nixos-toolkit-helper`.

`SHELL` is always removed from the child environment (pkexec rejects some nix-develop shells).

One helper **process per operation**. Apply and dry-run share `run_apply_chain`: one process, `EnsureDirectories` then `Apply`. Successful `Switch` also sends `WriteState`. Dry-build does not.

## IPC

Internally tagged JSON, one object per line.

```json
{"type":"ReadState"}
{"type":"Apply","payload":{ "...": "..." }}
```

Thirteen request variants: `CheckPermissions`, `GetSystemInfo`, `Validate`, `Generate`, `Apply`, `EnsureDirectories`, `ReadState`, `WriteState`, `ListGenerations`, `RollbackGeneration`, `DeleteGenerations`, `RunMaintenance`, `GetDiskUsage`.

The GUI actually spawns:

| UI | Request |
|----|---------|
| Startup | `ReadState` |
| Apply | `EnsureDirectories` → `Apply` (`Switch`) → `WriteState` |
| Dry run | `EnsureDirectories` → `Apply` (`DryBuild`); no `WriteState` |
| Generations | `ListGenerations`, `RollbackGeneration`, `DeleteGenerations` |
| Maintenance | `RunMaintenance`, `GetDiskUsage` |

It does **not** send `CheckPermissions`, `GetSystemInfo`, `Validate`, or `Generate`. Preview is local. System info is `integration.rs` (and `flatpak-spawn --host` inside the sandbox).

`RebuildType` in IPC: `Switch`, `Boot`, `Test`, `Build`, `DryBuild`. The GUI only uses `Switch` and `DryBuild`. Rollback `activate` is `"switch"` or `"boot"`.

Streaming: during apply/rebuild the helper may emit many `Log` lines before a terminal `ApplyComplete` / `Error`.

Full field list: [reference.md](reference.md).

## Nix generation

### `selected.nix`

Import-only module. Conditional imports:

- `../profiles/<id>.nix`
- `../bundles/<id>.nix` for each enabled bundle
- `./hostname.nix`, `./dns.nix`, `./users.nix`, `./custom-packages.nix`, `./hardware.nix`, `./network.nix`, `./services.nix` when the corresponding state is set

### Profiles

Helper `read_template(profile.template)` from `NIXOS_TOOLKIT_TEMPLATES_DIR` (else `./nix/templates`). On success, atomic-write to `/etc/nixos/nixos-toolkit/profiles/<id>.nix`. On failure, `generate_fallback_profile` in the helper.

### Bundles (GUI apply)

1. `ToggleBundle { enabled: true }` copies every catalog package id into `state.bundle_packages[id]`.
2. Apply sends that map.
3. Helper: if `bundle_packages` **contains** the id **or** the template is missing → `generate_fallback_bundle` (package list + `bundle_module_stub`).
4. Template copy happens only when the id is **absent** from `bundle_packages` and the file exists. That is the path for old `state.json` or `reconstruct_state_from_nix()` (enabled bundle ids only). A toggle-on in this GUI always inserts the key.

### Preview vs apply

`generate_preview_full_from` always inlines profile **and** bundle template files when they exist. It does not read `bundle_packages` and does not simulate `bundle_module_stub`. Unchecking catalog packages still previews the full template. The Apply page can therefore show a richer (or different) bundle file than the helper will write.

### Priority

| Snippet | Nix priority |
|---------|----------------|
| hostname | `lib.mkForce` |
| DNS nameservers | `lib.mkDefault` |
| everything else | plain assignment |

### Dry-build

`Apply` with `DryBuild` still writes the managed tree so `nixos-rebuild dry-build` can evaluate it, then restores a snapshot of `/etc/nixos/nixos-toolkit`. Failure to **take** the snapshot is `HelperResponse::Error`. Failure to **restore** after a finished dry-build is a log line only.

`Generate { dry_run: true }` does not write. The GUI does not call `Generate`.

### Atomic writes

Helper `atomic_write`: write `*.tmp` (extension replaced, so `selected.nix` → `selected.tmp`), rename, read back. `WriteState` uses `state.json.tmp` then rename, **without** read-back. `EnsureDirectories` placeholder uses plain `fs::write`.

Leftover files are not removed when a profile/bundle/snippet is no longer imported.

## State

| Store | Path | Who |
|-------|------|-----|
| Apply selections | `/etc/nixos/nixos-toolkit/state/state.json` | helper `ReadState` / `WriteState` |
| Theme | `$XDG_CONFIG_HOME/nixos-toolkit/preferences.json` (`dirs::config_dir()`) | GUI; cosmic-config is a best-effort mirror under `APP_ID` |
| Managed Nix | `/etc/nixos/nixos-toolkit/{state,profiles,bundles}` | helper |

If `state.json` is missing or invalid, the helper reconstructs a **partial** `AppState` from `selected.nix` (profile + bundle ids), `custom-packages.nix`, and `hostname.nix`. Network, services, hardware, DNS, and groups are not reconstructed.

## Integration detection

Two implementations, different strings:

**GUI** (`crates/gui/src/integration.rs`), in order:

1. If `/etc/nixos/configuration.nix` is readable and contains `nixos-toolkit` → `Integrated`.
2. Else if `/etc/nixos/nixos-toolkit/state/selected.nix` exists: `Integrated` only when `configuration.nix` contains `nixos-toolkit`; otherwise **`NotIntegrated` without reading `flake.nix`**.
3. Else if `flake.nix` contains `nixos-toolkit` → `Integrated`.
4. Else `NotIntegrated`.

So a flake-only import, after the placeholder `selected.nix` exists, is **NotIntegrated** in the GUI. `detect_integration()` never returns `Unknown`; `SystemInfo::default()` does when host probes are skipped.

**Helper** (`crates/helper/src/commands.rs`): `selected.nix` must exist, then `configuration.nix` or `flake.nix` must contain `nixos-toolkit/state/selected.nix` (or `./` / `/etc/nixos/` prefix variants). Helper `detect_integration()` never returns `Unknown`.

The GUI never calls helper `GetSystemInfo`.

## Flake packaging

- crane + rust-overlay `stable.latest` (comment says 1.93+; not a 1.93 pin)
- GUI wrap: `NIXOS_TOOLKIT_TEMPLATES_DIR`, `NIXOS_TOOLKIT_HELPER`
- Helper wrap: templates dir, `PATH` prefix `nix`, `nixos-rebuild`, `git`, `hostname`
- Helper `postInstall` installs polkit policy and substitutes the exec path to `$out/bin/nixos-toolkit-helper`
- `nixosModules.default`: `programs.nixos-toolkit.enable` → polkit + systemPackages GUI (`programs.nixos-toolkit.package`) **and** `self.packages.${system}.helper` (the helper is not swapped by `.package`). No import of `selected.nix`, no directory creation
- Overlay: `nixos-toolkit`, `nixos-toolkit-helper`

## Flatpak

Manifest builds `-p gui --bin nixos-toolkit` only. finish-args: Wayland, fallback X11, DRI, talk to `org.freedesktop.Flatpak`, read-only `/etc/NIXOS`, `/etc/nixos`, `/etc/hostname`. No network share, no host filesystem.

Host helper probe: `flatpak-spawn --host -- test -x` on the usual paths.

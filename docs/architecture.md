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
  │  nix flake update --flake /etc/nixos (when channel update is requested on a flake host)
  ▼
NixOS evaluation of the user's config, which must import
  /etc/nixos/nixos-toolkit/state/selected.nix
```

The GUI never writes `/etc/nixos`. The helper never edits `configuration.nix` or `flake.nix`.

## Crates

| Crate | Path | Artifact |
|-------|------|----------|
| `common` | `crates/common` | library: catalogs, IPC types, path constants, Nix string generation (including `bundle_module_stub`) |
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

One helper **process per operation**. Apply (`Switch` / `Boot` / `Test` / `Build`) uses `run_apply_chain`: one process, `EnsureDirectories` then `Apply`, then `WriteState` on success. Dry-build sends only `Apply` (`DryBuild`) so the helper can snapshot the managed tree before creating directories.

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
| Apply | `EnsureDirectories` → `Apply` (`Switch`/`Boot`/`Test`/`Build`) → `WriteState` |
| Dry run | `Apply` (`DryBuild`) only; no `EnsureDirectories`, no `WriteState` |
| Generations | `ListGenerations`, `RollbackGeneration`, `DeleteGenerations` |
| Maintenance | `RunMaintenance`, `GetDiskUsage` |

It does **not** send `CheckPermissions`, `GetSystemInfo`, `Validate`, or `Generate`. Preview is local. System info is `integration.rs` (and `flatpak-spawn --host` inside the sandbox).

`RebuildType` in IPC: `Switch`, `Boot`, `Test`, `Build`, `DryBuild`. The Apply page dropdown uses Switch/Boot/Test/Build; Dry Run is a separate button (`DryBuild`). Rollback `activate` is `"switch"` or `"boot"`.

Streaming: during apply/rebuild the helper may emit many `Log` lines before a terminal `ApplyComplete` / `Error`.

Full field list: [reference.md](reference.md).

## Nix generation

Shared generators live in `crates/common/src/nix.rs`. The helper writes them in `crates/helper/src/nix_gen.rs`.

### `selected.nix`

Import-only module. Conditional imports:

- `../profiles/<id>.nix`
- `../bundles/<id>.nix` for each enabled bundle
- `./hostname.nix`, `./dns.nix`, `./users.nix`, `./custom-packages.nix`, `./hardware.nix`, `./network.nix`, `./services.nix` when the corresponding state is set
- `./unfree.nix` when `needs_allow_unfree` is true

### Profiles

Helper `read_template(profile.template)` from `NIXOS_TOOLKIT_TEMPLATES_DIR` (else `./nix/templates`). On success, atomic-write to `/etc/nixos/nixos-toolkit/profiles/<id>.nix`. On failure, `generate_fallback_profile` in the helper.

### Bundles (preview and GUI apply)

1. `ToggleBundle { enabled: true }` copies every catalog package id into `state.bundle_packages[id]`.
2. Apply sends that map. Local preview uses the same `NixGenOptions`.
3. If `bundle_packages` **contains** the id **or** the template is missing → `generate_fallback_bundle` (resolved nixpkgs attrs + `bundle_module_stub`).
4. Template copy happens only when the id is **absent** from `bundle_packages` and the file exists. That is the path for old `state.json` or a reconstructed **copied template**. Reconstruct restores `bundle_packages` from generated fallback files (`# Generated by NixOS Toolkit`). A toggle-on in this GUI always inserts the key.

`bundle_module_stub` and `generate_fallback_bundle` live in `common::nix` so preview and apply cannot drift on package lists or stubs. Fonts fallback uses `fonts.packages` plus `fonts.fontconfig.enable`. Catalog ids can map to a different attr (`PackageDef::nix_attr`: `bitwarden` → `bitwarden-desktop`, `julia` → `julia-bin`). `is_nix_attrpath` allows a leading `_`.

### Preview vs apply

`generate_preview_full_from` uses the same bundle rule as apply. Profile templates are inlined when the file exists; apply copies that file, or a helper fallback if it is missing (preview then omits the profile file). Dump order matches `generate_all_files` (selected, hostname, dns, users, hardware, profile, bundles, custom-packages, network, services, unfree). A missing profile template is still omitted from preview; apply writes the helper fallback.

### Network

`generate_network_nix` always emits firewall TCP/UDP lists when `NetworkConfig::has_settings()` is true. Fail2Ban is independent of SSH (with SSH it also enables `jails.sshd`). WireGuard sets `networking.wireguard.enable`, opens the listen UDP port, and writes `# nixos-toolkit.wireguardListenPort = <port>;` so reconstruct can round-trip the spin-button value. It does not emit peers, addresses, or keys.

### Hardware / PipeWire

`HardwareConfig` default `audio_server` is `0` (PipeWire). `has_settings()` is false for the struct default, so an apply with no hardware changes does **not** write `hardware.nix`. Whenever `hardware.nix` **is** written (any non-default field, including bluetooth-only), `generate_hardware_nix` emits PipeWire for `audio_server == 0` (low-latency extraConfig only if that toggle is on). PulseAudio (`1`) disables PipeWire; `2` disables both.

### Priority

| Snippet | Nix priority |
|---------|----------------|
| hostname | `lib.mkDefault` |
| DNS nameservers | `lib.mkDefault` |
| everything else | plain assignment |

`mkDefault` means an explicit `networking.hostName` / `networking.nameservers` in the user's config wins. If the user has no hostname set, the toolkit value is used.

### Dry-build

`Apply` with `DryBuild`:

1. Snapshot `/etc/nixos/nixos-toolkit` (or record that it did not exist) **before** `EnsureDirectories`.
2. Ensure directories / placeholder, generate files (`dry_run: false` so evaluation can see them), run `nixos-rebuild dry-build`.
3. Restore the snapshot. If the dir did not exist, restore deletes it.

Failure to **take** the snapshot is `HelperResponse::Error`. Failure to **restore** (after ensure, generate, or rebuild) is `HelperResponse::Error` via `dry_build_response` (combined with a rebuild/generate error when both failed).

`Generate { dry_run: true }` does not write. The GUI does not call `Generate`.

### Atomic writes and stale cleanup

Helper `atomic_write`: refuse paths that do not stay under `/etc/nixos/nixos-toolkit` (absolute, no `..`, existing-prefix canonicalize). Write `*.tmp` (extension replaced, so `selected.nix` → `selected.tmp`, `state.json` → `state.tmp`), rename, read back. This is a path-prefix check, not a kernel sandbox. Generated Nix, the `selected.nix` placeholder, and `WriteState` all use it.

After a successful generate (not helper `Generate` dry-run), `cleanup_stale_managed_files` deletes `.nix` files in `profiles/` and `bundles/` whose stems are not the current profile/bundles, and deletes unused managed snippets listed in `MANAGED_SNIPPETS` (`hostname.nix`, `dns.nix`, `users.nix`, `custom-packages.nix`, `hardware.nix`, `network.nix`, `services.nix`, `unfree.nix`). `selected.nix` and `state.json` are not in that list.

### CheckPermissions

`check_write_permission` creates an ephemeral tempfile (`prefix(".write_test-")`) in the directory and relies on `Drop` to remove it. It does not leave a stable `/etc/nixos/.write_test` file. The GUI does not send `CheckPermissions`.

## State

| Store | Path | Who |
|-------|------|-----|
| Apply selections | `/etc/nixos/nixos-toolkit/state/state.json` | helper `ReadState` / `WriteState` |
| Theme | `$XDG_CONFIG_HOME/nixos-toolkit/preferences.json` (`dirs::config_dir()`) | GUI; cosmic-config is a best-effort mirror under `APP_ID` |
| Managed Nix | `/etc/nixos/nixos-toolkit/{state,profiles,bundles}` | helper |

If `state.json` is missing or invalid, the helper reconstructs `AppState` from managed Nix with line-oriented parsers (`reconstruct_state_from_nix`):

- `selected.nix` — profile + bundle ids
- generated fallback `bundles/<id>.nix` — `bundle_packages` (copied templates skipped)
- `custom-packages.nix`, `hostname.nix`, `dns.nix`, `users.nix`
- `hardware.nix`, `network.nix` (including `wireguard_enabled` and listen port from the toolkit comment), `services.nix`

This is not a Nix parser. Unusual formatting, extra comments that look like options, or hand-edited modules can be missed or misread.

## Integration detection

GUI (`crates/gui/src/integration.rs`) and helper (`crates/helper/src/commands.rs`) both call `common::config::detect_integration_status` on the contents of `/etc/nixos/configuration.nix` and `/etc/nixos/flake.nix`. Either file is sufficient. Callers must not skip `flake.nix` when `selected.nix` already exists.

Markers (substring, after stripping `#` line comments and `/* */` blocks): `./nixos-toolkit/state/selected.nix`, `/etc/nixos/nixos-toolkit/state/selected.nix`, `nixos-toolkit/state/selected.nix`. A bare `nixos-toolkit` word is not enough. `selected.nix` existing is not required for the status.

`detect_integration()` never returns `Unknown`; `SystemInfo::default()` does when host probes are skipped. The GUI never calls helper `GetSystemInfo`.

## Flake packaging

- crane + rust-overlay `stable.latest` (comment says 1.93+; not a 1.93 pin)
- GUI wrap: `NIXOS_TOOLKIT_TEMPLATES_DIR`, `NIXOS_TOOLKIT_HELPER`
- Helper wrap: templates dir, `PATH` prefix `nix`, `nixos-rebuild`, `git`, `hostname`
- Helper `postInstall` installs polkit policy and substitutes the exec path to `$out/bin/nixos-toolkit-helper`
- `nixosModules.default`: `programs.nixos-toolkit.enable` → polkit + `systemPackages` of `programs.nixos-toolkit.package` (GUI) **and** `programs.nixos-toolkit.helperPackage` (helper). No import of `selected.nix`, no directory creation
- Overlay: `nixos-toolkit`, `nixos-toolkit-helper`

## Flatpak

Manifest builds `-p gui --bin nixos-toolkit` only. finish-args: Wayland, fallback X11, DRI, talk to `org.freedesktop.Flatpak`, read-only `/etc/NIXOS`, `/etc/nixos`, `/etc/hostname`. No network share, no host filesystem.

Host helper probe: `flatpak-spawn --host -- test -x` on the usual paths.

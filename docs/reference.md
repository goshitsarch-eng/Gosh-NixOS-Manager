# Technical reference

The product is **NixOS Toolkit**; the repository is `Gosh-NixOS-Manager`. The desktop executable is `nixos-toolkit`, the helper is `nixos-toolkit-helper`, and the Flatpak ID is `io.github.goshitsarch_eng.NixosToolkit`. The license is [GPL-3.0-or-later](../LICENSE).

## Source map

| Location | Responsibility |
| --- | --- |
| [desktop/lib](../desktop/lib/) | Flutter UI, state controller, FFI, helper transport, preferences |
| [crates/common/src/actions.rs](../crates/common/src/actions.rs) | Profile, bundle, and maintenance catalogs |
| [crates/common/src/ipc.rs](../crates/common/src/ipc.rs) | Request/response types and saved state |
| [crates/common/src/nix.rs](../crates/common/src/nix.rs) | Validation-related generation rules, package mapping, previews |
| [crates/common/src/config.rs](../crates/common/src/config.rs) | Managed paths and integration detection |
| [crates/desktop-core](../crates/desktop-core/) | Unprivileged JSON FFI API |
| [crates/helper/src](../crates/helper/src/) | Privileged operations, recovery, concurrency guard, generation |
| [crates/fake-helper](../crates/fake-helper/) | Test protocol peer |
| [crates/gui](../crates/gui/) | Former libcosmic frontend, retained for comparison |
| [nix/templates](../nix/templates/) | Desktop templates and historical bundle templates |
| [packaging/nix](../packaging/nix/) | Prebuilt installer and public cache key |

## Runtime paths

| Path | Purpose |
| --- | --- |
| `/etc/nixos/nixos-toolkit/state/selected.nix` | Entry point imported by the host configuration |
| `/etc/nixos/nixos-toolkit/state/state.json` | Saved Toolkit selections |
| `/etc/nixos/nixos-toolkit/state/*.nix` | Generated system, package, hardware, network, service, and unfree settings |
| `/etc/nixos/nixos-toolkit/profiles/` and `bundles/` | Generated/copied selection modules |
| `/etc/nixos/nixos-toolkit-install.nix` | Prebuilt installer's app/helper module |
| `/var/lib/nixos-toolkit/recovery/` | Pending app-apply recovery |
| `/var/lib/nixos-toolkit-installer/` | Installer receipt, pending journal, and private backups |
| `/run/lock/nixos-toolkit.lock` | Shared helper/installer mutation lock |
| `$XDG_CONFIG_HOME/nixos-toolkit/preferences.json` | User theme/window preferences; base defaults to `~/.config` |
| `$XDG_STATE_HOME/nixos-toolkit/launcher.log` | Installer launch output; base defaults to `~/.local/state` |

`NIXOS_TOOLKIT_CORE_LIBRARY` selects a native core library for development/tests. `NIXOS_TOOLKIT_TEMPLATES_DIR` overrides the template location. `NIXOS_TOOLKIT_HELPER` selects an absolute executable helper path. Packaged builds supply their normal paths automatically.

## Helper protocol

IPC uses one JSON object per line with `type` and optional `payload`. See [ipc.rs](../crates/common/src/ipc.rs) for the complete schema; the core's FFI API version is separate from this protocol.

| Requests | Purpose |
| --- | --- |
| `BeginSession`, `Cancel` | Interactive sessions and cooperative cancellation |
| `ReadStateSnapshot`, `ReadState`, `WriteState` | State plus revision, legacy state reading, state saving |
| `Apply` | Rebuild mode, selections, and optional `expected_revision` |
| `GetRecovery`, `RecoverApply` | Inspect and recover unfinished applies |
| `ListGenerations`, `RollbackGeneration`, `DeleteGenerations` | System generation management |
| `RunMaintenance`, `GetDiskUsage` | Allowlisted maintenance and store usage |
| `CheckPermissions`, `GetSystemInfo`, `Validate`, `Generate`, `EnsureDirectories` | Supporting/legacy requests |

The current frontend requires snapshot/revision support. It validates and previews through the local core, then uses `Apply` and `WriteState` for a successful saved change. Dry Run omits `WriteState`. [Architecture](../ARCHITECTURE.md) describes session, locking, and recovery behavior; the [usage guide](USAGE.md#choose-a-rebuild-mode) explains rebuild modes.

## Configuration rules

- Integration detection looks for an uncommented `nixos-toolkit/state/selected.nix` path in `configuration.nix` or `flake.nix`. It is a source-text check, not proof that Nix evaluates the intended module.
- The helper expects `/etc/nixos`. On flake hosts it runs `nixos-rebuild <mode> --flake /etc/nixos`; the default output normally matches the current hostname. The installer can accept an explicit host name, but it does not change this app behavior.
- Hostname and DNS use `lib.mkDefault`; desktop profiles can force display-manager choices. Generated settings do not override every manual option.
- Bundle output uses selected catalog packages and required service configuration. It deduplicates aliases such as npm/JavaScript/Node and Flutter/Dart. [Software bundles](SOFTWARE_BUNDLES.md) records the recent mappings and older-channel fallback.
- Known proprietary selections enable `allowUnfree`. Insecure-package checks remain enabled. Unknown custom unfree packages may need a user-supplied Nix setting.
- Reconstruction reads Toolkit's generated formats and markers; it is not a general Nix parser or host inventory.
- WireGuard controls enable support and a listen port. Peer addresses and keys remain manual configuration.

Maintenance accepts only garbage collection, store optimization, integrity verification, and channel/flake updates. The **Update Channels** request becomes `nix flake update --flake /etc/nixos` when that directory contains a flake.

## Packages and host module

[flake.nix](../flake.nix) exports `default`/`gui`, `helper`, `core`, `templates`, `full`, `installer`, and `reference` packages. `nixosModules.default` provides `programs.nixos-toolkit.enable`, `.package`, and `.helperPackage`; it installs matching packages and polkit policy. `nixosModules.activationReloadCompat` supplies the NixOS 25.05 reload fix.

Polkit authenticates privileged requests; the desktop itself never becomes root. The prebuilt installer adds the host imports, while manual module users add the selections import themselves. See [INSTALLING.md](../INSTALLING.md) and [BUILDING.md](../BUILDING.md) for complete commands.

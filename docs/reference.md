# Reference

Facts from the current source. Catalog tables are `default_*` in `crates/common/src/actions.rs` unless noted.

## Identity

| Item | Value |
|------|--------|
| Repository | `https://github.com/goshitsarch-eng/Gosh-NixOS-Manager` |
| Product name | NixOS Toolkit |
| App ID | `io.github.goshitsarch_eng.NixosToolkit` |
| GUI crate / binary | `gui` / `nixos-toolkit` |
| Helper crate / binary | `helper` / `nixos-toolkit-helper` |
| Version | `0.1.0` |
| Edition / MSRV | 2021 / 1.93 |
| License | GPL-3.0-or-later (`LICENSE`; Cargo `GPL-3.0-or-later`; flake `licenses.gpl3Plus`) |
| libcosmic rev | `7cc116803b18d7b888eb511b009f775f036c3da7` |

## Paths

`crates/common/src/config.rs` `paths`:

| Constant | Path |
|----------|------|
| `MANAGED_DIR` | `/etc/nixos/nixos-toolkit` |
| `STATE_DIR` | `/etc/nixos/nixos-toolkit/state` |
| `PROFILES_DIR` | `/etc/nixos/nixos-toolkit/profiles` |
| `BUNDLES_DIR` | `/etc/nixos/nixos-toolkit/bundles` |
| `SELECTED_NIX` | `…/state/selected.nix` |
| `STATE_JSON` | `…/state/state.json` |
| `HOSTNAME_NIX` | `…/state/hostname.nix` |
| `DNS_NIX` | `…/state/dns.nix` |
| `USERS_NIX` | `…/state/users.nix` |
| `CUSTOM_PACKAGES_NIX` | `…/state/custom-packages.nix` |
| `HARDWARE_NIX` | `…/state/hardware.nix` |
| `NETWORK_NIX` | `…/state/network.nix` |
| `SERVICES_NIX` | `…/state/services.nix` |
| `UNFREE_NIX` | `…/state/unfree.nix` |

User prefs (GUI): `$XDG_CONFIG_HOME/nixos-toolkit/preferences.json`.

Templates: `$NIXOS_TOOLKIT_TEMPLATES_DIR`, else `./nix/templates`.

Host probes: `/etc/NIXOS`, `/etc/os-release`, `/etc/nixos/flake.nix`, `/etc/nixos/configuration.nix`, `/etc/hostname`.

## IPC

`#[serde(tag = "type", content = "payload")]`. Line-delimited JSON. No protocol version field; new fields use `#[serde(default)]`.

### `HelperRequest` (13)

| Variant | Payload |
|---------|---------|
| `CheckPermissions` | — |
| `GetSystemInfo` | — |
| `Validate` | `selected_profile`, `enabled_bundles`, `hostname` |
| `Generate` | selection fields + `dry_run` |
| `Apply` | selection fields + `rebuild_type` |
| `EnsureDirectories` | — |
| `ReadState` | — |
| `WriteState` | `state: AppState` |
| `ListGenerations` | — |
| `RollbackGeneration` | `generation: u32`, `activate: String` (default `"switch"`) |
| `DeleteGenerations` | `generations: Vec<u32>` |
| `RunMaintenance` | `command: String` |
| `GetDiskUsage` | — |

Generate/Apply selection fields: `selected_profile`, `enabled_bundles`, `bundle_packages`, `hostname`, `dns_servers`, `user_groups`, `username`, `bluetooth_enabled`, `custom_packages`, `network_config`, `services_config`, `hardware_config`.

The GUI does not send `CheckPermissions`, `GetSystemInfo`, `Validate`, or `Generate`.

### `RebuildType`

`Switch` → `switch`, `Boot` → `boot`, `Test` → `test`, `Build` → `build`, `DryBuild` → `dry-build`.

GUI: dropdown Switch/Boot/Test/Build; Dry Run button sends `DryBuild`. `WriteState` runs after successful Switch/Boot/Test/Build, not DryBuild.

Flake hosts: helper appends `--flake /etc/nixos#{hostname}` (`/etc/hostname`, default `nixos`). Classic: no extra args. No `--impure`, no timeout.

### `HelperResponse`

`Ok`, `Error { message, details }`, `Permissions { can_read_config, can_write_managed, can_run_rebuild }`, `SystemInfo`, `ValidationResult { valid, errors, warnings }`, `GenerationResult { files, preview }`, `Log { level, message }`, `ApplyComplete { success, message }`, `State`, `Generations`, `MaintenanceOutput { stdout, stderr, success }`, `DiskUsage`.

### `AppState` (persisted)

`selected_profile`, `enabled_bundles`, `bundle_packages`, `hostname`, `dns_servers`, `user_groups`, `username`, `bluetooth_enabled`, `last_applied`, `custom_packages`, `network_config`, `services_config`, `hardware_config`.

### `NetworkConfig`

`firewall_enabled` (default true), `allowed_tcp_ports`, `allowed_udp_ports`, `ssh_enabled`, `ssh_port` (22), `ssh_password_auth`, `ssh_root_login` (`"no"`), `fail2ban_enabled`, `tailscale_enabled`, `wireguard_enabled`, `wireguard_listen_port` (51820).

`has_settings()` is true if TCP/UDP lists are non-empty, SSH is on, Tailscale is on, Fail2Ban is on, the firewall is **off**, SSH port ≠ 22, password auth is on, root-login ≠ `"no"`, or WireGuard is on. Changing only `wireguard_listen_port` while WireGuard is off does not count.

`generate_network_nix` emits firewall (including WireGuard listen UDP), optional OpenSSH, Fail2Ban (with `jails.sshd` only when SSH is on), Tailscale, and `networking.wireguard.enable`.

### `HardwareConfig`

| Field | Meaning |
|-------|---------|
| `nvidia_driver` | `None` omit; `0` stable; `1` beta; `2` open (`nvidiaPackages.latest`); `3` nouveau |
| `nvidia_modesetting` | default true |
| `nvidia_powermanagement` | |
| `nvidia_open` | `hardware.nvidia.open` |
| `audio_server` | `0` PipeWire (emitted whenever `hardware.nix` is written); `1` PulseAudio; `2` none |
| `audio_lowlatency` | PipeWire 32-quantum extraConfig |
| `bluetooth_enabled` / `bluetooth_autopower` | Blueman + `powerOnBoot` |
| `power_profile` | `0` balanced (no PPD); `1` performance; `2` power-saver |
| `tlp_enabled` | TLP on, PPD off |
| `thermald_enabled` | |

`has_settings()` is `self != Default`. Default `audio_server` is 0, so PipeWire-only is **not** a non-default field: `hardware.nix` is omitted until some other field changes. Bluetooth-only apply writes `hardware.nix` and therefore also enables PipeWire.

Sibling `bluetooth_enabled` on Generate/Apply/AppState is OR-ed into hardware via `with_bluetooth_or`.

### Maintenance allowlist

Exact match in `commands.rs` (`ALLOWED_MAINTENANCE_COMMANDS`):

```
nix-collect-garbage
nix-collect-garbage -d
nix-store --optimise
nix-store --verify --check-contents
nix-channel --update
nix flake update --flake /etc/nixos
```

GUI catalog still has 5 actions. `update_channels` sends `nix-channel --update`. If `/etc/nixos/flake.nix` exists, the helper rewrites that request to `nix flake update --flake /etc/nixos` (the rewritten string is also on the allowlist).

### Validation

Uses `default_profiles()` / `default_bundles()` at runtime (not a hardcoded helper list). Unknown profile → error. Unknown bundle → warning. Hostname: non-empty, alphanumeric or `-`, max 63.

The GUI does not call `Validate`.

## Profiles (13)

| id | display_manager | arm | template |
|----|-----------------|-----|----------|
| `gnome` | gdm | Full | `profiles/gnome.nix` |
| `kde` | sddm | Full | `profiles/kde.nix` |
| `xfce` | lightdm | Full | `profiles/xfce.nix` |
| `mate` | lightdm | Full | `profiles/mate.nix` |
| `cinnamon` | lightdm | Full | `profiles/cinnamon.nix` |
| `pantheon` | lightdm | Full | `profiles/pantheon.nix` |
| `cosmic` | cosmic-greeter | Full | `profiles/cosmic.nix` |
| `hyprland` | sddm | Full | `profiles/hyprland.nix` |
| `sway` | sddm | Full | `profiles/sway.nix` |
| `i3` | lightdm | Full | `profiles/i3.nix` |
| `budgie` | lightdm | Full | `profiles/budgie.nix` |
| `lxqt` | sddm | Full | `profiles/lxqt.nix` |
| `enlightenment` | lightdm | Full | `profiles/enlightenment.nix` |

No `session` field on `ProfileDef`. What the templates enable (apply copies these files):

| id | DE/WM option | Wayland-related |
|----|--------------|-----------------|
| gnome | `services.desktopManager.gnome` | `gdm.wayland = true`; also `services.xserver.enable` |
| kde | `services.desktopManager.plasma6` | `sddm.wayland.enable` |
| cosmic | `services.desktopManager.cosmic` | greeter; no `xserver.enable` |
| hyprland | `programs.hyprland` + xwayland | SDDM Wayland |
| sway | `programs.sway` | SDDM Wayland |
| xfce, mate, cinnamon, pantheon, budgie, enlightenment, i3 | `services.xserver.*` | LightDM, X11 |
| lxqt | `services.xserver.desktopManager.lxqt` | SDDM without `wayland.enable` |

## Bundles (16 defs / 15 files)

If `bundle_packages` contains the bundle id (always true after enabling it in this GUI), preview and apply write **catalog packages** (resolved attrs) + **stub**. The template file is copied only when that key is absent (legacy/reconstructed state) and the file exists.

### Catalog + stub

`bundle_module_stub` is `crates/common/src/nix.rs`.

| id | Catalog package ids | `bundle_module_stub` |
|----|---------------------|----------------------|
| `devtools` | git, neovim, emacs, vscode, vscodium, zed-editor, rustup, nodejs, python3, docker | git + docker daemon |
| `ai-tools` | ollama | none; **no** `ai-tools.nix` (preview and apply both use fallback) |
| `gaming` | steam, lutris, heroic, bottles, mangohud, gamemode | Steam (firewall/gamescope session), 32-bit GL, GameMode. **Not** gamescope program, wine, PAM limits |
| `virtualization` | virt-manager, qemu, spice-gtk | kvm-amd+kvm-intel, libvirtd, swtpm, ovmf enable, virt-manager. Stub does not set `ovmf.packages` |
| `virtualbox` | virtualbox | `allowUnfree`, host + extension pack. **Not** guest / guest.x11 |
| `containers` | podman, docker-compose, buildah, skopeo | Podman `dockerCompat`. Docker commented in template; stub does not enable Docker |
| `flatpak` | flatpak | `services.flatpak` + `xdg.portal` (no gnome-software, no extra portal package) |
| `multimedia` | vlc, mpv, gimp, inkscape, obs-studio, audacity | empty stub (no PipeWire) |
| `office` | libreoffice, onlyoffice-desktopeditors, thunderbird, evince, obsidian | empty (no CUPS/SANE) |
| `security` | keepassxc, bitwarden (`bitwarden-desktop`), `_1password-gui`, veracrypt, gnupg, age | empty (no gnupg.agent) |
| `communication` | discord, signal-desktop, element-desktop, slack, zoom-us | empty |
| `browsers` | firefox, chromium, google-chrome, brave, tor-browser-bundle-bin | empty (no `programs.firefox` policies) |
| `science` | octave, julia (`julia-bin`), gnuplot | empty |
| `cad` | blender, freecad, openscad, kicad | empty |
| `utilities` | htop, btop, fastfetch, tmux, tree, unzip, wget, curl, appimage-run | empty |
| `fonts` | nerd-fonts.fira-code, nerd-fonts.jetbrains-mono, fira-code, jetbrains-mono, inter, noto-fonts | `fonts.fontconfig`; fallback uses `fonts.packages` |

### Template extras (legacy / reconstruct path)

These appear in `nix/templates/bundles/` and are copied only when `bundle_packages` omits the id.

| File | Extra vs catalog (incomplete; see the file) |
|------|-----------------------------------------------|
| `devtools.nix` | git-lfs, gh, lazygit, cmake/ninja, nodejs_22, go, ripgrep, fd, docker-compose, … |
| `gaming.nix` | wineWowPackages, protonup-qt, gamescope, PAM nofile; Steam via `programs.steam` not the package list |
| `multimedia.nix` | krita, PipeWire, gstreamer, celluloid, … |
| `office.nix` | printing drivers, SANE, logseq, … |
| `security.nix` | wireshark, nmap, sops, bitwarden-desktop (catalog id is `bitwarden`) |
| `science.nix` | R, rstudio, texlive, julia-bin (catalog id is `julia`) |
| `utilities.nix` | neofetch, 30+ CLI tools; **no** `appimage-run` |
| `fonts.nix` | many nerd/corefonts/vistafonts + `fonts.fontconfig` |
| `browsers.nix` | ungoogled-chromium, qutebrowser, nyxt; **no** `google-chrome` |
| `communication.nix` | telegram, thunderbird, jitsi, … |
| `virtualbox.nix` | guest + guest.x11 |
| `containers.nix` | lazydocker, dive; podman via module not package |

ARM notes in catalog that do not match package lists: science mentions RStudio; gaming mentions Wine; browsers mention Chrome (Chrome **is** in the catalog).

### Unfree

`needs_allow_unfree` writes `unfree.nix` (`nixpkgs.config.allowUnfree = true`) when:

- NVIDIA driver index is 0..=2
- bundle id is `gaming` or `virtualbox`
- bundle id is `fonts` and `bundle_packages` has no entry (template includes corefonts/vistafonts)
- a selected catalog/custom attr is in `attr_needs_unfree`: `steam`, `google-chrome`, `_1password-gui`, `corefonts`, `vistafonts`, `discord`, `slack`, `zoom-us`, `virtualbox`, `vscode`, `obsidian`, `onlyoffice-desktopeditors`

This is not nixpkgs `meta.unfree`.

## System actions (5)

`hostname` (text), `dns` (text), `libvirtd_user`, `docker_user`, `vboxusers`. Toggle/Select variants exist on the enum and are unused.

The System page renders username plus the three `UserGroup` actions. Hostname/DNS are separate inputs, not the catalog TextInput widgets.

## Services (21)

| id | Generated Nix |
|----|----------------|
| `printing` | CUPS + Avahi + nssmdns4 |
| `avahi` | avahi + nssmdns4 + publish |
| `fwupd` | `services.fwupd.enable` |
| `upower` | `services.upower.enable` |
| `networkmanager` | `networking.networkmanager.enable` |
| `resolved` | `services.resolved.enable` |
| `rustdesk` | `environment.systemPackages = [ rustdesk ]` |
| `syncthing` | `services.syncthing.enable` |
| `locate` | `services.locate` + `pkgs.plocate` |
| `flatpak` | `services.flatpak` + `xdg.portal.enable` |
| `gnome_keyring` | `services.gnome.gnome-keyring.enable` |
| `gnome_tweaks` | `environment.systemPackages = [ gnome-tweaks ]` |
| `dconf` | `programs.dconf.enable` |
| `docker` | `virtualisation.docker.enable` |
| `libvirtd` | libvirtd + `programs.virt-manager.enable` |
| `postgresql` | `services.postgresql.enable` |
| `redis` | `services.redis.servers."".enable` |
| `earlyoom` | `services.earlyoom.enable` |
| `auto_upgrade` | `system.autoUpgrade.enable` |
| `auto_gc` | weekly gc `--delete-older-than 30d` |
| `store_optimize` | `nix.settings.auto-optimise-store` |

`generate_services_nix` also has a `"tailscale"` arm; `ServicesConfig::enabled_services()` never yields it. Tailscale is generated in `network.nix`.

## Maintenance (5 GUI / 6 allowlist strings)

| id | Command sent | Warning dialog |
|----|--------------|----------------|
| `gc_unreachable` | `nix-collect-garbage` | no |
| `gc_all` | `nix-collect-garbage -d` | yes |
| `optimize_store` | `nix-store --optimise` | no |
| `verify_store` | `nix-store --verify --check-contents` | no |
| `update_channels` | `nix-channel --update` (rewritten on flake hosts) | no |

## Polkit

File: `data/polkit/org.nixos-toolkit.helper.policy`.

| Action | Defaults | Exec annotation |
|--------|----------|-----------------|
| `org.nixos-toolkit.helper.manage-system` | any/inactive `auth_admin`; active `auth_admin_keep` | `/run/current-system/sw/bin/nixos-toolkit-helper` in source; Nix substitute → `$out/bin/…`; `allow_gui=true` |
| `org.nixos-toolkit.helper.write-config` | active `auth_admin_keep` | same `exec.path` / `allow_gui` |
| `org.nixos-toolkit.helper.rebuild` | active `auth_admin` (no keep) | same `exec.path` / `allow_gui` |

## Desktop entry

`data/nixos-toolkit.desktop`: Name `NixOS Toolkit`, Exec `nixos-toolkit`, Icon `nixos-toolkit`, Categories `System;Settings;`, StartupWMClass `io.github.goshitsarch_eng.NixosToolkit`. Not DBus-activatable.

`install.sh` writes a different desktop file (`nix run …`, icon `preferences-system`).

## Flake outputs

Per system: `packages.{default,gui,helper,templates,full}`, `apps.{default,helper}`, `devShells.default`, `checks.{nixos-toolkit-gui,nixos-toolkit-helper,clippy,fmt,audit}`.

Top-level: `nixosModules.default` (`programs.nixos-toolkit.enable` / `.package` / `.helperPackage`), `overlays.default`.

No formatter, hydraJobs, cachix, or `nixConfig` substituters.

GitHub Actions: cargo job (build/clippy/test) and flake job (`fmt` + `audit` checks) on push/PR. Flatpak job is `workflow_dispatch` only.

## Integration

`common::config::detect_integration_status` (GUI and helper). Markers: `./nixos-toolkit/state/selected.nix`, `/etc/nixos/nixos-toolkit/state/selected.nix`, `nixos-toolkit/state/selected.nix`. Either `configuration.nix` or `flake.nix` is enough. `#` line comments and `/* */` blocks are stripped.

## Reconstruct

`reconstruct_state_from_nix` in `crates/helper/src/commands.rs` is line-oriented. It reads `selected.nix`, generated fallback `bundles/<id>.nix` (`bundle_packages`), `custom-packages.nix`, `hostname.nix`, `dns.nix`, `users.nix`, `hardware.nix`, `network.nix` (including WireGuard enable + `# nixos-toolkit.wireguardListenPort =`), `services.nix`. Copied templates do not fill `bundle_packages`.

## Known limitations

1. i18n is English Fluent only (`crates/gui/i18n/en/`). Catalog names in `actions.rs` stay English. Service rows and the channel/flake maintenance labels are Fluent. Apply dialogs are Fluent.
2. GitHub Actions Flatpak job is `workflow_dispatch` only (disk). Flake `fmt`/`audit` run on PRs. No Cachix / binary cache.
3. Write jail is a path-prefix check under `/etc/nixos/nixos-toolkit`, not a kernel sandbox. `atomic_write` covers generated Nix, the `selected.nix` placeholder, and `state.json`.
4. Reconstruct is line-oriented, not a Nix parser. Unusual hand-edited modules may be missed. Generated fallback bundles restore `bundle_packages`; copied templates do not.
5. Unfree is an allowlist of known attrs/bundles, not nixpkgs `meta.unfree`. Unknown unfree attrs can still fail eval.
6. WireGuard UI enables `networking.wireguard.enable` and opens the listen UDP port. It does not write peers, addresses, or private keys.
7. Preview vs apply: bundle generation and dump order match apply. A missing profile template is omitted from preview; apply still writes the helper fallback.
8. GUI never sends `CheckPermissions` / `GetSystemInfo` / `Validate` / `Generate`.
9. Hostname `lib.mkDefault` still sets the hostname when the user has no explicit `networking.hostName`.

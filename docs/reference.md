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
| License (Cargo / flake) | GPL-3.0-or-later / `licenses.gpl3Plus` |
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

### `RebuildType`

`Switch` → `switch`, `Boot` → `boot`, `Test` → `test`, `Build` → `build`, `DryBuild` → `dry-build`.

Flake hosts: helper appends `--flake /etc/nixos#{hostname}` (`/etc/hostname`, default `nixos`). Classic: no extra args. No `--impure`, no timeout.

### `HelperResponse`

`Ok`, `Error { message, details }`, `Permissions { can_read_config, can_write_managed, can_run_rebuild }`, `SystemInfo`, `ValidationResult { valid, errors, warnings }`, `GenerationResult { files, preview }`, `Log { level, message }`, `ApplyComplete { success, message }`, `State`, `Generations`, `MaintenanceOutput { stdout, stderr, success }`, `DiskUsage`.

### `AppState` (persisted)

`selected_profile`, `enabled_bundles`, `bundle_packages`, `hostname`, `dns_servers`, `user_groups`, `username`, `bluetooth_enabled`, `last_applied`, `custom_packages`, `network_config`, `services_config`, `hardware_config`.

### `NetworkConfig`

`firewall_enabled` (default true), `allowed_tcp_ports`, `allowed_udp_ports`, `ssh_enabled`, `ssh_port` (22), `ssh_password_auth`, `ssh_root_login` (`"no"`), `fail2ban_enabled`, `tailscale_enabled`.

`has_settings()` is true if TCP/UDP lists are non-empty, SSH is on, Tailscale is on, or the firewall is **off**. Fail2Ban, SSH port, and root-login alone do not count.

### `HardwareConfig`

| Field | Meaning |
|-------|---------|
| `nvidia_driver` | `None` omit; `0` stable; `1` beta; `2` open (`nvidiaPackages.latest`); `3` nouveau |
| `nvidia_modesetting` | default true |
| `nvidia_powermanagement` | |
| `nvidia_open` | `hardware.nvidia.open` |
| `audio_server` | `0` PipeWire (emitted only if `audio_lowlatency`); `1` PulseAudio; `2` none |
| `audio_lowlatency` | PipeWire 32-quantum extraConfig |
| `bluetooth_enabled` / `bluetooth_autopower` | Blueman + `powerOnBoot` |
| `power_profile` | `0` balanced (no PPD); `1` performance; `2` power-saver |
| `tlp_enabled` | TLP on, PPD off |
| `thermald_enabled` | |

Sibling `bluetooth_enabled` on Generate/Apply/AppState is OR-ed into hardware via `with_bluetooth_or`.

### Maintenance allowlist

Exact match in `commands.rs`:

```
nix-collect-garbage
nix-collect-garbage -d
nix-store --optimise
nix-store --verify --check-contents
nix-channel --update
```

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

If `bundle_packages` contains the bundle id (always true after enabling it in this GUI), apply writes **catalog packages** + **stub**. The template file is copied only when that key is absent (legacy/reconstructed state) and the file exists.

### Catalog + stub

| id | Catalog package ids | `bundle_module_stub` |
|----|---------------------|----------------------|
| `devtools` | git, neovim, emacs, vscode, vscodium, zed-editor, rustup, nodejs, python3, docker | git + docker daemon |
| `ai-tools` | ollama | none; **no** `ai-tools.nix` |
| `gaming` | steam, lutris, heroic, bottles, mangohud, gamemode | Steam (firewall/gamescope session), 32-bit GL, GameMode. **Not** gamescope program, wine, PAM limits |
| `virtualization` | virt-manager, qemu, spice-gtk | kvm-amd+kvm-intel, libvirtd, swtpm, ovmf enable, virt-manager. Stub does not set `ovmf.packages` |
| `virtualbox` | virtualbox | `allowUnfree`, host + extension pack. **Not** guest / guest.x11 |
| `containers` | podman, docker-compose, buildah, skopeo | Podman `dockerCompat`. Docker commented in template; stub does not enable Docker |
| `flatpak` | flatpak | `services.flatpak` + `xdg.portal` (no gnome-software, no extra portal package) |
| `multimedia` | vlc, mpv, gimp, inkscape, obs-studio, audacity | empty stub (no PipeWire) |
| `office` | libreoffice, onlyoffice-desktopeditors, thunderbird, evince, obsidian | empty (no CUPS/SANE) |
| `security` | keepassxc, bitwarden, `_1password-gui`, veracrypt, gnupg, age | empty (no gnupg.agent) |
| `communication` | discord, signal-desktop, element-desktop, slack, zoom-us | empty |
| `browsers` | firefox, chromium, google-chrome, brave, tor-browser-bundle-bin | empty (no `programs.firefox` policies) |
| `science` | octave, julia, gnuplot | empty |
| `cad` | blender, freecad, openscad, kicad | empty |
| `utilities` | htop, btop, fastfetch, tmux, tree, unzip, wget, curl, appimage-run | empty |
| `fonts` | nerd-fonts.fira-code, nerd-fonts.jetbrains-mono, fira-code, jetbrains-mono, inter, noto-fonts | empty (fallback puts these in `environment.systemPackages`, not `fonts.packages`) |

### Template extras (preview / unused-on-GUI-apply)

These appear in `nix/templates/bundles/` and in local preview when the file exists. They are **not** installed by default GUI apply.

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

## Maintenance (5)

| id | Command | Warning dialog |
|----|---------|----------------|
| `gc_unreachable` | `nix-collect-garbage` | no |
| `gc_all` | `nix-collect-garbage -d` | yes |
| `optimize_store` | `nix-store --optimise` | no |
| `verify_store` | `nix-store --verify --check-contents` | no |
| `update_channels` | `nix-channel --update` | no |

## Polkit

File: `data/polkit/org.nixos-toolkit.helper.policy`.

| Action | Defaults | Exec annotation |
|--------|----------|-----------------|
| `org.nixos-toolkit.helper.manage-system` | any/inactive `auth_admin`; active `auth_admin_keep` | `/run/current-system/sw/bin/nixos-toolkit-helper` in source; Nix substitute → `$out/bin/…`; `allow_gui=true` |
| `org.nixos-toolkit.helper.write-config` | active `auth_admin_keep` | none |
| `org.nixos-toolkit.helper.rebuild` | active `auth_admin` (no keep) | none |

## Desktop entry

`data/nixos-toolkit.desktop`: Name `NixOS Toolkit`, Exec `nixos-toolkit`, Icon `nixos-toolkit`, Categories `System;Settings;`, StartupWMClass `io.github.goshitsarch_eng.NixosToolkit`. Not DBus-activatable.

`install.sh` writes a different desktop file (`nix run …`, icon `preferences-system`).

## Flake outputs

Per system: `packages.{default,gui,helper,templates,full}`, `apps.{default,helper}`, `devShells.default`, `checks.{nixos-toolkit-gui,nixos-toolkit-helper,clippy,fmt,audit}`.

Top-level: `nixosModules.default` (`programs.nixos-toolkit.enable` / `.package`), `overlays.default`.

No formatter, hydraJobs, cachix, or `nixConfig` substituters.

## Known limitations

1. Bundle preview always inlines the template file when it exists and ignores `bundle_packages` (unchecks do not change the preview). GUI apply after enable writes catalog + stub.
2. `ai-tools` has no template; preview omits a bundle file; apply writes packages-only.
3. Fonts fallback uses `environment.systemPackages`, not `fonts.packages`.
4. Helper does not delete stale managed files (`profiles/`, `bundles/`, or leftover `hostname.nix` / `network.nix` / … once `selected.nix` stops importing them).
5. No UDP widget; no WireGuard.
6. `NetworkConfig.has_settings` ignores Fail2Ban-only / SSH-option-only changes; Fail2Ban Nix is nested under SSH enable.
7. Default PipeWire is not emitted unless low-latency is on (or you use the unused multimedia template).
8. `apply_is_empty` ignores network, services, hostname, DNS, and user groups. Those still get the destructive empty-apply dialog.
9. Hostname `mkForce` can override the user's hostname.
10. Integration: GUI substring vs helper path.
11. Reconstruct-from-Nix ignores most settings.
12. Unfree: only the VirtualBox stub sets `allowUnfree`.
13. `nix-channel --update` on flake systems.
14. GUI does not expose `Boot` / `Test` / `Build` rebuild types.
15. No write jail: helper writes the constant paths; `atomic_write` does not check a prefix.
16. CheckPermissions may create `/etc/nixos/.write_test` or `…/nixos-toolkit/.write_test`.
17. Dry-build: snapshot failure is `Error`; restore failure after a finished dry-build is a log line.
18. i18n English only; many strings hardcoded.
19. No LICENSE file in the repository root.
20. GitHub Actions does not run flake fmt/audit or Flatpak on PRs.
21. Fallback package lists are filtered by `is_nix_attrpath` (must start with a letter). Catalog id `_1password-gui` is dropped from generated Nix.
22. Catalog ids that are not nixpkgs attrs (`bitwarden` vs template `bitwarden-desktop`, `julia` vs template `julia-bin`) are interpolated as-is in fallback Nix.
23. `programs.nixos-toolkit.package` replaces the GUI package only; the helper is always `self.packages.${system}.helper`.
24. Polkit actions `write-config` and `rebuild` have no `exec.path`; only `manage-system` annotates pkexec.

# NixOS Toolkit

A [libcosmic](https://github.com/pop-os/libcosmic) (iced) desktop app for declarative NixOS management.

The repository is `Gosh-NixOS-Manager`. The application name, desktop entry, About dialog, and binaries are **NixOS Toolkit**: `nixos-toolkit` (GUI) and `nixos-toolkit-helper` (privileged helper).

You pick a desktop profile, software bundles, packages, and system settings in the GUI. The helper writes Nix modules under `/etc/nixos/nixos-toolkit/` and runs `nixos-rebuild`. It does not edit your `configuration.nix` or `flake.nix`.

Version `0.1.0`. License: GPL-3.0-or-later (`LICENSE` in the repository root; also declared in `Cargo.toml` and `flake.nix`).

---

## Quick start

1. Install the GUI **and** the helper (the [NixOS module](#nixos-module-recommended) is the supported way).
2. Create `/etc/nixos/nixos-toolkit/state/selected.nix` and import it from your NixOS config ([One-time setup](#one-time-setup)).
3. Rebuild once so the empty placeholder evaluates.
4. Run `nixos-toolkit`, choose a profile/bundles/settings, review the preview, click **Apply Changes**.

Privileged work (apply, generations, maintenance) uses `pkexec` and needs a running polkit authentication agent.

---

## Table of contents

- [Installation](#installation)
- [One-time setup](#one-time-setup)
- [Using the app](#using-the-app)
- [What apply actually writes](#what-apply-actually-writes)
- [Features](#features)
- [Architecture](#architecture)
- [Security](#security)
- [Development](#development)
- [Known limitations](#known-limitations)
- [Further documentation](#further-documentation)

---

## Installation

### NixOS module (recommended)

This puts both binaries on `PATH`, enables polkit, and installs the helper's polkit policy:

```nix
# flake.nix
{
  inputs.nixos-toolkit.url = "github:goshitsarch-eng/Gosh-NixOS-Manager";

  outputs = { nixpkgs, nixos-toolkit, ... }: {
    nixosConfigurations.your-hostname = nixpkgs.lib.nixosSystem {
      modules = [
        nixos-toolkit.nixosModules.default
        # ...
      ];
    };
  };
}
```

```nix
# system config
{
  programs.nixos-toolkit.enable = true;
}
```

Optional package overrides:

- `programs.nixos-toolkit.package` — GUI package (default `self.packages.${system}.default`)
- `programs.nixos-toolkit.helperPackage` — helper package (default `self.packages.${system}.helper`)

The module does **not** create `/etc/nixos/nixos-toolkit/` and does **not** add the `selected.nix` import. You still need [one-time setup](#one-time-setup).

### `nix run`

Requires flakes (or extra experimental features):

```bash
nix run github:goshitsarch-eng/Gosh-NixOS-Manager
```

Without flakes enabled:

```bash
nix --extra-experimental-features 'nix-command flakes' run github:goshitsarch-eng/Gosh-NixOS-Manager
```

`nix run` launches the GUI wrapped with `NIXOS_TOOLKIT_HELPER` pointing at the helper in the Nix store. Apply still goes through `pkexec`. For a polkit policy that matches a system-profile helper path, use the NixOS module.

### Profile install

```bash
nix profile install github:goshitsarch-eng/Gosh-NixOS-Manager
```

Installs the GUI package (`packages.default`). The wrapper still points `NIXOS_TOOLKIT_HELPER` at a store helper, but that does not put the helper on `PATH` or install the polkit policy. Prefer the NixOS module for privileged operations.

### Convenience script (`install.sh`)

```bash
curl -sSL https://raw.githubusercontent.com/goshitsarch-eng/Gosh-NixOS-Manager/main/install.sh | bash
```

This does **not** install packages. It optionally:

- adds a shell alias/function `nixos-toolkit` that runs `nix run github:goshitsarch-eng/Gosh-NixOS-Manager`
- writes `~/.local/share/applications/nixos-toolkit.desktop` with `Icon=preferences-system` (not the app SVG)
- offers to run `nix run` immediately

### Flatpak

App ID: `io.github.goshitsarch_eng.NixosToolkit`. The Flatpak is **GUI only** (Freedesktop Platform 25.08). It does not ship `nixos-toolkit-helper`.

Apply, generations, and maintenance call `flatpak-spawn --host -- pkexec <host-helper>`. Install the helper on the host (NixOS module). If the host helper is missing, the GUI still opens for preview and local preferences.

Local build: `scripts/build-flatpak.sh` (needs `flatpak/cargo-sources.json` from `scripts/generate-cargo-sources.sh`).

### Enable flakes (if needed)

```nix
nix.settings.experimental-features = [ "nix-command" "flakes" ];
```

Then `sudo nixos-rebuild switch`.

---

## One-time setup

NixOS evaluates every import. If you import a file that does not exist, rebuild fails with `path '…/selected.nix' does not exist`.

### Classic `configuration.nix`

```bash
sudo mkdir -p /etc/nixos/nixos-toolkit/state
sudo tee /etc/nixos/nixos-toolkit/state/selected.nix > /dev/null << 'EOF'
{ config, lib, pkgs, ... }: { imports = []; }
EOF
```

Add the import **inside** the `imports` list:

```nix
{ config, pkgs, ... }:
{
  imports = [
    ./hardware-configuration.nix
    ./nixos-toolkit/state/selected.nix
  ];
}
```

```bash
sudo nixos-rebuild switch
```

### Flake-based config

Same placeholder file, then add the path to the host's `modules` list (often in `/etc/nixos/flake.nix`):

```nix
modules = [
  ./configuration.nix
  ./nixos-toolkit/state/selected.nix
];
```

```bash
sudo nixos-rebuild switch --flake .#
```

The helper creates the directory tree and placeholder on **Apply** (`EnsureDirectories` first). **Dry Run** skips that request so it can snapshot the managed tree, then creates directories inside `Apply` and restores afterwards. The import line in *your* config is always manual.

### Verifying integration

**Getting Started** shows **Integrated**, **Not integrated**, or **Unknown** (Unknown only if host probes were skipped).

The GUI and the helper share `detect_integration_status` in `crates/common/src/config.rs`. Either `/etc/nixos/configuration.nix` or `/etc/nixos/flake.nix` is enough. The check is a substring match for one of:

- `./nixos-toolkit/state/selected.nix`
- `/etc/nixos/nixos-toolkit/state/selected.nix`
- `nixos-toolkit/state/selected.nix`

A bare `nixos-toolkit` mention does **not** count. Comments are not stripped, so a comment that contains one of those exact strings is treated as integrated. The status does not require `selected.nix` to exist yet.

After setup, rebuild once and use **Verify integration**.

---

## Using the app

Sidebar pages (11):

| Page | What it does |
|------|----------------|
| Getting Started | Host detection, integration status, copyable import snippet |
| Desktop Profiles | One of 13 desktop/WM profiles; local template preview |
| Software Bundles | 16 bundles; expand to toggle catalog packages |
| Custom Packages | Extra nixpkgs attribute paths |
| System Settings | App theme, hostname, DNS (IPv4), username, `libvirtd` / `docker` / `vboxusers` |
| Hardware | GPU label from `lspci`; NVIDIA widgets when the vendor string contains `nvidia`; audio, Bluetooth, TLP, power profile, thermald |
| Network | Firewall, TCP and UDP ports, SSH, Fail2Ban, Tailscale, WireGuard module + listen UDP port |
| Services | 21 toggles (some install packages, not daemons) |
| Generations | List / switch now / next boot / delete (through the helper) |
| Maintenance | Five catalog actions + disk usage |
| Apply Changes | Local Nix preview, rebuild mode (switch / boot / test / build), **Apply Changes**, **Dry Run** (`nixos-rebuild dry-build`) |

Appearance (System / Light / Dark) is a local preference (`$XDG_CONFIG_HOME/nixos-toolkit/preferences.json`, typically `~/.config/nixos-toolkit/preferences.json`). It is not written into Nix.

---

## What apply actually writes

**Desktop profiles:** the helper copies `nix/templates/profiles/<id>.nix` into `/etc/nixos/nixos-toolkit/profiles/`. If the file is missing, it writes a built-in fallback. Preview inlines the same template when it exists; a missing profile template is omitted from preview and still gets a fallback on apply.

**Software bundles:** turning a bundle **on** in this GUI always writes the **catalog package list** from `crates/common/src/actions.rs` into `bundle_packages`. Preview and apply then use the same rule:

- if `bundle_packages` **contains** the bundle id (always true after enabling it in this GUI) **or** the template is missing → generate catalog packages + `bundle_module_stub` (`crates/common/src/nix.rs`)
- template copy (`nix/templates/bundles/<id>.nix`) only when that id is **missing** from `bundle_packages` (old `state.json`, or state reconstructed from Nix)

Unchecking catalog packages updates both preview and apply. `ai-tools` has no template file; both paths emit a packages-only module (`ollama`).

When generated config needs non-free packages (known catalog attrs, NVIDIA proprietary, gaming/VirtualBox, or an uncustomized fonts template), apply also writes `/etc/nixos/nixos-toolkit/state/unfree.nix` (`nixpkgs.config.allowUnfree = true`) and imports it from `selected.nix`.

After a successful write, the helper deletes leftover managed `.nix` files that this apply did not generate: unused `profiles/*.nix`, unused `bundles/*.nix`, and unused state snippets (`hostname.nix`, `dns.nix`, `users.nix`, `custom-packages.nix`, `hardware.nix`, `network.nix`, `services.nix`, `unfree.nix`). `selected.nix` and `state.json` are kept.

---

## Features

### Desktop profiles (13)

Display managers come from `ProfileDef` and the matching template. Session type is not a field in code; the notes below are what the Nix templates enable.

| id | Name | Display manager | Template notes |
|----|------|-----------------|----------------|
| `gnome` | GNOME | GDM | `gdm.wayland = true`; Tweaks, Extension Manager, Loupe, … |
| `kde` | KDE Plasma | SDDM | Plasma 6, SDDM Wayland, KDE Connect |
| `xfce` | XFCE | LightDM | X11; Whisker menu and plugins |
| `mate` | MATE | LightDM | X11 |
| `cinnamon` | Cinnamon | LightDM | X11 |
| `pantheon` | Pantheon | LightDM | X11 `services.xserver` (file header mentions Wayland; options do not) |
| `cosmic` | COSMIC | cosmic-greeter | Needs COSMIC in nixpkgs / overlay |
| `hyprland` | Hyprland | SDDM | Wayland + XWayland; waybar, wofi, … |
| `sway` | Sway | SDDM | Wayland |
| `i3` | i3 | LightDM | X11 |
| `budgie` | Budgie | LightDM | X11 |
| `lxqt` | LXQt | SDDM | X11 (`sddm.wayland` not set) |
| `enlightenment` | Enlightenment | LightDM | X11 |

Only one profile can be selected. Templates `mkForce` their display manager on and force some others off; they do not disable every other DE/WM module.

### Software bundles (16 catalog entries)

GUI apply installs the **catalog** packages (and the helper stub, if any). ARM: bundles marked x86_64-only are disabled in the UI.

| id | Name | Catalog packages (GUI apply) | Extra module stub on apply |
|----|------|------------------------------|----------------------------|
| `devtools` | Development Tools | git, neovim, emacs, vscode, vscodium, zed-editor, rustup, nodejs, python3, docker | `programs.git`, `virtualisation.docker` |
| `ai-tools` | AI Tools | ollama | none (no template file; preview and apply both use the packages fallback) |
| `gaming` | Gaming | steam, lutris, heroic, bottles, mangohud, gamemode | `programs.steam`, 32-bit graphics, GameMode |
| `virtualization` | KVM/QEMU | virt-manager, qemu, spice-gtk | libvirtd, KVM modules, swtpm, OVMF, virt-manager |
| `virtualbox` | VirtualBox | virtualbox | `allowUnfree`, host + extension pack |
| `containers` | Container Runtime | podman, docker-compose, buildah, skopeo | Podman with `dockerCompat` (Docker daemon not enabled) |
| `flatpak` | Flatpak Support | flatpak | `services.flatpak`, `xdg.portal` |
| `multimedia` | Multimedia | vlc, mpv, gimp, inkscape, obs-studio, audacity | none |
| `office` | Office & Productivity | libreoffice, onlyoffice-desktopeditors, thunderbird, evince, obsidian | none |
| `security` | Security Tools | keepassxc, bitwarden (`bitwarden-desktop` in Nix), `_1password-gui`, veracrypt, gnupg, age | none |
| `communication` | Communication | discord, signal-desktop, element-desktop, slack, zoom-us | none |
| `browsers` | Web Browsers | firefox, chromium, google-chrome, brave, tor-browser-bundle-bin | none |
| `science` | Science & Math | octave, julia (`julia-bin` in Nix), gnuplot | none |
| `cad` | 3D & CAD | blender, freecad, openscad, kicad | none |
| `utilities` | System Utilities | htop, btop, fastfetch, tmux, tree, unzip, wget, curl, appimage-run | none |
| `fonts` | Fonts Collection | nerd-fonts.fira-code, nerd-fonts.jetbrains-mono, fira-code, jetbrains-mono, inter, noto-fonts | `fonts.fontconfig`; packages go in `fonts.packages` |

Richer lists in `nix/templates/bundles/` (Go, ripgrep, Wireshark, TeXLive, PipeWire, …) are copied on apply only when `bundle_packages` has no entry for that bundle.

Catalog ids that are not nixpkgs attrs are mapped before interpolation (`bitwarden` → `bitwarden-desktop`, `julia` → `julia-bin`). Names may start with `_` (`_1password-gui`).

Unfree software: the toolkit writes `unfree.nix` for a **known allowlist** of attrs/bundles (Steam, Chrome, Slack, Zoom, Discord, VS Code, Obsidian, OnlyOffice, 1Password, VirtualBox, NVIDIA proprietary, …). That is not `nixpkgs` `meta.unfree`; an unknown unfree attr can still fail eval. You can also set `nixpkgs.config.allowUnfree` in your own config.

### Custom packages

Accepted input:

- `zed-editor`
- `pkgs.zed-editor`
- `nixpkgs#ripgrep`
- `environment.systemPackages = [ pkgs.foo pkgs.bar ];`
- comma, newline, or space separated names

Names must start with a letter or `_`; `[A-Za-z0-9_.-]`; max 128 characters. `1password` is rejected (`_1password-gui` is the catalog id).

A name that already appears in **any** catalog bundle (enabled or not) is not added; you get a toast pointing at that bundle.

### Hardware

- GPU string from `lspci` (NVIDIA preferred on hybrid). AMD/Intel are labels only.
- NVIDIA dropdown (stable / beta / open / nouveau) plus modesetting, power management, open kernel modules — shown when the GPU string contains `nvidia` and the CPU is not ARM. Detecting NVIDIA also defaults an unwritten driver to stable in the GUI state.
- Audio: PipeWire (default), PulseAudio, or none. PipeWire is emitted whenever `hardware.nix` is written (`audio_server == 0`), including bluetooth-only applies. An apply with **no** non-default hardware fields does not write `hardware.nix` and does not change the host audio stack. Low-latency adds a 32-quantum PipeWire extraConfig block.
- Bluetooth enable + power-on-boot (also enables Blueman).
- Power profile (balanced / performance / power-saver via power-profiles-daemon when not Balanced), TLP, thermald (thermald control disabled on ARM).

### Network

- Firewall on/off (default on).
- TCP presets: 22, 80, 443, 8080, plus a custom TCP field.
- UDP presets: 53, 123, 443, 51820, plus a custom UDP field.
- SSH: enable, port, password auth, root login (`no` / `prohibit-password` / `yes`).
- Fail2Ban: independent of SSH (with SSH it also enables the `sshd` jail).
- Tailscale.
- WireGuard: enables `networking.wireguard.enable` and opens the listen UDP port on the firewall. It does **not** write peers, addresses, or private keys.

A network snippet is written when `NetworkConfig` is non-default (ports, SSH, Fail2Ban, Tailscale, WireGuard enabled, SSH option changes, or firewall disabled). Changing only the WireGuard listen port while WireGuard is off does not count.

### Services (21)

| Group | Toggles |
|-------|---------|
| Hardware | Printing (CUPS), Avahi/mDNS, fwupd, UPower |
| Network | NetworkManager, systemd-resolved |
| Remote | RustDesk (**client package**, not a server) |
| Sync | Syncthing, locate (`plocate`) |
| Desktop | Flatpak, GNOME Keyring, GNOME Tweaks (**package**), dconf |
| Development | Docker, libvirtd, PostgreSQL, Redis |
| System | Early OOM, Auto Upgrade, Auto GC, Store Optimization |

Printing also enables Avahi. The empty-apply confirmation looks at profile, bundles, custom packages, hardware, hostname, DNS, groups, network, and services.

### System

- Hostname: written with `lib.mkDefault` so an explicit `networking.hostName` in your config wins. If you have no hostname set, this value is used. Clearing the field to the current host name stores `None` and omits `hostname.nix`.
- DNS: IPv4 only, `lib.mkDefault`.
- Groups: `libvirtd`, `docker`, `vboxusers` for the username field (seeded from `$USER`).

### Generations and maintenance

Generations go through the helper (`nix-env` on `/nix/var/nix/profiles/system`, then `switch-to-configuration switch|boot`).

Maintenance catalog (5 actions). The helper allowlist is exact strings:

- `nix-collect-garbage`
- `nix-collect-garbage -d`
- `nix-store --optimise`
- `nix-store --verify --check-contents`
- `nix-channel --update`
- `nix flake update --flake /etc/nixos`

The GUI button is still named **Update Channels** and sends `nix-channel --update`. On hosts with `/etc/nixos/flake.nix`, the helper rewrites that to `nix flake update --flake /etc/nixos`.

Apply rebuild modes: **Switch**, **Boot**, **Test**, **Build** (dropdown) plus **Dry Run** (`dry-build`). Successful Switch/Boot/Test/Build also persist `state.json`. Dry-build writes the managed tree so evaluation can run, then restores the previous files (restore failure is an error).

---

## Architecture

```
┌──────────────────────────┐  JSON lines, stdin/stdout   ┌────────────────────────────┐
│ nixos-toolkit            │  typically via pkexec       │ nixos-toolkit-helper       │
│ libcosmic GUI, unprivileged│ ─────────────────────────► │ privileged                  │
└──────────────────────────┘                             └────────────────────────────┘
           │ writes ~/.config/nixos-toolkit/                       │ writes
           ▼                                                       ▼
  preferences.json (theme)                          /etc/nixos/nixos-toolkit/
                                                    state/selected.nix  ← you import this
                                                    state/*.nix, state.json
                                                    profiles/*.nix
                                                    bundles/*.nix
```

Workspace crates: `common`, `gui` (binary `nixos-toolkit`), `helper` (binary `nixos-toolkit-helper`), `fake-helper` (test double; never touches `/etc/nixos`).

Flake packages: `default`/`gui`, `helper`, `templates`, `full`. Apps: `default` (`nixos-toolkit`), `helper`. Overlay attributes: `nixos-toolkit`, `nixos-toolkit-helper`.

Details: [docs/architecture.md](docs/architecture.md), [docs/reference.md](docs/reference.md).

---

## Security

- The GUI does not write `/etc/nixos`.
- The helper writes under `/etc/nixos/nixos-toolkit/` (path-prefix jail on `atomic_write`) and runs rebuild / `nix-env` / allowlisted store commands. It reads `configuration.nix` / `flake.nix` only to detect integration.
- Polkit actions `org.nixos-toolkit.helper.{manage-system,write-config,rebuild}` all annotate `exec.path` as `/run/current-system/sw/bin/nixos-toolkit-helper` (Nix package substitute uses `$out/bin/nixos-toolkit-helper`).
- Hostname and DNS use `lib.mkDefault`.

---

## Development

```bash
nix develop
cargo run -p gui          # binary name: nixos-toolkit
cargo run -p helper       # binary name: nixos-toolkit-helper
nix build                 # packages.default → result/bin/nixos-toolkit
nix build .#helper
```

Rust: workspace `rust-version = "1.93"`, edition 2021. The flake uses rust-overlay `stable.latest`, not a 1.93 pin.

Full contributor notes: [docs/development.md](docs/development.md).

---

## Known limitations

- i18n is English Fluent only (`crates/gui/i18n/en/`). Catalog names in `actions.rs` and service row labels/descriptions in `view/services.rs` stay English.
- GitHub Actions on PR runs cargo build/clippy/test **and** flake `fmt`/`audit`. Flatpak smoke is still `workflow_dispatch` (disk). No Cachix / binary cache in this flake.
- The write jail is a path-prefix check under `/etc/nixos/nixos-toolkit`, not a kernel sandbox. The helper still writes constant managed paths (`EnsureDirectories` placeholder and `WriteState` are not the jailed `atomic_write`).
- Reconstruct-from-Nix is line-oriented, not a Nix parser. Unusual hand-edited modules may be missed. `bundle_packages` is not reconstructed (legacy template copy may run on the next apply).
- Unfree is an allowlist of known attrs/bundles, not nixpkgs `meta.unfree`. Unknown unfree attrs can still fail eval.
- WireGuard UI enables the module and opens the listen UDP port. It does not write peers, addresses, or private keys.
- Preview vs apply: bundle generation now matches apply. Profile templates are still inlined when present (apply copies the same file, or a fallback if the file is missing — preview omits a missing profile template). File order in the preview dump can differ from the helper write order.
- The GUI never sends `CheckPermissions` / `GetSystemInfo` / `Validate` / `Generate`; detection is local (and `flatpak-spawn --host` inside the sandbox).
- Integration: a comment containing the exact `selected.nix` path still counts as integrated.
- Hostname `lib.mkDefault` still sets the hostname when the user has no explicit `networking.hostName`.
- The Maintenance catalog name is still **Update Channels**; on flake hosts the helper runs `nix flake update --flake /etc/nixos` instead.

More: [docs/reference.md](docs/reference.md#known-limitations).

---

## Troubleshooting

**`path '…/selected.nix' does not exist`** — create the placeholder before adding the import.

**`syntax error, unexpected PATH`** — the import path is outside the `imports = [ … ];` list.

**Not integrated** — confirm the import uses one of the `nixos-toolkit/state/selected.nix` path forms in `configuration.nix` or `flake.nix`, rebuild, then Verify integration. A comment with that path is a false positive. A flake-only import is enough for both GUI and helper.

**Privileged helper not found** — install `nixos-toolkit-helper` on the host (NixOS module). Flatpak cannot apply without it.

**pkexec / “Unexpected response from helper”** — no polkit agent, or `SHELL` pointing at a nix-develop shell (the GUI strips `SHELL` when spawning). Run a polkit agent, or `security.polkit.enable = true` (the NixOS module sets this).

**Unfree eval errors** — the toolkit writes `unfree.nix` for known attrs. If eval still fails, enable `nixpkgs.config.allowUnfree` in your own config, or uncheck the unfree package.

**Dry-run left files under `/etc/nixos/nixos-toolkit/`** — dry-build restores a snapshot taken before writes. A restore error is reported as a helper `Error`. First-time dry-run (no managed dir yet) restores to “directory absent”.

---

## Further documentation

| Doc | Contents |
|-----|----------|
| [docs/README.md](docs/README.md) | Index |
| [docs/architecture.md](docs/architecture.md) | Crates, IPC, generation, spawn, state |
| [docs/development.md](docs/development.md) | Build, test, CI, adding profiles/bundles |
| [docs/reference.md](docs/reference.md) | Paths, env, IPC variants, catalogs, stubs |
| [docs/migration/](docs/migration/) | Historical GTK → libcosmic notes (not current) |

## Contributing

Issues and pull requests are welcome. Match the code, not the migration docs.

## License

GPL-3.0-or-later. See [`LICENSE`](LICENSE).

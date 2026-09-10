# NixOS Toolkit

A [libcosmic](https://github.com/pop-os/libcosmic) (iced) desktop app for declarative NixOS management.

The repository is `Gosh-NixOS-Manager`. The application name, desktop entry, About dialog, and binaries are **NixOS Toolkit**: `nixos-toolkit` (GUI) and `nixos-toolkit-helper` (privileged helper).

You pick a desktop profile, software bundles, packages, and system settings in the GUI. The helper writes Nix modules under `/etc/nixos/nixos-toolkit/` and runs `nixos-rebuild`. It does not edit your `configuration.nix` or `flake.nix`.

Version `0.1.0`. License: GPL-3.0-or-later (declared in `Cargo.toml`; there is no `LICENSE` file in the tree).

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

The helper creates the directory tree and placeholder on **Apply** and **Dry Run** (`EnsureDirectories` first). The import line in *your* config is always manual.

### Verifying integration

**Getting Started** shows **Integrated** or **Not integrated** (or **Unknown** if host probes were skipped).

The GUI and the helper do not use the same check:

| Who | Treated as integrated when |
|-----|----------------------------|
| GUI | `configuration.nix` contains `nixos-toolkit`. If that file is missing **and** `selected.nix` does not exist yet, `flake.nix` containing `nixos-toolkit` also counts. Once `selected.nix` exists, the GUI **does not read `flake.nix`**. A flake-only import after creating the placeholder is **Not integrated**. |
| Helper | `selected.nix` exists **and** `configuration.nix` or `flake.nix` contains `nixos-toolkit/state/selected.nix` (or the `./` / absolute variants) |

The GUI can report Integrated for an unrelated `nixos-toolkit` mention in `configuration.nix`. After setup, rebuild once and use **Verify integration**. If you only import from `flake.nix`, the helper check is the one that matches the setup recipe.

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
| Network | Firewall, TCP ports, SSH, Fail2Ban, Tailscale |
| Services | 21 toggles (some install packages, not daemons) |
| Generations | List / switch now / next boot / delete (through the helper) |
| Maintenance | Five allowlisted Nix store commands + disk usage |
| Apply Changes | Local Nix preview, **Apply Changes** (`nixos-rebuild switch`), **Dry Run** (`nixos-rebuild dry-build`) |

Appearance (System / Light / Dark) is a local preference (`$XDG_CONFIG_HOME/nixos-toolkit/preferences.json`, typically `~/.config/nixos-toolkit/preferences.json`). It is not written into Nix.

---

## What apply actually writes

This is the part the old README got wrong.

**Desktop profiles:** the helper copies `nix/templates/profiles/<id>.nix` into `/etc/nixos/nixos-toolkit/profiles/`. If the file is missing, it writes a built-in fallback.

**Software bundles:** turning a bundle **on** in this GUI always writes the **catalog package list** from `crates/common/src/actions.rs` into `bundle_packages`. On apply, the helper then **generates** `/etc/nixos/nixos-toolkit/bundles/<id>.nix` from those names plus a small module stub (Docker, Steam, libvirt, VirtualBox, Podman, Flatpak — see [docs/reference.md](docs/reference.md)).

The helper copies `nix/templates/bundles/<id>.nix` only when that id is **missing** from `bundle_packages` (old `state.json`, or state reconstructed from Nix). A toggle in this GUI never leaves it missing.

The Apply page **preview** still inlines the on-disk bundle templates when they exist, so the preview can show more packages than apply will write after a normal enable.

**`ai-tools`:** listed in the GUI; there is no `nix/templates/bundles/ai-tools.nix`. Apply generates a packages-only module (`ollama`).

Stale files under `/etc/nixos/nixos-toolkit/` are **not deleted** when you disable a bundle or change profile. `selected.nix` simply stops importing them.

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
| `ai-tools` | AI Tools | ollama | none (no template file) |
| `gaming` | Gaming | steam, lutris, heroic, bottles, mangohud, gamemode | `programs.steam`, 32-bit graphics, GameMode |
| `virtualization` | KVM/QEMU | virt-manager, qemu, spice-gtk | libvirtd, KVM modules, swtpm, OVMF, virt-manager |
| `virtualbox` | VirtualBox | virtualbox | `allowUnfree`, host + extension pack |
| `containers` | Container Runtime | podman, docker-compose, buildah, skopeo | Podman with `dockerCompat` (Docker daemon not enabled) |
| `flatpak` | Flatpak Support | flatpak | `services.flatpak`, `xdg.portal` |
| `multimedia` | Multimedia | vlc, mpv, gimp, inkscape, obs-studio, audacity | none |
| `office` | Office & Productivity | libreoffice, onlyoffice-desktopeditors, thunderbird, evince, obsidian | none |
| `security` | Security Tools | keepassxc, bitwarden (template uses `bitwarden-desktop`), `_1password-gui` (dropped on apply: names must start with a letter), veracrypt, gnupg, age | none |
| `communication` | Communication | discord, signal-desktop, element-desktop, slack, zoom-us | none |
| `browsers` | Web Browsers | firefox, chromium, google-chrome, brave, tor-browser-bundle-bin | none |
| `science` | Science & Math | octave, julia (template uses `julia-bin`), gnuplot | none |
| `cad` | 3D & CAD | blender, freecad, openscad, kicad | none |
| `utilities` | System Utilities | htop, btop, fastfetch, tmux, tree, unzip, wget, curl, appimage-run | none |
| `fonts` | Fonts Collection | nerd-fonts.fira-code, nerd-fonts.jetbrains-mono, fira-code, jetbrains-mono, inter, noto-fonts | none |

Richer lists in `nix/templates/bundles/` (Go, ripgrep, Wireshark, TeXLive, PipeWire, …) are copied on apply only when `bundle_packages` has no entry for that bundle. Enabling a bundle in this GUI always creates that entry.

Unfree software (Steam, Chrome, Slack, Zoom, VirtualBox extension pack, …) needs `nixpkgs.config.allowUnfree` in *your* config except for the VirtualBox stub, which sets it.

### Custom packages

Accepted input:

- `zed-editor`
- `pkgs.zed-editor`
- `nixpkgs#ripgrep`
- `environment.systemPackages = [ pkgs.foo pkgs.bar ];`
- comma, newline, or space separated names

Names must start with a letter; `[A-Za-z0-9_.-]`; max 128 characters. `1password` is rejected (`_1password-gui` is the catalog id).

A name that already appears in **any** catalog bundle (enabled or not) is not added; you get a toast pointing at that bundle.

### Hardware

- GPU string from `lspci` (NVIDIA preferred on hybrid). AMD/Intel are labels only.
- NVIDIA dropdown (stable / beta / open / nouveau) plus modesetting, power management, open kernel modules — shown when the GPU string contains `nvidia` and the CPU is not ARM.
- Audio: PipeWire, PulseAudio, or none. Selecting PipeWire writes a Nix block **only** when **low-latency** is on; PipeWire without that toggle emits nothing and will not undo a previous PulseAudio apply.
- Bluetooth enable + power-on-boot (also enables Blueman).
- Power profile (balanced / performance / power-saver via power-profiles-daemon when not Balanced), TLP, thermald (thermald control disabled on ARM).

### Network

- Firewall on/off (default on).
- TCP presets: 22, 80, 443, 8080, plus a custom TCP field.
- **No UDP editor** in the UI (`allowed_udp_ports` exists on the JSON type only).
- SSH: enable, port, password auth, root login (`no` / `prohibit-password` / `yes`), Fail2Ban (emitted only if SSH is enabled).
- VPN: **Tailscale only**. There is no WireGuard UI or generator.

A network snippet is written only when `NetworkConfig` is non-default (ports, SSH, Tailscale, or firewall disabled). Fail2Ban by itself does not count.

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

Printing also enables Avahi. The empty-apply confirmation looks only at profile, bundles, custom packages, and hardware. Hostname, DNS, groups, network, and services-only still use that warning.

### System

- Hostname: written with `lib.mkForce` (overrides `networking.hostName` in your config without editing that file). Clearing it to the current host name stores `None` and omits `hostname.nix`.
- DNS: IPv4 only, `lib.mkDefault`.
- Groups: `libvirtd`, `docker`, `vboxusers` for the username field (seeded from `$USER`).

### Generations and maintenance

Generations go through the helper (`nix-env` on `/nix/var/nix/profiles/system`, then `switch-to-configuration switch|boot`).

Maintenance allowlist (exact strings):

- `nix-collect-garbage`
- `nix-collect-garbage -d`
- `nix-store --optimise`
- `nix-store --verify --check-contents`
- `nix-channel --update` (channel-based; not a flake update)

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
- The helper writes under `/etc/nixos/nixos-toolkit/` and runs rebuild / `nix-env` / allowlisted store commands. It reads `configuration.nix` / `flake.nix` only to detect integration.
- Polkit action `org.nixos-toolkit.helper.manage-system` annotates `pkexec` of the helper. Source policy path: `/run/current-system/sw/bin/nixos-toolkit-helper` (Nix package substitute uses `$out/bin/nixos-toolkit-helper`).
- Hostname uses `mkForce`; DNS uses `mkDefault`.

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

- Bundle **preview** always inlines `nix/templates/bundles/*.nix` when the file exists, including after you uncheck catalog packages. **Apply** after a GUI enable writes catalog packages + stub.
- `ai-tools` has no template.
- No UDP or WireGuard UI.
- Helper does not remove leftover profile/bundle files.
- Channel update is `nix-channel --update`.
- Unfree packages other than the VirtualBox stub do not set `allowUnfree`.
- GitHub Actions on PR: `cargo build`, clippy `-D warnings`, tests. Flake `fmt` / `audit` are `nix flake check` only. Flatpak smoke is `workflow_dispatch`.
- No Cachix / binary cache in this flake.
- i18n: English Fluent only (`crates/gui/i18n/en/`).
- GUI never sends `GetSystemInfo` / `Validate` / `Generate` / `CheckPermissions`; detection is local (and `flatpak-spawn --host` inside the sandbox).

More: [docs/reference.md](docs/reference.md#known-limitations).

---

## Troubleshooting

**`path '…/selected.nix' does not exist`** — create the placeholder before adding the import.

**`syntax error, unexpected PATH`** — the import path is outside the `imports = [ … ];` list.

**Not integrated** — confirm the file exists, the import uses `nixos-toolkit/state/selected.nix`, rebuild, then Verify integration. If the import is only in `flake.nix` and `selected.nix` already exists, the GUI will stay **Not integrated**; the helper still accepts `flake.nix`. Put the same import in `configuration.nix`, or ignore the GUI label and rely on a successful rebuild.

**Privileged helper not found** — install `nixos-toolkit-helper` on the host (NixOS module). Flatpak cannot apply without it.

**pkexec / “Unexpected response from helper”** — no polkit agent, or `SHELL` pointing at a nix-develop shell (the GUI strips `SHELL` when spawning). Run a polkit agent, or `security.polkit.enable = true` (the NixOS module sets this).

**Unfree eval errors** — enable `nixpkgs.config.allowUnfree` in your own config.

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

GPL-3.0-or-later

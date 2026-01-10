# NixOS Toolkit

A GTK4/libadwaita GUI application for declarative NixOS system management.

## Quick Start

1. **Install**: `nix run github:Goshitsarch/NixOSApp`
2. **Setup**: Create the toolkit directory and placeholder file ([see One-Time Setup](#one-time-setup))
3. **Integrate**: Add the import to your `configuration.nix`
4. **Rebuild**: `sudo nixos-rebuild switch`

> **Having issues?** See [Troubleshooting](#troubleshooting) for common errors like "path does not exist" or "syntax error".

---

## Table of Contents

- [Quick Start](#quick-start)
- [Installation](#installation)
- [One-Time Setup](#one-time-setup)
- [Troubleshooting](#troubleshooting)
- [Features](#features)
- [Development](#development)
- [Architecture](#architecture)
- [Security Model](#security-model)
- [Known Issues](#known-issues)
- [Roadmap](#roadmap)

---

## Features

### Desktop Environment Profiles

Choose from 13 desktop environments and window managers:

| Profile | Display Manager | Session Type | Notes |
|---------|-----------------|--------------|-------|
| GNOME | GDM | Wayland | Modern, elegant desktop with GNOME Tweaks, Extension Manager |
| KDE Plasma | SDDM | Wayland | Feature-rich, highly customizable with KDE Connect |
| COSMIC | cosmic-greeter | Wayland | System76's Rust-based desktop (requires NixOS with COSMIC support) |
| XFCE | LightDM | X11 | Lightweight and fast with Whisker menu, plugins |
| MATE | LightDM | X11 | Traditional GNOME 2 experience |
| Cinnamon | LightDM | X11 | Modern traditional desktop from Linux Mint |
| Pantheon | LightDM | X11 | Desktop from elementary OS with elementary apps |
| Hyprland | SDDM | Wayland | Dynamic tiling compositor with smooth animations |
| Sway | SDDM | Wayland | i3-compatible tiling Wayland compositor |
| i3 | LightDM | X11 | Popular tiling window manager, highly configurable |
| Budgie | LightDM | X11 | Modern desktop focusing on simplicity and elegance |
| LXQt | SDDM | X11 | Lightweight Qt-based desktop environment |
| Enlightenment | LightDM | X11 | Unique, visually stunning desktop with compositing effects |

### Custom Packages

Add individual packages from nixpkgs without creating bundles:

- **Flexible Input**: Paste package names in multiple formats:
  - Simple names: `zed-editor`, `htop`, `neofetch`
  - With prefix: `pkgs.zed-editor`
  - Full expressions: `environment.systemPackages = [ pkgs.zed ];`
  - Multiple packages: comma-separated, space-separated, or newline-separated
- **Duplicate Detection**: Warns if a package is already in an enabled bundle
- **Persistent**: Custom packages are saved and restored between sessions
- **Preview**: See exactly what will be installed before applying

### Software Bundles

Enable curated software collections (15 bundles):

| Bundle | Category | Key Components |
|--------|----------|----------------|
| Development Tools | Development | Git, Neovim, VS Code, Rust, Node.js, Python, Go, Docker, ripgrep, fd |
| Gaming | Gaming | Steam, Lutris, MangoHud, Gamemode, Heroic launcher, Wine, Proton tools |
| KVM/QEMU Virtualization | Virtualization | virt-manager, QEMU, libvirt, OVMF (UEFI), SPICE, TPM emulation |
| VirtualBox | Virtualization | Oracle VirtualBox, Extension Pack (non-free), sets `allowUnfree` |
| Container Runtime | Virtualization | Podman, docker-compose, buildah, skopeo (Docker compatibility) |
| Flatpak Support | System | Flatpak, gnome-software, XDG portals |
| Multimedia | Multimedia | VLC, mpv, GIMP, Inkscape, OBS Studio, Krita, PipeWire, GStreamer |
| Office & Productivity | Office | LibreOffice, Thunderbird, Obsidian, OnlyOffice, printing/scanning |
| Security Tools | Security | KeePassXC, Bitwarden, VeraCrypt, GnuPG, age, Wireshark, nmap |
| Communication | Network | Discord, Signal, Element, Slack, Zoom |
| Web Browsers | Network | Firefox, Chromium, Brave, Tor Browser |
| Science & Math | Development | Octave, Julia, R, TeXLive, gnuplot |
| 3D & CAD | Multimedia | Blender, FreeCAD, OpenSCAD, KiCad |
| System Utilities | System | htop, btop, neofetch, tmux, tree, wget, curl |
| Fonts Collection | System | Nerd Fonts, Fira Code, JetBrains Mono, Inter, Noto |

### Hardware Configuration

Configure hardware settings through the GUI:

- **GPU Detection**: Auto-detects NVIDIA, AMD, or Intel graphics
- **NVIDIA Configuration**: Driver selection (stable/beta/open/nouveau), modesetting, power management, open kernel modules
- **Audio**: PipeWire or PulseAudio selection, low-latency audio option
- **Bluetooth**: Enable/disable, power-on-boot settings
- **Power Management**: TLP for laptops, Thermald for Intel CPUs

### Network & Security

Configure network and security settings:

- **Firewall**: Enable/disable, preset ports (SSH, HTTP, HTTPS), custom TCP/UDP ports
- **SSH Server**: Enable, port configuration, password auth, root login policy, Fail2Ban
- **VPN**: Tailscale integration, WireGuard (manual)

### System Services

Toggle 19 common NixOS services:

| Category | Services |
|----------|----------|
| Hardware | Printing (CUPS), Avahi/mDNS, Firmware Updates (fwupd), UPower |
| Network | NetworkManager, systemd-resolved |
| Sync & Backup | Syncthing, Locate Database (plocate) |
| Desktop | Flatpak, GNOME Keyring, dconf |
| Development | Docker, libvirtd, PostgreSQL, Redis |
| System | Early OOM, Auto Upgrade, Auto GC, Store Optimization |

### System Configuration

- **Hostname**: Change the system hostname
- **Custom DNS**: Set custom DNS resolvers (e.g., 1.1.1.1, 8.8.8.8)
- **User Groups**: Add users to libvirtd, docker, or vboxusers groups

### Generation Management

- **List Generations**: View all NixOS system generations with dates
- **Rollback**: Switch to previous or specific generations
- **Delete Generations**: Remove old generations to free space
- **Boot Options**: Set generation for next boot or switch immediately

### Maintenance Tools

- Garbage collection (unreachable objects)
- Delete old generations (keeps current only)
- Optimize Nix store (deduplication)
- Verify store integrity
- Update channels

### Core Design

- **Safe Integration**: All changes are managed in `/etc/nixos/nixos-toolkit/`, never overwriting your existing config
- **Live Preview**: See exactly what Nix code will be generated before applying
- **Declarative**: All changes integrate with NixOS's generation system for easy rollback
- **Privilege Separation**: GUI runs unprivileged; privileged operations use pkexec

## Installation

### Easy Install (Recommended)

Run the installer script to set up aliases and desktop entry:

```bash
curl -sSL https://raw.githubusercontent.com/Goshitsarch/Gosh-NixOS-Manager/main/install.sh | bash
```

This will:
- Check if flakes are enabled
- Create a shell alias `nixos-toolkit`
- Optionally create a desktop entry for your application menu

### Quick Run

If you have flakes enabled:

```bash
nix run github:Goshitsarch/NixOSApp
```

If flakes are NOT enabled (longer command):

```bash
nix --extra-experimental-features 'nix-command flakes' run github:Goshitsarch/Gosh-NixOS-Manager --no-write-lock-file
```

### Enable Flakes (Recommended)

Add this to your `/etc/nixos/configuration.nix` to enable flakes permanently:

```nix
nix.settings.experimental-features = [ "nix-command" "flakes" ];
```

Then run `sudo nixos-rebuild switch` and you can use the shorter commands.

### Install to Profile

```bash
nix profile install github:Goshitsarch/NixOSApp
```

### Add to your NixOS configuration

```nix
# In your flake.nix inputs:
inputs.nixos-toolkit.url = "github:Goshitsarch/NixOSApp";

# In your nixosConfigurations:
modules = [
  nixos-toolkit.nixosModules.default
  # ...
];

# In your configuration.nix or system config:
programs.nixos-toolkit.enable = true;
```

## One-Time Setup

The toolkit manages configuration in `/etc/nixos/nixos-toolkit/`. To integrate with your system, you need to:

1. Create the toolkit directory structure
2. Create a placeholder configuration file
3. Add an import to your NixOS configuration

### Why These Steps Are Needed

NixOS evaluates all imports when you run `nixos-rebuild`. If you add an import to a file that doesn't exist yet, the build will fail with:

```
error: path '/etc/nixos/nixos-toolkit/state/selected.nix' does not exist
```

The placeholder file we create is a valid (but empty) NixOS module. This allows `nixos-rebuild` to succeed immediately after setup. When you later use the toolkit to select profiles and bundles, the app will overwrite this placeholder with your actual configuration.

### Classic Configuration (configuration.nix)

**Step 1: Create the directory structure**

```bash
sudo mkdir -p /etc/nixos/nixos-toolkit/state
```

**Step 2: Create a placeholder file**

```bash
sudo tee /etc/nixos/nixos-toolkit/state/selected.nix > /dev/null << 'EOF'
# NixOS Toolkit - Placeholder
# This file will be replaced when you apply changes in the toolkit
{ config, lib, pkgs, ... }: { imports = []; }
EOF
```

**Step 3: Add the import to your configuration**

Edit `/etc/nixos/configuration.nix` and add the import **inside** the `imports` list:

```nix
{ config, pkgs, ... }:

{
  imports = [
    ./hardware-configuration.nix
    ./nixos-toolkit/state/selected.nix  # <-- Add this line
  ];

  # ... rest of your configuration
}
```

> **Important**: The import path must be inside the `imports = [ ... ];` brackets, not as a standalone line.

**Step 4: Apply the changes**

```bash
sudo nixos-rebuild switch
```

### Flake-Based Configuration

**Step 1: Create the directory structure**

```bash
sudo mkdir -p /etc/nixos/nixos-toolkit/state
```

**Step 2: Create a placeholder file**

```bash
sudo tee /etc/nixos/nixos-toolkit/state/selected.nix > /dev/null << 'EOF'
# NixOS Toolkit - Placeholder
# This file will be replaced when you apply changes in the toolkit
{ config, lib, pkgs, ... }: { imports = []; }
EOF
```

**Step 3: Add the import to your flake**

Edit your `flake.nix` and add the import to your `modules` list:

```nix
nixosConfigurations.your-hostname = nixpkgs.lib.nixosSystem {
  modules = [
    ./configuration.nix
    ./nixos-toolkit/state/selected.nix  # <-- Add this line
  ];
};
```

**Step 4: Apply the changes**

```bash
sudo nixos-rebuild switch --flake .#
```

### Quick Setup Script

For convenience, you can run all setup steps with this one-liner:

```bash
sudo mkdir -p /etc/nixos/nixos-toolkit/state && \
sudo tee /etc/nixos/nixos-toolkit/state/selected.nix > /dev/null << 'EOF'
{ config, lib, pkgs, ... }: { imports = []; }
EOF
```

Then manually add the import line to your configuration and rebuild.

### Verifying Integration

After setup, launch the toolkit. The "Getting Started" page will show:
- **"Integrated"** if the toolkit detected the import in your configuration
- **"Not integrated"** if the import is missing or the file doesn't exist

The toolkit will also show a status banner at the top of the window indicating integration status.

## Troubleshooting

### "path does not exist" error during nixos-rebuild

```
error: path '/etc/nixos/nixos-toolkit/state/selected.nix' does not exist
```

**Cause**: You added the import to your configuration before creating the placeholder file.

**Solution**: Run the setup commands to create the directory and placeholder:

```bash
sudo mkdir -p /etc/nixos/nixos-toolkit/state
sudo tee /etc/nixos/nixos-toolkit/state/selected.nix > /dev/null << 'EOF'
{ config, lib, pkgs, ... }: { imports = []; }
EOF
sudo nixos-rebuild switch
```

### "syntax error, unexpected PATH, expecting INHERIT"

```
error: syntax error, unexpected PATH, expecting INHERIT
at /etc/nixos/configuration.nix:15:1
```

**Cause**: The import path was placed outside the `imports` list, as a standalone line.

**Wrong** (import outside the list):
```nix
  imports = [
    ./hardware-configuration.nix
  ];

./nixos-toolkit/state/selected.nix   # Wrong! This is outside the list
```

**Correct** (import inside the list):
```nix
  imports = [
    ./hardware-configuration.nix
    ./nixos-toolkit/state/selected.nix  # Correct! Inside the brackets
  ];
```

### Toolkit shows "Not integrated" after setup

**Possible causes**:
1. The import line wasn't added to your configuration
2. The `selected.nix` file doesn't exist
3. You haven't run `nixos-rebuild switch` after adding the import

**Solution**:
1. Verify the file exists: `ls -la /etc/nixos/nixos-toolkit/state/selected.nix`
2. Verify the import is in your config: `grep -r "nixos-toolkit" /etc/nixos/`
3. Run `sudo nixos-rebuild switch` and restart the toolkit

### Permission denied when creating files

**Cause**: The `/etc/nixos/` directory requires root privileges.

**Solution**: Use `sudo` for all setup commands, or run the toolkit's helper which uses `pkexec` for privilege escalation.

---

## Development

### Prerequisites

- NixOS with flakes enabled
- Nix 2.4+

### Enter development shell

```bash
nix develop
```

### Build and run

```bash
# Run the GUI
cargo run -p gui

# Run the helper (requires root for actual operations)
cargo run -p helper

# Build all packages
nix build

# Build specific package
nix build .#gui
nix build .#helper
```

### Project Structure

```
.
├── Cargo.toml              # Workspace configuration (Rust 1.75+, GPL-3.0)
├── flake.nix               # Nix flake with build definitions
├── crates/
│   ├── common/             # Shared types and utilities
│   │   └── src/
│   │       ├── actions.rs  # Action registry (13 profiles, 15 bundles, 5 system actions, 5 maintenance actions)
│   │       ├── ipc.rs      # IPC message types (JSON over stdin/stdout)
│   │       ├── config.rs   # System configuration types and paths
│   │       └── nix.rs      # Nix code generation (selected, hostname, DNS, users, firewall, SSH, hardware, services)
│   ├── gui/                # GTK4/libadwaita GUI application
│   │   └── src/
│   │       ├── app.rs      # Application setup
│   │       ├── window.rs   # Main window with sidebar navigation
│   │       ├── state.rs    # Application state management
│   │       └── pages/      # UI pages:
│   │           ├── onboarding.rs   # First-run setup
│   │           ├── profiles.rs     # Desktop environment selection
│   │           ├── bundles.rs      # Software bundle toggles
│   │           ├── packages.rs     # Custom package management
│   │           ├── system.rs       # Hostname, DNS, user groups
│   │           ├── hardware.rs     # GPU, audio, bluetooth, power
│   │           ├── network.rs      # Firewall, SSH, VPN
│   │           ├── services.rs     # System service toggles
│   │           ├── generations.rs  # Generation management
│   │           ├── maintenance.rs  # Maintenance actions
│   │           └── apply.rs        # Preview and apply changes
│   └── helper/             # Privileged helper binary
│       └── src/
│           ├── main.rs     # JSON IPC server over stdin/stdout
│           ├── commands.rs # Request handlers (12 commands)
│           ├── nix_gen.rs  # Nix file generation with fallback templates
│           └── rebuild.rs  # nixos-rebuild execution
├── nix/
│   └── templates/          # Nix module templates
│       ├── profiles/       # 13 desktop environment profiles
│       └── bundles/        # 15 software bundles
└── data/
    ├── nixos-toolkit.desktop  # Desktop entry
    ├── icons/                 # Application icon (SVG)
    └── polkit/                # Polkit policy for privilege escalation
```

## Architecture

```
┌─────────────────────┐     JSON/stdin-stdout   ┌─────────────────────┐
│   nixos-toolkit     │ ───────────────────────►│ nixos-toolkit-helper│
│   (GUI, unprivileged)│       via pkexec        │   (privileged)      │
└─────────────────────┘                         └─────────────────────┘
         │                                                │
         │ reads                                          │ writes
         ▼                                                ▼
┌─────────────────────┐                         ┌─────────────────────┐
│ /etc/nixos/         │                         │/etc/nixos/nixos-    │
│ configuration.nix   │ ◄── imports ─────────── │toolkit/             │
│ (user-managed)      │                         │  state/selected.nix │
└─────────────────────┘                         │  state/custom-      │
                                                │       packages.nix  │
                                                │  profiles/*.nix     │
                                                │  bundles/*.nix      │
                                                └─────────────────────┘
```

### IPC Protocol

The helper binary accepts JSON requests over stdin and responds on stdout:

| Request | Description |
|---------|-------------|
| `CheckPermissions` | Verify read/write/rebuild capabilities |
| `GetSystemInfo` | Detect NixOS version, config mode, integration status |
| `Validate` | Validate configuration before applying |
| `Generate` | Generate Nix files (supports dry-run) |
| `Apply` | Generate files and run nixos-rebuild |
| `EnsureDirectories` | Create managed directory structure |
| `ReadState` / `WriteState` | Persist/restore application state |
| `ListGenerations` | List all NixOS system generations |
| `RollbackGeneration` | Switch to a specific generation |
| `DeleteGenerations` | Remove specific generations |
| `RunMaintenance` | Execute allowed maintenance commands |

## Adding New Profiles/Bundles

### Add a new profile

1. Create a template file in `nix/templates/profiles/your-profile.nix`
2. Add the profile definition in `crates/common/src/actions.rs` to `default_profiles()`:

```rust
ProfileDef {
    id: "your-profile".into(),
    name: "Your Profile".into(),
    description: "Description of the profile".into(),
    icon: "desktop-symbolic".into(),
    template: "profiles/your-profile.nix".into(),
    display_manager: "gdm".into(),
}
```

### Add a new bundle

1. Create a template file in `nix/templates/bundles/your-bundle.nix`
2. Add the bundle definition in `crates/common/src/actions.rs` to `default_bundles()`:

```rust
BundleDef {
    id: "your-bundle".into(),
    name: "Your Bundle".into(),
    description: "Description of the bundle".into(),
    icon: "icon-name-symbolic".into(),
    category: ActionCategory::YourCategory,
    template: "bundles/your-bundle.nix".into(),
    packages: vec!["pkg1".into(), "pkg2".into()],
}
```

## Security Model

- The GUI runs as an unprivileged user
- Privileged operations (file writes, nixos-rebuild) are handled by a separate helper binary
- The helper is invoked via `pkexec` for authentication (polkit)
- The toolkit NEVER modifies your existing configuration.nix or flake.nix
- All managed files are in `/etc/nixos/nixos-toolkit/` and marked as auto-generated
- Changes integrate with NixOS generations for rollback capability
- Maintenance commands are allowlisted (only specific nix commands permitted)

## Known Issues

1. **Channel updates on flakes**: The "Update Channels" maintenance action uses `nix-channel --update` which may not be relevant for flake-based configurations.

2. **Validation hardcoding**: The helper's validation function has hardcoded profile/bundle lists that may lag behind newly added definitions. New profiles/bundles will work but may show validation warnings.

## Roadmap / Next Steps

- **Binary Cache (Cachix)**: Currently the app builds from source on first run. Setting up a Cachix binary cache would provide pre-built binaries for instant installation. This involves:
  1. Creating a cache at [cachix.org](https://cachix.org)
  2. Adding cache configuration to `flake.nix`
  3. Setting up GitHub Actions to automatically push builds to the cache

- **More Desktop Profiles**: Additional window managers and desktop environments

- **Backup/Restore**: Export and import toolkit configurations

- **Theme Support**: Light/dark mode preferences for generated configs

## License

GPL-3.0-or-later

## Contributing

Contributions are welcome! Please feel free to submit issues and pull requests.

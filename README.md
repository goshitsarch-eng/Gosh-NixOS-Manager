# NixOS Toolkit

A desktop app for managing your NixOS configuration. Choose a desktop, install software, adjust system settings, and review the Nix configuration before applying it.

## Install

On **x86_64 NixOS**, open a terminal as your normal user and run:

```sh
curl -fsSL https://github.com/goshitsarch-eng/Gosh-NixOS-Manager/releases/latest/download/install.sh | bash
```

Enter your sudo password when asked. The installer downloads signed, prebuilt packages, adds **NixOS Toolkit** to your application menu, and opens it. The app and helper are already compiled; NixOS still needs to build and activate the system integration, including any pending changes in your configuration.

Using flakes? Need to update, recover, or uninstall? See the [installation guide](INSTALLING.md). The current prebuilt release supports x86_64; ARM64 still needs validation.

## What you can do

- Choose from 13 desktop profiles. Click a selected desktop again to deselect it.
- Pick individual apps from 16 software bundles, or add your own Nix packages.
- Configure hardware, audio, networking, firewall rules, and 21 service settings.
- Review additions, removals, settings changes, and generated Nix before applying.
- Build first, activate now, or prepare a configuration for the next boot.
- Manage system generations, check store usage, and run maintenance.

Recent additions include **Bazaar** in the Flatpak bundle; **Claude Code, Codex, Grok CLI, Cursor desktop/CLI, Mistral Vibe, OpenCode, and Pi** alongside Ollama; and **Distrobox, DistroShelf, language SDKs, GTK4, and Qt6** in Development Tools. See the [software catalog](docs/SOFTWARE_BUNDLES.md) for package names and setup notes.

![NixOS Toolkit in dark mode](docs/screenshots/linux-dark.png)

## Before your first change

Start with **Getting Started**, then make a small selection and open **Apply Changes**. Review it and run **Dry Run** before activating. The review compares your draft with saved Toolkit selections; it is not an inventory of every app on your machine.

Toolkit can coexist with manual Nix modules, Home Manager, and Flatpak. Keep your own settings outside `/etc/nixos/nixos-toolkit/`, and use one place to manage each setting. The [usage guide](docs/USAGE.md) explains file ownership, rebuild modes, and recovery.

The app detects configuration changes made after it loaded and blocks an outdated draft. **Reload saved selections** loads a fresh copy. Interrupted applies have a **Restore previous system** action. The installer also includes the NixOS 25.05 fix for the observed firewall/D-Bus activation timeout.

## Project status

[v0.1.0](https://github.com/goshitsarch-eng/Gosh-NixOS-Manager/releases/tag/v0.1.0) is available. The installer, native app, and authenticated Flatpak host operations have been tested on a NixOS 25.05 VM. Physical hardware, every desktop login, ARM64, and screen readers still need testing. Hosted CI was blocked by an account billing issue for this release; its validation was local. See [platform coverage](PLATFORM_SUPPORT.md) and the [live test report](docs/audit/LIVE_VM_QA.md).

The desktop uses Flutter with a shared Rust core and a separate privileged helper. For development, start with [BUILDING.md](BUILDING.md). The [documentation index](docs/README.md) links the technical guides and test history.

[Report an issue](https://github.com/goshitsarch-eng/Gosh-NixOS-Manager/issues) · [GPL-3.0-or-later](LICENSE)

# Install NixOS Toolkit

## Quick install

Run this from a terminal in your **x86_64 NixOS** desktop session, as your normal user:

```sh
curl -fsSL https://github.com/goshitsarch-eng/Gosh-NixOS-Manager/releases/latest/download/install.sh | bash
```

The installer asks for sudo, downloads the prebuilt app and matching helper, adds **NixOS Toolkit** to your application menu, and opens it. The app runs as your normal user and uses polkit authentication for host operations.

Only missing dependencies are downloaded. The v0.1.0 release has about 184 MiB of compressed runtime packages before reuse from your Nix store. Download and activation time depend on your connection and host configuration; the app itself is not compiled locally.

NixOS still builds and activates a system generation. **Any pending changes in your existing configuration are included**, so start from a configuration you are ready to activate. The installer does not update channels or flake locks. Package signatures are checked using a key trusted only for this download.

To read the script before running it:

```sh
curl -fsSL https://github.com/goshitsarch-eng/Gosh-NixOS-Manager/releases/latest/download/install.sh -o install-toolkit.sh
less install-toolkit.sh
bash install-toolkit.sh
```

The commands below use this saved script. Add `--no-launch` if you do not want the app to open.

## Flakes and supported layouts

Automatic installation expects a regular `/etc/nixos/configuration.nix` with a conventional module body and a literal `imports = [ ... ];` list, or no imports yet. It adds the Toolkit imports and keeps your other settings.

For a flake host that imports this file, specify its output name:

```sh
bash install-toolkit.sh --flake /etc/nixos#YOUR_HOST
```

Replace `YOUR_HOST` with your `nixosConfigurations` name. New files in a Git flake are marked intent-to-add so Nix can see them. Review and commit the changes yourself.

For later changes in the app, the name in your flake must match your computer's hostname. For example, if your flake calls this computer `my-desktop` but its hostname is `nixos`, installation can succeed while the app's Apply button fails because it looks for `nixos`. In that setup, rebuild from the terminal with your usual `--flake /etc/nixos#my-desktop` argument. The installer does not save that name as an app setting.

Symlinked configuration files, computed imports, remote flakes, and other configuration locations need [manual integration](#manual-integration). The app's host operations expect `/etc/nixos`; installing its module elsewhere does not change that. Remove previous Toolkit package/module declarations before changing installation methods to avoid duplicate definitions.

## Update

Run the quick-install command again to get the latest release. If using a saved script, request the latest version explicitly:

```sh
bash install-toolkit.sh --version latest
```

A saved release script otherwise keeps its original version. Use `--version v0.1.0` to select that release, and include `--flake /etc/nixos#YOUR_HOST` again on a flake host. Updating preserves your Toolkit selections.

## Files, recovery, and uninstall

| Location | What belongs there |
| --- | --- |
| `/etc/nixos/configuration.nix` and your own modules | Your manual settings; the installer adds marked imports here. |
| `/etc/nixos/nixos-toolkit-install.nix` | Installer-owned app/helper integration. Do not hand-edit it. |
| `/etc/nixos/nixos-toolkit/` | Generated Toolkit selections. Keep manual settings elsewhere. |
| `/var/lib/nixos-toolkit-installer/backups/` | Private installer backups, accessible to root. |

A failed build restores the files edited by the installer. Failed activation also attempts to restore the previous running and next-boot systems. After an interrupted installation, run:

```sh
bash install-toolkit.sh --recover
```

Recovery preserves files changed by another editor and leaves its backup available for manual recovery. For an interrupted change **inside the app**, use **Restore previous system** there instead. You can also boot an older NixOS generation. These actions do not restore application or service data.

To remove the app and helper:

```sh
bash install-toolkit.sh --uninstall
```

Your generated desktop, software, and service selections remain imported and active. Uninstall removes the installer module and its marked import, then activates the resulting system.

## Manual integration

For an existing flake, add this input alongside your current inputs:

```nix
inputs.toolkit.url = "github:goshitsarch-eng/Gosh-NixOS-Manager/v0.1.0";
```

Include `toolkit` in your `outputs` arguments, then add these entries to the host's existing `modules` list:

```nix
toolkit.nixosModules.default
{ programs.nixos-toolkit.enable = true; }
```

Rebuild with your usual command and open `nixos-toolkit`. **Getting Started** provides the steps to create a missing `selected.nix` placeholder and import it into your configuration. Preserve any existing selections. In a Git flake, track the initial integration files after reviewing them.

The module installs the app, helper, and polkit policy; the selections import connects app changes to your system. Keep app and helper versions matched. Manual flake builds may compile packages locally; use the prebuilt installer for the download-only app path.

Native archive and Flatpak build instructions are in [BUILDING.md](BUILDING.md). Flatpak also requires a matching helper on the NixOS host. For daily use and advice about mixing installation methods, see the [usage guide](docs/USAGE.md).

# Using NixOS Toolkit

## Make your first change

1. Open **Getting Started** and check that the helper is available and integration is detected.
2. Choose a desktop, software, or setting. Click a selected desktop again, or use **Clear desktop selection**, to undo that choice.
3. Open **Apply Changes**. Review the requested additions, removals, settings changes, and generated Nix.
4. Run **Dry Run** to check evaluation, then choose a rebuild mode when ready.

The review compares your draft with the last saved Toolkit selections. It does not list everything installed through manual Nix files, Home Manager, user profiles, or Flatpak. Clearing a desktop selection removes Toolkit's choice; desktops declared elsewhere remain governed by those files.

## Choose a rebuild mode

| Mode | What happens |
| --- | --- |
| **Dry Run** | Evaluates what would build. Restores temporarily generated files and does not save selections or activate anything. |
| **Build** | Builds the system and saves your proposed Toolkit settings, without activating it. |
| **Boot** | Builds and saves the configuration for the next boot. |
| **Test** | Activates now and saves selections, without changing the next-boot system. There is no automatic rollback timer. |
| **Switch** | Activates now and makes the configuration the next-boot system. |

A local Nix preview needs no administrator access. Dry Run and other host operations use the authenticated helper. A successful Dry Run checks evaluation; it does not prove a driver, service, or desktop will work after activation.

## Use Toolkit alongside other tools

Manual Nix modules, Home Manager, and Flatpak can coexist with Toolkit. The main source of surprises is managing the same setting in more than one place.

- Keep manual settings in `configuration.nix` or your own imported files, such as `/etc/nixos/my-packages.nix`.
- Leave `/etc/nixos/nixos-toolkit/` to the app. It regenerates these files. Leave `nixos-toolkit-install.nix` to the installer too.
- Prefer one installation method per app. Duplicate installations can show duplicate launchers or put a different version on your `PATH`.
- Choose one owner for desktop, audio, firewall, and service settings. Nix may merge, override, or reject overlapping definitions; some desktop profiles deliberately override display-manager choices.
- Finish manual edits and rebuilds before applying a Toolkit draft. Keep backups or Git history for your own configuration, and keep a known-working boot generation.

Removing a Toolkit package selection does not remove a copy installed elsewhere. Hostname and DNS use Nix defaults, so explicit values in your own configuration take precedence. A rebuild also includes pending manual changes in the host configuration.

If the app reports that configuration changed after loading, resolve your manual edits, then use **Reload saved selections**. This replaces your draft after confirmation. The guard covers files under `/etc/nixos` and system selections; it cannot detect every external input or user-level install. Technical details are in [ARCHITECTURE.md](../ARCHITECTURE.md#configuration-change-detection).

## Software notes

The [software catalog](SOFTWARE_BUNDLES.md) lists the recent Bazaar, AI, and development additions. You can uncheck individual packages in a bundle. AI tools may need provider sign-in or API keys; installation does not supply an account or subscription. Bazaar/Flatpak remote setup is your choice.

Language SDKs and GTK/Qt packages do not configure every project's dependencies. Use project shells for compiler libraries, native gems, or extra Qt modules. Distrobox/DistroShelf container images are created when you use those tools.

## Recovery and maintenance

If an apply is interrupted, use **Restore previous system** before making another change. It restores Toolkit files and the previous running/next-boot configuration. A system rollback does not restore databases, home files, or other service data. Installer recovery and uninstall are covered in [INSTALLING.md](../INSTALLING.md#files-recovery-and-uninstall).

**Generations** lets you activate an earlier system or select one for the next boot. Deleting generations or collecting old generations reduces your rollback options. **Maintenance** also offers store optimization, integrity checks, and channel updates; on flake hosts the update action updates `/etc/nixos/flake.lock` instead.

## If something fails

| Symptom | What to check |
| --- | --- |
| Helper missing or incompatible | Update the app and helper together using the installer or your NixOS module. |
| Authentication does not appear | Run the app as your desktop user and check that the session has a polkit authentication agent. |
| Not integrated | Follow Getting Started and check the `selected.nix` import in `configuration.nix` or `flake.nix`. |
| A setting has no effect, or Nix reports conflicting definitions | Check your manual modules and other configuration managers for the same option. |
| Nix blocks an insecure package | Deselect the named package or use a host channel with a supported version. Security checks stay enabled. |
| Firewall activation times out on NixOS 25.05 | Update the installer/module so the D-Bus reload compatibility fix is installed. Other failures still appear in the apply log. |
| The installer finishes but no window opens | Launch **NixOS Toolkit** from the menu, or run `nixos-toolkit` in your desktop terminal. Check `~/.local/state/nixos-toolkit/launcher.log` if needed. |

For an issue report, include the app version, NixOS version, relevant error log, and steps to reproduce. Remove credentials and private configuration details before posting.

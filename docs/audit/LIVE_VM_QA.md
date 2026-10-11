# Live NixOS VM QA — 2026-10-10

This follow-up runs the actual Flutter app and privileged helper on the user's disposable x86_64 NixOS 25.05 GNOME VM. It supplements the [initial release audit](NIXOS_RELEASE_QA.md), which used isolated command fixtures. The installed channel is `ac62194c3917`; toolkit packages use the repository's pinned nixpkgs.

## Recovery and method

Before changing the VM, the original `/etc/nixos` and system/profile information were copied to a root-only recovery directory. A Nix GC root retains the original system closure. The app and helper were installed as system packages, with polkit enabled and the managed import connected to the real host configuration.

The integration target drives real widgets, dialogs, clipboard, launchers, Rust FFI, pkexec and host commands. It requires `NIXOS_TOOLKIT_LIVE_VM_QA=1` and a VM backup marker. It is deliberately excluded from ordinary `flutter test`. A temporary polkit rule limited to this local VM account and the exact QA helper paths permits repeated automated operations. Final cleanup removes that rule.

## Results

| Check | Result |
|---|---|
| Native UI | 86 live checkpoints passed: all 11 pages, all 13 profile previews, all 16 bundles and their package checkboxes, custom package parsing/removal, available hardware/network/service controls, invalid drafts, port presets, three themes, clipboard, configuration-folder/browser launch, About and minimum-size navigation |
| Rebuild modes | Real DryBuild, Build, Test, Boot and Switch passed through the UI; `hello` appeared in the activated system; state persistence succeeded |
| Generations | Real next-boot rollback, immediate activation and inactive-generation deletion passed; both selected and running generations rejected deletion while they differed |
| Failed apply | A nonexistent package caused real Nix evaluation failure; managed Nix files and saved state stayed unchanged; the next helper request succeeded |
| Cancellation | Actual Interrupt dialog stopped a rebuild, restored managed files and retained saved state; separate authenticated store-verification cancellation stopped promptly |
| Maintenance | Real GC, old-system-generation cleanup, optimization, full content verification and channel update all passed; store usage refreshed; user profile generations 1–3 survived cleanup |
| Automated regression gate | 253 Rust tests, 28 Flutter tests, 18 isolated helper tests and the installer suite passed; rustfmt, Clippy, Dart formatter and analyzer passed |
| Services | All 21 options built together. Nineteen were activated (automatic upgrade/GC timers excluded); NetworkManager, resolved, CUPS, Avahi, Syncthing, Docker, libvirt, PostgreSQL, Redis and earlyoom were confirmed active. Saved configuration restored; no failed systemd units afterward |
| Desktop lifecycle | Fresh startup restored saved state; Follow System responded to real GNOME light/dark changes; keyboard/menu refresh and unsaved-close protection passed |
| Release packages | Final Nix GUI/helper builds passed; packaged diagnostics passed on native Wayland, including helper discovery and Virtio GPU detection |

Additional network activation checks and final VM cleanup are pending at this checkpoint.

The update action created a new root channel generation with the same nixpkgs revision. The original channel selection was restored afterward. The original system closure remains protected from GC. Boot-mode checks exercised the bootloader configuration; a power-cycle test has not been performed.

## Defects found by live testing

- The frontend could not signal an authenticated root helper. A real store verification continued for roughly 51 seconds after the old interruption request. Cooperative cancellation now crosses the authenticated pipe and terminates the command's process group. The fixed live verification stopped about 25 milliseconds after the request. Closing an interactive session also cancels work, including queued requests; legacy batch input remains compatible.
- Calling pkexec through `/run/current-system/sw/bin/nixos-toolkit-helper` selected the generic policy instead of the installed toolkit action. Discovery now resolves the real executable path on both native and Flatpak hosts.
- A successful retry left the previous operation's error banner visible. Starting a new operation now clears the obsolete error; logs retain the failure history.
- Installing the helper made ordinary widget tests contact the actual host. Those tests now substitute host capability probes while retaining the real Rust domain API. Only the opt-in live suite performs host operations.
- Helper tracing used the wrong target filter. Corrected logging is readable without terminal escape sequences, and maintenance/generation activation progress streams while commands run.

## Reproduction

Use only a backed-up disposable NixOS VM with the toolkit helper/policy installed and the managed import enabled. Do not fabricate the backup marker on a machine without a recovery plan. Authentication must be available; an unattended runner needs an appropriately scoped test policy.

```sh
nix develop
cd desktop
NIXOS_TOOLKIT_LIVE_VM_QA=1 GDK_BACKEND=x11 \
  flutter test integration_test/live_vm_test.dart -d linux --dart-define=QA_PHASE=ui
```

Other phases are `apply`, `generations`, `failure`, `cancel`, `maintenance`, `services`, `lifecycle`, and `cleanup`. Run apply before generation checks; the generation phase needs a previous QA generation. Maintenance really collects store paths, removes old system generations, optimizes/verifies the store and updates inputs. The services phase builds all options, activates services, and restores the saved configuration. It does not activate the automatic upgrade/GC timers alongside its own mutations. Cleanup applies an empty selection. Evidence defaults to `/tmp/toolkit-live-evidence/`.

## Scope limits

One Virtio VM cannot validate physical NVIDIA GPUs, Bluetooth devices, audio hardware, laptop power management, remote VPN enrollment, or ARM64. Profile/module evaluation and UI checks do not establish successful login to all thirteen desktops or operation of every bundled third-party application. The first audit's NixOS 25.05 Heroic/Electron security restriction remains enforced. Flatpak host discovery changes have protocol coverage but require a fresh authenticated Flatpak run. No universal “every feature works on every machine” claim follows from this report.

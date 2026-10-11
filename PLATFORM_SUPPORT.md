# Platform coverage

The prebuilt release supports **x86_64 NixOS**. Native Linux and Flatpak builds use the same Flutter frontend; changing the host system requires NixOS and the matching helper.

These results come from the [live NixOS VM report](docs/audit/LIVE_VM_QA.md), [initial release audit](docs/audit/NIXOS_RELEASE_QA.md), and earlier desktop checks. A passed configuration evaluation is not proof that every installed service or physical device works.

| Area | Tested | Still to verify |
| --- | --- | --- |
| Prebuilt installer | Public v0.1.0 URL on x86_64 NixOS 25.05; signatures, menu entry, normal-user launch, repeat install, uninstall/reinstall | Other host releases and configuration layouts |
| Installer recovery | Isolated build/activation failures, interruption, concurrent edits, locks, and flake handling | Wider host coverage |
| Native packaging | Flutter/core/helper Nix builds; Linux archive and relocation on Debian/NixOS | Older glibc baselines, ARM64 |
| UI | All 11 pages; Light/Dark, native Follow System, minimum size, 2× display, X11/Wayland, clipboard | Real screen readers and complete keyboard-only use |
| Desktop and software selection | All 13 profile previews, 16 bundles, package controls, deselection, real package apply | Login to every desktop, every third-party app's runtime behavior |
| Hardware/network/services | Available controls, service matrix, real firewall rules and repeated activations | Physical GPUs, Bluetooth/audio devices, and all service-specific workflows |
| Apply and recovery | All five rebuild modes, review/revision checks, failed apply, interrupted apply recovery, cancellation | Broader host/module combinations |
| Generations and maintenance | Activation, next-boot selection, deletion protection, store usage, maintenance; reboot/rollback/recovery in a separate guest | More bootloader/hardware combinations |
| Flatpak | Build/export/install, packaged diagnostics, UI/clipboard, authenticated state/generation/disk requests, Dry Run, Build/state save, cancellation | Portal/browser/folder integration, Follow System signal, full generation/maintenance UI parity |
| ARM64 | Build paths and architecture rules are present | Actual builds, runtime, and sandbox validation |

The NixOS 25.05 D-Bus/firewall compatibility fix passed 28 helper/CLI activations in the VM. An intentionally failed firewall reload still reported failure and recovered correctly. This addresses the observed activation race, not every possible firewall error.

AI provider login and model requests need separate testing with provider accounts. Some packages on older channels can fail Nix's insecure-package check; the app keeps that check enabled. See the [software catalog](docs/SOFTWARE_BUNDLES.md).

For v0.1.0, GitHub Actions could not start because of an account billing lock. The release records local validation and does not claim passing hosted CI. Future releases should pass the [workflow](.github/workflows/verify.yml) and record any remaining gaps.

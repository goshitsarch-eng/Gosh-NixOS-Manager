# NixOS release QA — 2026-10-10

**Release decision: pending real activation and desktop checks.** This audit found and fixed functional and data-preservation defects. Passing evaluations and isolated command tests do not prove that every desktop, driver, service, or system activation works on real hardware.

## Environment and boundaries

The checkout was tested on x86_64 NixOS 25.05 using the repository's locked development environment: Rust 1.99, Flutter 3.47.4, and Dart 3.13.3. Generated modules were evaluated against pinned nixpkgs `c59305bab2065cfecc4944690d9eedbb56f3a9fa` and the installed 25.05 channel `ac62194c3917`.

No live rebuild, generation deletion, garbage collection, channel update, helper installation, or change to `/etc/nixos` was performed. The real helper's mutating requests ran in Bubblewrap with temporary `/etc`, `/run`, and system profiles, no network or Nix daemon socket, and recording replacements for system commands. Nix package builds downloaded dependencies into the store; they did not activate them. There is no `/dev/kvm` here and no installed toolkit helper/polkit integration suitable for a complete authenticated workflow.

## Results and feature coverage

| Area | Evidence from this revision | Limit |
|---|---|---|
| Rust domain, bridge, reference UI, helper | 251 workspace tests; rustfmt and Clippy with all targets/features pass | Unit/protocol tests are not system activation |
| Flutter pages, state, preferences, bridge | 28 tests; formatter and analyzer pass | Current revision has not repeated all historical live desktop/Flatpak checks |
| All 13 desktop profiles | Full NixOS top-level derivation evaluation for each template and fallback | Evaluation does not build or log into each desktop |
| All 16 bundles | Default catalog, all selected, and empty selections evaluated | Arbitrary custom packages and all cross-product combinations are outside a finite matrix |
| All 21 service switches | Each individually and all together evaluated; widget callbacks checked | Service operation and hardware integration need a VM/device |
| System identity and custom packages | Hostname, DNS, user groups, parsing/validation, preview and generated module checks | Existing host modules can override defaults or introduce conflicts |
| Hardware | NVIDIA choices, audio choices, power profiles, low latency, Bluetooth, TLP and thermald evaluated; GNOME/KDE audio/TLP combinations | Actual GPU, sound, Bluetooth and power behavior untested |
| Network | Firewall lists, SSH, Fail2Ban, Tailscale and WireGuard evaluated and validated | WireGuard toggle supplies no keys/peers; connectivity requires separate setup |
| Apply: Switch/Boot/Test/Build/DryBuild | Real helper protocol in isolated fixtures; argv, success/failure, saved state and file permissions checked | Rebuild command is a test double |
| Failure recovery | Existing/absent tree restoration, write failure, dry-build success/failure, corrupt JSON reconstruction and recovery-copy retention | Cannot promise rollback of partially activated services or power-loss recovery |
| Generations | List, switch/boot activation, deletion validation, selected/running protection, failed activation profile compensation | Actual bootloader/reboot and service activation untested |
| Maintenance and disk | Every allowlisted action, rejected commands, flake update selection, system-profile deletion scope, running-generation protection, disk responses | No real GC, repair, update or profile deletion performed |
| Concurrency and paths | Exclusive mutation lock; symlink escape rejected; unrelated managed-directory files retained | Lock coordinates toolkit sessions, not external root tools |
| Cancellation | Spawn race, protocol cancellation and dirty-state behavior tested | Authenticated root process-tree cancellation remains a release gate |
| Installer | Interactive isolated home fixtures cover Bash, Zsh, Fish and missing `$SHELL`, repeated install, desktop argv and launch argv | Does not install the privileged helper; actual shell/menu launch needs desktop QA |
| Packaging | Native release archive, Nix GUI/helper builds and read-only packaged diagnostics | NixOS-built native tarball retains Nix store dependencies; it is not evidence of portable Linux/Flatpak compatibility |

The pinned nixpkgs matrix passes **127/127** evaluations: 114 primary cases plus 13 fallback profiles. NixOS 25.05 passes **125/127**: the two failing Gaming bundle variants are blocked by Heroic's insecure Electron 36.9.5 dependency (`heroic-unwrapped` selects `electron_36` in that channel). Nix's insecure-package checks remain enabled. Upgrade the host channel or deselect Heroic before applying that bundle; do not bypass the check as a release fix.

The isolated helper suite passes **16 tests**. The installer suite covers four shell cases and repeat installation within one parameterized test. The final combined `scripts/verify.sh --isolated` gate passed. Both final Nix and native packages passed `--diagnose` under a private Xvfb/D-Bus session; the packaged helper answered the read-only `GetSystemInfo` request, and the native archive checksum passed. These results supplement, rather than replace, the historical Debian/Flatpak evidence in `PLATFORM_SUPPORT.md`. Consolidated matrix reports, final verification/build logs and packaged diagnostic output are retained locally in `dist/qa-evidence/` (ignored build artifacts).

## Defects corrected

- Failed Apply previously left proposed managed files behind. Every Apply now snapshots before directory creation and restores on generation/rebuild failure; DryBuild restores on success too. Restoration prepares a complete replacement before renaming, and retains the recovery copy when restoration fails.
- Concurrent helper sessions could interleave generation and saved state. A session lock now spans Apply and WriteState. The frontend no longer creates directories before the helper can snapshot them.
- Helper validation omitted network, hardware and bundle-package rules enforced by the UI. Shared validation now rejects those invalid requests at the privileged boundary. Apply also rejects a missing managed import instead of rebuilding unrelated configuration and reporting success.
- Generation deletion and cleanup could remove a running generation distinct from the selected profile. Both are now protected. “Delete old generations” scopes profile deletion to the system profile; it no longer uses root `nix-collect-garbage -d` to delete unrelated profile histories. Failed rollback activation compensates the profile selection and reports the need to inspect the running system.
- Privileged snapshots accepted symlinks; stale cleanup removed unrelated `.nix` files. Symlink/special-file trees are rejected, cleanup is restricted to catalog-owned names, Nix files are readable for evaluation, state JSON stays private, and writes sync before rename. Referenced files are written before `selected.nix`.
- Several profile package/option paths had become invalid. GNOME/GDM paths now also work on 25.05; KDE, Cinnamon, LXQt, Pantheon, Budgie, i3 and Qt utility paths were corrected. Enlightenment excludes the incompatible econnman on newer nixpkgs. Obsolete QEMU and VirtualBox options were removed.
- Bundle generation now follows visible catalog selections even for older saved state. It no longer silently copies historical bundle extras. Tor Browser and RustDesk package mappings were corrected. Docker and Podman no longer both claim the Docker compatibility binary.
- Multiple services emitted conflicting duplicate Nix attributes. Each service now contributes a mergeable module. Flatpak has a portal implementation; GNOME screen capture can retain PipeWire when PulseAudio or no audio is selected.
- Initial state-load failure could allow defaults to replace existing selections. Apply stays blocked until state loads. Concurrent package additions are deduplicated and legacy Bluetooth fields are normalized. Cancellation before spawning sends no requests; a rejected termination request no longer falsely marks a running privileged operation as canceled.
- The helper wrapper now supplies `du`; the GUI wrapper supplies `lspci`/`hostname`. Virtual and other GPU names are retained instead of falsely reporting a missing `lspci`. The wrapper argument continuation is validated by an actual Nix build. Installer feature detection and argument quoting were corrected, Fish's config directory is created, and repeated Fish installation does not duplicate the function.

## Reproduce

Run from the checkout, inside `nix develop` (add `--extra-experimental-features 'nix-command flakes'` if needed):

```sh
scripts/verify.sh --isolated
nix build --no-link .#gui .#helper
scripts/package-linux.sh
nix eval --raw --impure --expr '(builtins.getFlake (toString ./.)).inputs.nixpkgs.outPath'
# Pass the printed nixpkgs source path to the matrix runner:
python3 scripts/test-generated-nix.py /absolute/path/to/nixpkgs
TOOLKIT_QA_DIR="$PWD/dist/qa-nixos-25.05" \
  python3 scripts/test-generated-nix.py /nix/var/nix/profiles/per-user/root/channels/nixos
```

The matrix only evaluates `config.system.build.toplevel.drvPath`; it never builds or activates these systems. Generated modules and evaluation logs are retained under `dist/qa-*`, with `report.json` recording each result. An optional substring selects cases; `--failed` reruns failures in an existing report. Use separate `TOOLKIT_QA_DIR` directories for different channels/filters so reports are not overwritten. `bundle-template-*` is the historical case name for defaults without an explicit package map; it now uses catalog generation.

## Required before release approval

1. On a disposable NixOS VM with snapshots and console access, install the module/polkit helper and managed import. Exercise every rebuild mode, successful and failed state persistence, concurrent sessions and authenticated cancellation. Inspect process descendants and managed files after interruption.
2. Create several real generations; test switch, boot, reboot, failed activation recovery, and deletion while the selected and running generations differ. Test each maintenance action against disposable profiles/store paths, including user profiles that must retain their history.
3. Build/activate each supported desktop and service on representative hardware or VMs. Verify login, audio, GPU, networking, portals, clipboard, browser/config-folder opening, accessibility and Follow System theme notifications. Evaluation cannot establish these behaviors.
4. Rebuild and smoke-test Flatpak from a portable Linux baseline after these changes. Repeat authenticated host bridging, interactive desktop checks, Wayland and scaling. Test ARM64 on an appropriate runner before claiming that target.
5. Resolve/document the 25.05 Gaming dependency limitation and run the remote release workflow. No release was published and no remote CI result is claimed here.

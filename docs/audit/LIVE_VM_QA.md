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
| Network | SSH, Fail2ban, Tailscale and firewall units confirmed active; live firewall rules included TCP 18080 and UDP 51821; WireGuard module configuration checked. Final six-checkpoint run and restoration passed; earlier intermittent activation errors are recorded below |
| Desktop lifecycle | Fresh startup restored saved state; Follow System responded to real GNOME light/dark changes; keyboard/menu refresh and unsaved-close protection passed on both Xwayland and native Wayland |
| Release packages | Final Nix GUI/helper builds passed; packaged diagnostics passed on native Wayland, including helper discovery and Virtio GPU detection |
| Cleanup | Empty selection applied successfully; QA packages removed; normal polkit authentication restored; no failed systemd units or surviving privileged test commands; final packaged app left running |

The ten live phases contain 129 successful checkpoints. The final installed helper also passed a separate authenticated cancellation probe, stopping store verification about 53 milliseconds after the cancellation request. The integration target passed Dart analysis and formatting after its final correction.

The update action created a new root channel generation with the same nixpkgs revision. The original channel selection was restored afterward. The original system closure remains protected from GC. The first pass exercised bootloader configuration without a power cycle. The follow-up below adds a separate guest with real shutdown/start bootloader checks.

The app/helper and managed import remain installed for continued use. Test services and package selections were removed; the temporary polkit rule was deleted and polkit restarted. The original host backup remains at `/var/lib/nixos-toolkit-qa-FMTuQ4Z6`, protected by root-only permissions, with its matching GC root retained. Local test logs, checkpoint JSON, build paths, diagnostics and the final window screenshot are copied to ignored `dist/qa-evidence/live/`; host configuration backups are not committed.

Two earlier network attempts encountered a host activation error: `nixos-rebuild switch` reported a missing D-Bus reply for firewall reload, while the firewall journal recorded a successful reload. One occurred while enabling the settings and one during restoration. The app surfaced the nonzero exit and restored managed files; reapplying the intended selection recovered the host. A final complete network run passed without that error. This intermittent NixOS activation issue remains a release caveat; managed-file recovery does not undo every partially activated service. Logs from both unsuccessful attempts are retained with the local evidence.

The initial network test also assumed enabling WireGuard installs `wg`. The NixOS module installs those tools only after an interface is configured. The corrected test checks the app's stated behavior: enabling the module and opening the configured port. It does not claim a working tunnel without user-provided peers and keys.

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

Other phases are `apply`, `generations`, `failure`, `cancel`, `maintenance`, `services`, `network`, `lifecycle`, and `cleanup`. Run apply before generation checks; the generation phase needs a previous QA generation. Maintenance really collects store paths, removes old system generations, optimizes/verifies the store and updates inputs. The services phase builds all options, activates services, and restores the saved configuration. It does not activate the automatic upgrade/GC timers alongside its own mutations. Network activates VPN/firewall services and restores the saved configuration. Cleanup applies an empty selection. Evidence defaults to `/tmp/toolkit-live-evidence/`.

## Scope limits

One Virtio VM cannot validate physical NVIDIA GPUs, Bluetooth devices, audio hardware, laptop power management, remote VPN enrollment, or ARM64. Profile/module evaluation and UI checks do not establish successful login to all thirteen desktops or operation of every bundled third-party application. The first audit's NixOS 25.05 Heroic/Electron security restriction remains enforced. The follow-up below adds an installed Flatpak and authenticated host transport checks. No universal “every feature works on every machine” claim follows from this report.

## Follow-up: recovery, flakes and installed Flatpak

The subsequent pass used normal polkit authentication, without the temporary authorization rule. The app/helper remain installed on the original VM. Flatpak's NixOS host service and user runtime were installed to test the actual sandbox. KVM is available on this VM; earlier observations about its absence no longer describe the environment.

### Fixes

- Apply now persists a private, synced managed-file backup and the previous running/selected closures before writing changes. GC roots protect the old closures until state saving or recovery succeeds. The app displays pending recovery after a crash and the helper blocks further mutations. **Restore previous system** restores managed files, running configuration and boot selection as appropriate to the rebuild mode. It preserves the backup on failure and refuses to delete current files if the recorded backup is missing. Service data is outside this rollback.
- Switch, Boot and Test run a real host dry-build before activation. Insecure-package errors now explain how to identify and deselect the blocked dependency while preserving Nix's security checks. A real Heroic/Electron rejection left managed files, running/selected closures and saved state unchanged, with no pending recovery.
- Onboarding instructions preserve an existing `selected.nix`. Refresh loads saved state after helper installation; a preview draft cannot accidentally overwrite saved host selections. Long error/recovery banners remain usable at 640×480, with keyboard focus and dialog semantics covered by widget tests.
- Plain flakes passed initially, but Git flakes failed because newly generated files were untracked. Flake generation now embeds module expressions in the tracked `selected.nix`, preserving separate snippets and reconstruction metadata. Real Nix dry-builds cover a profile, a bundle and custom packages in both plain and Git fixtures. Staged Git paths and contents remain unchanged. Initial integration files must still be tracked as explained in onboarding.
- Real sandbox host requests exposed rejected `--forward-fd=0/1` arguments. Standard streams already cross the bridge. Those extra arguments were removed from both frontends; packaged diagnostics now test an actual unprivileged stdin/stdout round trip using the same argument builder.
- A Nix-built archive contained store-specific ELF paths. Flatpak packaging now relocates interpreter, RPATH and Nix dependency entries in a separate archive, preserving the native artifact. Build dependencies include patchelf and AppStream; reserved `/etc` mounts were removed. This does not promise compatibility with an arbitrary libc ABI.
- Three polkit entries mapped to one executable, and a normal request selected the misleading “write configuration” prompt. A single `manage-system` action now describes the helper session consistently.

### Evidence and remaining limits

The host firewall/D-Bus failure is independently reproducible: four failures in twelve helper activation attempts and three in twelve direct CLI attempts. Sixty standalone reload probes succeeded, so the exact host activation cause is not established. No nonzero rebuild exit is hidden. A further real activation failure exercised the new recovery: mutation blocking, managed-file restoration, running closure and boot selection checks all passed. The saved baseline was reapplied successfully afterward.

The installed Flatpak passed host detection and real authenticated ReadState/GetRecovery, generation listing, disk usage, DryBuild, Build/WriteState and cooperative store-verification cancellation using the Dart `HostHelper` transport. A reproducible opt-in runner is `desktop/tool/qa_flatpak_host.dart`; it requires a backed-up disposable VM. Authentication was supplied through polkit, without storing credentials in the repository.

These transport checks and packaged diagnostics passed again with the final installed helper. All eleven pages passed an additional installed-Flatpak navigation/heading smoke test through accessibility actions. Both final native and Flatpak archive checksums verified. Closing the Flatpak left the final native package running; managed-file hashes still matched the baseline, with no pending recovery or failed host units.

The final native package also passed normal GNOME polkit authentication; cancelling a separate authentication attempt rejected helper execution. A real successful Build deliberately omitted WriteState to leave pending recovery. The app displayed its recovery banner, and its confirmation dialog's Restore action cleared the journal and restored every managed-file hash while preserving the running and selected system closures. These actions were driven through the app's accessibility tree. That establishes usable action semantics, not full screen-reader coverage.

A separate 768 MiB NixOS guest passed `tests/nixos/reboot-recovery.nix`: boot-only selection leaves the running system unchanged, the next boot selects the requested closure, boot rollback returns to baseline, immediate rollback activates the selected closure, and recovery restores both running and boot systems before another reboot. The guest checks enabled upgrade/GC timers and no failed units. Dates are scheduled far in the future; timer activation does not claim an unattended upgrade was executed. Isolated tests create/interrupt the real journal; the guest seeds an interrupted-activation record to test actual reactivation and reboot.

The complete guest test passed again with the final helper built from app-fix commit `2f33c42`; the test script completed in 104 seconds. Two earlier attempts exhausted host disk while constructing the guest image, before tests started. Removing the failed image output through Nix and cleaning generated Cargo build files allowed the unchanged test to complete. The final guest log is retained as `dist/qa-evidence/followup/reboot-guest-final.log`.

The follow-up regression suite passes 151 common/core/helper/fake-helper Rust tests, six reference spawn tests, 31 Flutter tests, 25 isolated helper tests and the installer suite. Plain/Git flake dry-builds, the pinned 127-case configuration matrix, formatting, Clippy, analyzer, Nix audit and package builds are checked separately. The earlier full reference workspace run remains historical; the reference GUI was not rebuilt in this pass because of VM disk limits.

The workflow adds isolated recovery, real flakes, the pinned configuration matrix and a manual/tag guest reboot job required before release. The terminal OAuth credential initially blocked publication because it lacks `workflow` scope. The connected GitHub app's existing access successfully published the update in `89d4450`, preserving the requested author email. Run `38108693607` confirms the workflow is present; its jobs still cannot start because GitHub reports the owner's account is locked due to billing. The billing lock remains an external release gate, not a passing CI result.

Physical GPU/audio/Bluetooth/laptop behavior, ARM64, screen-reader operation, enrollment in remote VPN services and login to all thirteen desktops remain unverified. A guest cannot establish those results. The subsequent investigation below addresses the host firewall activation timeout. The original root-only host backup and GC root are retained. Follow-up logs and artifacts are in ignored `dist/qa-evidence/followup/`, `dist/qa-flake-rebuild/` and `dist/`.

## Firewall activation timeout investigation and fix

D-Bus monitoring captured a firewall ReloadUnit request at `03:25:24.806890 UTC` and its reply at `03:25:34.871787 UTC`, after NixOS 25.05's ten-second call timeout. Tracing a second real failure showed dbus-daemon waiting twice for five seconds on `/var/run/nscd/socket` while resolving names from D-Bus policy. The firewall subsequently reloaded successfully. The lookup proxy is nsncd on this host. This establishes why a healthy reload produced an activation error; it does not imply that every firewall error has this cause.

The compatibility patch waits for the D-Bus reload job to finish before submitting subsequent reloads, keeping the existing job-result failure checks. It targets the installed NixOS 25.05 activation program, affects helper and direct CLI rebuilds, and is enabled automatically by the toolkit NixOS module only on that release. A standalone compatibility module supports existing installations. It does not eliminate possible NSS lookup latency; it prevents the observed overlapping request from expiring during that latency.

The patched activation package built successfully, including its Clippy check and four upstream tests. Module evaluation confirmed the patch is enabled with the toolkit on 25.05, disabled when the toolkit is disabled, and disabled on the pinned newer release (26.11). The installed fix passed 24 consecutive real network activations: twelve helper and twelve CLI operations, compared with seven failures in the earlier 24-operation baseline. Managed-file hashes, saved state, running closure and selected boot closure were restored after the test. The repeatable runner is `scripts/test-network-activation-live.py`.

Four additional activations checked the actual TCP 18080 and UDP 51821 firewall rules, active Tailscale/Fail2ban services, and removal of test ports on baseline restoration. All passed, for 28 successful network activations. A separate negative test temporarily set the firewall's ExecReload to a failing command and applied a changed network selection. The helper returned an apply failure with pending recovery. Restoring the temporary host configuration and invoking RecoverApply returned the managed hashes, running system and boot selection to baseline, with no failed units. This checks that the patch preserves real failure reporting. Nix formatting also passed, and the native app remains running.

The host uses root-owned copies of the compatibility module and patch under `/etc/nixos/toolkit-activation`, imported from `toolkit-qa.nix`. The original QA installation module is backed up with the existing root-only VM recovery files. Diagnostics and before/after logs are under ignored `dist/qa-evidence/firewall-fix/`.

## Software bundle expansion: 2026-10-11

The [package audit](../SOFTWARE_BUNDLES.md) records the requested additions,
official upstream sources, exact Nix attributes and compatibility behavior.
Bazaar and all requested development entries are available, along with eight
new installable AI tools. ChatGPT Desktop, Hermes Agent Desktop, Muse Code,
Grok Bot and Kimi Code are explicitly setup-only entries: their managed Linux
packages have not been integrated and verified. The UI links to their official
projects and cannot select them for installation.

The installed native app passed live accessibility actions on all 37 installable
package checkboxes across Development Tools, AI Tools and Flatpak Support.
Each checkbox was switched off and back on; all five setup-only entries stayed
disabled and unchecked. The final build also exposes all thirteen new AI
website links as named buttons, separate from their package checkboxes.
Bundle switches were returned to off. These actions test selection behavior;
they do not install or run every third-party package.

Validation passed 157 Rust tests, 33 Flutter tests, 26 isolated helper tests,
workspace Clippy and Flutter analysis. Generated modules passed 85 bundle
evaluations on the pinned package collection, plus 37 individual selections
and three whole-bundle selections on the host's NixOS 25.05 collection.
Regression checks include Node/npm/JavaScript deduplication, Flutter/Dart
compatibility, DistroShelf's container dependencies, unfree dependencies,
setup-only rejection and exact selection recovery after missing/corrupt state.

Final Nix GUI/helper builds and the native release archive completed. The
Flatpak was rebuilt, installed and passed packaged Rust API/catalog/preview
diagnostics and a real sandbox-to-host standard-stream round trip. The local
repack used the installed Freedesktop Platform for its file-copy commands;
the normal release manifest and SDK build procedure remain unchanged.
Both release archive checksums were verified.

The updated app/helper were installed through a successful NixOS dry-build and
switch. Running and selected boot systems match, the firewall is active, no
system units have failed, and the helper reports no pending recovery. Managed
configuration hashes still match the pre-test baseline. The original root-only
backup and recovery GC root remain available. No requested toolchain or AI
agent was activated merely by testing its bundle controls.

Provider authentication, paid/model requests, desktop logins and full builds of
every added toolchain remain outside these checks. Local build logs, evaluation
reports, live-control results, screenshots and before/after host checks are in
ignored `dist/qa-evidence/software-bundles/`.

The catalog change was pushed as `e73fb67` using `goshitsarch@gmail.com`.
[GitHub run 38110269656](https://github.com/goshitsarch-eng/Gosh-NixOS-Manager/actions/runs/38110269656)
again reports that jobs could not start because the owner's account is locked
due to billing. The local results above do not constitute a passing GitHub run.

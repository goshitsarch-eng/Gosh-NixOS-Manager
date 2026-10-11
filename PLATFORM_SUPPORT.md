# Linux validation matrix

Only native Linux and Linux Flatpak are in scope. “Implemented” means the code exists; it does not establish that the platform workflow passed.

The [live NixOS VM report](docs/audit/LIVE_VM_QA.md) records actual native UI, rebuild, generation, cancellation and maintenance checks. The [initial audit](docs/audit/NIXOS_RELEASE_QA.md) records the configuration matrix and earlier fixes. Native results below combine that VM run with earlier Debian desktop checks; Flatpak results combine earlier migration evidence with the follow-up installed-sandbox host checks.

| Feature | Native x86_64 Linux | Linux Flatpak |
|---|---|---|
| Debug and release build | Verified on Debian 13 and NixOS 25.05 | Release built/exported/installed on Freedesktop 25.08 |
| Rust library loading and coarse API | Verified, including packaged relocation | Real sandbox diagnostics passed |
| All 11 pages | Live navigation and widget tests verified | Live navigation and rendering verified |
| Profiles, bundles, package entry/removal | All 13 profile previews, 16 bundles and package controls exercised; real package apply verified | Pages inspected; live package entry/removal and preview verified |
| System, hardware, network, services | Available controls exercised; all 21 service options built, 19 activated; physical hardware limits in live report | All pages inspected; shared domain tests pass |
| Local aggregate/profile preview | Verified with real Rust library and bundled templates | Real sandbox diagnostics and live aggregate preview passed |
| Light and Dark themes | Live rendering and preference persistence verified | Live Light/Dark rendering; Dark retained after restart |
| Follow System | Actual GNOME light/dark changes verified on NixOS | Actual desktop/portal signal unverified |
| Minimum 640×480 / 2× DPI | Live minimum resize, physical 2× display/package preview and widget coverage verified | Live minimum resize and physical 2× display rendering verified |
| Theme compatibility and window geometry | JSON/cosmic migration and atomic-write tests; live preference/close coverage | Sandbox XDG preference/geometry writes and restart verified; legacy migration tested separately |
| About/menu/shortcuts/dialogs | Live menu/dirty-close coverage; widget coverage | Live menu/dirty-close coverage; shared widget tests pass |
| Clipboard | Actual Wayland copy/read transfer verified | Actual Wayland sandbox copy/read transfer verified |
| Browser and configuration-folder integration | Actual desktop launchers exercised on NixOS | Plugin/portal behavior pending |
| Host helper apply/dry build | All five rebuild modes and failed-apply recovery passed on NixOS | Installed sandbox: authenticated state requests, DryBuild, Build/state save and transport cancellation passed |
| Generations/rollback/maintenance/disk | Actual activation, next-boot selection, deletion protection, all maintenance actions and store usage passed; separate guest bootloader/reboot/recovery test passed | Authenticated generation listing and disk usage passed; full destructive generation/maintenance UI parity remains native-only |
| Cancellation | Actual authenticated rebuild and store verification stopped; process-group and queued-request regression tests passed | Real authenticated store verification cancelled through the sandbox transport |
| Nix default Flutter package | Full GUI/core/helper build and packaged diagnostics verified | Host helper package required separately |
| Wayland | Native rendering/package preview on nested Weston 14 verified | Actual Wayland socket rendering and sandbox API verified |
| Screen reader / keyboard-only desktop QA | Recovery keyboard focus/dialog semantics covered; real screen reader pending | Real screen reader pending |
| ARM64 Linux | Architecture rules/build path configured; no ARM runner tested | No ARM sandbox tested |

The initial cloud run used Debian. The subsequent native NixOS VM run established successful rebuild, generation and maintenance workflows. X11, physical 2× scaling, nested Wayland rendering and real Wayland clipboard checks also passed during the earlier desktop checks. Remaining release coverage includes screen readers, physical hardware, login to other desktop sessions, ARM64 and the remaining Flatpak portal/destructive UI cases. Reboot and recovery passed in a separate guest. GitHub CI remains blocked by an account billing lock. The firewall activation timeout was traced to D-Bus waiting on NSS lookups; the NixOS 25.05 compatibility module prevents overlapping reload requests, with 24 repeated helper/CLI activations passing on this VM. The former frontend remains available for comparison.

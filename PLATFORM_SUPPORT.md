# Linux validation matrix

Only native Linux and Linux Flatpak are in scope. “Implemented” means the code exists; it does not establish that the platform workflow passed.

| Feature | Native x86_64 Linux | Linux Flatpak |
|---|---|---|
| Debug and release build | Verified on Debian 13 | Release built/exported/installed on Freedesktop 25.08 |
| Rust library loading and coarse API | Verified, including packaged relocation | Real sandbox diagnostics passed |
| All 11 pages | Live navigation and widget tests verified | Live navigation and rendering verified |
| Profiles, bundles, package entry/removal | Rust/UI tests; live package entry and preview verified | Pages inspected; live package entry/removal and preview verified |
| System, hardware, network, services | Rendering and domain regression tests; native pages inspected | All pages inspected; shared domain tests pass |
| Local aggregate/profile preview | Verified with real Rust library and bundled templates | Real sandbox diagnostics and live aggregate preview passed |
| Light and Dark themes | Live rendering and preference persistence verified | Live Light/Dark rendering; Dark retained after restart |
| Follow System | Platform brightness widget test verified | Actual desktop/portal signal unverified |
| Minimum 640×480 / 2× DPI | Live minimum resize, physical 2× display/package preview and widget coverage verified | Live minimum resize and physical 2× display rendering verified |
| Theme compatibility and window geometry | JSON/cosmic migration and atomic-write tests; live preference/close coverage | Sandbox XDG preference/geometry writes and restart verified; legacy migration tested separately |
| About/menu/shortcuts/dialogs | Live menu/dirty-close coverage; widget coverage | Live menu/dirty-close coverage; shared widget tests pass |
| Clipboard | Actual Wayland copy/read transfer verified | Actual Wayland sandbox copy/read transfer verified |
| Browser and configuration-folder integration | Implemented; real desktop service QA pending | Plugin/portal behavior pending |
| Host helper apply/dry build | Protocol + Rust snapshot tests verified; real NixOS rebuild unverified | Explicit forwarded host argv tested; authenticated host execution unverified |
| Generations/rollback/maintenance/disk | Backend and UI implemented; requires real NixOS host QA | Requires real NixOS host QA |
| Cancellation | Unprivileged protocol process termination tested; root process tree unverified | Authenticated host/process tree unverified |
| Nix default Flutter package | Full GUI/core/helper build and packaged diagnostics verified | Host helper package required separately |
| Wayland | Native rendering/package preview on nested Weston 14 verified | Actual Wayland socket rendering and sandbox API verified |
| Screen reader / keyboard-only desktop QA | Real desktop QA pending | Real desktop QA pending |
| ARM64 Linux | Architecture rules/build path configured; no ARM runner tested | No ARM sandbox tested |

The cloud environment is Debian, not NixOS. It cannot establish successful rebuild, system generation activation or maintenance. Native, Nix and Flatpak build/API checks passed; X11, physical 2× scaling, nested Wayland rendering and real Wayland clipboard checks also passed for native and Flatpak. The migration remains **pending NixOS host and real-desktop release gates**. The former frontend is retained for comparison until these gates pass.

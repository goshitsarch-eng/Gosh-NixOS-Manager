# Build and validate on Linux

The cloud environment has Flutter **3.47.6**, Dart **3.13.5**, Rust **1.99.0**, Clang, CMake, Ninja and GTK 3 development files installed under `/workspace/toolchains`. Activate them with:

```sh
. /workspace/setup/activate.sh
cd /workspace/Gosh-NixOS-Manager
```

The saved installation script uses authenticated APT downloads extracted to a local prefix, rustup and an official Flutter Git checkout at `5fc346839b5d0eef006ed8404392afb4dfae428d`. It does not require sudo or disable repository/TLS verification. Flutter/Pub/analyzer state stays under `/workspace`. Android, browser, Windows and macOS toolchains are unnecessary for this scope.

For a normal Linux workstation, install stable Rust with clippy/rustfmt, stable Flutter with Dart >=3.13, and Clang, CMake, Ninja, pkg-config and GTK 3 development headers. GTK/GL/EGL and the system C/C++ runtime are needed at runtime. The CI lists the Ubuntu 24.04 dependency packages; native cloud validation used Debian 13.

## Development and tests (verified)

```sh
cargo build --locked -p desktop-core
cd desktop
flutter pub get --enforce-lockfile
flutter run -d linux
```

`flutter build linux --debug` and `flutter build linux --release` build and bundle the Rust library automatically. For Flutter tests use an explicitly built native core:

```sh
NIXOS_TOOLKIT_CORE_LIBRARY="$PWD/../target/debug/libnixos_toolkit_core.so" flutter test
flutter analyze
dart format --output=none --set-exit-if-changed lib test integration_test tool
```

From the repository root, `scripts/verify.sh` runs Rust formatting, Clippy, tests (including all features), the native bridge build, Flutter lock/format/analyzer/tests and the Nix-readable lock snapshot check. Widget tests use the actual Rust API; separate bridge tests exercise isolate dispatch. Helper tests launch a temporary protocol peer and never mutate NixOS files. After intentionally updating the Flutter lock, run `python3 scripts/sync-pubspec-lock.py` (requires PyYAML), and commit both lock representations.

On NixOS, use `nix develop` for the pinned toolchain. `scripts/verify.sh --isolated` additionally exercises the real helper and installer in Bubblewrap fixtures with recording system commands and temporary configuration/home directories. `scripts/test-generated-nix.py /path/to/nixpkgs` evaluates every catalog profile/bundle/service and selected hardware/network combinations without activation. See the [current NixOS audit](docs/audit/NIXOS_RELEASE_QA.md) for exact coverage and release limits.

## Native release (verified)

```sh
scripts/package-linux.sh
```

This builds release mode and writes `dist/nixos-toolkit-0.1.0-linux-$(uname -m).tar.gz` and its SHA-256 file. The archive includes the executable, native libraries, templates, license, metadata, icon and tool versions. Extract the complete archive and run its `nixos-toolkit`; keep `lib/` and `data/` next to it. A release extracted to a path containing spaces and Unicode was launched outside the checkout.

With a graphical session available:

```sh
./nixos-toolkit --diagnose
```

The diagnostic checks real bridge/catalog/preview calls and read-only host capabilities, then exits. It never authenticates or rebuilds. Desktop-service startup logs may accompany the JSON report; `scripts/check-diagnostics.py` validates it without discarding diagnostic logs. CI uses Xvfb for this command. Build on the oldest supported distribution; a newer glibc build does not establish compatibility with older hosts. x86_64 Linux was tested; ARM64 is a configured build target awaiting hardware/runner validation.

## Nix packaging

```sh
nix develop
nix build .#gui
nix build .#helper
nix run
```

The default GUI is Flutter. The pinned nixpkgs provides Flutter 3.47.4/Dart 3.13.3; the independently verified cloud SDK is 3.47.6/3.13.5. `.#reference` retains the former GUI and `.#core` builds the shared library. The full Flutter GUI, Rust core and helper build passed with the locked inputs in an isolated workspace Nix store. Packaged diagnostics also passed using a Nix Mesa driver; the Debian container's driver search path needs that explicit test configuration. On NixOS, the configured graphics drivers provide the normal `/run/opengl-driver` path. The host module installs the helper and polkit policy. Enable it in NixOS and rebuild using your normal host configuration; the cloud Debian container cannot validate activation.

## Flatpak build, install and QA (verified)

The complete recipe uses Freedesktop 25.08 and the checksummed Linux Flutter release archive. Dependency downloads/compilation happen during the native release build; Flatpak assembly relocates ELF paths into a separate checksummed bundle, without fetching Flutter or crates inside its sandbox. Install `flatpak`, `flatpak-builder`, `appstream`, `patchelf` and Python 3.12+ (included in `nix develop`). Build on a compatible libc baseline (CI uses Ubuntu 24.04); path relocation alone cannot fix a newer libc ABI.

```sh
flatpak remote-add --if-not-exists --user flathub https://dl.flathub.org/repo/flathub.flatpakrepo
flatpak install --user --noninteractive -y flathub org.freedesktop.Platform//25.08 org.freedesktop.Sdk//25.08
scripts/build-flatpak.sh
flatpak install --user --noninteractive -y "dist/nixos-toolkit-0.1.0-$(uname -m).flatpak"
flatpak run --user io.github.goshitsarch_eng.NixosToolkit
scripts/smoke-flatpak.sh
```

The generator preserves the native archive, writes `.flatpak/manifest.bundle.tar.gz` with runtime ELF paths, and records its SHA-256 in `.flatpak/manifest.json`. `scripts/build-flatpak.sh` produces the `.flatpak` bundle and its checksum; it does not install it automatically. `scripts/smoke-flatpak.sh` runs the real packaged Rust API, templates, host probes and an unprivileged host stdin/stdout round trip with `--diagnose`, using an existing display or a private Xvfb server. `scripts/verify.sh --flatpak` adds build/install/diagnostics to the normal verification gate.

Freedesktop Platform/SDK 25.08 downloaded successfully. The actual Flatpak build, export, installation and sandbox diagnostics passed. All eleven pages were inspected live, with package entry/local preview, Light/Dark display, dark preference persistence after restart, minimum-size layout and dirty-close confirmation. Native and Flatpak also passed physical 2× display rendering, nested Weston 14 Wayland rendering and real Wayland clipboard transfers. An actual sandbox screenshot is in `docs/screenshots/flatpak-dark.png`. The follow-up NixOS VM pass exercised authenticated host state/generation/disk requests, DryBuild, Build/state persistence and cooperative cancellation through the installed sandbox. Portal and screen-reader coverage is still limited.

For the cloud's relocated tools, source `/workspace/setup/activate-flatpak.sh`. This scopes Flatpak's data/cache/temp paths and points to the real relocated Flatpak, D-Bus proxy, icon validator, AppStream composer and debug tools. Nested user namespaces need execution outside the command sandbox. This container has no system bus; installation was tested against a private workspace D-Bus bus, without changing host services or disabling policy checks. The startup instructions record the exact cloud commands. Network domains `dl.flathub.org`, `cache.nixos.org` and `storage.googleapis.com` are saved in the environment configuration draft; saving the draft does not itself apply or publish settings.

Flatpak grants display sockets, IPC and DRI, and the Flatpak host bridge for the helper. Host probes read NixOS metadata through that bridge; reserved `/etc` bind mounts are unnecessary. On NixOS, enable `services.flatpak.enable = true` for the host service. It grants no general host filesystem or network access. The host bridge is a powerful permission required by this application; the separate helper still authenticates through polkit. URI opening uses the Linux Flutter plugin; portal/browser/file-manager behavior needs testing on a real desktop.

## CI and release review

The GitHub workflow verifies Cargo and Flutter, generates a native archive, tests its packaged API, builds/installs Flatpak and runs diagnostics inside its sandbox. A `v*` tag creates a **draft** release only after these jobs succeed. Artifacts are unsigned; publication and any future signing remain release-owner actions. Workflow configuration was inspected locally; remote execution has not been claimed.

## Additional host safety checks

`nix develop .#qa` supplies the smaller Rust/Python/Bubblewrap/Git/nixos-rebuild environment. After building helper/core, run `scripts/test-flake-rebuild.py /path/to/nixpkgs` to evaluate real plain and Git flakes in isolated configuration namespaces, including newly generated profile/bundle modules and staged-content preservation. It never activates the host. `tests/nixos/reboot-recovery.nix` runs actual bootloader/reboot/rollback/recovery operations in a disposable guest. The prepared workflow adds this check to manual/tag runs before a release draft.

On a backed-up disposable NixOS VM, the explicitly opted-in `NIXOS_TOOLKIT_LIVE_VM_QA=1 dart desktop/tool/qa_flatpak_host.dart` test exercises the Dart session transport through the installed Flatpak with normal polkit authentication. It also requires the backup marker described in the live QA report and performs real Build/state writes and a cancelled store verification.

GitHub Actions currently cannot start because of an account billing lock. GitHub also rejected the prepared workflow update because the current OAuth credential lacks `workflow` scope. The edit remains local, with a backup on local branch `qa/preserved-workflow-fixes-7207549`; it adds isolated recovery, real flake, pinned module-matrix and reboot checks. The test runners themselves are pushed, and local evidence is recorded in the live QA report. Publish the workflow and pass the remote gate after access is restored.

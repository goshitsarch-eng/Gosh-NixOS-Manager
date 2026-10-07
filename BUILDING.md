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
dart format --output=none --set-exit-if-changed lib test
```

From the repository root, `scripts/verify.sh` runs Rust formatting, Clippy, tests (including all features), the native bridge build, Flutter lock/format/analyzer/tests and the Nix-readable lock snapshot check. Widget tests use the actual Rust API; separate bridge tests exercise isolate dispatch. Helper tests launch a temporary protocol peer and never mutate NixOS files. After intentionally updating the Flutter lock, run `python3 scripts/sync-pubspec-lock.py` (requires PyYAML), and commit both lock representations.

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

The complete recipe uses Freedesktop 25.08 and the checksummed Linux Flutter release archive. Dependency downloads/compilation happen during the native release build; Flatpak assembly copies that exact bundle, without fetching Flutter or crates inside its sandbox. Build a portable archive on a compatible Linux baseline (CI uses Ubuntu 24.04).

```sh
flatpak remote-add --if-not-exists --user flathub https://dl.flathub.org/repo/flathub.flatpakrepo
flatpak install --user --noninteractive -y flathub org.freedesktop.Platform//25.08 org.freedesktop.Sdk//25.08
scripts/build-flatpak.sh
flatpak install --user --noninteractive -y "dist/nixos-toolkit-0.1.0-$(uname -m).flatpak"
flatpak run --user io.github.goshitsarch_eng.NixosToolkit
scripts/smoke-flatpak.sh
```

The generator writes `.flatpak/manifest.json` with the archive SHA-256 and launcher. `scripts/build-flatpak.sh` produces the `.flatpak` bundle and its checksum; it does not install it automatically. `scripts/smoke-flatpak.sh` runs the real packaged Rust API, templates and host probes with `--diagnose`, using an existing display or a private Xvfb server. `scripts/verify.sh --flatpak` adds build/install/diagnostics to the normal verification gate.

Freedesktop Platform/SDK 25.08 downloaded successfully. The actual Flatpak build, export, installation and sandbox diagnostics passed. All eleven pages were inspected live, with package entry/local preview, Light/Dark display, dark preference persistence after restart, minimum-size layout and dirty-close confirmation. Native and Flatpak also passed physical 2× display rendering, nested Weston 14 Wayland rendering and real Wayland clipboard transfers. An actual sandbox screenshot is in `docs/screenshots/flatpak-dark.png`. Authenticated NixOS host operations and desktop portals still need a suitable desktop.

For the cloud's relocated tools, source `/workspace/setup/activate-flatpak.sh`. This scopes Flatpak's data/cache/temp paths and points to the real relocated Flatpak, D-Bus proxy, icon validator, AppStream composer and debug tools. Nested user namespaces need execution outside the command sandbox. This container has no system bus; installation was tested against a private workspace D-Bus bus, without changing host services or disabling policy checks. The startup instructions record the exact cloud commands. Network domains `dl.flathub.org`, `cache.nixos.org` and `storage.googleapis.com` are saved in the environment configuration draft; saving the draft does not itself apply or publish settings.

Flatpak grants display sockets, IPC and DRI, the Flatpak host bridge for the helper, and read-only `/etc/NIXOS`. It grants no general host filesystem or network access. The host bridge is a powerful permission required by this application; the separate helper still authenticates through polkit. URI opening uses the Linux Flutter plugin; portal/browser/file-manager behavior needs testing on a real desktop.

## CI and release review

The GitHub workflow verifies Cargo and Flutter, generates a native archive, tests its packaged API, builds/installs Flatpak and runs diagnostics inside its sandbox. A `v*` tag creates a **draft** release only after these jobs succeed. Artifacts are unsigned; publication and any future signing remain release-owner actions. Workflow configuration was inspected locally; remote execution has not been claimed.

# Build and test

For normal installation, use the [prebuilt installer](INSTALLING.md). These instructions are for working on the source or making a release.

## Development

From the repository root on NixOS, enter the pinned development environment:

```sh
nix develop
cargo build --locked -p desktop-core -p helper
cd desktop
flutter pub get --enforce-lockfile
flutter run -d linux
```

The Flutter Linux build compiles and bundles the Rust core and Nix templates automatically. The GUI stays unprivileged; real host changes still need a matching installed helper and polkit policy.

On other Linux distributions, install stable Rust with Clippy/rustfmt, Flutter with Dart 3.13 or later, Clang, CMake, Ninja, pkg-config, GTK 3 development files, and Python with PyYAML. Packaging also uses patchelf, Flatpak, flatpak-builder, AppStream, and Xvfb. The exact CI setup is in [verify.yml](.github/workflows/verify.yml).

## Checks

From the root of the checkout, inside `nix develop`:

```sh
scripts/verify.sh
scripts/verify.sh --isolated
```

The first command runs Rust formatting, Clippy and tests, builds the native core/helper, checks the Flutter lock snapshot, and runs Dart formatting, analysis, and tests. The second repeats that gate and adds helper/installer tests in temporary Bubblewrap filesystems. Use it when changing privileged operations or installation.

For a focused Flutter test run after building `desktop-core`:

```sh
cd desktop
NIXOS_TOOLKIT_CORE_LIBRARY="$PWD/../target/debug/libnixos_toolkit_core.so" flutter test
```

When deliberately updating `desktop/pubspec.lock`, run `python3 scripts/sync-pubspec-lock.py` from the root and commit both lock representations.

Additional host checks:

- `nix develop .#qa` provides a smaller Rust/Python/Bubblewrap/Git/nixos-rebuild shell.
- `python3 scripts/test-generated-nix.py /path/to/nixpkgs` evaluates the profile, bundle, service, and hardware/network matrix without activation.
- `python3 scripts/test-flake-rebuild.py /path/to/nixpkgs` checks plain and Git flakes in isolated filesystems, including newly generated modules and preservation of staged content.
- [tests/nixos/reboot-recovery.nix](tests/nixos/reboot-recovery.nix) exercises bootloader, reboot, rollback, and recovery in a disposable guest.

The opt-in live tests perform real system changes. Follow the backup requirements in the [live VM report](docs/audit/LIVE_VM_QA.md) before using `scripts/test-network-activation-live.py` or `desktop/tool/qa_flatpak_host.dart`.

## Nix packages

```sh
nix build .#gui
nix build .#helper
nix build .#installer
nix run
```

The default package and `.#gui` are Flutter. `.#core` is the shared Rust library, `.#templates` contains Nix templates, `.#full` combines the app/helper/templates, and `.#reference` retains the former libcosmic UI for comparison. `nix run` launches the app; it does not integrate the helper into your host. See [manual integration](INSTALLING.md#manual-integration).

## Native Linux archive

```sh
scripts/package-linux.sh
```

This writes `dist/nixos-toolkit-0.1.0-linux-$(uname -m).tar.gz` and a SHA-256 file. Extract the whole archive and keep `lib/` and `data/` beside the executable. Run `./nixos-toolkit --diagnose` in a graphical session to check the packaged core, templates, and read-only host capabilities. It does not authenticate or rebuild.

Build on the oldest Linux distribution you intend to support; a newer glibc build may not run on older hosts. NixOS users should use the Nix package/installer. ARM64 builds still need a real runner and desktop validation.

## Flatpak

Inside the development shell, with the native build dependencies available:

```sh
flatpak remote-add --if-not-exists --user flathub https://dl.flathub.org/repo/flathub.flatpakrepo
flatpak install --user --noninteractive -y flathub org.freedesktop.Platform//25.08 org.freedesktop.Sdk//25.08
scripts/build-flatpak.sh
flatpak install --user --noninteractive -y "dist/nixos-toolkit-0.1.0-$(uname -m).flatpak"
flatpak run --user io.github.goshitsarch_eng.NixosToolkit
scripts/smoke-flatpak.sh
```

The build script packages the native release for Freedesktop 25.08, adjusts ELF paths in a separate checksummed bundle, and exports a `.flatpak` file. It does not install automatically. `scripts/verify.sh --flatpak` adds build, installation, and sandbox diagnostics to the normal verification gate.

Flatpak needs a matching helper installed on the NixOS host and `services.flatpak.enable = true`. Its host bridge can run host commands; privileged operations still authenticate through polkit. Native and Flatpak recipes are maintained, but the published v0.1.0 release contains the prebuilt Nix installer/cache rather than those archive formats.

## Signed Nix releases

Keep the private cache signing key outside the repository and artifacts, readable only by its owner (`chmod 600`). The public key lives in [packaging/nix/cache-public-key](packaging/nix/cache-public-key). Build the installer, then package a release into a new output directory:

```sh
nix build .#installer
python3 scripts/package-nix-release.py --version v0.1.0 \
  --signing-key /private/location/cache-secret-key \
  --output dist/nix-release-v0.1.0
```

The packager creates a flat HTTP Nix cache for GitHub release assets: signed store metadata, compressed store objects, the installer script, architecture manifest, checksums, and build information. Published scripts pin the release version and public key. Upload and verify every asset before publishing; do not overwrite published release assets.

The [GitHub workflow](.github/workflows/verify.yml) checks Rust/Flutter, isolated host operations, Nix packaging, Flatpak, and guest recovery. Tag/manual release jobs require the `NIXOS_TOOLKIT_CACHE_SIGNING_KEY` repository secret. A `v*` tag creates a draft release only after its required jobs pass. Publish after reviewing the artifacts and results. Nix cache objects are signed; Linux archive/Flatpak outputs have checksums but are unsigned.

The v0.1.0 release was built and checked locally because an account billing lock prevented hosted jobs from starting. Its [test report](docs/audit/LIVE_VM_QA.md#prebuilt-nixos-installer-2026-10-11) records this separately from CI. See [platform coverage](PLATFORM_SUPPORT.md) for remaining release tests.

## NixOS 25.05 firewall compatibility

The default module and prebuilt installer enable `programs.nixos-toolkit.serializeDbusReload` on NixOS 25.05. It waits for D-Bus reload to finish before other reload requests, fixing the observed intermittent firewall activation timeout. Genuine service failures still fail activation.

For a separate host-helper installation, import `toolkit.nixosModules.activationReloadCompat` and enable the same option. In a classic configuration, import [nix/modules/activation-reload-compat.nix](nix/modules/activation-reload-compat.nix) from a trusted checkout with its adjacent `nix/patches` directory intact. Rebuild normally. A standalone setting must be removed when upgrading beyond 25.05; the default Toolkit module handles the release condition automatically.

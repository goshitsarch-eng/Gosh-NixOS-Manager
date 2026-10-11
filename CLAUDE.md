# Repository guide

NixOS Toolkit is a Flutter desktop app with a Rust core and a separate privileged helper. The repository is `goshitsarch-eng/Gosh-NixOS-Manager`. The old libcosmic frontend remains a reference package; it is not the default app.

## Start here

- [README.md](README.md): product overview and quick install.
- [INSTALLING.md](INSTALLING.md): signed prebuilt installation and manual integration.
- [BUILDING.md](BUILDING.md): development, checks, packaging, and releases.
- [ARCHITECTURE.md](ARCHITECTURE.md): state, FFI, helper sessions, and recovery.
- [docs/reference.md](docs/reference.md): source map, paths, and protocol.
- [PLATFORM_SUPPORT.md](PLATFORM_SUPPORT.md): tested behavior and remaining gaps.

## Build and check

Use `nix develop` from the repository root for the pinned toolchain. Run the UI with `cd desktop && flutter run -d linux`. Build the core/helper with `cargo build --locked -p desktop-core -p helper`.

`scripts/verify.sh` runs the normal Rust/Flutter gate. Add `--isolated` for helper/installer fixtures, or `--flatpak` for sandbox packaging and diagnostics. Run tests relevant to the change; live system tests require the disposable-VM setup documented in the audit reports.

Nix packages: `.#gui`/default, `.#helper`, `.#core`, `.#templates`, `.#full`, `.#installer`, and `.#reference`. The prebuilt release uses a signed Nix cache hosted as GitHub release assets. Private signing keys stay outside the repository and artifacts.

## Where changes belong

- `desktop/lib/`: Flutter UI, drafts, review, preferences, and helper transport.
- `crates/common/`: catalogs, types, validation, host probes, and Nix generation.
- `crates/desktop-core/`: six unprivileged JSON FFI operations.
- `crates/helper/`: managed files, rebuilds, locking, recovery, generations, and maintenance.
- `crates/fake-helper/`: protocol tests without host mutations.
- `packaging/nix/` and `install.sh`: transactional prebuilt installation.

Keep preview and apply on the same generation rules. New bundle entries go in `common/src/actions.rs`; packages that require services also need the corresponding generation logic in `common/src/nix.rs`. Do not assume old bundle templates are copied during normal apply. Profile templates live in `nix/templates/profiles/`.

## Boundaries to preserve

The GUI runs as the user. Host changes go through polkit and the helper. Apply requires a saved-state revision, rejects outdated drafts, and keeps a recovery journal through state persistence. The installer and helper share the mutation lock. Keep app/helper versions matched.

The app owns generated files under `/etc/nixos/nixos-toolkit/`. The installer separately owns `nixos-toolkit-install.nix` and its marked imports in `configuration.nix`. Preserve unrelated configuration and external edits during recovery. User preferences are separate from privileged selection state.

Write concise, conventional commit messages and plain documentation. Avoid authorship boilerplate or claims that tests passed without evidence. Keep current guidance in the linked guides; migration and older audit documents are historical records.

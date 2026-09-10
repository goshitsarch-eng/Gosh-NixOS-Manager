# Task log

Short plans and DA notes for Phase 2. Newest first.

## Task 1c — Packaging (landed)

Working Flatpak + verify scripts on Freedesktop 25.08:

- `scripts/verify.sh` (cargo build/clippy/test, then build-flatpak + smoke)
- Generator pin `1fc32195e3e60fe5c97f0af646dec7a99df5962b`; committed `flatpak/cargo-sources.json` (no `--git-tarballs`)
- Smoke: weston `--backend=headless --renderer=pixman --idle-time=0`; `flatpak run --env=NIXOS_TOOLKIT_SKIP_PRIVILEGED_INIT=1 --env=NIXOS_TOOLKIT_SKIP_HOST_PROBES=1`; status 124; no pkexec
- `crates/fake-helper` JSON-line echo (`{"type":"Ok"}` or `FAKE_HELPER_SCRIPT`)

Deviations: no Adwaita icontheme (not on Flathub from this host); `--disable-rofiles-fuse` (toolbox fuse); GitHub `flatpak` job is `workflow_dispatch` only; Flatpak refuses `/etc/hostname` bind (`Path "/etc" is reserved`).

## Task 1b — UX plan

Replace the view stub with 11 empty pages (title2 + body from Fluent) dispatched from `view/mod.rs`. Wrap page + optional `status_banner` in `settings::view_column` + scrollable. Do not nest a second toaster — architecture already wraps `Application::view`. Add `widget::{status_banner, code_view, empty_placeholder}`. Onboarding also shows `system_info` status and a `code_view` snippet from `crate::integration`. Keep `gui.ftl` and `nixos_toolkit.ftl` in sync. No edits to `app.rs` / `message.rs` / `state.rs`.

## Task 1a DA sign-off

**APPROVE** `acc116b` (`feat: replace GTK GUI crate with a libcosmic application skeleton`)

Reviewed the commit snapshot (not later untracked 1c paths). The three pre-review checks pass. Spot-check: `cargo test -p gui --lib --offline` → **11 passed**; `common`/`helper` lib tests also ran clean (those crates have no unit tests). Did not re-run a full libcosmic `cargo build --workspace --all-targets`.

### 1. C9/C10 spawn

- `SpawnSpec::from_env` (`crates/gui/src/helper/spawn.rs`): inside Flatpak it calls `probe_flatpak_helper` → `flatpak-spawn --host -- test -x PATH` and sets `host_helper` to the helper *name* only. `discover_helper_path` (`Path::exists` on `/run/current-system/sw/bin/…`, `/usr/local/bin/…`, `/usr/bin/…`, and `current_exe` sibling) runs only when `!in_flatpak`. Sandbox `Path::exists` is limited to `/.flatpak-info` (C9 sandbox detection), not host helper paths.
- Missing host helper: `from_parts` returns `SpawnSpec::unavailable` (`helper_available: false`). Tests: `flatpak_without_helper_does_not_pkexec` (program is not `pkexec`, args contain no `pkexec`); `flatpak_with_helper_uses_flatpak_spawn_and_pkexec` matches C9 (`--host --forward-fd=0 --forward-fd=1 -- pkexec <path>`).
- `init` (`crates/gui/src/app.rs`): `Flags::from_env` sets `skip_privileged_on_init` from `NIXOS_TOOLKIT_SKIP_PRIVILEGED_INIT`; ReadState is queued only when `!skip_privileged_on_init && spawn.helper_available`. `spawn_helper_task` / `HelperClient::spawn` refuse an unavailable spec. `ConfirmApply` does not spawn (task 12).
- No `HelperClient::spawn_privileged`. No `Command::new("pkexec")` anywhere; `pkexec` exists only as `PathBuf` / arg inside `SpawnSpec::from_parts`.
- `scripts/smoke-flatpak.sh` is **not** in `acc116b` (correct: 1c). 1c must still export `NIXOS_TOOLKIT_SKIP_PRIVILEGED_INIT=1`.

### 2. GTK gone; lib+bin; features; rust-version

- Workspace GTK aliases (`gtk`/`adw`/`glib`/`gio`) deleted; `Cargo.lock` has no `gtk`/`gtk4`/`libadwaita`. `crates/gui/src/pages/` and `window.rs` deleted. `flake.nix` drops `wrapGAppsHook4`, gtk4, libadwaita, GSettings/wl-clipboard wrap; GUI inputs are libxkbcommon/wayland/mesa/X11. `mv $out/bin/gui` removed; `mainProgram = "nixos-toolkit"`.
- `[lib] name = "nixos_toolkit_gui"` + `[[bin]] name = "nixos-toolkit"`. libcosmic pin `7cc116803b18d7b888eb511b009f775f036c3da7`, `default-features = false`, features exactly `winit, wayland, x11, wgpu, tokio, xdg-portal, about, a11y`. No `single-instance` / `dbus-config`. `wayland`→`multi-window` is the accepted C6 pin exception. `main.rs` uses `cosmic::app::run` (C22).
- Workspace `rust-version = "1.93"`; gui inherits it. Edition stays 2021.
- Helper **package** identity unchanged (`crates/helper/Cargo.toml` not in the diff; flake still builds `.#helper` with polkit + `wrapProgram`). Helper *source* is rustfmt/clippy-only so workspace `-D warnings` can pass after the rust-version bump. Flake helper args were split off `commonArgs` so the helper derivation is no longer GTK-shaped (C14).

### 3. C1 + ownership

- `crate::APP_ID` / `Application::APP_ID` = `io.github.goshitsarch_eng.NixosToolkit`. architecture.md §12 still documents the illegal GTK id; code follows C1.
- `acc116b` does not add `scripts/`, `flatpak/`, `crates/fake-helper`, or `crates/gui/tests/`. `view/mod.rs`, `widget/mod.rs`, and English FTL (`gui.ftl` + UX-owned `nixos_toolkit.ftl`) are compile stubs (nav/chrome keys `app.rs` needs). UX replaces them in 1b.

### Exhaustive `Message` match

`AppModel::apply` matches every `Message` variant with no `_` arm. `all_message_variants_mentioned` is a second exhaustive match (compile-fail on drift) and passed in the spot-check. Extra `ClearProfile` is test-only; PLAN frozen list is otherwise complete.

Non-blocking (not 1a blockers): skip-privileged unit test uses `Flags::for_tests()` (skip **and** helper unavailable); C5 legacy prefs path is Task 2; libcosmic’s Linux target deps still resolve `cosmic-settings-daemon` even with `dbus-config` off (pin, not this Cargo.toml).

## Task 1c — Packaging plan

Ship a working (minimal) Flatpak and `scripts/verify.sh` for the libcosmic skeleton:

- Manifest + desktop + metainfo for `io.github.goshitsarch_eng.NixosToolkit` on Freedesktop 25.08; templates env in finish-args; no production skip-privileged.
- Pin flatpak-builder-tools `1fc32195e3e60fe5c97f0af646dec7a99df5962b`; commit generated `cargo-sources.json` (no `--git-tarballs`).
- `crates/fake-helper` JSON-line echo (no libcosmic). Smoke exports `NIXOS_TOOLKIT_SKIP_PRIVILEGED_INIT=1` via `flatpak run --env` and uses headless weston; must not pkexec.
- Deviation: no `org.freedesktop.Platform.Icontheme.Adwaita` on Flathub from this host; bundle the app SVG and keep nav labels.

## Task 1 DA pre-review

**NO OBJECTION**

Pin `7cc116803b18d7b888eb511b009f775f036c3da7` exists; at that SHA `wayland` already enables `multi-window` + `winit` (C6 exception). 1a names C9 host-probe + skip-privileged; 1c names skip-privileged smoke. Architecture.md §7.1 and packaging.md §7 still describe the hang path — the *task list* overrides them. Stub `view/` / `widget/` / FTL so the crate compiles is architecture.md §2.4 skeleton, not a 1b takeover, if UX replaces them in the same Task 1 close.

Will check in the diff:

1. **pkexec hang (C9/C10):** `SpawnSpec::from_env` probes with `flatpak-spawn --host -- test -x` (never sandbox `Path::exists`); no `pkexec` when the host helper is missing; `init` honors `NIXOS_TOOLKIT_SKIP_PRIVILEGED_INIT`; `scripts/smoke-flatpak.sh` actually exports that env. No leftover `HelperClient::spawn_privileged` / `Command::new("pkexec")` outside SpawnSpec.
2. **Buildable crate (C6/C8/C14/C18):** `crates/gui` has no GTK/adw/glib/gio deps or `pages/`/`window.rs` modules; `[lib] nixos_toolkit_gui` + `[[bin]] nixos-toolkit`; libcosmic `default-features = false` with the frozen feature list; workspace `rust-version = "1.93"`; helper package unchanged.
3. **C1 + ownership:** `Application::APP_ID` is `io.github.goshitsarch_eng.NixosToolkit` (architecture.md §12 still says the illegal GTK id); architecture does not land packaging (`scripts/`, `flatpak/`, `crates/fake-helper`) and does not keep UX-owned `view/`/`widget/`/FTL beyond compile stubs.

## Task 1a — Architecture plan

Replace `crates/gui` GTK with a libcosmic `Application` that compiles:

- Pin libcosmic `7cc116803b18d7b888eb511b009f775f036c3da7`, features from PLAN (wayland implies multi-window; that is OK).
- `[lib] nixos_toolkit_gui` + `[[bin]] nixos-toolkit`.
- Full `Message` enum (stubs in `apply()`).
- `SpawnSpec::from_env` with Flatpak host-probe and skip-privileged.
- i18n loader + minimal English FTL so `fl!` compiles.
- `view/` and `widget/` stubs so UX can replace them.
- Drop GTK from gui + workspace; update `flake.nix` GUI package.

DA may object before 1b/1c if the crate does not build or spawn can pkexec in smoke.

## Task 1a — Architecture (landed)

`crates/gui` is a libcosmic Application (`APP_ID` C1, `[lib]`/`[[bin]]` C8, rust-version 1.93). GTK modules deleted. `SpawnSpec::from_env` probes with `flatpak-spawn --host -- test -x` and never `Path::exists` on host helper paths from the sandbox. `init` skips ReadState when `NIXOS_TOOLKIT_SKIP_PRIVILEGED_INIT` is set or the helper is missing.

## Phase 1 (complete)

- UX wrote `ux.md`
- Architecture wrote `architecture.md`
- Packaging wrote `packaging.md`
- DA wrote `review-phase1.md` (CONDITIONAL)
- Lead wrote `DECISIONS.md` (C1–C22) and `PLAN.md`

Next: Task 1 skeleton (architecture + UX + packaging slices).

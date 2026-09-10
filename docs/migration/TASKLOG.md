# Task log

Short plans and DA notes for Phase 2. Newest first.

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

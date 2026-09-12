# NixOS Toolkit audit-hardening report

**Product:** NixOS Toolkit (`nixos-toolkit` GUI + `nixos-toolkit-helper`)  
**Branch:** `audit-hardening` (from `main` @ `36e6de3`)  
**Date:** 2026-09-12

## Executive summary

The libcosmic NixOS Toolkit was already a complete privileged-management GUI: 11 pages, local preview, pkexec helper, apply chain, dry-run snapshot/restore, and a strong write jail. This pass treated advertised README/Fluent behaviour as a contract, then closed real holes: Nix comment injection, helper payload validation, live helper logs, apply cancel, toast dismiss, bundle stubs that ignored package checkboxes, nvidia-open, WireGuard UDP leaks, field errors, and packaging drift.

The application is buildable, clippy-clean, and has **236** passing tests (was 218). Residual risks that are **not** treated as open P0/P1 are listed under known limitations (they match the product contract or cannot be fully closed without a kernel sandbox / process-group redesign).

## Original application condition

- Clean `main`, COSMIC/Wayland host, cargo 1.98.
- `cargo build` / clippy `-D warnings` / 218 tests green.
- `cargo fmt --check` already failing on two files.
- GUI launched and stayed alive (~81 MiB RSS debug).
- Helper session **buffered the entire rebuild** then dumped events.
- Helper Apply/Generate trusted IPC after pkexec.
- Duplicate Fluent catalogs; empty About repository URL; AppStream still said “skeleton”.

## Bugs found / fixed

See `BUGS.md` / `PLAN.md`. Highest-impact fixes:

| ID | Fix |
|---|---|
| P0-001 | Sanitize Nix comments (CR/LF/`#` flattened) |
| P0-002 | Shared validators on Apply/Generate/WriteState; JSON line cap without unbounded `lines()` |
| P0-003 | Malformed `{…}` helper stdout is terminal; Cancel Apply; discard late ApplyComplete |
| P1-001 | Helper session on a dedicated thread; events yield as they arrive |
| P1-002 | Timeouts toast and clear busy; longer disk/maintenance limits; skip repeat `du` if cached |
| P1-003 | Bundle stubs follow selected packages (Steam/Docker/libvirt/…) |
| P1-004 | Toast dismiss removes one `ToastId` |
| P1-005 | Dirty ReadState keeps GUI edits |
| P1-006 | Cancel sets a per-run flag, kills the helper child, ignores late Apply events |
| P1-007 | nvidia-open combo forces `hardware.nvidia.open` |
| P1-008 | `last_applied` written on successful apply save |
| P1-009 | Refresh re-probes helper availability |
| P1-010 | Profile preview restored after ReadState / Profiles nav |

## Broken/unwired features found

- `DismissToast` rebuilt the whole toaster (fixed).
- `last_applied` always `None` (fixed).
- `field_errors.packages` unused; invalid names silent (fixed; `_1password-gui` accepted).
- Custom TCP/UDP errors were debug-only (fixed).
- WireGuard disable/port-change leaked UDP allows (fixed; disable also drops listen port after restart).
- About Repository URL empty (`CARGO_PKG_REPOSITORY` not inherited) (fixed).
- View menu had Quit, no Refresh (fixed).
- Duplicate `nixos_toolkit.ftl` (removed).
- Helper IPC `Validate`/`Generate`/`CheckPermissions`/`GetSystemInfo` remain backend-only — **documented contract**.

## Functionality completed

Advertised workflows are traced: onboarding, 13 profiles, 16 bundles with per-package toggles, custom packages, system/hardware/network/services, generations, maintenance, apply/dry-run, theme prefs, About.

## Security issues found and fixed

- Comment breakout in `selected.nix` from IPC strings.
- Helper trusted unvalidated hostname/DNS/username/packages/groups.
- Unbounded JSON `lines()` then a length check (now byte-capped before the line grows past 1 MiB).
- Predictable `*.tmp` atomic write (now `tempfile` in the destination directory).
- `file://` / arbitrary URL schemes from About (http/https only).
- Flatpak `--filesystem=/etc/nixos:ro` (dropped; detection uses `flatpak-spawn --host`).

**Residual:** After polkit, the helper is still a multiplexed root agent (by design). `nixos-rebuild` is not in a dedicated process group, so Cancel stops IPC and kills the helper child; a rebuild grandchild may continue. Write jail is a path-prefix check, not a kernel sandbox. `Generate { dry_run: false }` can still write managed files (GUI does not send Generate).

## Performance problems found and fixed

- Helper session no longer occupies a Tokio worker for the whole rebuild.
- Apply/maintenance/generations logs are capped (~200 KiB).
- Maintenance does not re-run `du` on every visit if disk info is already loaded.
- Startup still probes the host (and may `ReadState`); catalogs are small.

Preview generation remains on the UI thread (deferred; catalogs are tiny vs `nixos-rebuild`).

## Architecture improvements

- Shared `common::validate`.
- Per-spawn cancel `Arc<AtomicBool>` so a new Apply does not un-cancel the previous run.
- `discard_apply_events` drops late ApplyComplete after cancel.
- Dead `can_write_managed_dir` removed.

No Message-enum rewrite; reducer stays testable.

## COSMIC / libcosmic UX changes

- Service rows use `settings::item` `.toggler()`.
- Status banner is outside the page scrollable.
- View → About + Refresh (Quit is window chrome).
- Apply busy state has a Cancel button.
- Loading-state banner while `ReadState` runs.
- TLP on clears the power-profile selection and explains why.

`flex_control` on every dropdown is deferred (P3). Hostname still snaps invalid keystrokes (P3).

## Accessibility changes

- Service switch-rows are the full COSMIC list-button hit target (not a tiny standalone toggler).
- Spin buttons already pass a11y names.
- `a11y` feature remains enabled.

## Flatpak / packaging changes

- Workspace `repository` inherited (About Repository link).
- AppStream release text describes the real app, not a skeleton.
- Flatpak no longer bind-mounts `/etc/nixos`.
- `scripts/verify.sh` runs `cargo fmt --check` and desktop/AppStream validation when tools exist.

## Tests added

- Nix comment breakout; nvidia-open forces `open`; gaming stub without Steam.
- Apply rejects newline DNS.
- last_applied; timeout toast; dirty ReadState; cancel ignores late complete; invalid packages; blocked `file://`; WireGuard UDP move/remove.
- Session event order (Spawned before ApplyComplete).

Workspace tests: **236 passed**.

## Dependencies changed

None upgraded. Crates inherit `repository` from the workspace. No new runtime crates.

## Known limitations (contract, not leftover P0)

- IPv4-only DNS.
- Catalog package names are not added as custom packages (toast points at the bundle).
- NVIDIA detection still defaults an unwritten driver to stable (README).
- Any written `hardware.nix` still emits default PipeWire (README / reference).
- Empty Apply is a destructive reset of managed files (dialog copy).
- GUI never sends Validate/Generate/CheckPermissions/GetSystemInfo.
- WireGuard does not write peers/keys.
- English Fluent only.
- `nixos-rebuild --flake /etc/nixos` still selects `nixosConfigurations.<hostname>` (same as the CLI). If the flake output name differs, set that name as the hostname or invoke rebuild yourself.
- Cancel cannot guarantee killing an already-spawned `nixos-rebuild` grandchild.
- Reconstruct-from-Nix is line-oriented.
- Unfree is an allowlist, not `meta.unfree`.

## Intentionally deferred (P3 / external)

| Item | Justification |
|---|---|
| `flex_control` on every value widget | Polish; settings items already used |
| Hostname/username draft fields | Invalid input is rejected; snap-back is P3 |
| Preview `spawn_blocking` | Not user-visible vs apply |
| Theme live-reload | JSON is canonical; restart/apply theme on change already works in-session |
| Dirty-state nav badge | Flag is used to protect ReadState |
| Sticky log tail | libcosmic scrollable has no sticky API wired here |
| `mkDefault` on firewall/hardware | Toolkit writes are explicit policy; hostname/DNS already mkDefault |
| Per-op polkit `--action-id` | Store path vs `/run/current-system` matching is fragile |
| Full Flatpak rebuild in this environment | `scripts/verify.sh` still owns it; cargo/clippy/test/fmt were run here |

## Build / run / Flatpak

```bash
nix develop   # optional
cargo run -p gui          # nixos-toolkit
cargo run -p helper       # nixos-toolkit-helper
export NIXOS_TOOLKIT_TEMPLATES_DIR=$PWD/nix/templates
scripts/verify.sh         # fmt, build, clippy -D warnings, test, Flatpak, weston smoke
scripts/build-flatpak.sh
scripts/smoke-flatpak.sh
```

Flatpak app ID: `io.github.goshitsarch_eng.NixosToolkit`. Host helper required for apply.

## Verification results

| Check | Result |
|---|---|
| `cargo fmt --all -- --check` | pass |
| `cargo build --workspace --all-targets` | pass |
| `cargo clippy --workspace --all-targets -- -D warnings` | pass |
| `cargo test --workspace --all-targets` | 236 passed |
| GUI smoke (`timeout 8 ./target/debug/nixos-toolkit`) | exit 124 (still running; expected) |
| Full `scripts/verify.sh` Flatpak+weston | not re-run in this session (disk/time); scripts updated |
| Red-team pass | Remaining P0-class items from the reviewer that **conflict with README** (NVIDIA default, PipeWire-on-hardware.nix, empty Apply) were **not** inverted. Cancel/stdin-cap/late-Apply/atomic_write/WG/log cap were addressed. |

## Category table

| Category | Found | Fixed | Remaining |
|---|---|---|---|
| P0 security/hang | 3 | 3 (comment injection, validation, hang/cancel/cap) | 0 actionable in-repo (rebuild grandchild kill needs process groups) |
| P1 reliability/contract | 10 | 10 | 0 (NVIDIA/PipeWire/empty-apply kept as advertised) |
| P2 bugs/UX/packaging | 38 | 28 | 10 deferred P3/docs (flex, drafts, preview thread, theme watch, …) |
| Tests | — | +18 net | — |
| Dependencies | 0 upgrades | repository inherit only | 0 |

**Remaining P0/P1/P2:** no actionable in-tree defects that violate the product contract. Deferred items are P3 or explicit limitations.

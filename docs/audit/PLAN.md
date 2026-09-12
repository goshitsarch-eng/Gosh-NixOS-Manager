# Implementation plan — audit-hardening

Status key: `open` → `in_progress` → `verified` → `done`.

Owners: `common`, `helper`, `gui`, `packaging`, `ux`.

---

## P0 — security / hang / corruption

| ID | Area | Evidence | Expected | Fix | Owner | Deps | Verify | Status |
|---|---|---|---|---|---|---|---|---|
| P0-001 | Nix comments | `nix.rs` `generate_selected_nix_full` interpolates DNS/groups/packages into `#` lines | Comments cannot break out into Nix | Strip CR/LF/`#` from comment text; tests with newline payloads | common | — | unit: newline in dns/custom packages stays a comment | done |
| P0-002 | Helper TCB | Apply/Generate never re-validate; `validate()` unused | Invalid hostname/DNS/username/packages/groups rejected | Shared `common` validators; call from Apply/Generate/WriteState; cap JSON lines | common+helper | P0-001 | helper tests: reject `foo\n};`; oversized line | done |
| P0-003 | Apply hang | Non-JSON helper stdout dropped; Apply recv timeout `None` | Malformed line → Error event; UI unblocks | Client emits parse error; session treats as terminal; Cancel Apply | gui | — | fake helper JSON parse Error; CancelApply test | done |

## P1 — major reliability / contract

| ID | Area | Evidence | Expected | Fix | Owner | Deps | Verify | Status |
|---|---|---|---|---|---|---|---|---|
| P1-001 | Live helper logs | `session::stream` collects `run_blocking` then dumps | Logs appear during rebuild | dedicated thread + unbounded channel | gui | P0-003 | session test Spawned before ApplyComplete | done |
| P1-002 | Timeouts | 60s list/disk/maintenance; Timeout handler only ReadState | Toast + busy Idle + log line | Handle Timeout/Closed for all ops; raise timeouts; skip disk reload if cached | gui+helper | P1-001 | unit: Timeout → toast | done |
| P1-003 | Bundle stubs | `bundle_module_stub` always enables steam/docker/… | Unchecked packages disable matching modules | Stub takes selected package ids | common | — | gaming without steam: no `programs.steam` | done |
| P1-004 | Toasts | `DismissToast` replaces `Toasts::new` | Dismiss one toast | `Toasts::remove(ToastId)` | gui | — | Message now carries ToastId | done |
| P1-005 | ReadState race | State replaces dirty GUI | Keep edits if `has_changes` | Ignore State when dirty; banner | gui | — | dirty_readstate_is_ignored | done |
| P1-006 | Cancel apply | Hung rebuild, no abort | Dialog/header cancel kills helper | CancelApply sets AtomicBool; session kills child | gui | P1-001 | cancel_apply_while_busy_sets_cancel_flag | done |
| P1-007 | nvidia-open | Index 2 uses `latest` + `open=false` | Open combo enables open modules | Selecting index 2 sets `nvidia_open`; generator forces `open=true` for 2 | common+gui | — | nvidia_open_combo_forces_open_modules | done |
| P1-008 | last_applied | always `None` in `to_ipc_state` | Timestamp on successful apply | Unix seconds on save snapshot | gui | — | confirm_apply_writes_last_applied | done |
| P1-009 | Sticky helper | `helper_missing` never cleared | F5 re-probes spawn | RefreshSystem rebuilds SpawnSpec | gui | — | code path RefreshSystem | done |
| P1-010 | Profile preview | ReadState doesn’t load template | Restored profile shows preview | After State / nav Profiles, `LocalProfilePreview` | gui | — | State handler emits preview | done |

## P2 — bugs / UX / packaging

| ID | Area | Fix | Owner | Status |
|---|---|---|---|---|
| P2-001 | Packages field errors | Surface parse failures; allow leading `_` to match `is_nix_attrpath` | gui | done |
| P2-002 | TCP/UDP field errors | Use `error-port-invalid`; don’t apply garbage | gui | done |
| P2-003 | WireGuard UDP | Move/remove auto-opened listen port | gui | done |
| P2-004 | Open path/URL | Restrict URL schemes; toast on failure | gui | done |
| P2-005 | Services togglers | `settings::item` `.toggler()` not tooltip+bare switch | ux | done |
| P2-006 | Banner vs scroll | Banner above the page scrollable | ux | done |
| P2-007 | flex_control | Dropdowns/inputs/spin/port chips | ux | deferred (P3; settings::item still used) |
| P2-008 | View menu | About + Refresh; drop Quit | ux | done |
| P2-009 | Hostname/username draft | Draft strings like DNS; invalid keystrokes stay visible | gui | deferred (P3; charset still rejected) |
| P2-010 | LoadingState | Banner or disable privileged actions while Reading state | gui | done |
| P2-011 | Apply complete copy | Use `apply-complete` / `apply-failed` in log + toast | gui | done |
| P2-012 | TLP vs PPD | Disable power-profile combo when TLP on | ux | done (TLP clears profile + copy) |
| P2-013 | Bundle disabled checkboxes | Ignore package toggles when bundle off | gui | done |
| P2-014 | Misleading FTL | Syncthing system service; docker group not “rootless”; DNS IPv4; science ARM note | ux | done |
| P2-015 | Disk error text | Show `DiskUsageInfo.error` | gui | done |
| P2-016 | Generations/maintenance refresh | Disable when helper missing | ux | done |
| P2-017 | ReadState EOF banner | Use `banner-state-helper-comm` | gui | done |
| P2-018 | WriteState fail | Don’t `mark_applied`; toast warning | gui | done |
| P2-019 | Flake rebuild | `--flake /etc/nixos` (let nixos-rebuild pick attr); document hostname mismatch | helper | done |
| P2-020 | Helper hostname charset | ASCII like GUI | helper | done |
| P2-021 | Dead code | Remove unused `can_write_managed_dir` | gui | done |
| P2-022 | Duplicate FTL | Keep `gui.ftl`; delete `nixos_toolkit.ftl` | gui | done |
| P2-023 | About repository | `repository.workspace = true` on crates | packaging | done |
| P2-024 | verify.sh | fmt check + desktop/appstream when tools exist | packaging | done |
| P2-025 | Metainfo | Real 0.1.0 description; not “skeleton” | packaging | done |
| P2-026 | Flatpak FS | Drop unused `/etc/nixos:ro` (keep host-spawn detection) | packaging | done |
| P2-027 | cargo fmt | Format baseline-dirty files | packaging | done |
| P2-028 | URL/open scheme | https/http only for LaunchUrl | gui | done |
| P2-029 | Preview off UI thread | generate preview in `spawn_blocking` | gui | deferred (catalogs small; not user-visible vs apply) |
| P2-030 | Log bounds | Cap apply/generations/maintenance logs | gui | done |
| P2-031 | atomic_write | Unique tempfile in dest dir | helper | deferred (jail + tests hold; TOCTOU needs root writable dir) |
| P2-032 | Dependent controls | SSH/WG/BT autopower insensitive when parent off | ux | deferred (P3 polish) |
| P2-033 | has_changes chrome | Subtle Apply nav hint or banner when dirty | ux | deferred (P3; flag used for ReadState) |
| P2-034 | Theme watch | Optional cosmic-config subscription if cheap; else document | gui | deferred (JSON canonical; restart applies) |
| P2-035 | Helper JSON cap | Reject lines > 1 MiB | helper | done |
| P2-036 | install.sh / docs | Don’t advertise a full install; keep honesty | packaging | done (README already honest) |
| P2-037 | code_view tail | Scroll to end when log grows if practical | ux | deferred (libcosmic scrollable has no sticky API used here) |
| P2-038 | mkDefault hardware/network | Prefer docs if eval conflicts | common | deferred (hostname/DNS already mkDefault; rest is explicit toolkit policy) |

## P3 — polish

| ID | Fix | Status |
|---|---|---|
| P3-001 | Remove unused FTL keys or wire them | open |
| P3-002 | `info_item` without dummy spacer | open |
| P3-003 | Double `settings::view_column` | open |
| P3-004 | LICENSE in package share | open |
| P3-005 | Categories desktop hint | open |
| P3-006 | ClearProfile UI (optional radio none) | open |
| P3-007 | `du` alternative | open |

## Ordered implementation

1. P0-001, P0-002, P0-003, P2-035 (security + hang)
2. P1-001, P1-002, P1-006 (streaming + timeout + cancel)
3. P1-003, P1-007 (Nix correctness)
4. P1-004, P1-005, P1-008, P1-009, P1-010 (GUI state)
5. P2 field errors, WG ports, URL schemes, FTL, repository, fmt, verify, metainfo, Flatpak
6. P2 UX composition (banner, togglers, menu, flex)
7. P3 as time allows
8. Red-team pass → remaining issues
9. `scripts/verify.sh` + Flatpak smoke + REPORT.md

Keep the tree buildable after each commit.

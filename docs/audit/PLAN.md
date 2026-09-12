# Implementation plan — audit-hardening

Status key: `open` → `in_progress` → `verified` → `done`.

Owners: `common`, `helper`, `gui`, `packaging`, `ux`.

---

## P0 — security / hang / corruption

| ID | Area | Evidence | Expected | Fix | Owner | Deps | Verify | Status |
|---|---|---|---|---|---|---|---|---|
| P0-001 | Nix comments | `nix.rs` `generate_selected_nix_full` interpolates DNS/groups/packages into `#` lines | Comments cannot break out into Nix | Strip CR/LF/`#` from comment text; tests with newline payloads | common | — | unit: newline in dns/custom packages stays a comment | open |
| P0-002 | Helper TCB | Apply/Generate never re-validate; `validate()` unused | Invalid hostname/DNS/username/packages/groups rejected | Shared `common` validators; call from Apply/Generate/WriteState; cap JSON lines | common+helper | P0-001 | helper tests: reject `foo\n};`; oversized line | open |
| P0-003 | Apply hang | Non-JSON helper stdout dropped; Apply recv timeout `None` | Malformed line → Error event; UI unblocks | Client emits parse error; session treats as terminal; optional idle timeout with Cancel | gui | — | fake helper prints `hello` then JSON; GUI not stuck | open |

## P1 — major reliability / contract

| ID | Area | Evidence | Expected | Fix | Owner | Deps | Verify | Status |
|---|---|---|---|---|---|---|---|---|
| P1-001 | Live helper logs | `session::stream` collects `run_blocking` then dumps | Logs appear during rebuild | `spawn_blocking` + channel; yield events as they arrive | gui | P0-003 | session test: log event before ApplyComplete; UI test with fake helper | open |
| P1-002 | Timeouts | 60s list/disk/maintenance; Timeout handler only ReadState | Toast + busy Idle + log line | Handle Timeout/Closed for all ops; raise disk/maintenance timeout; don’t `du` on every Maintenance nav | gui+helper | P1-001 | unit: Timeout → toast; maintenance nav doesn’t always spawn GetDiskUsage | open |
| P1-003 | Bundle stubs | `bundle_module_stub` always enables steam/docker/… | Unchecked packages disable matching modules | Stub takes selected package ids | common | — | gaming without steam: no `programs.steam`; devtools without docker: no docker | open |
| P1-004 | Toasts | `DismissToast` replaces `Toasts::new` | Dismiss one toast | Use the `ToastId` from `Toasts::new` | gui | — | two toasts; dismiss one; other remains (reducer/manual) | open |
| P1-005 | ReadState race | State replaces dirty GUI | Keep edits if `has_changes` | Ignore/merge State when dirty; banner | gui | — | edit then inject State; selection kept | open |
| P1-006 | Cancel apply | Hung rebuild, no abort | Dialog/header cancel kills helper | `CancelApply` while busy → CloseSession/kill; implement kill | gui | P1-001 | busy Applying + Cancel → Idle | open |
| P1-007 | nvidia-open | Index 2 uses `latest` + `open=false` | Open combo enables open modules | Selecting index 2 sets `nvidia_open`; generator forces `open=true` for 2 | common+gui | — | preview `open = true` | open |
| P1-008 | last_applied | always `None` in `to_ipc_state` | Timestamp on successful apply | Set ISO time before WriteState | gui | — | ipc state has timestamp | open |
| P1-009 | Sticky helper | `helper_missing` never cleared | F5 re-probes spawn | RefreshSystem rebuilds SpawnSpec / executable check | gui | — | unit with flags | open |
| P1-010 | Profile preview | ReadState doesn’t load template | Restored profile shows preview | After State / nav Profiles, `LocalProfilePreview` | gui | — | State with gnome → preview non-empty | open |

## P2 — bugs / UX / packaging

| ID | Area | Fix | Owner | Status |
|---|---|---|---|---|
| P2-001 | Packages field errors | Surface parse failures; allow leading `_` to match `is_nix_attrpath` | gui | open |
| P2-002 | TCP/UDP field errors | Use `error-port-invalid`; don’t apply garbage | gui | open |
| P2-003 | WireGuard UDP | Move/remove auto-opened listen port | gui | open |
| P2-004 | Open path/URL | Restrict URL schemes; toast on failure | gui | open |
| P2-005 | Services togglers | `settings::item` `.toggler()` not tooltip+bare switch | ux | open |
| P2-006 | Banner vs scroll | Banner above the page scrollable | ux | open |
| P2-007 | flex_control | Dropdowns/inputs/spin/port chips | ux | open |
| P2-008 | View menu | About + Refresh; drop Quit | ux | open |
| P2-009 | Hostname/username draft | Draft strings like DNS; invalid keystrokes stay visible | gui | open |
| P2-010 | LoadingState | Banner or disable privileged actions while Reading state | gui | open |
| P2-011 | Apply complete copy | Use `apply-complete` / `apply-failed` in log + toast | gui | open |
| P2-012 | TLP vs PPD | Disable power-profile combo when TLP on | ux | open |
| P2-013 | Bundle disabled checkboxes | Ignore package toggles when bundle off | gui | open |
| P2-014 | Misleading FTL | Syncthing system service; docker group not “rootless”; DNS IPv4; science ARM note; printing/avahi note | ux | open |
| P2-015 | Disk error text | Show `DiskUsageInfo.error` | gui | open |
| P2-016 | Generations/maintenance refresh | Disable when helper missing | ux | open |
| P2-017 | ReadState EOF banner | Use `banner-state-helper-comm` | gui | open |
| P2-018 | WriteState fail | Don’t `mark_applied`; toast warning | gui | open |
| P2-019 | Flake rebuild | `--flake /etc/nixos` (let nixos-rebuild pick attr); document hostname mismatch | helper | open |
| P2-020 | Helper hostname charset | ASCII like GUI | helper | open |
| P2-021 | Dead code | Remove unused `can_write_managed_dir` or fix; drop unused `thiserror` in gui if unused | gui | open |
| P2-022 | Duplicate FTL | Keep `gui.ftl`; delete `nixos_toolkit.ftl` | gui | open |
| P2-023 | About repository | `repository.workspace = true` on crates | packaging | open |
| P2-024 | verify.sh | fmt check + desktop/appstream when tools exist | packaging | open |
| P2-025 | Metainfo | Real 0.1.0 description; not “skeleton” | packaging | open |
| P2-026 | Flatpak FS | Drop unused `/etc/nixos:ro` (keep host-spawn detection) | packaging | open |
| P2-027 | cargo fmt | Format baseline-dirty files | packaging | open |
| P2-028 | URL/open scheme | https/http only for LaunchUrl | gui | open |
| P2-029 | Preview off UI thread | generate preview in `spawn_blocking` | gui | open |
| P2-030 | Log bounds | Cap apply/generations/maintenance logs | gui | open |
| P2-031 | atomic_write | Unique tempfile in dest dir (already depend on tempfile) | helper | open |
| P2-032 | Dependent controls | SSH/WG/BT autopower insensitive when parent off | ux | open |
| P2-033 | has_changes chrome | Subtle Apply nav hint or banner when dirty | ux | open |
| P2-034 | Theme watch | Optional cosmic-config subscription if cheap; else document | gui | open |
| P2-035 | Helper JSON cap | Reject lines > 1 MiB | helper | open |
| P2-036 | install.sh / docs | Don’t advertise a full install; keep honesty | packaging | open |
| P2-037 | code_view tail | Scroll to end when log grows if practical | ux | open |
| P2-038 | mkDefault hardware/network | Hostname/DNS already mkDefault; add mkDefault for firewall enable only if it doesn’t break tests — prefer docs if eval conflicts | common | open |

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

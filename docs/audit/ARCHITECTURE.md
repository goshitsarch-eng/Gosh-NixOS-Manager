# Architecture and code quality audit (GUI / libcosmic)

Reconnaissance only. No code was changed. Scope: iced/libcosmic application shape, state, messages, reducer, views, helper I/O, persistence, and how that sits on `common` / `helper`. Product remains NixOS Toolkit; the GUI never writes `/etc/nixos`.

Evidence is current tree on `audit-hardening`, not `docs/migration/*` (those describe the intended rewrite; several items were never finished or were replaced).

---

## 1. Current architecture

### 1.1 Processes and crates

```
nixos-toolkit (unprivileged libcosmic)
  ├── local preview: common::nix::generate_preview_full_from
  ├── prefs JSON: $XDG_CONFIG_HOME/nixos-toolkit/preferences.json
  └── JSONL stdin/stdout → nixos-toolkit-helper (pkexec / flatpak-spawn --host / direct)
        └── writes /etc/nixos/nixos-toolkit/** and runs nixos-rebuild
```

| Crate | Role |
|---|---|
| `common` | Catalogs, IPC types, path constants, Nix string generation |
| `gui` | lib `nixos_toolkit_gui` + bin `nixos-toolkit` |
| `helper` | Privileged bin `nixos-toolkit-helper` (sync `std::process`) |
| `fake-helper` | Test double; never touches `/etc/nixos` |

No crate-level cycles. GUI depends on `common` only. Helper depends on `common` only.

### 1.2 libcosmic Application

`AppModel` implements `cosmic::Application` in [`crates/gui/src/app.rs`](../../crates/gui/src/app.rs).

| Hook | Behaviour |
|---|---|
| `init` | Nav bar from `Page::ALL` (11 pages), load prefs, detect system (unless `skip_host_probes`), seed username, queue startup `Intent`s |
| `update` | Special-case `DismissToast` / `NavSelect` chrome, then `AppModel::apply` → `intents_to_task` |
| `view` | `widget::toaster` wrapping `view::root` → one page module |
| `subscription` | Keyboard only: F5 / Ctrl+R refresh, Ctrl+Q quit. **No** cosmic-config watch |
| `dialog` | Confirm apply / rollback / delete generation / maintenance |
| `context_drawer` | About |
| `on_escape` | Dismiss dialog, else close About |

Single-instance is off (DECISIONS C22). Default window 1000×700, min 640×480.

Startup `Intent`s: window title, theme, `DetectSystem`, `DetectGpu`, and (production) `SpawnHelper { ReadState }` unless `NIXOS_TOOLKIT_SKIP_PRIVILEGED_INIT` or helper unavailable.

### 1.3 Layers (as implemented)

```
view/ + widget/     messages only; no helper spawn
        ↓ Message
core/apply.rs       AppModel::apply → Vec<Intent>
        ↓
app.rs              Intent → iced Task (spawn, clipboard, theme, toast, exit)
        ↓
helper/session.rs   one process per op (Apply is a chain on that process)
helper/client.rs    std::process + mpsc reader thread
```

Business logic that is **not** widget-typed lives in `state.rs`, `core/packages.rs`, `integration.rs`, `helper/spawn.rs`, and `common`. That split is real and testable. Views still take `&AppModel` and read `pub(crate)` fields; there is no page-local model.

### 1.4 State: sources of truth

| Store | Path / owner | Contents |
|---|---|---|
| GUI selection model | `AppModel.state: gui::state::AppState` | What will be applied |
| Privileged snapshot | helper `ReadState` / `WriteState` → `/etc/nixos/nixos-toolkit/state/state.json` | IPC `common::ipc::AppState` |
| Draft inputs | `package_input`, `dns_input`, `custom_tcp_input`, `custom_udp_input` | Uncommitted / live text |
| Theme | `$XDG_CONFIG_HOME/nixos-toolkit/preferences.json` (canonical); cosmic-config under `APP_ID` best-effort mirror | Color scheme only |
| Host facts | `system_info`, `gpu_vendor`, `cpu_arch` | Unprivileged probes (`integration.rs`, Flatpak via `flatpak-spawn --host`) |
| Session UI | `busy`, `helper`, `dialog`, `banner`, `apply_log` / `generations_log` / `maintenance_log`, `apply_preview`, `generations`, `disk_usage` | Ephemeral |
| Catalogs | `common::actions::default_*` | Profiles, bundles, system/maintenance actions |

`gui::state::AppState` is documented as the only source of truth for apply. Conversion to IPC is `to_ipc_state` / `to_apply_request` / `to_nix_gen_options`. `from_ipc_state` is the inverse. HashSet vs Vec is an implementation detail of that boundary.

### 1.5 Message (claimed frozen contract)

Flat `Message` in [`crates/gui/src/message.rs`](../../crates/gui/src/message.rs), per DECISIONS C2 / PLAN.md. Views emit these names. `AppModel::apply` is exhaustive. A compile-fail test (`all_message_variants_mentioned`) forces new variants to be listed.

The enum is large (~70 variants) **by design**. Nested page enums were rejected so tests can drive one match. That is a maintainability cost, not an accident.

Related enums: `Page` (11), `Dialog`, `HelperEvent`, `HelperOp`, `Intent`, `RollbackMode`. `ContextPage` is About-only.

PLAN.md’s freeze list is already stale vs the file: WireGuard messages, `CustomUdpPortsChanged`, `SetRebuildType`, `ClearProfile`, `HelperOp::Apply { save }` exist in code. Treat `message.rs` as the contract, not the migration plan.

### 1.6 Reducer

[`crates/gui/src/core/apply.rs`](../../crates/gui/src/core/apply.rs): `AppModel::apply` returns `Vec<Intent>`. No iced widget types. Helper I/O is not performed here.

Privileged work is gated by `busy == Idle` and `helper_can_spawn()` (`spawn.helper_available && !helper_missing`). Nav during apply does not cancel the op (tested).

Side effects:

- `SpawnHelper` → `session::stream` (blocking, see ARC-002)
- `DetectSystem` / `DetectGpu` → `cosmic::task::future`
- `LocalPreview` → generate **now**, then `Message::PreviewReady`
- `LocalProfilePreview` → write `profile_preview` **synchronously** in `intents_to_task` (not a follow-up message)
- `SavePrefs` / `ApplyTheme` / clipboard / open path or URL / toast / exit

`Intent::SendOnSession` and `Intent::CloseSession` are **no-ops** (`// Session chaining is task 12.`). Chaining lives inside `session.rs` for Apply only.

### 1.7 View

[`crates/gui/src/view/mod.rs`](../../crates/gui/src/view/mod.rs) dispatches on `app.page()`. One module per page. Shared chrome: status banner, settings column, scrollable. Catalogs cached in `OnceLock`. Widgets in `widget/` (bundle expander, code view, port chip, empty placeholder, banner).

Views do not scrape widgets in `update`. Live `on_input` matches GTK `connect_changed` (hostname, DNS, ports).

### 1.8 Helper session

[`crates/gui/src/helper/session.rs`](../../crates/gui/src/helper/session.rs):

- One helper **process per operation**.
- Apply `Switch`/`Boot`/`Test`/`Build`: `EnsureDirectories` → `Apply` → `WriteState` on success (`save` snapshot taken at click time).
- Dry-build: `Apply { DryBuild }` only (helper snapshots before creating directories).
- Missing helper → `SpawnFailed`, no pkexec.
- Timeouts: ReadState 300s, EnsureDirectories 10s, WriteState 30s, list/disk/maintenance 60s, **Apply unbounded**.

[`crates/gui/src/helper/spawn.rs`](../../crates/gui/src/helper/spawn.rs) `SpawnSpec` is the only spawn policy (C9). `HelperClient` is the only spawn implementation.

GUI **does not** send `CheckPermissions`, `GetSystemInfo`, `Validate`, or `Generate`. Preview is local. System info is `integration.rs`.

### 1.9 Persistence and integration

- Apply selections persist only via helper `WriteState` after a successful non-dry apply.
- If `state.json` is missing, the helper reconstructs from managed Nix (line-oriented, not a Nix parser).
- Integration: GUI and helper both call `common::config::detect_integration_status` on `/etc/nixos/configuration.nix` and `flake.nix`.
- Hostname/DNS Nix use `lib.mkDefault`.

---

## 2. Strengths

1. **Process isolation is real.** Unprivileged GUI, privileged helper, injectable `SpawnSpec`, Flatpak host probe without sandbox `Path::exists` on helper paths.
2. **Reducer is testable.** `AppModel::apply` + `Intent` lets tests assert apply/dry-run/rollback/maintenance without a display. Coverage in `apply.rs` is substantial (empty-apply dialogs, NVIDIA defaults, helper missing, busy gating).
3. **Nix generation has a single crate.** Preview and apply share `common::nix` (bundle stubs, dump order). That is the right place for “what files get written.”
4. **Apply chain is one process.** Session tests assert spawn count 1 and request types `EnsureDirectories`, `Apply`, `WriteState`. Dry-run does not write state.
5. **Busy flag prevents overlapping privileged ops** and does not cancel an in-flight apply on nav.
6. **Catalog-driven bundles.** Enabling a bundle fills `bundle_packages`; preview/apply use that map rather than silently copying templates.
7. **Flags for CI.** `skip_privileged_on_init` / `skip_host_probes` / unavailable spawn prevent pkexec hangs in smoke.
8. **Flat Message + exhaustive match** is a working test contract. Do not nest it for style.
9. **i18n is wired** (Fluent). Catalog names in `actions.rs` stay English on purpose.

Keep these. Do not rewrite the Application trait object, flatten crates, or split `Message` into page enums unless a new product requirement forces it.

---

## 3. Problems

### ARC-001 — Startup `ReadState` overwrites in-flight user edits

`init` sets `Busy::LoadingState` and spawns `ReadState` (up to 300s, including pkexec). The UI is fully interactive. `HelperResponse::State` does:

```rust
self.state = AppState::from_ipc_state(ipc);
self.sync_draft_inputs();
```

No dirty check. Edits made before the helper answers are discarded. `has_changes` is forced false.

`GpuDetected` (also started at init) can write `hardware_config.nvidia_driver = Some(0)` **without** `has_changes`. Ordering vs `ReadState` is racy: GPU-first then State wipes the default; State-first then GPU mutates loaded state as if the user chose Stable NVIDIA.

**Fix (small):** If `has_changes` or any draft input differs, ignore or merge `State` and keep a banner. Do not mutate `nvidia_driver` in `GpuDetected`; keep `hardware_for_nix()` as the only apply-time default.

### ARC-002 — Helper session buffers the whole process on the async executor

`session::stream` is:

```rust
stream::once(async move { run_blocking(spec, op, request) }).flat_map(stream::iter)
```

`run_blocking` runs **inside async**, not `spawn_blocking`. It collects every `HelperEvent` into a `Vec` and only then yields. Consequences:

- During `nixos-rebuild`, the apply log does not update until the helper exits, even though the helper streams `Log` lines immediately (`helper/src/rebuild.rs` `send_log`).
- A multi-minute rebuild occupies a Tokio worker. Other tasks (`DetectSystem`, clipboard, later helper ops after timeout) can stall.
- `HelperClient`’s reader thread is incremental; the session layer throws that away.

**Fix (material):** Drive the client from `spawn_blocking` or a dedicated thread and **yield events as they arrive** (`futures::channel` / iced stream). This is the highest-value helper-GUI change. Do not redesign IPC.

### ARC-003 — Stale async results have no generation

There is no request id / epoch on:

| Result | Risk |
|---|---|
| `PreviewReady` | `LocalPreview` snapshots Nix **in `intents_to_task`**, then posts a message. Any edit before `PreviewReady` is applied shows a **stale** preview on top of newer state. |
| `SystemDetected` | `VerifyIntegration` sets `verify_pending`. `RefreshSystem` (F5) also sends `DetectSystem`. Whichever probe finishes first consumes the flag and toasts “verified” for a refresh. |
| `GpuDetected` | Two probes (init + Hardware nav) last-write-wins (usually harmless). |
| `DiskUsageLoaded` / `GenerationsLoaded` | Gated by `busy`, so overlap is unlikely unless a timeout flips `busy` to Idle while a stream is still queued. Apply has no timeout, so a hung rebuild leaves the UI `Busy::Applying` forever with **no cancel**. |

`Intent::CloseSession` is a stub, so the user cannot abort a helper.

**Fix:** Monotonic `op_id` on helper/preview tasks; ignore mismatches. `CancelApply` while applying should kill the child (`HelperClient::kill` already exists). Separate `verify_pending` from generic `SystemDetected` or include a token.

### ARC-004 — Contradictory / duplicated bluetooth and NVIDIA state

- `AppState.bluetooth_enabled` **and** `hardware_config.bluetooth_enabled`. Setters try to keep them aligned; `from_ipc_state` prefers hardware on diverge; `effective_hardware()` ORs them. Three functions exist because the field is duplicated. IPC still has both for old `state.json`.
- NVIDIA: widget shows `unwrap_or(0)` when GPU is NVIDIA; `GpuDetected` writes `Some(0)` into state; `hardware_for_nix()` also defaults `None` → `0` at apply/preview. Three sources. `has_changes` is not set by GPU detect, but `apply_is_empty()` becomes false once state is mutated — dirty flag and “empty apply” disagree.

**Fix:** Persist bluetooth only on `HardwareConfig`. Keep a serde alias on IPC if old files must load. Default NVIDIA only in `hardware_for_nix()` / the view, not in `GpuDetected`.

### ARC-005 — `helper_missing` is sticky; spawn is not re-probed

`SpawnFailed` with “not available” / “not found” sets `helper_missing = true`. `helper_can_spawn` is then permanently false for the process. F5 does not re-run `SpawnSpec::from_env()`. Installing the helper (or fixing PATH) requires an app restart. Apply/Dry Run buttons stay disabled (`view/apply.rs` `buttons_enabled`).

**Fix:** On `RefreshSystem`, re-probe spawn and clear the flag if the helper is now executable.

### ARC-006 — Draft vs committed hostname/username coupling

Hostname widget binds to `state.hostname.or(system_info.hostname)`, not a draft string. Invalid charset leaves state unchanged (good) but iced will snap the field back. Typing the **current system** hostname stores `None` (intentional mkDefault behaviour) so the box shows the system name and `has_changes` may still be true from the previous `set_hostname`.

Username: empty `UsernameChanged` sets `None`; the view then shows `std::env::var("USER")` every frame. `SystemDetected` → `seed_username_from_host` fills `USER` again after the user cleared the field.

DNS/ports have real draft fields; hostname/username do not. Inconsistent and easy to get wrong.

### ARC-007 — Dead compatibility layers and unfinished iced wiring

| Item | Status |
|---|---|
| `Intent::SendOnSession`, `Intent::CloseSession` | No-ops; apply chain is in `session.rs` |
| `Message::UpdateConfig` | Handled, **never emitted** (no cosmic-config `subscription`) |
| `ClipboardCopied { ok: false }` | Clipboard task always maps success |
| `HelperOp::{CheckPermissions,GetSystemInfo,Validate,Generate}` | GUI never constructs; helper still implements |
| `AppState::to_generate_request` | Tests only; GUI does not send `Generate` |
| `AppState::parse_and_add_tcp_ports` | Tests only; UI uses replace-custom |
| `AppState::summary`, `toggle_bundle` | Tests / unused by reducer (`ToggleBundle` uses enable/disable) |
| `FieldErrors.packages` | Never set or read |
| `integration::can_write_managed_dir` | Unused; uses sandbox `Path` (wrong in Flatpak) |
| `thiserror` in `gui/Cargo.toml` | Unused |
| `common::nix::generate_selected_nix` / `generate_preview` | “legacy signature for compatibility” |
| GTK comments | `core/packages.rs`, session WriteState, view onboarding height |

`HelperClient` is a reasonable low-level type; it is not unused. The **session** is the GUI-facing API. Do not add a second spawn path.

Migration docs (`docs/migration/architecture.md`) still describe `spawn_privileged()`, nested toasts, `EnableBundleAll`, cosmic-config subscription, and `Intent::SpawnHelper` refused when `helper != Idle`. Code uses `Busy`, not `HelperStatus`, as the mutex.

### ARC-008 — Giant modules, god `AppModel`, stringly services

- `core/apply.rs` is ~2k lines (about half tests). The match is one function. Fine for a freeze; painful to extend.
- `AppModel` holds chrome, catalogs-adjacent drafts, helper session, logs, generations, disk, field errors, verify flag.
- `set_service` / `service_enabled` / Fluent maps are parallel `&str` matches (21 ids). Unknown id still sets `has_changes` (`state.rs`).
- `ToggleBundle` calls `default_bundles()` every time; views use `OnceLock`. Wasteful, not wrong.
- `NetworkConfig::has_settings` ignores listen-port-only changes when WireGuard is off (probably intended). Disabling WireGuard does **not** remove the auto-opened UDP port (commented on purpose; leftover holes).

These are maintainability issues. They do not require a framework rewrite.

### ARC-009 — Error handling and logging gaps

- Invalid custom TCP/UDP: `tracing::debug` only; no `field_errors` (DNS/hostname do show errors).
- Helper `Timeout` for ReadState sets a banner; for Apply, timeout is `None` so it never fires.
- `HelperEvent::Closed` on ReadState always uses the **timeout** banner copy (`banner-state-timeout`), even on clean EOF.
- `DismissToast` **replaces** the entire `Toasts` widget, dropping other toasts.
- Open path/URL failures are `tracing::warn` plus `Action::None` — no toast (C11 asked for a toast on failure).
- `last_applied` on IPC state is always `None` from the GUI (`to_ipc_state`).

### ARC-010 — Model/view leakage (acceptable, but keep it from growing)

Views clone `apply_preview` / logs into `code_view`. Username view reads `USER` itself. Hardware view duplicates NVIDIA “Stable” default. Bundle page recomputes selected package ids for a summary.

Not a correctness bug. Page functions taking `&AppModel` make isolated snapshot tests harder; reducer tests already cover behaviour.

---

## 4. Fragile coupling / sources of truth (summary)

```
User widgets ──► Message ──► apply() ──► AppState (intended SoT)
                                      ├─ drafts (dns/ports/packages)
                                      ├─ hardware_for_nix() (NVIDIA default overlay)
                                      └─ bluetooth sibling OR

ReadState ──► replaces AppState          (ARC-001)
GpuDetected ──► mutates hardware_config  (ARC-001 / ARC-004)
PreviewReady ──► apply_preview           (ARC-003, may be stale)
Busy vs HelperStatus                     two locks; Busy is the one that matters
helper_missing vs spawn.helper_available  sticky vs Flags (ARC-005)
```

IPC `AppState` vs GUI `AppState` is a justified dual: persistence vs UI sets/`has_changes`/`expanded_bundles`. The unjustified dual is bluetooth.

---

## 5. Proposed refactors (only if they pay)

Do **not**: nest `Message`, introduce a general actor framework, merge GUI and helper, or “clean up” working SpawnSpec/Flatpak probes for style.

Do, in roughly this order:

1. **Stream helper events; stop blocking async** (ARC-002). Yield `Log` lines during rebuild. Use `spawn_blocking` or a thread. This is correctness + UX + performance.
2. **Protect dirty state from `ReadState`** (ARC-001). Ignore or merge if `has_changes`. Stop writing NVIDIA into state from `GpuDetected`.
3. **Operation ids + cancel** (ARC-003). Ignore stale `PreviewReady`. Kill helper on explicit cancel. Unstick `Busy::Applying` if the child dies.
4. **Delete stubs** (ARC-007): `SendOnSession`/`CloseSession`, unused `FieldErrors.packages`, `can_write_managed_dir`, unused `thiserror`. Either wire `UpdateConfig` via cosmic-config subscription or drop the variant (that **does** touch the frozen enum — prefer wiring if COSMIC users exist; otherwise leave the variant and document it as unused).
5. **Collapse bluetooth to `HardwareConfig`** (ARC-004) with serde default/alias. Keep IPC `bluetooth_enabled` until reconstruct/old JSON is gone, but stop mirroring in GUI logic.
6. **Split `apply.rs` by domain** (`apply_chrome`, `apply_network`, `apply_helper`) without changing `Message`. Move the giant `#[cfg(test)]` module to `crates/gui/src/core/apply_tests.rs` or keep tests next to the reducer — either is fine.
7. **Re-probe helper on refresh** (ARC-005).
8. **Service id enum** in `common` used by view + `set_service`. Only if you are already touching services; not a drive-by.

Optional, low priority: hostname/username draft fields like DNS; surface port parse errors; toast on `xdg-open` failure; stop calling `default_bundles()` in the reducer.

---

## 6. Testability notes

Already good: `Flags::for_tests()`, `AppModel::test_model()`, session tests with a Python fake helper, spawn `from_parts` tests, package parser tests, `state_mutations.rs`.

Missing / weak:

- No test that `ReadState` after local edits preserves `has_changes`.
- No test that `PreviewReady` after a newer edit is ignored (because it is not).
- No test that helper `Log` lines appear before `ApplyComplete` (they cannot, today).
- `UpdateConfig` is only in the exhaustive match sample.

---

## 7. Verdict

The iced/libcosmic shape is **sound and mostly idiomatic**: flat messages, Intent-returning reducer, views as functions of `AppModel`, helper behind `SpawnSpec`. The GTK rewrite left a few unfinished rails (session Intents, config subscription) and two real races (ReadState vs edits, buffered helper I/O). Fix those; do not rebuild the application architecture.

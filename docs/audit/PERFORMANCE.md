# Performance and resource audit

Reconnaissance only (branch `audit-hardening`). No fixes were implemented.

Product: NixOS Toolkit (libcosmic/iced GUI + privileged helper). Scope: startup, UI-thread work, redraws, helper IPC, preview generation, caches, idle resource use, Flatpak.

## Method

- Read `crates/gui/src/app.rs`, `core/apply.rs`, `helper/*`, `view/*`, `widget/*`, `crates/common/src/nix.rs`, `crates/helper`.
- Traced `init` → `startup_intents` → `subscription` → `update`/`view` → helper session → rebuild logs.
- This pass had no interactive GUI timing harness (`cargo run -p gui` would open a window and block). Timings below are from code-path analysis plus well-known costs of the called tools (`du -sh /nix/store`, `pkexec`, `flatpak-spawn`, wgpu). Re-measure on a NixOS host before treating any number as a SLO.

Healthy patterns (not issues):

- No timer/polling subscription. The only `Application::subscription` is a keyboard listener (`F5` / Ctrl+R / Ctrl+Q). Idle CPU from app-owned subscriptions should be ~0.
- No GUI network I/O. No image decode of user content (`APP_ICON` is `include_bytes!`; list icons are named symbolic icons).
- Catalog sizes are small (13 profiles, 16 bundles, ~75 packages, 21 service toggles, 11 pages). Allocating them is not a user-visible problem except where the catalog is rebuilt per lookup (see P8).
- View catalogs are cached in `OnceLock` (`crates/gui/src/view/mod.rs`).
- `busy` gates overlapping privileged ops. Helper client parses JSON on a dedicated reader thread.
- Helper-side `nixos-rebuild` already streams logs line-by-line to stdout.

## Hottest issues

| ID | Issue | Impact |
|----|--------|--------|
| P1 | Helper session buffers the whole op, then dumps every log as its own `Message` | Apply/maintenance logs appear only after the process ends; then the UI can hitch on thousands of updates |
| P2 | Maintenance page runs `du -sh /nix/store` on every visit | Seconds to minutes of disk I/O; helper (and therefore the page) stay busy |
| P3 | Startup does host probes twice, plus prefs twice, plus a privileged `ReadState` | Slow first paint; Flatpak multiplies this with process spawns |
| P4 | Nix preview is generated on the UI thread when opening Apply | Main-thread hitch proportional to enabled bundles/templates |
| P5 | Unbounded logs cloned into widgets every frame; nested scrollables | Growing memory; layout/clone cost after long rebuilds |
| P6 | One `pkexec` process per helper op (session chaining stubbed) | Auth + process startup on every Generations/Maintenance/Apply action |

---

### P1 — Helper IPC is not a live stream (blocking collect + message storm)

**Issue.** The GUI does not consume helper stdout as an async stream. `session::stream` runs the entire privileged operation to completion, collects every `HelperEvent` in a `Vec`, then yields them.

**Evidence.**

```14:20:crates/gui/src/helper/session.rs
pub fn stream(
    spec: SpawnSpec,
    op: HelperOp,
    request: HelperRequest,
) -> impl Stream<Item = HelperEvent> + Send {
    stream::once(async move { run_blocking(spec, op, request) }).flat_map(stream::iter)
}
```

`run_blocking` sits in an `async` block without `spawn_blocking`. libcosmic is built with the `tokio` feature (`crates/gui/Cargo.toml`), so this occupies a Tokio worker for the whole `nixos-rebuild` (Apply has **no** recv timeout: `timeout_for(HelperOp::Apply)` is `None` in `session.rs`).

The helper itself *does* stream:

```103:107:crates/helper/src/rebuild.rs
    while let Ok((level, message)) = rx.recv() {
        send_log(level, message);
    }
```

Those lines are read on `HelperClient`’s background thread (`crates/gui/src/helper/client.rs`), then parked in `events` until `run_blocking` returns. Each `HelperResponse::Log` becomes one `Message::Helper`. The reducer appends one line:

```677:694:crates/gui/src/core/apply.rs
            HelperResponse::Log { message, .. } => {
                match op {
                    HelperOp::RunMaintenance { .. } => { /* push_str */ }
                    HelperOp::Rollback { .. }
                    | HelperOp::DeleteGenerations { .. }
                    | HelperOp::ListGenerations => { /* push_str */ }
                    _ => {
                        self.apply_log.push_str(&message);
                        self.apply_log.push('\n');
                    }
                }
                Vec::new()
            }
```

Iced then `update`s and typically `view`s per message. Combined with P5 (clone of the whole log in `view`), a rebuild that emits thousands of nix lines can stall the UI *after* the build finishes.

`Intent::SendOnSession` / `CloseSession` are explicitly unimplemented (`app.rs` “Session chaining is task 12”), so there is also no cancellation of an in-flight op other than `HelperClient::Drop` killing the child when the blocking function returns.

**Impact.** During Apply/DryRun the spinner can move, but the log pane stays empty until the helper exits. After exit, a burst of log messages can freeze the Apply page. Users cannot see progress or errors live. A blocked Tokio worker can delay other tasks (`DetectSystem`, clipboard, theme).

**Proposed fix (do not implement here).**

1. Turn `HelperClient` into a real stream: `mpsc` → `Stream` that yields `HelperEvent` as lines arrive; run `run_blocking` / stdin writes on `spawn_blocking` or a dedicated thread.
2. Coalesce logs (batch ~50ms or N lines) into one `Message` so a rebuild is tens of updates, not thousands.
3. Optional: cap/ring-buffer the visible log (P5).

**Before/after.** Before: 0 log widgets updates during a 2-minute rebuild, then O(lines) updates. After: ~1–10 Hz log updates during the rebuild, bounded message rate.

---

### P2 — `du -sh /nix/store` on every Maintenance navigation

**Issue.** Opening the Maintenance page always spawns the helper (`GetDiskUsage`) which walks the entire Nix store with `du`.

**Evidence.**

```779:786:crates/gui/src/core/apply.rs
    fn nav_intents(&mut self, page: Page) -> Vec<Intent> {
        self.page = page;
        let mut intents = vec![Intent::SetWindowTitle(self.window_title())];
        match page {
            Page::Apply => intents.push(Intent::LocalPreview),
            Page::Generations => intents.extend(self.load_generations_intents()),
            Page::Maintenance => intents.extend(self.load_disk_usage_intents()),
```

```1457:1463:crates/helper/src/commands.rs
    match Command::new("du").args(["-sh", "/nix/store"]).output() {
        Ok(output) if output.status.success() => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            if let Some(size) = stdout.split_whitespace().next() {
                store_size = size.to_string();
            }
```

`du -sh /nix/store` is a full tree walk. On a developed NixOS machine the store is tens to hundreds of gigabytes and millions of inodes. The helper also re-runs `nix-env --list-generations` here even though the Generations page already has that command.

Because of P1, the GUI will not even show “calculating” progress from helper logs (this command has no logs); `Busy::LoadingDisk` is set, then the UI waits until `du` finishes.

**Impact.** Navigating to Maintenance can take a long time, burn disk bandwidth, and block other helper ops (`busy != Idle`). Repeating the visit (or clicking Refresh) repeats the walk. No cache, no `nix path-info --json`, no `df` on the store filesystem.

**Proposed fix.** Prefer `statvfs`/`df -h /nix/store` for “how full is the volume”, or `nix path-info -Sh /nix/store` / `nix store diff-closures` only on demand. Cache the last result for the session. Do not start `du` automatically on nav; use an explicit “Calculate store size” button. Drop the extra `nix-env` count or reuse `ListGenerations`.

**Before/after.** Before: every Maintenance visit = full store walk. After: nav is a cheap `df` or a cached value; expensive measure is opt-in.

---

### P3 — Duplicate, blocking work before and just after first frame

**Issue.** First paint waits on host probes that are then repeated asynchronously. Preferences are loaded twice. Flatpak turns each probe into `flatpak-spawn --host`.

**Evidence.**

`main` loads prefs to pick the window theme, then `init` loads them again:

```22:35:crates/gui/src/main.rs
    let flags = Flags::from_env();
    let prefs = UserPreferences::load(flags.prefs_path.as_deref());
    let settings = cosmic::app::Settings::default()
        ...
        .theme(prefs.color_scheme.to_theme());
```

```318:324:crates/gui/src/app.rs
        let prefs = UserPreferences::load(flags.prefs_path.as_deref());
        ...
        let system_info = if flags.skip_host_probes {
            SystemInfo::default()
        } else {
            detect_system()
        };
```

`detect_system()` is synchronous on the thread that constructs `AppModel` (before the first `view`). `startup_intents` then queues **the same** `Intent::DetectSystem` plus `DetectGpu` plus `ReadState`:

```232:247:crates/gui/src/app.rs
        let mut intents = vec![
            Intent::SetWindowTitle(self.window_title()),
            Intent::ApplyTheme,
            Intent::DetectSystem,
            Intent::DetectGpu,
        ];
        if !self.flags.skip_privileged_on_init && self.flags.spawn.helper_available {
            self.busy = Busy::LoadingState;
            intents.push(Intent::SpawnHelper {
                op: HelperOp::ReadState,
                request: HelperRequest::ReadState,
            });
        }
```

`Intent::DetectSystem` runs `detect_system()` inside `cosmic::task::future(async move { ... })` with no `spawn_blocking` (`app.rs`).

`detect_system` itself duplicates filesystem checks (`detect_config_mode` vs `find_config_path`) and, in Flatpak, one process per call:

```12:21:crates/gui/src/integration.rs
pub fn detect_system() -> SystemInfo {
    SystemInfo {
        is_nixos: is_nixos(),
        nixos_version: get_nixos_version(),
        config_mode: detect_config_mode(),
        config_path: find_config_path(),
        integration_status: detect_integration(),
        hostname: get_hostname(),
        current_desktop: get_current_desktop(),
    }
}
```

Typical host syscalls (native): `exists(/etc/NIXOS)`, `read(/etc/os-release)`, `exists(flake.nix)` and/or `configuration.nix` **twice**, `read` both config files, `read(/etc/hostname)` and maybe `hostname(1)`. In Flatpak each `host_path_exists` / `host_read_to_string` / `host_command_output` is `flatpak-spawn --host` (`integration.rs`). That is on the order of **8 sequential spawns per `detect_system`**, done twice at startup, plus `lspci` for GPU, plus `probe_flatpak_helper` (`test -x` per candidate in `spawn.rs`).

`Flags::from_env()` also walks `PATH` / system helper paths before `run` (`spawn.rs`).

`ReadState` is a full `pkexec` helper process (P6) on every launch.

Wgpu/libcosmic window init (enabled in `Cargo.toml`: `winit`, `wayland`, `x11`, `wgpu`) dominates cold start for this stack; that is expected, not an app bug. The duplicate probes are extra on top.

**Impact.** Delayed first frame on native; much worse in Flatpak. Integration banner can flicker (sync result, then async overwrite). Polkit prompt can appear before the window is useful.

**Proposed fix.** Do not call `detect_system()` in `init`; start with `SystemInfo::default()` and one background probe (thread/`spawn_blocking`). Deduplicate `find_config_path` with `detect_config_mode`. In Flatpak, batch host reads (`sh -c` one script, or a single helper query). Load prefs once and pass them in `Flags`. Defer `ReadState` until after first frame (or until Apply/needs-state). Cache `detect_gpu` (already skipped on Hardware if `gpu_vendor` is set — keep that, drop the duplicate if startup already ran it).

**Before/after.** Before: 2× `detect_system` + 2× prefs + wgpu + pkexec. After: 1 prefs load, 1 background probe, first frame without `/etc` I/O.

---

### P4 — Local Nix preview generated on the UI thread

**Issue.** Navigating to Apply (and Refresh Preview) builds the full preview string inside `update` / `intents_to_task`, then posts `PreviewReady`.

**Evidence.**

```783:783:crates/gui/src/core/apply.rs
            Page::Apply => intents.push(Intent::LocalPreview),
```

```620:623:crates/gui/src/app.rs
                Intent::LocalPreview => {
                    let preview = generate_local_preview(self);
                    tasks.push(cosmic::task::message(Message::PreviewReady(preview)));
                }
```

```684:690:crates/gui/src/app.rs
fn generate_local_preview(app: &AppModel) -> String {
    let profiles = default_profiles();
    let bundles = default_bundles();
    let mut options = app.state.to_nix_gen_options(&profiles, &bundles);
    options.hardware_config = app.hardware_for_nix();
    common::nix::generate_preview_full_from(&options, &app.flags.templates_dir)
}
```

`generate_preview_full_from` concatenates every managed snippet and **reads profile/bundle templates from disk** (`crates/common/src/nix.rs`). `to_nix_gen_options` clones bundle maps, DNS, groups, packages, and network/services/hardware (`state.rs`). Preview also calls `default_profiles()` / `default_bundles()` again instead of the view `OnceLock` catalogs.

Profile preview is also sync on the UI thread (`Intent::LocalProfilePreview` assigns `self.profile_preview` in `intents_to_task`) and re-walks `default_profiles()`.

The Apply **view** then clones the whole preview string into `code_view` every redraw (P5).

**Impact.** Opening Apply with several bundles hitching the input thread (disk reads + large `String` build). Not catastrophic at current catalog size, but it is the only “compute Nix” path that is not off-thread, and it runs on every visit to Apply (no dirty flag / cache).

**Proposed fix.** Cache preview keyed by a cheap state fingerprint; regenerate only when `has_changes` or inputs change. Run generation on `spawn_blocking` with a generation counter so a stale `PreviewReady` cannot overwrite a newer one. Reuse `catalog_profiles()` / `catalog_bundles()`. Pass `&str` into `code_view` (it already takes `Cow`).

**Before/after.** Before: every Apply nav = full generate + template reads on UI thread. After: cache hit is a no-op; miss is off-thread.

---

### P5 — Unbounded logs, clone-per-frame, nested scrollables

**Issue.** `apply_log`, `generations_log`, and `maintenance_log` only grow (`push_str`). Views clone them (and the preview) on every `view()`. The window is a `scrollable` wrapping pages that embed another `scrollable` `code_view`.

**Evidence.**

```69:77:crates/gui/src/view/apply.rs
    let preview = if app.apply_preview.is_empty() {
        crate::fl!("apply-preview-placeholder")
    } else {
        app.apply_preview.clone()
    };
    let log = if app.apply_log.is_empty() {
        crate::fl!("apply-log-placeholder")
    } else {
        app.apply_log.clone()
    };
```

Same pattern: `view/profiles.rs` (`profile_preview.clone()`), `view/generations.rs` (`generations_log().to_owned()`), `view/maintenance.rs` (`maintenance_log().to_owned()`), `view/onboarding.rs` (`classic_integration_snippet()` / `flake_integration_snippet()` allocate a new `String` every frame).

```43:48:crates/gui/src/view/mod.rs
    settings::view_column(children)
        .padding(spacing.space_m)
        ...
        .apply(widget::scrollable)
```

```20:25:crates/gui/src/widget/code_view.rs
    container(text)
        ...
        .apply(widget::scrollable)
        .height(Length::Fixed(height))
```

`nixos-rebuild` output can be megabytes. P1 then clones that growing buffer once per log line during the post-op burst.

`code_view` uses `selectable_text::monotext` for the entire buffer (no virtualization). Large logs are laid out as one widget.

**Impact.** Memory grows for the process lifetime. After a noisy rebuild, Apply/Generations/Maintenance `view()` copies and lays out a large rope of text. Nested scrollables also fight pointer/wheel events (iced layout cost + UX).

**Proposed fix.** Keep logs in a ring buffer (e.g. last 2000 lines / 256 KiB). Pass `&str` / `Cow::Borrowed` into `code_view`. Consider a truncated preview in the widget (full text still copyable). Avoid wrapping the whole page in `scrollable` when the page already has internal scroll regions, or use a column with a non-scrollable chrome + one scroll body.

**Before/after.** Before: log size unbounded; clone + full layout per frame. After: bounded buffer; borrow in `view`.

---

### P6 — New privileged process for every helper operation

**Issue.** Every `Intent::SpawnHelper` starts a new process (`pkexec` / `flatpak-spawn --host -- pkexec`). Session reuse is stubbed.

**Evidence.**

```597:602:crates/gui/src/app.rs
                Intent::SpawnHelper { op, request } => {
                    tasks.push(spawn_helper_task(self.flags.spawn.clone(), op, request));
                }
                Intent::SendOnSession { .. } | Intent::CloseSession => {
                    // Session chaining is task 12.
                }
```

`HelperClient::spawn` always `Command::spawn`s (`client.rs`). `Drop` kills the child.

Nav to Generations → `ListGenerations`. Nav to Maintenance → `GetDiskUsage`. Apply → EnsureDirectories + Apply + WriteState on **one** process (good for the apply chain), but that is still a separate pkexec from ReadState at startup.

Polkit typically caches credentials for a short window; the process and JSON handshake cost remains.

**Impact.** Extra 100s of ms to seconds per action (pkexec + helper rust startup + tracing init). Flatpak adds another hop. Users may see repeated auth dialogs if the polkit timeout elapsed.

**Proposed fix.** Keep a long-lived helper session after the first successful auth (the unimplemented `SendOnSession` path). Idle-timeout and kill on quit. Still one process for Apply chain (already correct).

**Before/after.** Before: 1 pkexec per nav/action. After: 1 pkexec per GUI session (or per auth TTL).

---

## Additional real issues (lower than P1–P6)

### P7 — Generations listing does N+1 filesystem reads, every visit

**Evidence.** `nav_intents(Page::Generations)` → `ListGenerations`. `list_generations` parses `nix-env --list-generations`, then for **each** generation reads `system-N-link/nixos-version` and `read_link(kernel)` (`commands.rs` `generation_metadata`). Tens to hundreds of generations → hundreds of syscalls, plus a pkexec (P6), plus P1 buffering.

**Proposed fix.** Cache generations in the GUI until Refresh. Parse versions lazily (current + visible rows). `nix-env` once; don’t also count generations in `get_disk_usage`.

### P8 — `resolve_nix_attr` rebuilds the whole catalog per package

**Evidence.**

```63:70:crates/common/src/nix.rs
pub fn resolve_nix_attr(pkg_id: &str) -> String {
    for bundle in default_bundles() {
        if let Some(pkg) = bundle.packages.iter().find(|p| p.id == pkg_id) {
            return pkg.resolved_nix_attr().to_string();
        }
    }
    pkg_id.to_string()
}
```

Called from `nix_attrs_for_package_ids` and `needs_allow_unfree` (which also clones `get_bundle_packages` per bundle). Preview/apply with many packages is O(packages × catalog) **allocations**, not just lookups. Same anti-pattern: `ToggleBundle` / `classify_new_packages` call `default_bundles()` instead of `catalog_bundles()` (`apply.rs`).

At ~75 packages this is likely sub-millisecond, but it is needless work on the preview/apply path (P4). A static `OnceLock<HashMap>` is the fix.

### P9 — `atomic_write` writes, then re-reads the file to verify

**Evidence.** `crates/helper/src/nix_gen.rs` `atomic_write`: `fs::write` temp → `rename` → `fs::read_to_string` and compare. Extra read per managed file on every apply. Fine for small snippets; not needed for integrity on local disk after `rename`.

**Proposed fix.** Drop the read-back, or verify length only. Keep the jail prefix check.

### P10 — Dry-run copies the entire managed tree twice

**Evidence.** `snapshot_managed_dir` / `copy_dir_all` / `restore` in `commands.rs`. Required for correctness, but it is recursive `fs::copy` of `/etc/nixos/nixos-toolkit`. Size is usually small; only a problem if that tree ever grows (copied templates, leftover files). Combined with P1 the GUI still waits.

Not a bug; just don’t add large artifacts under the managed dir.

### P11 — `DismissToast` rebuilds the toast host

**Evidence.** `app.rs` `update`: every `Message::DismissToast` does `self.toasts = widget::Toasts::new(...)`. Extra allocation vs `toasts.remove`. Minor.

### P12 — Input-driven full-page `view` (iced default, watch the expensive pages)

Every keystroke (`PackageInputChanged`, `HostnameChanged`, `DnsServersChanged`, port fields) rebuilds the whole page. That is normal for iced. It becomes a problem only where `view` clones large strings (Apply/logs) or sorts HashSets (`view/packages.rs` clones + sorts `custom_packages` every frame). Keep packages list stable (sorted cache) if custom package counts grow.

Bundles page rebuilds expander bodies and a `BTreeSet` of all selected package ids every frame (`view/bundles.rs`). Catalog is small; not hot today.

### P13 — Onboarding snippet rebuilt every frame

`classic_integration_snippet()` / `flake_integration_snippet()` return owned multi-line `String`s (`integration.rs`). `onboarding::view` calls them every `view()`. Cheap strings, but easy `const` / `&'static str`.

---

## What was checked and is not a problem

| Area | Finding |
|------|---------|
| Idle CPU / timers | No `every`/`interval` subscription. Keyboard `listen_with` only. |
| Duplicate subscriptions | Single subscription; not keyed on changing IDs in a way that would leak. |
| Stale async (today) | Preview is generated synchronously in `update`, so `PreviewReady` cannot overtake a newer preview. **If P4 is moved off-thread, add a generation token** or this becomes a real bug. Helper results are ignored in the wrong `busy` state only loosely (Log still appends). |
| Unbounded caches | No in-memory package/search cache. Logs are the unbounded structure (P5). |
| Search/filter | No search box over large lists. |
| Serde | JSON line protocol; Apply payload is the user’s selection, not a dump of nixpkgs. `HelperResponse` is boxed in events. |
| Config writes | Prefs save is on theme change only, on the UI thread, tiny JSON. Helper `WriteState` is pretty-printed JSON once after apply. |
| Images | Compile-time SVG + symbolic icons. |
| `CpuArch::detect` | Compile-time `cfg`, not `/proc`. |
| Maintenance allowlist | Commands are few; cost is the user’s `nix`/`gc`, not the GUI. |
| `Message` traffic at rest | None beyond iced internals. |

---

## Flatpak startup (specific)

Ordered cost on a sandboxed launch:

1. Wgpu/libcosmic (expected).
2. `Flags::from_env`: several `flatpak-spawn --host -- test -x` helper probes (`spawn.rs` `probe_flatpak_helper`).
3. Sync `detect_system` (many `flatpak-spawn` cat/test).
4. Async `detect_system` again + `lspci` via `flatpak-spawn`.
5. `pkexec` helper for `ReadState` (`flatpak-spawn --host --forward-fd … pkexec`).

Batching host I/O (P3) is the highest Flatpak-specific win after P1/P2.

---

## Suggested measurement plan (next pass)

Run on a NixOS host, once native and once Flatpak:

```bash
# First frame / helper
NIXOS_TOOLKIT_SKIP_PRIVILEGED_INIT=1 NIXOS_TOOLKIT_SKIP_HOST_PROBES=1 time cargo run -p gui
# vs default env (expect pkexec + detect_system)

# Isolated probes
time test -e /etc/NIXOS; time cat /etc/os-release /etc/nixos/configuration.nix /etc/nixos/flake.nix >/dev/null
time lspci >/dev/null
time du -sh /nix/store          # expect this to dominate Maintenance
time nix-env --list-generations -p /nix/var/nix/profiles/system

# Preview microbench (add a #[bench] or a one-off test around generate_preview_full_from)
```

Instrument with `tracing` spans around `detect_system`, `generate_local_preview`, `HelperClient::spawn`, and `run_blocking`. Count `Message::Helper` during a dry-run.

---

## Priority if a fix pass is scheduled

1. **P1** live helper stream + log coalescing (correctness of UX + avoids post-rebuild freeze).
2. **P2** stop walking `/nix/store` on nav.
3. **P3** single background `detect_system`; no duplicate prefs; Flatpak batching.
4. **P6** long-lived helper session (unblocks snappy Generations/Maintenance).
5. **P5** bounded logs + borrow in `view`.
6. **P4** cached/off-thread preview (small until catalogs grow; cheap to do with P5).
)
# Task log

Short plans and DA notes for Phase 2. Newest first.

## Phase 3 DA

**CLEAN** `a45894a` `bf3c05b` `b60213f` `3b4112a` on `cosmic-migration`.

Re-checked the six Phase 3 blockers in current `crates/gui` (HEAD `b60213f`). They are closed in source. Spot-check: `rg pkexec crates/gui` is `helper/spawn.rs` only; `rg nix-env crates/gui` empty. `cargo test -p gui --offline --lib --` the 14 named tests below **14 passed**. Did not re-run `scripts/verify.sh` or Flatpak/weston. Production skip envs are still smoke-only.

### Pass (the six)

1. **`detect_system` / `detect_gpu` host-spawn in Flatpak.** `integration.rs`: `host_path_exists` / `host_read_to_string` / `host_command_output` / `lspci_command` all `flatpak-spawn --host` when `in_flatpak()`. `is_nixos` is host `test -e /etc/NIXOS`; hostname is host `cat /etc/hostname` then host `hostname`; GPU is host `lspci`. Host `os-release` is read via spawn for `VERSION_ID` only (comment: do not bind-mount). `init` still calls `detect_system()` when `!skip_host_probes` so `refresh_banner` is honest before the async task. Tests: `host_path_helpers_use_local_fs_outside_flatpak`, `gpu_from_lspci_prefers_nvidia_on_hybrid`.

2. **`init` emits `DetectGpu`.** `AppModel::startup_intents` queues `DetectSystem` and `DetectGpu` (no helper). `intents_to_task` runs `integration::detect_gpu()` unless `skip_host_probes`. `RefreshSystem` emits both. First `NavSelect(Hardware)` emits `DetectGpu` while `gpu_vendor` is `None`. Tests: `init_queues_detect_system_and_gpu_without_helper`, `refresh_system_detects_system_and_gpu_without_helper`, `nav_hardware_queues_detect_gpu_once`, `gpu_detected_populates_vendor`.

3. **Username seeded from `$USER`.** `init` and `SystemDetected` / helper `State` call `seed_username_from_host`. Direct assign (not `set_username`) so seed does not dirty `has_changes`. `default_username` takes trimmed `$USER` when `!skip_host_probes` and charset `[A-Za-z0-9_-]`. Tests: `default_username_from_user_env`, `skip_host_probes_does_not_seed_username`. Groups + seeded name now reach `users.nix` (`username.is_some() && !user_groups.is_empty()`).

4. **Hostname charset rejected.** `HostnameChanged` trims; empty clears; invalid `[A-Za-z0-9-]` / length > 63 sets `field_errors.hostname` and does **not** `set_hostname`; match vs `system_info.hostname` is a no-op. View shows the error. Test: `hostname_rejects_invalid_charset_and_matches_system_noop` (`nixo_`, `bad.host` stay at last valid).

5. **`helper_missing` does not `SpawnHelper` on nav.** `helper_can_spawn()` is `helper_available && !helper_missing`. `nav_intents` → `load_generations_intents` / `load_disk_usage_intents`, plus `LoadGenerations` / `LoadDiskUsage` / `start_rebuild` / rollback / delete / maintenance, all return `Vec::new()` when it is false. `startup_intents` still skips ReadState when the helper is unavailable. Test: `helper_missing_does_not_spawn_on_nav_or_refresh_loaders` (nav Generations/Maintenance, loaders, Apply/Dry Run). Fail-fast `spawn_helper_task` remains.

6. **DNS/TCP raw buffers; TCP replace not merge.** Model has `dns_input` / `custom_tcp_input`. System DNS and Network custom TCP bind those strings (`view/system.rs`, `view/network.rs`). `DnsServersChanged` always stores the raw field; `parse_and_set_dns` only updates `state.dns_servers` on full IPv4s (`"1.1.1.1, 8"` keeps `"1.1.1.1"` + error, widget does not snap). `CustomTcpPortsChanged` always stores raw; `parse_and_set_custom_tcp_ports` **replaces** extras and keeps preset chips (`"9"`→`"90"`→`"9090"` → `[22, 9090]`, not 9+90+909+9090). Tests: `dns_partial_keeps_input_and_does_not_clobber_servers`, `custom_tcp_replace_does_not_accumulate_prefixes`, `custom_tcp_replace_keeps_presets_and_drops_prefixes`.

### Non-blocking (do not re-open 1–6)

- `GpuDetected` still only stores `gpu_vendor`. `hardware_for_nix()` patches NVIDIA `Some(0)` for Apply/preview; `apply_is_empty()` / WriteState snapshot do not. NVIDIA-only box can still get the destructive empty dialog while Nix emits Stable. GPU detect itself runs.
- Generations / Maintenance **Refresh** still attach `on_press` when `idle` (not `buttons_enabled`). Reducer no-ops, so no spawn and no helper-missing toast.
- `UsernameChanged` does not re-check charset (seed does).
- Custom TCP still parses on every keystroke; a prefix like `9` is live in state until the next digit (replace, not union).
- Manifest still lists `/etc/NIXOS:ro` binds Flatpak refuses — unused now that host-spawn exists.
- `parse_and_add_tcp_ports` merge helper remains for tests only.

Phase 3 REPORT may close on these six. Prior writeup kept below.

## Phase 3 DA (pre a45894a)

**FAILURES:** remaining flow breaks that should block `REPORT.md` (verify.sh 0 / smoke 124 / no pkexec do **not** cover these). Named greps that are clean are listed under Pass.

Walked `PLAN.md` parity ticks, `DECISIONS.md` C3/C9–C11/C19, `845cd84` + `b83ff31` + `2af80e6`, `crates/gui` / `crates/helper` / `crates/common`, and the installed Flatpak `io.github.goshitsarch_eng.NixosToolkit` (`flatpak --user run --command=sh` and `--command=printenv`). Did not re-run `scripts/verify.sh`. Spot-check: `cargo test -p common --offline --lib -- power_profile_maps bluetooth_audio_nvidia` **2 passed**; `cargo test -p gui --offline --lib -- nvidia_gpu_apply request_dry_run request_apply_empty confirm_apply_spawns` **6 passed**; `cargo test -p gui --offline --test state_mutations` **9 passed**.

### FAILURES (new tasks)

1. **Host-spawn NixOS / integration detection — Flatpak `/etc` binds are no-ops.**  
   Manifest finish-args `--filesystem=/etc/NIXOS:ro`, `/etc/nixos:ro`, `/etc/hostname:ro` print `F: Not sharing "/etc/hostname" with sandbox: Path "/etc" is reserved by Flatpak`. Inside the app sandbox: `/etc/NIXOS`, `/etc/nixos`, `/etc/hostname` **do not exist**; `/etc/os-release` is Freedesktop SDK 25.08. `integration::detect_system` (`crates/gui/src/integration.rs`) uses sandbox `Path::exists` / `read_to_string`, never `flatpak-spawn --host`, never `HelperOp::GetSystemInfo` (that variant is **dead** — only declared in `message.rs`). Production metadata has **no** `NIXOS_TOOLKIT_SKIP_*` (only smoke sets them). `refresh_banner` priority is Not NixOS first (`apply.rs`), so a NixOS user running the Flatpak gets a non-dismissible **"Not running on NixOS"** error, Getting Started stays “Not NixOS”, Verify Integration re-runs the same probe. Host `flatpak-spawn --host -- test -e /etc/NIXOS` exits 0 on this machine — the host is NixOS, the GUI cannot see it. Smoke hides this with `SKIP_HOST_PROBES=1`. PLAN ticks for the status banner and onboarding status row are false for the shipping app. **Task:** detect `/etc/NIXOS`, `/etc/nixos/{configuration,flake}.nix`, and hostname via `flatpak-spawn --host` (no pkexec), or document as a hard limitation and **untick** those checklist rows. Do not overlay host `os-release`.

2. **`Intent::DetectGpu` is never emitted — NVIDIA page is dead even though sandbox `lspci` works.**  
   Handler exists (`app.rs` `intents_to_task`). `rg Intent::DetectGpu crates/gui/src` is that arm only. `init` sets `gpu_vendor: None` and does not queue DetectGpu / DetectSystem (architecture.md §5.7 step 6; GTK ran lspci in `HardwarePage::setup_ui`). `NavSelect(Hardware)` and `RefreshSystem` (`Intent::DetectSystem` only) also skip it. Hardware view: `gpu_vendor` None → `hardware-gpu-unknown`, `show_nvidia` false. This Flatpak **has** `/usr/bin/lspci` and lists `NVIDIA Corporation GB205M [GeForce RTX 5070 Ti Mobile]` plus Intel VGA — widgets still never appear. Tests pass only because they assign `app.gpu_vendor` by hand (`nvidia_gpu_apply_defaults_unwritten_driver_to_stable`). Related: `apply_is_empty()` ignores `hardware_for_nix()`’s NVIDIA `Some(0)` patch, so a detected NVIDIA box with no other selections still gets the **destructive empty** dialog while Apply/preview emit Stable NVIDIA. **Task:** emit `DetectGpu` on init (and RefreshSystem / first Hardware nav per ux.md §6.14); on NVIDIA x86 set `nvidia_driver = Some(0)` in state (or make `apply_is_empty` / preview / WriteState snapshot use the same `hardware_for_nix()` value).

3. **User groups are a no-op unless the username field is edited.**  
   `view/system.rs` displays `$USER` / Fluent default when `state.username` is `None`. `UsernameChanged` is the only writer. `generate_selected_nix_full` / helper `generate_all_files` emit `users.nix` only when `username.is_some() && !user_groups.is_empty()`. Toggle libvirtd/docker/vboxusers with the prefilled name → Apply silently drops groups. GTK scraped the EntryRow. **Task:** seed `state.username` from `$USER` (and Flatpak host user) in `init` / `SystemDetected`; keep charset `[A-Za-z0-9_-]`.

4. **DNS and custom TCP cannot be typed; TCP merge-add opens extra ports.**  
   Both fields are controlled `text_input` bound to parsed `AppState`. `DnsServersChanged` → `parse_and_set_dns` requires a full `u8.u8.u8.u8` on **every** keystroke (`state.rs` `is_ipv4`). Replica: `"1"`, `"1.1"`, `"1.1.1"` fail; `"1.1.1.1, 8"` fails on `"8"` and the widget snaps back to the last valid join. Paste-only works. `CustomTcpPortsChanged` → `parse_and_add_tcp_ports` **merges** any `u16` (`"9090"` adds 9, 90, 909, 9090; no remove-by-edit). PLAN ticks “DNS comma-separated IPv4” / “preset vs custom TCP split” tested the parser, not the iced binding. **Task:** keep a draft string in the model (like `package_input`); parse on submit / trailing apply; TCP edits must replace the custom-port set, not union keystroke prefixes.

5. **Hostname charset is not enforced (false PLAN tick).**  
   ux.md / PLAN: `[A-Za-z0-9-]`, no-op vs `/etc/hostname`. `HostnameChanged` only trims and compares; `field_errors.hostname` is never set. Helper `validate()` would reject `_` / `.` but Apply never sends `Validate`. Invalid names go straight into `hostname.nix`. **Task:** reject live like helper (`A-Za-z0-9-`, length), surface `field_errors.hostname`, do not `set_hostname` on invalid input.

6. **`helper_missing` still fires helper sessions from nav and Refresh.**  
   Apply / Dry Run / rollback / per-generation switch-delete / maintenance **Run** check `!helper_missing` (`view/apply.rs`, `generations.rs`, `maintenance.rs`). Generations **Refresh** is `if idle` only (not `buttons_enabled`). Maintenance disk Refresh same. `nav_intents` always `load_generations_intents` / `load_disk_usage_intents` when idle. Reducer `RequestApply` / `start_rebuild` / `LoadGenerations` do not consult `helper_missing` (`test_app()` is helper-missing and still expects SpawnHelper). Flatpak: `spawn_helper_task` yields `SpawnFailed` (no pkexec) — visiting Generations still toasts the helper-missing wall of text. **Task:** treat helper-missing like busy in those loaders and Refresh `on_press`; keep the fail-fast spawn as belt-and-suspenders.

### Pass (named checks)

- **`pkexec` / `nix-env` in `crates/gui` besides `spawn.rs`:** `rg pkexec crates/gui` is `helper/spawn.rs` only; `rg nix-env crates/gui` empty. Views have no `Command`.
- **No UDP widget:** `view/network.rs` is firewall + TCP chips `{22,80,443,8080}` + custom TCP + SSH + Tailscale. No `SetUdp*`. `allowed_udp_ports` still round-trips (`state_mutations.rs`).
- **PPD not always-on:** `generate_hardware_nix` emits PPD only when `!tlp_enabled && power_profile != 0` (`crates/common/src/nix.rs`). Tests `power_profile_maps_when_tlp_off` + `bluetooth_audio_nvidia_only_omit_power_profiles_daemon` passed. TLP still forces `power-profiles-daemon.enable = false`.
- **Apply button when `helper_missing`:** Apply/Dry Run `on_press` omitted (`buttons_enabled = idle && !helper_missing`). Session `run_blocking` / `HelperClient::spawn` refuse unavailable specs (no pkexec). Incomplete — see failure 6.
- **WriteState on dry-run (GUI session):** `RequestDryRun` → `then_write_state: false`, `save: None`. Session tests `dry_run_does_not_write_state` / `request_dry_run_spawns_dry_build_without_write_state` passed. **Caveat (non-blocking, GTK-era helper):** `commands::apply` always `generate_all_files(..., false)` then `nixos-rebuild dry-build`, so Dry Run **does write** `/etc/nixos/nixos-toolkit/*.nix`; only `state.json` is skipped.
- **GTK leftover in gui crate:** `crates/gui/Cargo.toml` is libcosmic only; `rg gtk4|libadwaita|adw:: crates/gui` empty; `Cargo.lock` has no `gtk4`. `sctk-adwaita` is winit/wayland, not GTK. `flake.nix` GUI package has no wrapGApps/gtk4.

### Non-blocking (do not block REPORT if 1–6 land or are explicitly accepted)

- Apply logs still batched (`session::stream` = `run_blocking` then iter) — already in REPORT.
- `Intent::SendOnSession` / `CloseSession` still no-ops.
- `OpenPath` failure is `tracing::warn` only (C11 asked for a toast).
- Copy snippet toasts success before clipboard completes; `ClipboardCopied { ok: false }` is unreachable.
- `ai-tools` still has no template (15 bundle files vs 16 catalog ids) — in-scope exception.
- Host `SpawnSpec::from_parts` always `helper_available: true` (falls back to the helper **name**). C9 host path; C10 “refuse pkexec if not executable” is Flatpak-only. Native `cargo run` without helper can still pkexec-hang.
- Reconstruct-from-Nix / old `state.json` with `enabled_bundles` and no `bundle_packages` shows all checkboxes unchecked; checking one package first inserts an empty set (`ToggleBundlePackage`) and drops the rest.

Do not APPROVE Phase 3 / close REPORT until (1) Flatpak detection is honest on NixOS, (2) GPU detect actually runs, (3–5) System Settings groups/DNS/TCP/hostname match the ticked parity rows, (6) helper-missing does not spawn on nav.

## Task 12-14 DA

**APPROVE** `e01d7d4` (`feat: run local Nix preview and helper apply on the same session`)

Reviewed that commit snapshot on `cosmic-migration` (later `2af80e6` only disables helper-missing buttons). The six checks pass. Spot-check: `cargo test -p gui --lib --offline -- session::tests` → **4 passed**; `core::apply::tests` → **32 passed**; common rollback serde + helper `rollback_rejects_unknown_activate` passed. Did not re-run `scripts/verify.sh` or Flatpak/weston.

### Pass

- **Apply+WriteState is one helper process.** `crates/gui/src/helper/session.rs` `run_blocking` spawns `HelperClient` once. `HelperOp::Apply` runs `run_apply_chain` on that client: `EnsureDirectories` → `Apply` (recv until `ApplyComplete`) → `WriteState` on the same stdin when `success && then_write_state` and `save` is `Some`. GUI `ConfirmApply` emits a single `Intent::SpawnHelper`; `ApplyComplete` does not spawn a second save. Test `apply_plus_save_is_one_spawn_and_writes_state` asserts spawn count **1** and request types `EnsureDirectories`, `Apply`, `WriteState`. Helper `main` keeps reading stdin, so the chain is one process.
- **DryRun no WriteState.** `RequestDryRun` → `start_rebuild(DryBuild, false)` → `save: None`, `then_write_state: false`. Session writes state only when `success && then_write_state`. Tests: `request_dry_run_spawns_dry_build_without_write_state`, `dry_run_does_not_write_state` (even with a dummy `save`), `failed_apply_does_not_write_state`.
- **Rollback activate switch/boot.** `HelperRequest::RollbackGeneration.activate` defaults to `"switch"` (old JSON). Helper `rollback_generation` rejects anything else, then `switch-to-configuration` gets `switch` or `boot`. Dialog is Cancel / Set for Next Boot / Switch Now; `ConfirmRollback` maps `SwitchNow`→`"switch"`, `SetForNextBoot`→`"boot"`. Tests cover serde default, boot round-trip, GUI request difference, and unknown activate.
- **No `pkexec`/`nix-env` in `crates/gui` except `spawn.rs`.** `rg pkexec crates/gui` is `helper/spawn.rs` only. `rg nix-env crates/gui` is empty. Generations/maintenance go through `ListGenerations` / `RollbackGeneration` / `DeleteGenerations` / `GetDiskUsage` / allowlisted `RunMaintenance`. Maintenance log is `$ {command}`, not pkexec.
- **Nav does not cancel apply.** `nav_intents` changes `page` and title; it does not clear `busy` or emit `CloseSession`. `NavSelect(Generations|Maintenance)` while busy returns no spawn (`load_*` early-out). Test `nav_during_apply_does_not_spawn_or_cancel` keeps `Busy::Applying`. Helper session is a `Task::stream`, not a nav-keyed subscription.
- **Local preview no helper.** `SelectProfile` / `RefreshProfilePreview` → `Intent::LocalProfilePreview`; `NavSelect(Apply)` / `RefreshPreview` → `Intent::LocalPreview`. `app.rs` runs `generate_preview_full_from` / `read_template_from` on `Flags.templates_dir`. No `SpawnHelper`.

### Non-blocking

- `session::stream` is `stream::once(run_blocking).flat_map(iter)`: Log lines are read live on the helper pipe, then delivered to iced in one batch after the chain finishes. Spinner/busy still work; live log view does not. Not a second pkexec.
- `e01d7d4` Apply/Dry Run/rollback/maintenance buttons still armed when `helper_missing` (`busy == Idle` only). Click still cannot hang: unavailable spec → `SpawnFailed`, banner + toast, no pkexec. `2af80e6` disables those actions.
- `Intent::SendOnSession` / `CloseSession` remain no-ops (`// Session chaining is task 12.`). Chaining lives in `session.rs`; leave the stubs or delete them.

## Task 12–14 — Architecture plan

Make apply/preview/generations/maintenance actually run without a second pkexec.

- Local preview: `generate_preview_full` / profile `read_template` from Flags templates dir; `SelectProfile` and `NavSelect(Apply)` refresh; VerifyIntegration toasts GTK copy after DetectSystem.
- Session: one spawn for Apply — EnsureDirectories → Apply (stream Log) → WriteState on success when `then_write_state` (ipc state on `HelperOp::Apply.save`). DryBuild skips WriteState. SpawnFailed → banner + toast.
- C4: `RollbackGeneration.activate` default `"switch"`; helper `switch-to-configuration` uses it; generations/maintenance only via helper IPC (no gui `pkexec`/`nix-env`). Cheap generation nixos/kernel reads. `AppModel::test_model()` via `Core::default()`.

## Task 12–14 — Architecture (landed)

Local preview uses Flags `templates_dir` (`generate_preview_full_from` / `read_template_from`). `SelectProfile` emits `LocalProfilePreview`; `NavSelect(Apply)` refreshes preview. Apply confirm uses GTK empty / packages-only / normal copy; empty check includes non-default hardware. One helper process: EnsureDirectories → Apply (Log stream) → WriteState when `then_write_state`. DryBuild skips WriteState. SpawnFailed sets banner + toast.

`RollbackGeneration.activate` defaults to `"switch"` (old JSON). Helper `switch-to-configuration` uses `switch`|`boot`. ListGenerations fills nixos/kernel from `system-N-link` (two reads). Maintenance looks up `default_maintenance_actions()` and sends the exact allowlist string; log prints `$ {command}` (not pkexec). `rg pkexec crates/gui` is `spawn.rs` only. `AppModel::test_model()` uses `Core::default()`.


## Pages + hardware DA

**REQUEST CHANGES** `3d1d37c` (package parser), `7062ae6` (HardwareConfig Nix), `2a5d568` (AppState tests), `1914e54` (libcosmic settings pages)

Reviewed the four commit snapshots on `cosmic-migration` (HEAD = `1914e54`). Parser matches GTK. Views do not spawn `pkexec`. No UDP widget. NVIDIA widgets are **absent** (not insensitive); Thermald stays present + `toggler_maybe`. Those checks pass. Do not land: hardware.nix still hardcodes power-profiles-daemon, and the NVIDIA `None` vs shown-Stable / modesetting-default contract is wrong.

### Pass

- **Parser = GTK `PackagesPage`.** `crates/gui/src/core/packages.rs` is the GTK methods (`parse_package_input` / `clean_package_name` / `is_valid_package_name` / all-bundles map) with the same branch order. Unit tests cover brackets, commas, newlines, `with pkgs;`, `pkgs.` / `nixpkgs#`, ASCII ident 1–128, `pkgs.foo pkgs.bar` not split (GTK skips the space branch when the string starts with `pkgs.`), and in-bundle toasts against **all** catalog bundles (`git` → Development Tools). `AddPackagesFromInput` toasts match GTK copy and timeouts (3s / 4s).
- **No UDP widget.** `view/network.rs` is firewall + TCP chips `{22,80,443,8080}` + custom TCP + SSH + Tailscale. No UDP row, no `SetUdp*` message (C19). `allowed_udp_ports` still round-trips in `state_mutations.rs`.
- **No `pkexec` in views.** `rg pkexec crates/gui/src/view crates/gui/src/widget` is empty. No `Command` / `std::process` / `spawn` there. Hardware GPU string is `app.gpu_vendor` (architecture `Intent::DetectGpu` → `integration::detect_gpu`). Generations/maintenance emit `Message::*` only.
- **NVIDIA absent vs Thermald insensitive (the widgets).** `view/hardware.rs`: `show_nvidia = !is_arm && gpu_vendor.contains("nvidia")`; the four NVIDIA controls are not built otherwise. Thermald row is always built; ARM uses `toggler_maybe(..., (!is_arm).then_some(SetThermaldEnabled))` plus `hardware-thermald-arm` (GTK `.sensitive(!is_arm)` + “Intel-only…”). ARM banner copy matches ux.md.
- **C3 IPC shape.** `Generate`/`Apply` take `#[serde(default)] hardware_config`. Old GTK Apply JSON without the field deserializes. `generate_hardware_nix(&HardwareConfig)` consumes driver index 0–3, modesetting, NVIDIA PM, open, PulseAudio/None, low-latency, `powerOnBoot` from autopower. Import when `has_settings()`, not bluetooth-only. Sibling bluetooth is OR-ed. NVIDIA `powerManagement.enable` is **not** the old hardcoded `true` — it follows `hw.nvidia_powermanagement` (review-phase1 §0.4).
- **FTL.** `gui.ftl` and `nixos_toolkit.ftl` are byte-identical after `1914e54`.

### Blockers

1. **hardware.nix hardcodes power-profiles-daemon.** `generate_hardware_nix` (`crates/common/src/nix.rs`): if `tlp_enabled` write TLP and `power-profiles-daemon.enable = false`; **else always** write PPD enable + a oneshot `powerprofilesctl set {balanced|performance|power-saver}`. Tests lock that in (`power_profile_maps_when_tlp_off` asserts `HardwareConfig::default()` contains `power-profiles-daemon.enable = true`). GTK’s generator only emitted TLP when `tlp_enabled`; with TLP off it wrote **no** power section. `has_settings()` is false for default, so the file is skipped until any other field is set — then bluetooth-only / PulseAudio-only / NVIDIA-only apply **also** enables PPD. That is the C3 footgun with a new name: not NVIDIA `powerManagement.enable = true`, but system PM enabled as a stowaway. PipeWire (audio default 0) is omitted unless low-latency; Balanced PPD (power default 0) is not. Pick one rule: emit a section only when that field is non-default, or admit that Balanced is an applied default and make `has_settings()` agree (every apply writes hardware.nix).

2. **NVIDIA `None` vs shown Stable; modesetting default off.** GTK built the NVIDIA combo at selected 0 and Modesetting **on**; `get_hardware_config` then returned `Some(selected)` / `is_active()`. Cosmic:
   - `GpuDetected` only stores `gpu_vendor` (`apply.rs`). It does not `SetNvidiaDriver(Some(0))` or turn modesetting on.
   - The dropdown uses `hw.nvidia_driver.unwrap_or(0)` so the UI shows Stable while state stays `None`.
   - `HardwareConfig` Default has `nvidia_modesetting: false` (GTK widget default was `active(true)`).
   - `generate_hardware_nix` skips the GPU section when `nvidia_driver` is `None` (test `nvidia_none_omits_gpu_section`).
   Detected NVIDIA, user never touches the combo, Apply → no `videoDrivers`, modesetting would have been off anyway. `SetNvidiaDriver(Option<u8>)` means **absent widgets ⇒ None**, not “visible combo, None in state”. On detect (x86 NVIDIA): set `Some(0)` and `nvidia_modesetting = true`. ARM / non-NVIDIA: keep `None` and do not build the widgets (already done). Packaging still has no test that ARM/non-NVIDIA ⇒ `nvidia_driver is None` (PLAN task 10).

3. **Empty-apply tests lie.** `AppState::apply_is_empty()` (7062ae6) counts non-default hardware (bluetooth-only is **not** empty). `crates/gui/tests/gui_state.rs` (2a5d568) reimplements the old profile+bundles+packages predicate and asserts `set_bluetooth_enabled(true)` is still empty. That file does not call `AppState::apply_is_empty()`. Fix the integration test or it will keep passing while documenting the GTK toy.

### Other parity gaps (fix with the blockers or tick as task 5/8/12)

- **Profiles preview is dead.** `SelectProfile` only mutates state (`Vec::new()`). `RefreshProfilePreview` is never emitted from `view/profiles.rs`. `Intent::LocalProfilePreview` is a comment in `app.rs` (“task 12 / 5”). Radio select leaves `# Select a profile to see preview`.
- **Apply not disabled when helper-missing.** `view/apply.rs` enables Apply/Dry Run on `busy == Idle` only. PLAN: helper-missing ⇒ Apply disabled + banner, no pkexec. Banner exists; the buttons do not check `helper_missing`.
- **Hostname charset.** ux.md / GTK: `[A-Za-z0-9-]`. `HostnameChanged` trims, no-ops vs `system_info.hostname`, does not validate charset (`nixo_` can sit in state).
- **PackagesOnly vs Normal.** `RequestApply` is still a bool empty/not. `gui_state.rs` records that; PLAN three copies are not done (task 12).
- **Switch generation dialog** has no Boot vs Switch Now in `Dialog::ConfirmRollback { generation }` (task 13 / C4). View only emits `RequestRollback`.

Non-blocking: service names stay English in `view/services.rs` (GTK catalog); 21 ids + rustdesk info + Nix-option tooltip are there. Bundle `ArmCompat::None` uses `on_toggle_maybe` / `on_press_maybe` (insensitive, not missing). Custom TCP field shows non-preset ports only. `on_input` merge-add matches GTK `connect_changed` on the EntryRow (also only-add).

Do not APPROVE until (1) PPD is not a stowaway on every hardware.nix, (2) NVIDIA detect writes `Some(0)` + modesetting on and Apply actually emits the NVIDIA block, (3) `gui_state.rs` uses `AppState::apply_is_empty()`.

## Task 2 — Packaging tests (landed)

`crates/gui/tests/state_mutations.rs` covers AppState public methods: select/clear profile, bundle enable/disable (drops package set), bluetooth sibling mirror, TCP port split/parse, services, and NetworkConfig UDP vec IPC+JSON round-trip (no UDP widget).

`crates/gui/tests/gui_state.rs` records the current bool-ish empty-apply predicate (profile + bundles + custom packages only). PackagesOnly vs Normal is not distinct yet.

`AppModel::apply` ConfirmApply empty vs not, busy-ignore, and helper State/Log/ApplyComplete are tested next to `core/apply.rs` via `AppModel::init(Core::default(), Flags::for_tests())`. Exhaustive `Message` match is not duplicated. `core::packages` parser is public; unit coverage stays in `packages.rs`.

`crates/fake-helper` depends on `common` and replies `State` / `ApplyComplete` / `Log` JSON with `#[serde(tag="type", content="payload")]` (script shorthands or raw tagged JSON).

### Gap for architecture

`crates/gui/tests/` cannot construct `AppModel` without `cosmic::Core` and `Application::init`. `dialog` / `busy` are `pub(crate)`. Need a display-free `AppModel::test_model()` so packaging integration tests can drive `apply()` without libcosmic and inspect dialogs.

## Task 1b/1c DA sign-off

**APPROVE** `8cf7536` (1b: `feat: add libcosmic page chrome and empty nav pages`) and `7917f21` (1c: `chore: add Flatpak manifest, cargo-sources, and verify scripts`)

Reviewed those commit snapshots (HEAD = `7917f21`, tree clean). The listed 1b/1c checks pass. Spot-check: `cargo test -p gui --lib --offline` → **11 passed**; `cargo test -p fake-helper --offline` → **4 passed**. Did not re-run `scripts/verify.sh` or a full Flatpak/weston smoke.

### 1b — `8cf7536`

- **11 pages:** `view/mod.rs` `page()` matches every `Page` variant (Onboarding, Profiles, Bundles, Packages, System, Hardware, Network, Services, Generations, Maintenance, Apply). Each module is `view(...) -> Element<'_, Message>` with `title2` + `body` from Fluent (`titled_page` / onboarding column). No `_` arm.
- **status_banner not dismissible:** `widget/status_banner.rs` is icon + body in a toned container. No close button, no `on_press`, no dismiss message. GTK parity comment is accurate. Global banner is optional in `view::root`; ARM copies on Bundles/Hardware are the same widget.
- **no nested toaster:** `Application::view` (`app.rs`) is the only `widget::toaster`. `view::root` documents that and wraps banner + page in `settings::view_column` + scrollable only.
- **FTL in sync:** `crates/gui/i18n/en/gui.ftl` and `nixos_toolkit.ftl` are byte-identical (`md5 8749cc1f72c702c144e52db8b9ac5f37`). Chrome, nav, page titles/descriptions, onboarding, and banner keys are present in both.
- **views emit Message, not processes:** no `Command` / `Task` / `std::process` / `pkexec` / `spawn` under `view/` or `widget/`. Onboarding reads `app.system_info` and `crate::integration::{classic,flake}_integration_snippet` (string constants). ARM banners read `app.cpu_arch()`. Commit does not touch `app.rs` / `message.rs` / `state.rs`.

### 1c — `7917f21`

- **smoke sets skip-privileged:** `scripts/smoke-flatpak.sh` `flatpak run --env=NIXOS_TOOLKIT_SKIP_PRIVILEGED_INIT=1` (also `NIXOS_TOOLKIT_SKIP_HOST_PROBES=1`). `env_flag` treats `"1"` as true. `init` queues `ReadState` only when `!skip_privileged_on_init && helper_available`.
- **no production skip:** `SKIP_PRIVILEGED_INIT` is not in finish-args, desktop, metainfo, or `flake.nix` wrap. Manifest comment says not to set it. `Flags::from_env` defaults false; `for_tests()` skip is test-only.
- **templates env:** finish-args `--env=NIXOS_TOOLKIT_TEMPLATES_DIR=/app/share/nixos-toolkit/templates`; build copies `nix/templates` there (C9).
- **cargo-sources committed:** `flatpak/cargo-sources.json` in `7917f21` (~451K, git+archive+inline; no git-tarball). Generator pin `1fc32195e3e60fe5c97f0af646dec7a99df5962b`; no `--git-tarballs`.
- **fake-helper has no libcosmic:** `crates/fake-helper` depends on `serde_json` only (`Cargo.lock` matches). JSON-line echo (`{"type":"Ok"}` / `FAKE_HELPER_SCRIPT`).
- **verify.sh exists:** executable; cargo build/clippy/test then `build-flatpak.sh` + `smoke-flatpak.sh`.
- **pkexec not in smoke path:** skip-privileged prevents startup `ReadState`; empty pages have no apply/maintenance `on_press`; `RefreshSystem` is `Intent::DetectSystem` only; smoke greps logs for `pkexec|polkit`. Probe remains `flatpak-spawn --host -- test -x` (not pkexec). C9 still refuses pkexec when the host helper is missing.

APP_ID is `io.github.goshitsarch_eng.NixosToolkit` on the crate, desktop, metainfo, and manifest. No GTK reintroduced in 1b/1c.

Non-blocking (not 1b/1c blockers; already in the 1c landed notes): no Adwaita icontheme (PLAN 1c asked for it; symbolic nav/banner/refresh icons may be missing in the sandbox); GitHub `flatpak` job is `workflow_dispatch` only; `--filesystem=/etc/hostname:ro` may be rejected (`Path "/etc" is reserved`); banner sits inside the page scrollable (1b plan) rather than above it (`ux.md` §2.1). PLAN 1b menu/nav/header stay in 1a `app.rs` (correct ownership).

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

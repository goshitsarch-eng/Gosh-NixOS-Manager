# Phase 1 review — Devil's advocate

Status: **CONDITIONAL**. Do not start Phase 2 until the lead records the decisions in §3 in `docs/migration/DECISIONS.md` (and folds the blocking items into `PLAN.md`). This review does **not** approve the three contracts as-is.

Sources read: `docs/migration/{ux,architecture,packaging}.md` plus the GTK/common/helper tree they claim to describe. GTK/helper/common win when a plan disagrees with the code.

Priority used for recommended resolutions (as assigned):

1. Feature parity with the original app
2. Working correctly in the Flatpak sandbox
3. Accessibility
4. Simplicity and maintainability
5. COSMIC conventions

---

## 0. Source facts the plans must not hand-wave

### 0.1 Generations really bypasses the helper

`crates/gui/src/pages/generations.rs` never constructs `HelperClient` and never sends `ListGenerations` / `RollbackGeneration` / `DeleteGenerations`.

- **List** is an unprivileged filesystem walk of `/nix/var/nix/profiles/system-N-link` plus `nixos-version` and the `kernel` symlink (`generations.rs:217–290`, `get_generation_metadata` at `618–647`). No pkexec.
- **Switch** is two raw pkexecs (`generations.rs:484–501`):

```484:501:crates/gui/src/pages/generations.rs
                let switch_result = std::process::Command::new("pkexec")
                    .args([
                        "nix-env",
                        "-p",
                        "/nix/var/nix/profiles/system",
                        "--switch-generation",
                        &generation.to_string(),
                    ])
                    .output();
                // …
                        std::process::Command::new("pkexec")
                            .args(["/nix/var/nix/profiles/system/bin/switch-to-configuration", &mode])
                            .output()
```

`mode` is `"switch"` or `"boot"` from the three-button dialog (`444–474`).

- **Delete** is `pkexec nix-env … --delete-generations N` (`564–566`).

Helper `list_generations()` (`crates/helper/src/commands.rs:479–546`) uses `nix-env --list-generations` and **drops** `nixos_version` / `kernel_version` (`None` at `542–544`). Helper `rollback_generation()` **always** activates with `"switch"` (`565–566`) — there is no boot-mode field on `HelperRequest::RollbackGeneration` (`ipc.rs:68`).

Routing list/switch/delete through the helper is the right Flatpak fix. It is **not** behaviour-preserving unless the helper is patched. Architecture §5.6 notices boot-mode; it does **not** notice the metadata drop or the new pkexec prompt just to *view* the list.

### 0.2 Maintenance is split-brain

`crates/gui/src/pages/maintenance.rs`:

- **Disk usage** already speaks helper IPC (`GetDiskUsage`) via its own duplicated pkexec spawn (`394–480`). Same path discovery as `HelperClient`, copy-pasted.
- **Actions** do **not** send `HelperRequest::RunMaintenance`. They `pkexec` the raw command string (`254–318`), with a `nix-channel` fallback that runs *without* pkexec (`313–315`).

The helper allowlist exists (`commands.rs:621–627`) and matches `default_maintenance_actions()` command strings (`actions.rs:745–787`). The GTK page simply does not use it. Architecture §5.6 and packaging §3 are correct that this must stop. UX log copy `"$ pkexec {command}"` (`maintenance.rs:259`) will become a lie if they keep it after the IPC change.

### 0.3 Apply does not send `hardware_config`

`HelperRequest::Generate` / `Apply` (`ipc.rs:24–53`) have `bluetooth_enabled: bool` plus `network_config` / `services_config`. There is **no** `hardware_config` field.

`crates/gui/src/pages/apply.rs` `do_rebuild` scrapes `get_app_state()` then sends (`490–503`):

```490:503:crates/gui/src/pages/apply.rs
                let request = HelperRequest::Apply {
                    selected_profile,
                    enabled_bundles,
                    bundle_packages,
                    hostname,
                    dns_servers,
                    user_groups,
                    username,
                    bluetooth_enabled,
                    custom_packages,
                    network_config,
                    services_config,
                    rebuild_type,
                };
```

`bluetooth_enabled` is the **sibling** on GUI `AppState` (`state.rs:25`), not `hardware_config.bluetooth_enabled`. Nothing in the GUI crate calls `set_bluetooth_enabled` except the unused setter. The Hardware page never pushes into the sibling. `get_app_state()` *does* scrape `hardware_config` into window state (`window.rs:302–309`) and `WriteState` persists it — so the Hardware page is a **state.json round-trip toy**. It does not change Nix.

Even the advertised bluetooth stub is dead from the UI: Apply sends the sibling, which stays `false` unless something else wrote it.

Preview is the same lie: `apply.rs:243–255` builds `NixGenOptions` with `bluetooth_enabled: state.bluetooth_enabled` and no `HardwareConfig`. `generate_preview_full` only emits `hardware.nix` when `options.bluetooth_enabled` (`nix.rs:99–102`, `600–610`) and then calls `generate_hardware_nix(false, false, false, true, false, false)`.

### 0.4 `generate_hardware_nix` cannot consume `HardwareConfig` without a rewrite

Current signature (`nix.rs:381–388`):

```rust
pub fn generate_hardware_nix(
    nvidia_enabled: bool,
    nvidia_open: bool,
    audio_pipewire: bool,
    bluetooth_enabled: bool,
    tlp_enabled: bool,
    thermald_enabled: bool,
) -> String
```

| `HardwareConfig` field (`ipc.rs:360–396`) | Consumed? |
|---|---|
| `nvidia_driver: Option<u8>` (0 stable / 1 beta / 2 open / 3 nouveau) | **No.** Function has a boolean `nvidia_enabled` and always writes `videoDrivers = [ "nvidia" ]`. Nouveau is impossible. Beta/open packages are impossible. |
| `nvidia_modesetting` | **Ignored.** Hardcoded `modesetting.enable = true` (`nix.rs:397`). |
| `nvidia_powermanagement` | **Ignored.** Hardcoded `powerManagement.enable = true` (`nix.rs:398`). Default on the Hardware page is **off**. If they naively pass `nvidia_enabled=true` they would *enable* experimental PM even when the switch is off. |
| `nvidia_open` | Yes (parameter). |
| `audio_server: u8` (0 PipeWire / 1 PulseAudio / 2 None) | **Partial.** Only `audio_pipewire: bool`. PulseAudio is not generated. `None` is not generated. |
| `audio_lowlatency` | **No.** |
| `bluetooth_enabled` | Yes (parameter). |
| `bluetooth_autopower` | **Ignored.** Hardcoded `powerOnBoot = true` (`nix.rs:424`). |
| `power_profile: u8` (balanced / performance / power-saver) | **No.** |
| `tlp_enabled` | Yes. Also unconditionally disables `power-profiles-daemon` when TLP is on (`nix.rs:439`). |
| `thermald_enabled` | Yes. |

Helper `generate_all_files` (`helper/src/nix_gen.rs:109–127`) only writes `hardware.nix` when `bluetooth_enabled`, and hardcodes every other flag to `false`. Architecture §8.3 is directionally right and still understates the rewrite: this is not “thread a struct through”; the generator’s *shape* is wrong.

### 0.5 Bundles: 16 in code, 15 in README, 15 templates

`default_bundles()` (`actions.rs:373–639`) is **16** defs including `ai-tools` (`399–411`). `nix/templates/bundles/` has **15** files — **no** `ai-tools.nix`. Helper already falls back to `generate_fallback_bundle` when the template is missing (`nix_gen.rs:168–195`). UX is correct that the prompt’s “15” is stale. Architecture §9 already notes this. Do not “fix” by dropping `ai-tools` from the UI (parity). Do not invent a template in this migration unless the lead assigns it.

README Features still says “15 bundles” and “19” services. The Services page has **21** switches (`printing` … `store_optimize`, including `rustdesk` and `gnome_tweaks`). That is README drift, not a GTK inventory error.

---

## 1. Blocking objections

These must be resolved in `PLAN.md` / `DECISIONS.md` before Phase 2 implementation. Implementing now will produce a crate that does not compile against the other workstream, does not smoke, or ships a Hardware page that still lies.

### B1. APP_ID is unresolved and both sides are wrong about D-Bus

| Doc | ID |
|---|---|
| GTK `crates/gui/src/main.rs:17` | `org.nixos-toolkit.app` |
| UX §1.1, architecture §12 | **keep** `org.nixos-toolkit.app` |
| Packaging §1 | **`io.github.goshitsarch_eng.NixosToolkit`** |

Architecture §12 claims `org.nixos-toolkit.app` is “valid as a Flatpak id”. It is not. D-Bus well-known names and Flatpak IDs forbid a hyphen in a non-last component (`org.nixos-toolkit` is illegal). Flathub also rejects generic suffixes (`.app`). Packaging is right on the spec; architecture/UX are right that a rename splits prefs, cosmic-config, `StartupWMClass`, and any leftover GApplication id.

Polkit action ids (`org.nixos-toolkit.helper.*` in `data/polkit/org.nixos-toolkit.helper.policy`) are **independent** of the GUI APP_ID. They can stay.

**This is not optional coordination.** `cosmic::Application::APP_ID`, the desktop `Icon=` / `StartupWMClass`, the Flatpak manifest `id:`, metainfo, cosmic-config path, and (if enabled) single-instance D-Bus name must be one string. Three docs, two strings.

### B2. Message vocabulary is not a contract

UX Appendix B is nested (`Onboarding(CopySnippet)`, `Hardware(NvidiaDriver(u8))`, `Network(SshRootLogin(u8))`, `Bundles(Expand {…})`). Architecture §3 is flat (`CopyIntegrationSnippet`, `SetNvidiaDriver(Option<u8>)`, `SetSshRootLogin(String)`, no expand). UX §6 says “proposals; architecture owns the enum” — then UX §3 tables tell views to emit the nested names.

If UX implements Appendix B and architecture implements §3, **`cargo build -p gui` fails** as soon as views are wired. This is the first thing that will break Phase 2.

Specific mismatches that will not type-check even after a mechanical rename:

| UX emit | Architecture variant | Problem |
|---|---|---|
| `Bundles(Expand { id, expanded })` | **absent** | Expander chevron has nowhere to go. Architecture `AppModel` has **no** `expanded_bundles` field (grep of `architecture.md` for `expanded` is empty). |
| `Hardware(NvidiaDriver(u8))` | `SetNvidiaDriver(Option<u8>)` | `u8` vs `Option<u8>`. None = not NVIDIA. |
| `Network(SshRootLogin(u8))` | `SetSshRootLogin(String)` | Index vs `"no"\|"prohibit-password"\|"yes"`. |
| `System(HostnameApply)` / `DnsApply` / `UsernameApply` / `Network(CustomPortsApply)` | **absent** (only `*Changed`) | GTK `connect_changed` **and** `connect_apply` both mutate (`system.rs:157–166`, `201–209`, `233–241`; `network.rs:162–169`). Live-on-input is actual GTK behaviour; the extra Apply variants are optional. Architecture is closer to GTK. UX inventory still lists both. |
| `Maintenance(RequestRun { id })` | `RequestMaintenance { command, name, warning }` | Views must not look up allowlist strings. Architecture is right; UX must emit `id` **or** architecture must look up from `id`. Pick one. |
| `DialogKind::Apply { mode: Empty\|PackagesOnly\|Normal }` | `Dialog::ConfirmApply { empty_config: bool, heading, body }` | GTK has **three** bodies (`apply.rs:275–299`). A bool cannot encode PackagesOnly vs Normal unless heading/body are always precomputed. |
| `ClearProfile` | present in architecture §3 | UX §3.2: “GTK does not offer clear profile — do not add one.” |
| `EnableBundleAll` | present | UX enable-switch is `Bundles(Toggle { enabled: true })` which already means “check all”. Two messages for one switch. |
| `Toast(ToastRequest::…)` / `Toasts<Message>` | `Vec<Toast>` of `Info\|Warning\|Error` | libcosmic toaster is not a `Vec`. Architecture’s type will not plug into `widget::toaster` without a wrapper UX has not been told to write. |

**Lead must freeze one `Message` enum (architecture’s file) and a view-facing mapping table.** Nested vs flat is a style choice (priority 4/5). Missing variants are not.

### B3. Hardware apply gap is a lead-scope decision, not an “if assigned” footnote

Architecture §8 recommends closing it, then schedules it as task 11 “if assigned”, and task 8 ships “hardware Nix still stub”. UX Q10 says architecture *should* include it. Packaging does not own it.

If Phase 2 ships the Hardware page with the current generator, we have spent a full page of COSMIC widgets on a no-op. That is worse than GTK: GTK at least had the excuse of an incomplete backend. A rewrite that *knows* the page is a lie and still paints it is a product defect.

`generate_hardware_nix` **cannot** be “consumed” by threading `HardwareConfig` through `NixGenOptions`. See §0.4. Closing the gap is:

1. Serde field on `HelperRequest::Generate`/`Apply` (`#[serde(default)]`).
2. New `NixGenOptions.hardware_config`.
3. Import `hardware.nix` when any non-default hardware setting is set (not only bluetooth).
4. **Rewrite** `generate_hardware_nix` to take `&HardwareConfig` (or explode it fully): driver package selection, modesetting, powerManagement, open, PipeWire vs PulseAudio vs none, low-latency, bluetooth `powerOnBoot` from `bluetooth_autopower`, power-profile (power-profiles-daemon vs TLP mutex).
5. Mirror `bluetooth_enabled` sibling from `hardware_config` so old callers and `state.json` do not drift.
6. GUI Apply/preview use `state.hardware_config`.

That is common + helper + GUI. It is not “narrow”. It is also the only way the Hardware page is not theatre.

**Lead must say: in-scope bugfix for Phase 2 (recommended) or explicitly out of scope with in-app copy that hardware is display-only.** Silent stub after a toolkit rewrite is rejected.

### B4. `SpawnSpec::from_env` as specified will pkexec the wrong binary inside Flatpak

Architecture §7.1 step 1: “Resolve helper path (same 4-step list as today)” — `Path::exists` on `$NIXOS_TOOLKIT_HELPER`, next-to-exe, `/run/current-system/sw/bin/…`, `/usr/bin/…` (`client.rs:20–54`).

Packaging §3: **never** `Path::exists` on those paths from inside the sandbox; probe with `flatpak-spawn --host -- test -x`. Sandbox `/usr` is Freedesktop Platform. Sandbox `/run/current-system` does not exist unless granted (it is not in finish-args).

If architecture is implemented as written under `FLATPAK_ID`:

1. Helper path falls through to the string `"nixos-toolkit-helper"`.
2. Spawn becomes `flatpak-spawn --host -- pkexec nixos-toolkit-helper`.
3. Host PATH may or may not contain it; polkit `exec.path` annotation is the **absolute** NixOS path (`data/polkit/org.nixos-toolkit.helper.policy:20`). A PATH-relative pkexec **does not match** that annotation.

Architecture also says packaging “is expected to set `NIXOS_TOOLKIT_SPAWN` / `NIXOS_TOOLKIT_HELPER` in the wrapper”. The sketched Flatpak manifest (`packaging.md` §2) does **not** set those env vars, and does **not** install the optional `/app/libexec` wrapper. There is no `command-wrapper`. The heuristic is the only path.

**SpawnSpec must encode the packaging probe algorithm**, including fail-fast *without* invoking pkexec when the host helper is missing. “Injectable spawn” is not specified enough until that function is written in the architecture contract.

### B5. Headless smoke will try pkexec on Fedora

Architecture `init` always `Intent::SpawnHelper { ReadState }` unless `flags.skip_privileged_on_init` / `NIXOS_TOOLKIT_SKIP_PRIVILEGED_INIT`.

Packaging `scripts/smoke-flatpak.sh` does **not** set that variable. Fedora CI has no helper, no NixOS, no polkit agent on headless weston. `flatpak-spawn --host -- pkexec …` will either:

- hang on a polkit dialog that cannot appear (10s smoke then SIGTERM — **false pass**, UI never painted), or
- fail slowly past the 5s ReadState timeout and race the 10s kill.

Packaging §3 says the window **must still start** with helper absent. Architecture agrees the window should start. Neither connects the skip flag to the smoke script, nor specifies “probe host helper *before* pkexec”.

**Required:** smoke (and any helper-absent start) sets `NIXOS_TOOLKIT_SKIP_PRIVILEGED_INIT=1` **or** `SpawnSpec::from_env` refuses to pkexec when the host helper is not executable. Prefer both.

### B6. Per-task DoD requiring `flatpak-builder` + smoke is not realistic

Packaging line 5: *“Definition of done for every implementation task: `scripts/verify.sh` is green”* including `flatpak-builder` and headless smoke. Task 1 must therefore produce a working Flatpak, not a stub.

Architecture task 1 is `cargo build -p gui` with empty pages. Architecture task 12 is when packaging tests/Flatpak land. Those are different task 1s.

A dirty-tree `flatpak-builder --force-clean` of libcosmic + iced + wgpu on every view tweak is hours of wall clock and a generator pin (`TOOLS_COMMIT="master"` is **not a pin**, packaging.md:155). Clippy `-D warnings` on *our* code against a git API that moves weekly will fail for reasons unrelated to the task.

**Required DoD split:**

| Gate | When |
|---|---|
| `cargo build --workspace` + `clippy -D warnings` + `cargo test` | every task |
| `flatpak-builder` + weston smoke | task 1 once (skeleton), then CI on the branch, plus a named “slice complete” task — **not** every UX string change |
| `scripts/verify.sh` (full) | CI + human release checklist |

Task 1 still needs a real Flatpak skeleton so smoke exists. Subsequent tasks must not be blocked on rebuilding iced.

### B7. Binary name / lib target / rust-version / flake are unowned landmines

- GTK crate emits `[[bin]] name = "gui"` (`crates/gui/Cargo.toml:8–10`). Flake `mv $out/bin/gui $out/bin/nixos-toolkit` (`flake.nix:104`). Packaging Flatpak command is `cargo … -p gui --bin nixos-toolkit`. That **fails** until architecture changes the bin name. Architecture task 1 does not mention it.
- Packaging requires `[lib] name = "nixos_toolkit_gui"`. Architecture agrees in spirit (`lib.rs` re-exports) but does not put the Cargo.toml snippet in task 1.
- Workspace `rust-version = "1.75"` (`Cargo.toml:12`). libcosmic wants **1.93** / edition 2024. Packaging says architecture bumps it; architecture never lists the bump. Host `cargo build` on 1.75 will not compile libcosmic.
- `flake.nix` still `wrapGAppsHook4` + gtk4/libadwaita buildInputs (`flake.nix:56–70`). Architecture task 1: “no GTK”. `nix build .#gui` will break the moment GTK is deleted. Packaging: “out of scope unless assigned.” **Lead must assign flake.nix in the same task that drops GTK**, or document `nix build .#gui` as known-broken. Helper flake package can stay.

### B8. i18n loader is not in the architecture crate map

UX: Fluent from day one; `fl!` in every view; loader “architecture may own” (`ux.md:752`). Architecture crate map (`architecture.md` §2.2) has no `i18n.rs`, no `i18n-embed` / `rust-embed` deps, no `i18n/` in the file list. Packaging tests do not assert FTL completeness.

If UX writes `fl!("nav-onboarding")` and architecture does not add the embed + fallback, the crate does not compile. If they add Fluent with an empty catalog, every string renders as the key — not a GTK regression (GTK was English), but it is an incomplete UX contract and will fail any “English catalog is required and complete” claim.

**Required:** architecture task 1 or 2 owns `crates/gui/src/i18n.rs` + `i18n-embed` + `rust-embed`; UX owns `i18n/en/nixos_toolkit.ftl`; a test fails if `fl!` keys are missing (even a simple “every key used in views exists in en”). Catalog strings in `actions.rs` stay English unless the lead takes UX option 2.

### B9. Flatpak will have no icon theme and no templates env

Packaging finish-args: wayland, fallback-x11, ipc, dri, talk-name Flatpak, three `:ro` `/etc` binds. No icon theme extension, no `XDG_DATA_DIRS` hint, no `NIXOS_TOOLKIT_TEMPLATES_DIR`.

Freedesktop Platform does not ship Adwaita/COSMIC symbolic names (`user-desktop-symbolic`, `emblem-synchronizing-symbolic`, `power-profile-balanced-symbolic`, `sensors-temperature-symbolic`, `system-switch-user-symbolic`, …). UX Q3 flags this. Packaging is silent. Nav becomes eleven unlabeled-or-blank rows. That is an accessibility failure (priority 3) and a parity failure (priority 1: GTK had icons).

Manifest copies `nix/templates` into `/app/share/nixos-toolkit/templates` but never sets `NIXOS_TOOLKIT_TEMPLATES_DIR`. `common::config::paths::templates_dir()` (`config.rs:130–134`) defaults to `./nix/templates`, which inside the Flatpak is **not** the copied tree. Local preview — the entire helper-absent story — generates empty/missing-template output.

**Required:** Flatpak `command` wrapper or module env:

- `NIXOS_TOOLKIT_TEMPLATES_DIR=/app/share/nixos-toolkit/templates`
- icon theme: `org.freedesktop.Platform.Icontheme.Adwaita` (or bundle `cosmic-icons` / a fallback SVG set for the 11 nav icons + app icon, as UX asked)

### B10. libcosmic feature flags contradict each other and will not compile the same app

| Feature | UX | Architecture §7 | Packaging §9 |
|---|---|---|---|
| `default-features` | (unspecified) | **unspecified** (defaults ON) | **`false`** |
| `a11y` | required | omitted | on |
| `x11` | omitted (Q21) | omitted, but prose says “X11 fallback via winit” | on |
| `wgpu` | “packaging performance” | on | on |
| `multi-window` | omitted | omitted | on |
| `dbus-config` | omitted | pulled in if defaults stay on | off |
| `single-instance` | omitted | **must not enable** | not in feature list; prose still talks about the D-Bus name |

libcosmic defaults (`Cargo.toml` on pop-os/libcosmic master) include `dbus-config`, `a11y`, `x11`, `multi-window`. If architecture does not set `default-features = false`, the app talks to `com.system76.CosmicSettingsDaemon` which packaging explicitly does not grant. `watch_config` then depends on a daemon that does not exist on weston/GNOME.

**Required:** one feature list, `default-features = false`, owned by architecture, reviewed by packaging. Recommended: packaging’s list plus architecture’s “no single-instance”. Enable `x11` (smoke/Xwayland) and `a11y` (priority 3). Pin a **commit SHA** in `Cargo.toml`, not `git = "…"` floating master.

### B11. Rollback “Set for Next Boot” cannot be implemented on today’s helper

GTK dialog offers Switch Now / Set for Next Boot (`generations.rs:453–457`). Helper `RollbackGeneration { generation: u32 }` always `switch-to-configuration switch`. Architecture §5.6 leaves this as “lead decision / optional helper patch”. UX still draws the three-button dialog.

Shipping the dialog and mapping both buttons to `switch` is a silent behaviour change (priority 1). Omitting the Boot button is a visible regression. The only honest options are helper protocol extension or removing the Boot button with a documented gap.

Same bucket as B3: small helper patch, must be assigned, not “optional”.

---

## 2. Non-blocking objections

Track in DECISIONS.md / PLAN.md. Do not stop Phase 2 if the lead records them.

### N1. List-generations pkexec is a UX regression

GTK lists generations with no password prompt. Helper `ListGenerations` is a privileged session. Visiting the Generations page will pkexec. Disk usage already does this, so maintenance is not a new class of prompt; generations *is*.

Preferred (priority 1 then 2): unprivileged list when `/nix/var/nix/profiles` is readable; helper fallback inside Flatpak. Alternative: `--filesystem=/nix/var/nix/profiles:ro` and keep list in-process; switch/delete still helper. Do not grant `/nix` recursively.

### N2. Helper `ListGenerations` drops version metadata

GTK subtitles include NixOS and kernel versions (`generations.rs:368–373`, `618–647`). Helper parse sets both to `None` (`commands.rs:538–545`). After the IPC switch, every row loses the subtitle richness unless helper is taught `get_generation_metadata`. Assign with B11 or accept the poorer subtitle.

### N3. `parse_generation_line` on the GTK page is dead code

`generations.rs:293–321` is unused (list is a directory walk). Do not port it.

### N4. Integration detection strings differ between GUI and helper

GUI `integration.rs:67–99` treats any `"nixos-toolkit"` substring as Integrated. Helper `detect_integration` (`commands.rs:125–149`) requires `nixos-toolkit/state/selected.nix`. Architecture §11.6 says preserve both. Fine for the migration; do not “fix” it into one algorithm in this rewrite or banners will change meaning.

### N5. Empty-apply warning ignores network/services/hardware

GTK empty check is profile + bundles + custom packages only (`apply.rs:271–273`). A user with only firewall/SSH/services enabled still gets “Apply Empty Configuration? … remove ALL managed software”. Preserve that (priority 1) unless the lead expands the check as a bugfix. If HardwareConfig starts applying (B3), empty-check should probably count non-default hardware — that is a behaviour change; record it.

### N6. `bluetooth_enabled` sibling

Keep it for one IPC compat release as architecture says. `SetBluetoothEnabled` must write **both**. `from_ipc_state` should prefer `hardware_config.bluetooth_enabled` if they diverge (today they can: persisted hardware vs never-updated sibling).

### N7. Theme persistence: JSON vs cosmic-config

UX §5.2: cosmic-config primary, JSON migrate-once. Architecture §6.2: JSON canonical, cosmic-config mirror. Packaging: persist `xdg-config/nixos-toolkit` in Flatpak.

JSON canonical wins: priority 1 (GTK path `~/.config/nixos-toolkit/preferences.json`, `preferences.rs:39–40`) and priority 2 (Flatpak xdg-config is well-understood; cosmic-config dir is extra). Mirror into cosmic-config best-effort. Do not require cosmic-comp.

### N8. Clipboard / Open `/etc/nixos`

UX: `iced::clipboard::write`, `open` crate. Packaging: xdg-portal clipboard; `flatpak-spawn --host -- xdg-open /etc/nixos` (GTK file portal cannot expose `/etc`). Architecture: `Intent::CopyClipboard` / `OpenPath`.

Host-spawn xdg-open is required for Flatpak (priority 2). iced clipboard is fine as the first attempt. Do not shell out to `wl-copy` (flake currently injects `wl-clipboard`/`xclip` into PATH for GTK — that path must die with wrapGApps).

### N9. GPU detect (`lspci`) in the sandbox

GTK runs `lspci` in-process (`hardware.rs:296–319`). Packaging does not ship pciutils and does not host-spawn it. Flatpak Hardware page will always show `Unknown GPU (lspci not available)` — which is existing GTK fallback copy, so it is parity. Architecture §11.7 floating `--device=all` is the wrong hammer (`lspci` needs the binary + `/sys`, not every device node). Optional later: `flatpak-spawn --host -- lspci` without pkexec.

### N10. wgpu on headless weston

Real risk. Packaging mitigation (tiny-skia, grep panic/adapter, `--device=dri`, pixman compositor) is the right shape. Still likely to flake on NVIDIA-in-toolbox. Add `WGPU_BACKEND=gl` (and/or `LIBGL_ALWAYS_SOFTWARE=1`) as a smoke env fallback **before** treating adapter-fail as a red job. Do not enable `single-instance` (architecture is right; D-Bus well-known name + headless = smoke pain).

### N11. `TOOLS_COMMIT="master"`

`packaging.md:155` is a non-pin. First real generate must record a SHA. `cargo-sources.json` must be committed; `verify.sh` must not regenerate it (packaging already says this).

### N12. Clippy `-D warnings` vs libcosmic git

Their code is not our problem; **ours** must be clean. Pin the commit. Do not `#[allow]` our way around iced churn — wrap iced types at `app.rs` / `view/` as architecture already wants. If a libcosmic deprecation lands in a pin bump, the bump PR fixes our call sites. Do not make clippy `--all-targets -D warnings` skip `gui` because of the git dep; it does not clippy git deps.

### N13. Accessibility is a regression even with `a11y` on

GTK ActionRows/SwitchRows speak AT-SPI for free. iced/libcosmic `a11y` is not equivalent. Enable the feature (priority 3). Do not claim ATK parity. Icon-only buttons need tooltips **and** names (UX §7.1 is right). Dialog focus-without-text-field is a known libcosmic hole — keep Cancel as default so Escape is enough.

### N14. Architecture `toasts: Vec<Toast>` vs libcosmic `Toasts<Message>`

See B2. Non-blocking once the type is unified. Use `Duration::Custom(3s)` (and 4s for in-bundle, `packages.rs:359`) — UX is right that Short is 5s.

### N15. Banner priority

UX Q22: Not NixOS > state-load warning > integration. GTK last-writer-wins (`window.rs:232–257` then `387–393` overwrites). Improving this is allowed (priority 3: don’t hide “Not NixOS” behind a helper timeout). Record it as an intentional change.

### N16. `ai-tools` template hole

Leave it. Fallback generator already handles it. Do not drop the bundle (16 in UI). Do not silently add a template in the GUI rewrite.

### N17. README drift (15 bundles, 19 services, “custom TCP/UDP ports”, WireGuard)

Out of scope for the GUI rewrite. Do not invent UDP or WireGuard UI (UX is correct; architecture has no UDP messages — keep it that way). `allowed_udp_ports` still round-trips in `NetworkConfig` / `state.json` if somehow set.

### N18. `multi-window`

Packaging enables it; architecture does not. `Application::dialog()` is an overlay, not a window. Enable it only if a libcosmic pin requires it for dialogs/portals. Default off unless proven.

### N19. Polkit `exec.path` hardcoded

```20:20:data/polkit/org.nixos-toolkit.helper.policy
    <annotate key="org.freedesktop.policykit.exec.path">/run/current-system/sw/bin/nixos-toolkit-helper</annotate>
```

A helper in `/usr/bin` never matches. Packaging: “host packaging problem; document.” Correct, and **not** a Flatpak-manifest fix (polkit stays on the host). NixOS module is the supported install. Onboarding copy must say Flatpak users still need `programs.nixos-toolkit.enable` (or equivalent) on the host. Architecture §9 “packaging may wrap polkit for Flatpak” is wrong — packaging §3 says the policy is **not** in the Flatpak.

### N20. `dbus-config` / COSMIC daemon

See B10. Keep it off. `xdg-portal` covers light/dark on GNOME/KDE/weston.

### N21. Helper stdin through `flatpak-spawn --host -- pkexec`

Known fragile. Packaging already says add `--forward-fd=0 --forward-fd=1` if pipes drop. Probe this in task 1 with a dummy helper, not after Apply is written. Keep `.env_remove("SHELL")` on the host side of pkexec.

### N22. Architecture `Intent::SpawnHelper` refused if busy — Apply must not cancel on nav

§11.2 is correct and easy to get wrong in iced. Tests must navigate away during a fake Apply and assert the session still finishes.

### N23. Custom `widget::bundle_expander`

No expander in current libcosmic (UX §8). This is real work, not a rename. Architecture must store expanded ids (B2) or the widget is stateless-broken on re-view.

### N24. `widget::progress_bar::indeterminate_circular`

Current libcosmic examples use `widget::progress_bar::circular::Circular::new()`. UX name is likely stale. Non-blocking naming, but UX should check the pin before writing a helper that does not exist.

### N25. Services tooltips `NixOS option: {nix_option}`

Keep (a11y + power users). Not in architecture Message (good — view-local).

### N26. First-run pkexec for ReadState

GTK does this (`window.rs:329–384`). UX Q8: keep it. Packaging helper-absent: skip. Compatible if skip is env-driven (B5).

---

## 3. Contradictions table

Recommended resolution uses the assigned priority order. The lead records the chosen column in `DECISIONS.md`.

| # | Topic | UX | Architecture | Packaging | Recommended resolution |
|---|---|---|---|---|---|
| C1 | **APP_ID** | `org.nixos-toolkit.app` | keep `org.nixos-toolkit.app` (claims Flatpak-legal) | `io.github.goshitsarch_eng.NixosToolkit` | **Packaging ID.** Hyphenated GTK id is illegal as a D-Bus/Flatpak name (priority 2). Polkit actions stay `org.nixos-toolkit.helper.*`. One-time prefs migrate `~/.config/nixos-toolkit/` → new cosmic-config/JSON dir keyed by new id; also read the old path. Desktop `StartupWMClass` = new id. |
| C2 | **Message enum shape** | Nested `Page(PageMsg)` Appendix B | Flat `Message` §3 | Tests every architecture variant | **Architecture owns names (flat).** UX emits those names (or `.map` from private page enums **defined to match**). Architecture adds missing variants: `ExpandBundle { id, expanded }`, three-way apply warn (or precomputed heading/body covering Empty/PackagesOnly/Normal). Drop `ClearProfile` from UI. `SetSshRootLogin` takes `u8` **or** UX sends `u8` and architecture converts — pick `u8` in Message (matches combo index, priority 1). `RequestMaintenance` keyed by **action id**; architecture looks up command/warning from `default_maintenance_actions()`. |
| C3 | **Hardware Nix** | “Architecture should include it” (Q10) | Task 11 “if assigned”; task 8 ships stub | Unowned | **In-scope Phase 2 bugfix** (see B3). Priority 1 “parity” would preserve a lying page; the page’s *purpose* is to configure hardware. Closing the gap is correctness of the feature the GTK app already showed. Assign common+helper rewrite of `generate_hardware_nix`. |
| C4 | **Rollback boot mode** | Three-button dialog | Optional helper field | Route through helper; no protocol note | **Helper patch in the same bucket as C3:** `RollbackGeneration { generation, activate: Switch \| Boot }` with default Switch for old clients. Until it lands, do not ship a Boot button that actually switches. |
| C5 | **Theme store** | cosmic-config primary | JSON canonical + cosmic-config mirror | xdg-config/nixos-toolkit | **Architecture JSON canonical** (priority 1 + 2). Cosmic-config mirror best-effort. |
| C6 | **libcosmic features** | a11y, tokio, winit, wayland, xdg-portal, about; wgpu optional | about, wgpu, xdg-portal, winit, tokio, wayland; no a11y/x11; defaults unspecified | `default-features=false` + winit, wayland, x11, wgpu, tokio, xdg-portal, multi-window, about, a11y | **Packaging list, minus `multi-window` unless the pin requires it; `default-features = false`; no `single-instance`; no `dbus-config`.** Architecture Cargo.toml is source of truth; packaging copies it. Pin commit SHA. |
| C7 | **Task 1 / DoD** | n/a | cargo window, empty pages | `verify.sh` = cargo + clippy + test + flatpak + smoke **every task** | **Split DoD (B6).** Joint task 1: skeleton window **and** a working (minimal) Flatpak smoke. Later tasks: cargo gates. Full verify.sh on CI. |
| C8 | **Binary name** | n/a | unspecified (`gui` today) | `[[bin]] name = "nixos-toolkit"` + lib target | **Packaging.** Architecture task 1 includes the Cargo.toml rename and `[lib]`. Flake `mv` becomes a no-op / is deleted when flake is updated (C14). |
| C9 | **Helper spawn in Flatpak** | n/a | `FLATPAK_ID` → `flatpak-spawn --host -- pkexec <path from sandbox exists()>` | host `test -x` probe; optional wrapper; env vars not in the manifest | **Packaging algorithm in `SpawnSpec::from_env`.** Manifest **must** set `NIXOS_TOOLKIT_TEMPLATES_DIR` and either `NIXOS_TOOLKIT_HELPER` via a small `/app` wrapper that host-probes, or document that the Rust client *is* the wrapper. Fail fast if none exist (B4, B5). |
| C10 | **Smoke vs ReadState pkexec** | Keep startup pkexec (Q8) | skip flag exists, smoke does not set it | smoke = helper-absent Fedora | **Both:** production NixOS keeps startup ReadState (priority 1). Smoke/CI/`FLATPAK` without host helper sets skip **and** spawn fails without pkexec. Banner copy from packaging §3. |
| C11 | **Open `/etc/nixos`** | `open::that_detached` | `Intent::OpenPath` | `flatpak-spawn --host -- xdg-open` | **Host-spawn when `FLATPAK_ID` is set** (priority 2); `open` crate otherwise. Toast on failure (UX). |
| C12 | **Clipboard** | iced clipboard | `Intent::CopyClipboard` | xdg-portal / ashpd | iced first; portal is a libcosmic feature, not a second path. No `wl-copy`. |
| C13 | **i18n** | Fluent day one, complete en catalog | not in crate map | no FTL test | **Architecture loader in task 1/2; UX owns en FTL; missing-key test.** Not a GTK regression; empty catalogs are still a defect relative to the UX contract. |
| C14 | **`flake.nix`** | n/a | `nix build .#helper` remains; GUI unspecified | out of scope; host has no nix | **Assign with GTK deletion.** `wrapGAppsHook4` + gtk4 inputs cannot survive task 1. Either packaging or architecture updates flake in that task. `nix build .#helper` must stay green. |
| C15 | **Polkit in Flatpak** | n/a | “packaging may wrap it for Flatpak” | **not** installed by Flatpak; host module only | **Packaging.** Do not ship the policy inside the sandbox. Onboarding must say the host module is required. |
| C16 | **Icon theme** | bundle fallback + adwaita/cosmic-icons in wrapper | “reuse GTK names” | silent | **Packaging ships an icon theme (Adwaita extension or bundled SVGs).** Missing icons must not blank the row (UX). Priority 1 + 3. |
| C17 | **Tests layout** | n/a | packaging owns `tests/`; `AppModel::apply` display-free | workspace member `tests/` + fake-helper bin | **See §5.** Not blocking if `AppModel::apply` is public. Prefer `crates/gui/tests/` + `[[bin]] fake-helper` over a fourth workspace member unless they need a helper binary built without pulling libcosmic. |
| C18 | **rust-version** | n/a | unspecified | architecture bumps; min 1.93 | **Architecture bumps workspace `rust-version` to ≥1.93 in task 1.** Edition of *our* crates may stay 2021. |
| C19 | **UDP UI** | do not invent | no UDP messages | n/a | **UX.** Preserve the field; no widget. Architecture must not add `SetUdpPorts`. |
| C20 | **Preview path** | local `generate_preview_full` | local, no helper | risk table says “preview currently goes through helper Generate” | **Architecture/UX (and GTK `apply.rs:257–258`) are right.** Packaging risk row is false. Local preview + bundled templates (C9 env). |
| C21 | **Toaster type / dialog type** | `Toasts<Message>`, `DialogKind` | `Vec<Toast>`, `Dialog` | tests inspect dialog flags | **libcosmic `Toasts<Message>` in the model** (otherwise the widget does not work). Dialog enum: architecture file, with three apply copies (C2). |
| C22 | **`single-instance`** | n/a | disabled (headless + extra D-Bus) | APP_ID “is the D-Bus name used by single-instance” | **Disabled.** Packaging must not imply it is on. Headless smoke cannot take a session bus name reliably. |

---

## 4. Parity gaps the plans missed

Read from GTK source; not (or only partly) in the three docs.

1. **Hardware page does not even drive the bluetooth stub.** Plans talk as if `bluetooth_enabled` on Apply is the Hardware switch. It is not (`§0.3`). Restoring `hardware_config` from `state.json` still Apply-sends `false`. Closing C3 must include killing the sibling drift.

2. **Generations list is unprivileged and richer than helper IPC.** Plans treat “use ListGenerations” as pure win. It adds a pkexec and drops NixOS/kernel subtitles (`§0.1`, N1, N2).

3. **Maintenance log prefix and nix-channel fallback.** GTK prints `$ pkexec {command}` and, on pkexec failure, retries `nix-channel --update` unprivileged (`maintenance.rs:313–315`). Helper `RunMaintenance` always runs as root (the helper *is* privileged) and has no user-channel fallback. Channel update behaviour changes for `--update` of *user* channels. Record it.

4. **Disk-usage spawn is a third HelperClient.** Even after “one SpawnSpec”, someone must delete the copy-paste in `maintenance.rs:394–421`. Architecture says so; easy to leave a pkexec path behind.

5. **`save_state()` in apply.rs is dead** (`641–678`) but documents a **second** pkexec. Architecture forbids a second session for WriteState. Tests must assert one spawn per Apply+save.

6. **GTK hostname/DNS/username/ports mutate on every `changed`**, not only apply-button (`system.rs`, `network.rs`). If COSMIC `on_input` is wired to validate-and-set, invalid partial hostnames (`nixo-`) will sit in `AppState` and get applied. GTK `set_hostname` is similarly live. Preserve, but `core/validate.rs` must match GTK’s “unchanged vs `/etc/hostname` is a no-op” and charset rules. Do not invent “commit only on Enter” as a silent behaviour change unless the lead wants it (it would be *better* UX; it is not parity).

7. **Package parser lives on a GTK page** (`packages.rs:179–278`). Architecture Appendix A is the right extract. Missed detail: `with pkgs;` only special-cases when the string contains space and does not start with `pkgs.` (`228–234`). Tests must use the real GTK cases, not a cleaned-up grammar.

8. **In-bundle duplicate check uses *all* bundles, not only enabled ones** (`packages.rs:367–377`). A package in a disabled bundle still toasts “already in '{bundle}'” and is not added. Easy to “fix” while rewriting.

9. **Bundle enable-off drops `bundle_packages`; individual uncheck of all packages does *not* disable the bundle** (UX §3.3, GTK expander). Architecture `EnableBundleAll` vs `ToggleBundle` makes this easy to get wrong. Need tests.

10. **ARM: NVIDIA widgets are not built at all** if `lspci` is not nvidia or arch is ARM (`hardware.rs:130`). They are not “insensitive”; they are absent, so `nvidia_driver` is `None`. `SetNvidiaDriver(Option<u8>)` is the right shape. UX `NvidiaDriver(u8)` is not.

11. **Thermald is insensitive on ARM, still present** (`hardware.rs:266–275`). Different from NVIDIA (absent). Views must keep that distinction.

12. **Preset TCP ports vs custom entry split** (`network.rs` load path; UX §3.7). Custom field shows TCP minus `{22,80,443,8080}`. Architecture `ToggleTcpPort` + `CustomTcpPortsChanged` can double-count if parse replaces the whole vec. Port this carefully.

13. **`NetworkConfig.has_settings` is true when firewall is *disabled*** (`ipc.rs:275`). A user who only turns the firewall off still generates `network.nix`. Preview/apply import logic depends on that. Do not “simplify” to “any port open”.

14. **Fail2ban is nested under `ssh_enabled` in generated Nix** (`helper/src/nix_gen.rs:237–250`) but the GTK switch is independent and can be on with SSH off — in which case fail2ban is **silently not generated**. Parity: keep the silent drop, or generate fail2ban anyway as a bugfix. Record it.

15. **`get_app_state()` scrape of network/services/hardware** (`window.rs:280–311`) is the bug the rewrite exists to kill. If any view keeps local widget state, it comes back. Architecture rule is right. UX expander expanded-ids are UI state, not selection — they still need a model field (B2).

16. **About is unreachable in GTK** (action with no menu, `app.rs:71–81`). COSMIC View → About is an intentional improvement (UX §1.1). Fine. Acknowledgements must change “GTK4, libadwaita” → libcosmic/iced or the about page becomes a lie.

17. **Refresh does not re-ReadState** (`window.rs:225–274`, UX §6.14). Easy to “helpfully” reload helper state on F5.

18. **Nav always starts at Getting Started**, even after state restore (`window.rs:209–211`). Do not persist last page (UX §5.3).

19. **Status banner is not dismissible.** UX is right. Do not use `widget::warning`’s close button.

20. **Helper `reconstruct_state_from_nix` does not restore network/services/hardware** (`commands.rs:363–441`). Only profile, bundles, custom packages, hostname. Out of scope to fix, but tests using a missing `state.json` must not expect full restore.

21. **`generate_hardware_nix` hardcoded PM/modesetting vs Hardware page defaults.** If C3 is implemented by passing `nvidia_enabled=true` into the *current* function, modesetting+PM become true even when the UI says PM off. That is not a migration of GTK (GTK never generated them); it is a new footgun. Rewrite the function (B3) or do not call it.

22. **Flake still wraps `wl-clipboard`/`xclip`.** After GTK removal those are unused. Do not bring them into the Flatpak “just in case”.

23. **GTK `ApplicationFlags::FLAGS_NONE`** — not unique/single-instance (`app.rs:56–59`). Architecture disabling libcosmic `single-instance` is actual GTK parity, not just a smoke hack.

---

## 5. Test gaps

Packaging’s intent (drive `AppModel::apply` with a fake helper, exhaustive `Message` match) is the right spine. It is not enough as written.

### 5.1 Layout: `tests/` as a workspace member

Virtual workspace (`Cargo.toml` members = gui/helper/common) will **not** pick up a top-level `tests/*.rs`. Packaging is right that *something* must be a member.

A fourth package `tests/` that depends on `nixos_toolkit_gui` will:

- compile libcosmic for the tests crate,
- need the same native deps as `gui`,
- make `cargo test --workspace` fail on hosts without wayland/xkbcommon/expat even for helper unit tests if feature unification pulls gui.

**Recommendation:**

- Keep **crate tests** where they belong: `crates/common/src` (nix gen snapshots, IPC round-trip), `crates/helper/src` (allowlist, no real rebuilds), `crates/gui/src/core` unit tests next to `apply.rs`.
- GUI *integration* tests: `crates/gui/tests/*.rs` (picked up automatically once `[lib]` exists). No extra member.
- Fake helper: `crates/gui/tests/bin` is awkward; a `[[bin]]` `fake-helper` under `crates/helper` with `required-features = ["fake"]` **or** a tiny `crates/fake-helper` without libcosmic is cleaner than `tests/src/bin/fake-helper.rs` inside a package that already depends on gui.

If they still want `tests/` as a member: make it **not** depend on libcosmic — only on `common` + a public `nixos_toolkit_gui` that is careful about features — and accept slower `cargo test --workspace`. Do not put pixel tests there (packaging already said no).

### 5.2 What packaging listed that is still insufficient

| Gap | Why it matters |
|---|---|
| No test that **Apply JSON lacks `hardware_config` today and includes it after C3** | Serde will happily omit the field; helper will keep generating bluetooth-only until someone asserts the file. |
| No snapshot of `generate_hardware_nix(&HardwareConfig)` for each nvidia_driver index, PulseAudio, low-latency, autopower=false, power_profile vs TLP mutex | Without this, C3 will ship another stub. |
| No test that **WriteState happens on the same spawn** as Apply (one `pkexec` / one fake process) | GTK Bug 5; architecture invariant. |
| No test that **DryRun does not WriteState** | Easy to reuse the Apply session machine wrong. |
| No test that **nav during Apply does not cancel** the Task | iced will if the Task is scoped to the page. |
| No test for **Set for Next Boot** vs helper protocol | Will silently switch. |
| No test that **ListGenerations is used** and `Command::new("pkexec")` / `nix-env` **do not appear** in `crates/gui` | The whole point of killing the bypass. `rg 'pkexec' crates/gui` should be spawn-spec only. |
| No test that maintenance sends **exact allowlist strings** (`nix-collect-garbage -d`, etc.) | Helper rejects anything else (`commands.rs:629`). UX `RequestRun { id }` must not send the id as `command`. |
| No test for **bundle enable-all / disable-drops-set / empty-set-keeps-bundle** | GTK behaviour, easy to lose. |
| No test for **package in-bundle vs enabled-bundle-only** (GTK uses all bundles) | §4.8. |
| No test for **preset vs custom TCP split** | Double-count risk. |
| No test for **UDP vec round-trip without a UI** | Restore from state.json must not drop UDP, must not grow a widget. |
| No test for **empty-apply three copy variants** | PackagesOnly is missing from architecture’s bool. |
| No test for **banner priority** Not NixOS vs helper timeout | UX Q22 vs GTK last-writer. |
| No test that **startup skip flag** prevents spawn | Smoke depends on it (B5). |
| No test of **SpawnSpec** under `FLATPAK_ID` + missing host helper (no pkexec invoked) | B4. |
| No FTL missing-key test | C13. |
| No IPC round-trip with **old** `Apply` JSON (no `hardware_config` field) against a **new** helper | `#[serde(default)]` or you break mixed GUI/helper versions. |
| Exhaustive `match Message` in tests | Good. Must compile-fail when UX/architecture add a variant. Put it in `crates/gui` so it cannot drift from a separate tests package’s copy of the enum. |
| Fake helper scripted JSONL | Good. Must speak **real** `HelperResponse` tags (`type`/`payload` as in `ipc.rs:118`). |
| Headless smoke greps `panic\|no adapter` | Good. Add skip-privileged env. Do not treat tiny-skia fallback as fail. 10s “still running” is a weak assertion — the app can hang on pkexec and pass. |
| **No NixOS apply test** | Accepted (Fedora host). Manual checklist must exist or Apply is untested where it matters. |

### 5.3 Clippy / fmt / audit

Current flake clippy is `--all-targets -- --deny warnings` (`flake.nix:167–169`). Packaging copies that to cargo CI. After GTK deletion, flake clippy’s `commonArgs` still pull gtk4 unless C14 is done — CI-on-Fedora cargo job is the real gate. `cargo-audit` is not in `verify.sh`; flake has advisory-db. Non-blocking, but do not claim “CI equivalent of flake clippy” while flake still builds GTK.

---

## 6. Sign-off

**CONDITIONAL.**

The three documents are strong inventories. They already caught several real GTK bugs (state scrape, generations bypass, hardware IPC hole, `ai-tools` count). They are **not** yet one plan:

- Two APP_IDs, two Message dialects, two task-1 definitions, two helper-spawn algorithms, two feature lists, two theme stores, and an unassigned hardware generator rewrite that the current API cannot satisfy.
- Packaging’s every-task Flatpak DoD will stall UX/architecture or be ignored in practice (either is a process failure).
- Architecture’s `SpawnSpec` is not Flatpak-safe as written. Packaging’s smoke will pkexec-hang without the skip flag.
- I18n, icons, templates env, bin rename, rust-version, and `flake.nix` GTK hooks are unowned or contradictory. Any one of those fails “will this actually compile or ship?”

Phase 2 may start **after** the lead writes `docs/migration/DECISIONS.md` covering **every row in §3** (C1–C22) and `PLAN.md` includes:

1. Joint task 1 (lib target, `nixos-toolkit` bin, rust-version ≥1.93, frozen `Message`, i18n loader, SpawnSpec host-probe, skip-privileged, pinned libcosmic SHA, `default-features = false` feature list, skeleton Flatpak + templates env + icon theme, smoke that does not pkexec).
2. An assigned owner for HardwareConfig apply (C3) **or** an explicit “display-only” decision with UI copy.
3. An assigned owner for rollback activate-mode (C4) **or** removal of the Boot button.
4. DoD split (B6 / C7).
5. flake.nix owner for the GTK deletion commit (C14).

Approval is withheld until those decisions exist. This review does not edit `ux.md` / `architecture.md` / `packaging.md` and does not change source.

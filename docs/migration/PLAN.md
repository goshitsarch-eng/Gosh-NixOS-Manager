# NixOS Toolkit — libcosmic migration plan

Lead consolidation of `ux.md`, `architecture.md`, `packaging.md`, and `review-phase1.md`. Decisions: `DECISIONS.md`.

**App:** NixOS Toolkit (`Gosh-NixOS-Manager`), currently GTK4/libadwaita. Target: libcosmic GUI, feature parity, tests, Flatpak.

**APP_ID:** `io.github.goshitsarch_eng.NixosToolkit`  
**Command:** `nixos-toolkit`  
**Helper (host):** `nixos-toolkit-helper` via pkexec / `flatpak-spawn --host`

The GTK GUI crate is **replaced in place**. `crates/common` and `crates/helper` stay. The app must compile after every task below.

---

## Ownership

| Owner | Paths |
|---|---|
| UX | `crates/gui/src/view/`, `crates/gui/src/widget/`, `i18n/en/nixos_toolkit.ftl`, `docs/migration/ux.md` |
| Architecture | `crates/gui/src/{main,lib,app,message,state,config,integration,i18n}.rs`, `crates/gui/src/helper/`, `crates/gui/src/core/`, `crates/gui/Cargo.toml` (gui deps), workspace `Cargo.toml` rust-version / members, `flake.nix` GUI package, HardwareConfig IPC in `common`/`helper`, `docs/migration/architecture.md` |
| Packaging | `flatpak/`, `scripts/`, `crates/gui/tests/`, `crates/fake-helper/`, `.github/workflows/`, `.gitignore` Flatpak dirs, `docs/migration/packaging.md` |
| Devil's advocate | `docs/migration/review-phase1.md` (and later review notes). Sign-off required before a task is closed. |
| Lead | `docs/migration/PLAN.md`, `DECISIONS.md`, `REPORT.md`, task list, tie-breaks |

Cross-boundary tasks are split in the task list. Teammates state a short plan before editing; DA may object before work begins.

---

## Frozen contracts

### libcosmic features

```toml
libcosmic = { git = "https://github.com/pop-os/libcosmic.git", rev = "<pin in task 1>", default-features = false,
  features = ["winit", "wayland", "x11", "wgpu", "tokio", "xdg-portal", "about", "a11y"] }
```

No `single-instance`, no `dbus-config`, no `multi-window` unless the pin requires it.

### Message (architecture file; UX emits these)

Chrome: `NavSelect(Page)`, `ToggleAbout`, `LaunchUrl`, `Quit`, `RefreshSystem`, `SystemDetected`, `DismissToast`, `DismissDialog`, `UpdateConfig`, `ClipboardCopied { ok }`.

Onboarding: `CopyIntegrationSnippet`, `OpenEtcNixos`, `VerifyIntegration`.

Profiles: `SelectProfile(String)`, `RefreshProfilePreview`.

Bundles: `ToggleBundle { id, enabled }`, `ToggleBundlePackage { bundle_id, package, enabled }`, `ExpandBundle { id, expanded }`.

Packages: `PackageInputChanged(String)`, `AddPackagesFromInput`, `AddCustomPackages(Vec<String>)`, `RemoveCustomPackage(String)`.

System: `HostnameChanged`, `DnsServersChanged`, `UsernameChanged`, `ToggleUserGroup { group, enabled }`, `ColorSchemeChanged`.

Hardware: `SetNvidiaDriver(Option<u8>)`, `SetNvidiaModesetting`, `SetNvidiaPowerManagement`, `SetNvidiaOpen`, `SetAudioServer`, `SetAudioLowLatency`, `SetBluetoothEnabled`, `SetBluetoothAutoPower`, `SetPowerProfile`, `SetTlpEnabled`, `SetThermaldEnabled`, `GpuDetected`.

Network: `SetFirewallEnabled`, `ToggleTcpPort { port, enabled }`, `CustomTcpPortsChanged`, `SetSshEnabled`, `SetSshPort`, `SetSshPasswordAuth`, `SetSshRootLogin(u8)`, `SetFail2banEnabled`, `SetTailscaleEnabled`.

Services: `ToggleService { id, enabled }`.

Apply: `RefreshPreview`, `PreviewReady`, `RequestApply`, `ConfirmApply`, `CancelApply`, `RequestDryRun`, `ApplyFinished { success, message }`.

Generations: `LoadGenerations`, `GenerationsLoaded`, `RequestRollback { generation }`, `ConfirmRollback { generation, mode }`, `RequestDeleteGeneration`, `ConfirmDeleteGeneration`, `RollbackToPrevious`.

Maintenance: `LoadDiskUsage`, `DiskUsageLoaded`, `RequestMaintenance { id }`, `ConfirmMaintenance { id }`.

Helper: `Helper(HelperEvent)` with `HelperOp` including `Apply { rebuild, then_write_state }`.

`Page`: Onboarding, Profiles, Bundles, Packages, System, Hardware, Network, Services, Generations, Maintenance, Apply.

### Spawn / smoke

See DECISIONS C9–C10. Smoke sets `NIXOS_TOOLKIT_SKIP_PRIVILEGED_INIT=1`.

### DoD (implementation tasks)

Always: `cargo build --workspace --all-targets`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace --all-targets`.

Flatpak+smoke: Task 1 and packaging/CI/Phase 3 (DECISIONS C7).

Parity items touched by the task are ticked in this file with a verification note.

DA sign-off + a descriptive commit.

---

## Feature parity checklist

Tick when verified (message-driven test and/or running Flatpak). Notes in backticks.

### Shell

- [ ] Window ~1000×700, title `NixOS Toolkit — {page}`
- [ ] Nav bar, 11 pages, Getting Started selected on launch
- [ ] Compact nav collapse (COSMIC condensed / narrow window)
- [ ] Status banner: Not NixOS / Integrated / Setup required / helper-missing / state-load warning (priority: Not NixOS > helper/state > integration; not dismissible)
- [ ] Toasts 3s (4s in-bundle duplicate)
- [ ] View → About context drawer (intentional: GTK about was unwired)
- [ ] Ctrl+Q quit, Ctrl+R / F5 refresh (local detect only, not ReadState)
- [ ] Header refresh button
- [ ] Theme System / Light / Dark, applied before first frame, JSON prefs

### Getting Started

- [ ] Status row (NixOS / mode / integration / version)
- [ ] Classic vs flake snippet
- [ ] Copy snippet → clipboard + toast
- [ ] Open `/etc/nixos` (host-spawn in Flatpak)
- [ ] Verify Integration re-detects + toast

### Desktop Profiles

- [ ] 13 profiles, exclusive radio
- [ ] Template preview (local)
- [ ] No clear-profile control
- [ ] No invented ARM profile UI

### Software Bundles

- [ ] 16 bundles including `ai-tools` (fallback template OK)
- [ ] Enable switch checks all packages; disable drops `bundle_packages`
- [ ] Per-package checkboxes; empty set does **not** disable bundle
- [ ] Expander chevron (custom widget; expanded ids in model)
- [ ] ARM banner + ArmCompat::None insensitive; others warn
- [ ] Selected-packages summary

### Custom Packages

- [ ] Parser: brackets, commas, newlines, `with pkgs;`, spaces, `pkgs.` / `nixpkgs#`
- [ ] Validation: ASCII ident, 1–128, starts with letter
- [ ] Duplicate toast; in-bundle toast uses **all** bundles
- [ ] Remove; empty placeholder

### System Settings

- [ ] Style combo
- [ ] Hostname (live change; charset; no-op if unchanged vs `/etc/hostname`)
- [ ] DNS comma-separated IPv4 only
- [ ] Username + libvirtd / docker / vboxusers

### Hardware

- [ ] GPU detect (`lspci` or Unknown GPU fallback)
- [ ] NVIDIA widgets **absent** unless detected NVIDIA and not ARM
- [ ] Driver 0–3, modesetting default on, PM default off, open default off
- [ ] Audio PipeWire / PulseAudio / None; low latency
- [ ] Bluetooth + power-on-boot
- [ ] Power profile; TLP; Thermald present but insensitive on ARM
- [ ] **Applied to Nix** (C3), not a state.json toy
- [ ] `bluetooth_enabled` sibling mirrored

### Network

- [ ] Firewall default on; disabling still `has_settings`
- [ ] Preset chips 22/80/443/8080 vs custom TCP split (no double-count)
- [ ] SSH port 1–65535, password, root login 3 options, fail2ban
- [ ] Fail2ban Nix only when SSH on (GTK silent drop)
- [ ] Tailscale + `sudo tailscale up` note
- [ ] No UDP widget; UDP vec still round-trips

### Services

- [ ] 21 switches in GTK groups, including rustdesk and gnome_tweaks
- [ ] Tooltips show NixOS option

### Generations

- [ ] List via helper IPC (no `pkexec nix-env` in GUI)
- [ ] Refresh, Rollback to Previous
- [ ] Switch Now / Set for Next Boot / Cancel (C4 helper activate)
- [ ] Delete confirm
- [ ] Log view
- [ ] Boot-menu info row

### Maintenance

- [ ] 5 allowlisted actions via `RunMaintenance` (exact command strings)
- [ ] Confirm only when `warning` is Some
- [ ] Disk usage via `GetDiskUsage`
- [ ] No raw `pkexec` of nix-collect-garbage from GUI

### Apply

- [ ] Local preview (templates dir; no helper)
- [ ] Three confirm copies: empty / packages-only / normal
- [ ] Empty check: no profile+bundles+packages **and** default hardware
- [ ] Apply: one helper session, EnsureDirectories → Apply → WriteState on success
- [ ] Dry run: DryBuild, no WriteState, no dialog
- [ ] Streaming log, spinner, disable buttons while busy
- [ ] Nav during apply does **not** cancel the session
- [ ] Helper-missing: Apply disabled + banner (no pkexec hang)

### Persistence / helper

- [ ] ReadState on startup (skipped in smoke)
- [ ] WriteState only after successful non-dry Apply, same session
- [ ] `AppState` is the only selection source of truth (no widget scrape)
- [ ] Prefs JSON + cosmic-config mirror
- [ ] Injectable `SpawnSpec`; Flatpak host-probe; no sandbox `Path::exists` for helper

### i18n / a11y / packaging

- [ ] English Fluent catalog complete; missing-key test
- [ ] Icon-only buttons have tooltip + name
- [ ] Flatpak id, desktop, metainfo, SVG icon
- [ ] Templates env inside Flatpak
- [ ] Adwaita (or bundled) icon theme
- [ ] Works outside COSMIC (weston headless smoke)
- [ ] `scripts/verify.sh` from a clean checkout

---

## Risks

| Risk | Mitigation |
|---|---|
| libcosmic git + iced submodule vendoring | Pin SHA; `flatpak-cargo-generator` (not `cargo vendor`); no `--git-tarballs`; commit `cargo-sources.json` |
| wgpu panic on headless weston | `--device=dri`; tiny-skia OK; `WGPU_BACKEND=gl` then software; grep panic |
| pkexec hang in smoke | skip-privileged + host-probe fail-fast (C9/C10) |
| `flatpak-spawn` drops stdin | `--forward-fd=0 --forward-fd=1`; probe in task 1 |
| Polkit `exec.path` is NixOS store path | Host module is the supported helper install |
| Icon names missing on Freedesktop Platform | Adwaita icontheme + text labels |
| Clippy `-D warnings` vs iced churn | Pin SHA; iced types only in `app.rs` / `view/` |
| Hardware generator rewrite footgun | Snapshot tests per driver index / PulseAudio / autopower=false |
| Apply Task cancelled on nav | Session owned by AppModel, not the page |
| `ai-tools` missing template | Keep UI; helper fallback |
| flake GTK leftover | C14: update flake in GTK-deletion task |
| Real nixos-rebuild untested on Fedora | Fake helper + Phase 3 manual NixOS checklist in REPORT.md |
| List generations now pkexec | Accepted (one privileged path); document vs GTK |
| Channel update user vs root | Helper always privileged; document |

---

## Ordered task list

Each task keeps `cargo build` green. Owners only edit owned files. After each: cargo gates, DA sign-off, commit.

### Task 1 — Skeleton that compiles and smokes

Split internally but **one close** when all three slices land.

**1a Architecture**
- rust-version ≥ 1.93; `[lib]` + `[[bin]] name = "nixos-toolkit"`
- Pin libcosmic SHA, features from Frozen contracts
- `lib.rs` / `main.rs` / `app.rs` / `message.rs` (full enum, stubs OK) / `state.rs` / `config.rs` / `i18n.rs` / `helper/spawn.rs` (C9) / `core/apply.rs` returning `Vec<Intent>`
- `Flags::from_env` including `skip_privileged_on_init`
- `flake.nix` GUI package: drop GTK/wrapGApps; helper package unchanged
- No GTK deps in `crates/gui`

**1b UX**
- `view/` 11 empty pages (title + description)
- `widget/` stubs: `status_banner`, `code_view` (can be `text::body` in scrollable)
- Nav model icons + labels via `fl!`
- Menu File/View (Quit, About), header refresh
- `i18n/en/nixos_toolkit.ftl` with chrome + nav + page titles

**1c Packaging**
- `scripts/verify.sh` (cargo always; flatpak/smoke invoked)
- `scripts/build-flatpak.sh`, `smoke-flatpak.sh` (skip-privileged, weston headless)
- Manifest, desktop, metainfo, icon install, templates copy + env
- Adwaita icontheme extension
- `scripts/generate-cargo-sources.sh` with **SHA** of flatpak-builder-tools
- Commit `flatpak/cargo-sources.json`
- `crates/fake-helper` empty JSON echo
- `.gitignore` `.flatpak/`, generator cache
- Smoke must **not** pkexec

**DA:** object if smoke can hang on polkit, if GTK remains, if APP_ID mismatches.

### Task 2 — Testable reducer + prefs

Architecture: `AppModel::apply` handles chrome, theme, nav, toast/dialog dismiss; prefs JSON load/save + cosmic-config mirror; exhaustive `match Message` test module that fails to compile on new variants.

Packaging: `crates/gui/tests/` prefs temp XDG; fake-helper bin speaks `HelperResponse` tags.

UX: toaster + dialog slot wired to model.

### Task 3 — Integration detection + banner + helper session

Architecture: `integration.rs` without GTK; `helper/session.rs` Task stream; init DetectSystem + optional ReadState; helper-missing banner copy from packaging §3.

Packaging: tests skip-privileged prevents spawn; SpawnSpec under `FLATPAK_ID` without host helper does not invoke pkexec.

UX: `status_banner` tones (error/warning/success).

### Task 4 — Onboarding

UX: status row, snippet, three buttons.

Architecture: Copy / Open / Verify intents (C11/C12).

Packaging: flow test helper-absent + NotIntegrated.

### Task 5 — Profiles

UX: radio list + `code_view` preview.

Architecture: `SelectProfile`; local template read.

Packaging: select profile updates state + preview contains profile id.

### Task 6 — Bundles + expander

UX: `widget::bundle_expander`; ARM banner; summary.

Architecture: toggle/enable-all-via-ToggleBundle, package toggle, `expanded_bundles`.

Packaging: enable-all / disable-drops-set / empty-set-keeps-bundle tests.

### Task 7 — Custom packages

Architecture: extract `core/packages.rs` with GTK parser behaviour (including `with pkgs;` and all-bundles duplicate check).

UX: input, list, trash, placeholder.

Packaging: parser table from GTK cases; toast intents for duplicate / in-bundle.

### Task 8 — System settings

UX: appearance, hostname, DNS, groups.

Architecture: live Changed messages; JSON theme save; charset validation matching GTK.

Packaging: prefs + hostname no-op vs current.

### Task 9 — Network + services

UX: all GTK controls except UDP.

Architecture: preset vs custom TCP split; `has_settings` when firewall off; `ToggleService` by id.

Packaging: double-count test; UDP round-trip without UI; 21 service ids.

### Task 10 — Hardware UI

UX: GPU row; NVIDIA absent vs Thermald insensitive; all switches/combos.

Architecture: `GpuDetected`; Option nvidia driver; write both bluetooth fields.

Packaging: ARM/non-NVIDIA → `nvidia_driver is None`.

### Task 11 — HardwareConfig Nix (C3)

Architecture only (common + helper + GUI apply payload):

- `hardware_config` on Generate/Apply (`#[serde(default)]`)
- Rewrite `generate_hardware_nix(&HardwareConfig)`
- Import when non-default
- Old Apply JSON still deserializes

Packaging: snapshots per driver index, PulseAudio, low-latency, autopower=false, TLP vs power-profile mutex; serde old JSON.

### Task 12 — Apply / preview / dry-run

UX: preview, buttons, spinner, log, three dialogs.

Architecture: local preview; session EnsureDirectories → Apply → WriteState; DryBuild skips WriteState; busy ignores second Apply; nav does not cancel.

Packaging: one spawn per Apply+save; dry-run no WriteState; three copy variants; nav-during-apply.

### Task 13 — Generations (C4)

Architecture: helper list/rollback/delete; `activate` field; no `Command::new("pkexec")` in gui except SpawnSpec.

UX: list, refresh, rollback previous, three-button switch, delete, log, boot info.

Packaging: `rg pkexec crates/gui` is spawn-spec only; Boot vs Switch requests differ.

### Task 14 — Maintenance

Architecture: `RequestMaintenance { id }` → allowlist command; disk usage session.

UX: five actions, confirm when warning, disk rows, log. Do not print `$ pkexec` if the session is helper IPC (print the command string).

Packaging: exact allowlist strings; reject unknown.

### Task 15 — Integration flows + CI

Packaging: remaining `crates/gui/tests` flows; `.github/workflows/verify.yml` (cargo job always; flatpak job on this branch); `rg` that gui does not spawn `nix-env`.

UX: Fluent catalog complete; missing-key test green.

Architecture: clippy clean; `AppState::to_apply_request` includes hardware.

### Task 16 — Flatpak polish

Packaging: icon theme verified in smoke log or runtime; templates env; metainfo screenshot placeholder OK; regenerate cargo-sources if lock changed.

DA: attempt helper-absent paths, missing icons, clipboard, about, every nav page.

### Phase 3 — Harden

DA leads a full parity-checklist walk against the running Flatpak, tries to break every flow, files failures as new tasks. Repeat until clean.

Lead writes `docs/migration/REPORT.md`: what was built, deviations, limitations, build/run/install instructions.

---

## Phase 2 process

1. Teammate states a 3–8 line plan in the task log (`docs/migration/TASKLOG.md`).
2. DA may object with evidence before edits.
3. Owners edit only their files; split if needed.
4. Cargo gates; packaging runs Flatpak when the task list says so.
5. Tick checklist items with verification notes.
6. DA sign-off.
7. Lead commits with a conventional message (`feat:`, `fix:`, `docs:`, `chore:`, `test:`). No AI attribution.

Disagreements: one evidence exchange, then lead using the priority order, then a DECISIONS.md row.

---

## Out of scope

- Inventing UDP / WireGuard UI
- Adding `ai-tools.nix` template
- Unifying GUI vs helper integration-detection strings
- Flathub publication
- Binary cache / Cachix
- Claiming ATK-level a11y parity
- Keeping a GTK GUI crate alongside libcosmic

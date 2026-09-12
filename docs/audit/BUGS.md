# Bugs and completeness issues (reconnaissance)

Severity: **P0** crash/security/corruption, **P1** major broken feature, **P2** normal bug, **P3** polish.

Evidence paths are from the workspace root. These are confirmed from code paths unless marked *suspected*.

---

## P1 — major broken or dangerous behavior

### BUG-001 — NVIDIA detection silently selects proprietary drivers and makes Apply non-empty

- **Severity:** P1
- **Evidence:** `crates/gui/src/core/apply.rs:317–327` (`GpuDetected` sets `nvidia_driver = Some(0)`), `819–830` (`hardware_for_nix` injects stable if GPU string contains `nvidia`), test `nvidia_gpu_apply_defaults_unwritten_driver_to_stable` (`1578–1596`) asserts `!apply_is_empty()`.
- **Expected:** Hardware stays at catalog default until the user chooses a driver. Empty Apply should remain empty on a stock NVIDIA box.
- **Actual:** Startup `DetectGpu` writes Stable NVIDIA into state. Preview/Apply emit `hardware.nix` with proprietary `nvidia` + default PipeWire (see BUG-002). User never opened Hardware.
- **Verify:** On x86_64 with NVIDIA in `lspci`, launch with empty `state.json`, open Apply without touching Hardware. Preview contains `services.xserver.videoDrivers` / `hardware.nvidia`. Confirm dialog is the normal (non-empty) one.

### BUG-002 — Any non-default hardware field writes default PipeWire into `hardware.nix`

- **Severity:** P1
- **Evidence:** `crates/common/src/nix.rs:599–641` (`audio_server` `_` arm always emits PipeWire). Default `audio_server` is `0` (`ipc.rs:501`). `docs/reference.md` admits bluetooth-only apply enables PipeWire.
- **Expected:** Bluetooth/TLP/NVIDIA-only changes should not enable/disable the host audio stack unless the user set Audio Server.
- **Actual:** `generate_hardware_nix` always includes an audio block. Enabling Bluetooth (or auto-NVIDIA from BUG-001) can fight an existing PulseAudio/PipeWire setup in `configuration.nix` (no `mkDefault`).
- **Verify:** Enable only Bluetooth; preview `hardware.nix` contains `services.pipewire.enable = true`.

### BUG-003 — Empty Apply deletes previously managed profiles, bundles, and snippets

- **Severity:** P1 (data loss of toolkit-managed Nix; warned in dialog)
- **Evidence:** Dialog `apply-empty-body` (`nixos_toolkit.ftl:320–321`). Confirm still calls `start_rebuild` (`apply.rs:436–447`). Helper `cleanup_stale_managed_files` (`nix_gen.rs:256–264, 342–367`) deletes unused `profiles/*.nix`, `bundles/*.nix`, and all unmanaged snippets.
- **Expected:** Empty Apply should no-op or require a dedicated “reset managed config” action; Dry Run should not be the only non-destructive path.
- **Actual:** Confirming empty Apply writes a no-import `selected.nix` and removes prior toolkit files, then runs `nixos-rebuild`. Combined with BUG-001, a “fresh” NVIDIA machine may skip the empty warning entirely.
- **Verify:** Apply a profile, then clear by… (cannot clear profile in UI; use empty state after reconstruct fail) or start with no selections on a tree that still has files; confirm empty Apply; files under `/etc/nixos/nixos-toolkit/{profiles,bundles,state}` except `selected.nix`/`state.json` disappear.

### BUG-004 — Maintenance, disk usage, and generation list time out after 60s with little UI feedback

- **Severity:** P1
- **Evidence:** `crates/gui/src/helper/session.rs:283–293` — `ListGenerations`, `GetDiskUsage`, `RunMaintenance` timeout **60s**. `Timeout` handler (`apply.rs:568–576`) only special-cases `ReadState`. Maintenance uses `Command::output()` (`commands.rs:1434`) so the helper cannot stream. `du -sh /nix/store` (`commands.rs:1458`) and `nix-collect-garbage -d` / `nix-store --verify --check-contents` routinely exceed 60s.
- **Expected:** Long store operations run to completion with live logs, or show a timeout error toast.
- **Actual:** GUI marks busy Idle, no toast. Disk UI stays on “Loading”. Generations show “No generations found” / “installation issue”. Maintenance appears to hang then silently stop.
- **Verify:** On a large `/nix/store`, open Maintenance (disk) or run Verify Store / Delete Old Generations; wait 60s.

### BUG-005 — Flake rebuild attribute is `/etc/hostname`, not the flake output name

- **Severity:** P1
- **Evidence:** `crates/helper/src/rebuild.rs:29–36` — `nixos-rebuild <arg> --flake /etc/nixos#{hostname}` with hostname from `/etc/hostname` defaulting to `nixos`.
- **Expected:** Use the host’s flake output (or `nixos-rebuild --flake /etc/nixos` default) so Apply works on common `nixosConfigurations.<name>` setups where name ≠ hostname.
- **Actual:** Apply fails on flakes named `mysystem` / `laptop` when hostname differs. Classic mode is unaffected.
- **Verify:** Flake `nixosConfigurations.desktop` with hostname `desktop-pc`; Apply Switch; rebuild cannot find attr.

### BUG-006 — Helper non-JSON stdout is dropped; session waits until timeout

- **Severity:** P1
- **Evidence:** `crates/gui/src/helper/client.rs:62–72` — parse failure logs and **continues**; no Error event. Apply has **no** recv timeout (`session.rs:288`).
- **Expected:** Malformed helper output becomes `HelperResponse::Error` / SpawnFailed and unblocks the UI.
- **Actual:** pkexec password text or a tracing leak on stdout can freeze Apply (`Busy::Applying`) indefinitely.
- **Verify:** Point `NIXOS_TOOLKIT_HELPER` at a script that prints `hello\n` then valid JSON; Apply stays busy.

### BUG-007 — Bundle package checkboxes do not control module stubs

- **Severity:** P1 (misleading configuration)
- **Evidence:** `bundle_module_stub` (`nix.rs:894–955`) always enables docker (devtools), steam+gamemode (gaming), libvirtd (virtualization), virtualbox host, podman, flatpak. GUI enable always fills `bundle_packages` so templates are **not** copied (`nix_gen.rs:168–186`).
- **Expected:** Unchecking Steam/Docker in the expander disables those NixOS modules.
- **Actual:** Unchecking only drops the package attr; Steam/Docker/libvirt still enable. Enabling devtools always starts Docker on boot (`enableOnBoot = true`) even if the docker package is off.
- **Verify:** Enable Gaming, uncheck `steam`, Apply/preview; `programs.steam.enable = true` remains.

---

## P2 — normal bugs / incomplete wiring

### BUG-008 — Custom package input silently drops invalid names; `field_errors.packages` is dead

- **Severity:** P2
- **Evidence:** `packages.rs:112–126` requires leading ASCII letter (rejects `_1password-gui`, `1password`). `AddPackagesFromInput` (`apply.rs:153–198`) returns no toast if parse is empty. `FieldErrors.packages` (`app.rs:109`) is never written.
- **Expected:** Show `error-port`-style validation; accept catalog ids that start with `_`.
- **Actual:** Add does nothing. Catalog 1Password id cannot be added as a custom package.
- **Verify:** Type `_1password-gui` or `123abc`; press Add; no toast, no list row, no error.

### BUG-009 — Catalog packages cannot be added as custom even when the bundle is disabled

- **Severity:** P2
- **Evidence:** `classify_new_packages` (`packages.rs:148–172`) maps **all** bundles. Toast `toast-in-bundle`; package not added. Test `classify_in_bundle_uses_all_bundles_not_only_enabled`.
- **Expected:** Block only if that bundle is enabled, or add anyway with a warning.
- **Actual:** `git` / `htop` / `firefox` can never be custom packages.
- **Verify:** Packages page, add `git` with no bundles enabled.

### BUG-010 — Custom TCP/UDP parse errors are only `tracing::debug`

- **Severity:** P2
- **Evidence:** `apply.rs:338–355`. Network view has no `field_errors` for ports (`view/network.rs:107–131`). DNS/hostname do surface errors.
- **Expected:** Same live error as DNS (`error-port-invalid` exists in FTL).
- **Actual:** User can type `abc`; field keeps garbage; Apply uses last successful parse.
- **Verify:** Type `notaport` in Additional TCP Ports; no error row; Apply preview still old ports.

### BUG-011 — WireGuard listen-port / disable leaks UDP allows

- **Severity:** P2
- **Evidence:** `apply.rs:387–416` comments: open new port; **do not remove** previous auto-added port; disable does not close UDP.
- **Expected:** Enabling WG opens listen UDP; changing port moves it; disabling WG removes the auto-opened port (leave user chips).
- **Actual:** 51820 stays open after disable or after changing to 51821 (both open).
- **Verify:** Enable WG, change port to 51821, disable WG; `allowedUDPPorts` still contains 51820/51821.

### BUG-012 — Profile preview not restored after ReadState

- **Severity:** P2
- **Evidence:** `HelperResponse::State` (`apply.rs:582–592`) does not emit `LocalProfilePreview`. `nav_intents` (`779–789`) does not handle `Page::Profiles`.
- **Expected:** Saved profile shows template preview on Profiles.
- **Actual:** Placeholder `# Select a profile to see preview` until the user clicks the already-selected radio.
- **Verify:** Apply gnome, restart GUI; open Profiles.

### BUG-013 — Header Refresh does not reload privileged `state.json`

- **Severity:** P2
- **Evidence:** `Message::RefreshSystem` (`apply.rs:36`) only `DetectSystem` + `DetectGpu`. Comment in `message.rs:18`.
- **Expected:** Users treat Refresh like “reload configuration”.
- **Actual:** External helper/state edits and generation list are stale; GPU path may re-trigger BUG-001.
- **Verify:** Change `state.json` on disk; press header refresh; GUI selections unchanged.

### BUG-014 — Generations Refresh remains clickable when helper is missing

- **Severity:** P2
- **Evidence:** `view/generations.rs:20–21` enables Refresh when `idle` only. Rollback/apply correctly require `!helper_missing`. `load_generations_intents` then no-ops (`apply.rs:876–877`). Same pattern: Maintenance disk refresh (`maintenance.rs:79–81`).
- **Expected:** Disabled + helper-missing tooltip.
- **Actual:** Click does nothing.
- **Verify:** Launch without helper; Generations → Refresh.

### BUG-015 — ReadState close/timeout always shows “State read timeout”

- **Severity:** P2
- **Evidence:** `HelperEvent::Closed` for `ReadState` (`apply.rs:551–554`) sets `banner-state-timeout` even when error is `helper stdout closed`. Distinct FTL `banner-state-helper-comm` is unused.
- **Expected:** Distinguish spawn fail, parse/error, EOF, true timeout.
- **Verify:** Kill helper during ReadState; banner says timeout.

### BUG-016 — WriteState failure after a successful rebuild is a log line only

- **Severity:** P2
- **Evidence:** `session.rs:188–228` — WriteState errors/timeouts `push_log` warning; still forwards `ApplyComplete { success: true }`. GUI `mark_applied` (`apply.rs:461–463`).
- **Expected:** Toast that Nix was applied but UI state was not saved; keep `has_changes`.
- **Actual:** Success toast; next launch may reconstruct incompletely (copied templates omit `bundle_packages`).
- **Verify:** Make `state.json` unwritable after files are written (or fake-helper script Ok for Apply, Error for WriteState).

### BUG-017 — Test/Build rebuild modes persist managed files (and Test writes state)

- **Severity:** P2
- **Evidence:** `then_write_state` true for Test and Build (`apply.rs:439–445`). Helper Apply always `generate_all_files(..., dry_run: false)` except DryBuild snapshot restore (`commands.rs:260–335`). FTL: “Test (temporary activation)”, “Build only”.
- **Expected:** Test = activate without persisting toolkit files/state; Build = evaluate without writing `/etc/nixos/nixos-toolkit`.
- **Actual:** Both mutate the managed tree. Test activation is temporary; files are not. Next CLI rebuild/boot of a new generation will see them.
- **Verify:** Test apply; inspect `/etc/nixos/nixos-toolkit`; reboot — running system reverts, files remain.

### BUG-018 — NVIDIA “Open Source (nvidia-open)” combo does not set `hardware.nvidia.open`

- **Severity:** P2
- **Evidence:** Combo index 2 → `nvidiaPackages.latest` (`nix.rs:576–579`). Separate toggler `SetNvidiaOpen` → `open = {nvidia_open}`. Default `nvidia_open` is false.
- **Expected:** Choosing nvidia-open sets `open = true` (or hides the redundant toggler).
- **Actual:** Latest proprietary-capable package with `open = false`.
- **Verify:** Hardware → Open Source (nvidia-open); preview `open = false` unless the other switch is on.

### BUG-019 — Printing service unconditionally enables Avahi

- **Severity:** P2
- **Evidence:** `generate_services_nix` printing arm (`nix.rs:713–717`) sets `services.avahi.enable` / `nssmdns4`. Reconstruct (`commands.rs:1106–1113`) may mark avahi enabled.
- **Expected:** Printing toggle ≠ Avahi toggle.
- **Actual:** Disable Avahi in UI, leave Printing on; next apply still enables Avahi.
- **Verify:** Printing on, Avahi off; preview both avahi stanzas.

### BUG-020 — Users groups require `state.username`; UI can show `$USER` without persisting it

- **Severity:** P2
- **Evidence:** View (`system.rs:18–20`) displays `state.username` **or** `USER`. Generation (`nix.rs:162–164`, `nix_gen.rs:108–120`) needs both username and groups. `seed_username_from_host` runs in production but not `skip_host_probes`; `has_changes` is not set when seeding (`app.rs:208–216`).
- **Expected:** Displayed username is what Apply writes.
- **Actual:** Groups-only `apply_is_empty` is false (`state.rs:740–742`) but no `users.nix` if username is `None`.
- **Verify:** Tests with `skip_host_probes`; toggle docker group without typing username; preview lacks `users.nix`.

### BUG-021 — `has_changes` / `last_applied` unused in the product UI

- **Severity:** P2 (completeness)
- **Evidence:** `has_changes` never read in `view/`. `to_ipc_state` sets `last_applied: None` (`state.rs:363`). Successful apply only `mark_applied` (clears dirty flag).
- **Expected:** Dirty indicator; timestamp in state.json / Apply page.
- **Actual:** Dead fields. Old `last_applied` values are wiped on WriteState.
- **Verify:** Apply; read `state.json` `last_applied`.

### BUG-022 — Open `/etc/nixos` / URLs swallow errors

- **Severity:** P2
- **Evidence:** `app.rs:633–647` — `open_path` / `open::that_detached` failures `tracing::warn` then `Action::None`.
- **Expected:** Toast on failure.
- **Verify:** Flatpak without `xdg-open` on host; click Open /etc/nixos.

### BUG-023 — Clipboard failure path is unreachable

- **Severity:** P2
- **Evidence:** `app.rs:627–631` maps clipboard write to `ClipboardCopied { ok: true }` only. `ok: false` branch (`apply.rs:71–79`) unused. FTL `toast-clipboard-failed`.
- **Expected:** Report clipboard errors.
- **Verify:** Code inspection; no UI path.

### BUG-024 — Duplicate Fluent catalogs and unused strings

- **Severity:** P2/P3
- **Evidence:** `crates/gui/i18n/en/gui.ftl` and `nixos_toolkit.ftl` are duplicates. Unused: `menu-file`, `menu-refresh`, `page-onboarding` (vs `page-onboarding-title`), `banner-state-helper-comm`, `apply-complete`, `apply-failed`.
- **Expected:** One English catalog; every key referenced or removed.
- **Verify:** Diff the two FTL files; search `fl!("menu-file")`.

### BUG-025 — View menu omits Refresh despite keybind and FTL

- **Severity:** P2
- **Evidence:** `app.rs:402–412` menu items About + Quit only. `MenuAction::Refresh` and Ctrl+R / F5 exist (`326–347`, `533–550`).
- **Expected:** View → Refresh.
- **Verify:** Open View menu.

### BUG-026 — `UpdateConfig` is never produced

- **Severity:** P2
- **Evidence:** `message.rs:24`, reducer `apply.rs:67–70`. No cosmic-config watch / inotify in `subscription()`.
- **Expected:** External prefs edits apply live (comment says “cosmic-config / prefs file changed”).
- **Verify:** Edit `preferences.json` while running.

### BUG-027 — Intent session-chaining stubs are no-ops

- **Severity:** P2 (dead API; not user-facing if unused)
- **Evidence:** `app.rs:600–602` `Intent::SendOnSession` / `CloseSession` comment “Session chaining is task 12.” Real chain is `session.rs`.
- **Expected:** Remove stubs or implement.
- **Verify:** Grep constructors of those intents (none from reducer).

### BUG-028 — Helper IPC used only from backend: Validate / Generate / CheckPermissions / GetSystemInfo

- **Severity:** P2 (completeness; documented)
- **Evidence:** `docs/architecture.md:96`, `helper/main.rs:79–114`, GUI spawn sites only ReadState/Apply/ListGenerations/Rollback/Delete/RunMaintenance/GetDiskUsage.
- **Expected:** Either wire Validate before Apply, or drop from the frozen Message/`HelperOp` surface to avoid a false “implemented” API.
- **Actual:** Invalid hostname in `state.json` is not caught by helper `validate()` (which also uses Unicode `is_alphanumeric`, unlike the GUI ASCII check — `commands.rs:167`).
- **Verify:** Hand-edit state hostname to `foo.bar`; Apply writes `hostname.nix` without Validate.

### BUG-029 — Generation `config_rev` never populated; rollback/delete are not streamed

- **Severity:** P2
- **Evidence:** `parse_generation_line` (`commands.rs:1261–1268`) sets `config_rev: None`. Rollback/delete `Command::output()` (`1297–1372`).
- **Expected:** Flake rev in subtitle; live log like `nixos-rebuild`.
- **Verify:** Generations row never shows rev; rollback log is two GUI lines.

### BUG-030 — Disk usage `error` is not shown (except empty size)

- **Severity:** P2
- **Evidence:** `DiskUsageInfo.error` (`ipc.rs:211`). View (`maintenance.rs:54–70`) uses `store_size` empty → `maintenance-disk-error`; ignores `error` text; no log write.
- **Expected:** Surface `du failed` / `nix-env failed` in the maintenance log.
- **Verify:** Run without `du`; UI “Unknown” or “Error - see log” with empty log.

### BUG-031 — `code_view` does not follow the log tail

- **Severity:** P2
- **Evidence:** `widget/code_view.rs` scrollable fixed height; no `scroll_to` / sticky bottom. Migration UX asked for auto-scroll (`docs/migration/ux.md`).
- **Expected:** Apply/maintenance/generations logs stick to latest line.
- **Verify:** Long dry-run; scrollbar stays at top.

### BUG-032 — Preview is stale if the user never re-enters Apply

- **Severity:** P2
- **Evidence:** `nav_intents` LocalPreview only for `Page::Apply`. `RefreshPreview` is a button. No preview on every state mutation.
- **Expected:** Preview tracks current state while on the Apply page, or warn it is a snapshot.
- **Actual:** If iced keeps the page mounted… nav away and back refreshes. If user could split-view they cannot. Mostly OK; Refresh still required after in-page… Apply page has no other editors. **Lower if only reachable via Refresh.** Still stale if they click Refresh Preview then change rebuild type only (rebuild type is on Apply — preview Nix does not include rebuild type). Low impact.
- **Verify:** N/A high; keep as polish if preview is regenerated only on nav.

### BUG-033 — TLP and power-profile dropdown can both be on; Nix ignores PPD

- **Severity:** P2
- **Evidence:** UI no mutex (`view/hardware.rs:155–168`). Generator (`nix.rs:656–683`) TLP wins and sets `power-profiles-daemon.enable = false`.
- **Expected:** Disable power-profile combo when TLP is on (GTK mutex was discussed in migration notes).
- **Verify:** Enable TLP + Performance; preview has TLP only.

### BUG-034 — Misleading copy: Syncthing “user service”, Docker group “rootless”

- **Severity:** P2
- **Evidence:** `service-syncthing-desc` FTL; `generate_services_nix` `services.syncthing.enable`. System action `docker_user` description “rootless Docker” (`actions.rs:732–737`).
- **Expected:** System service / docker socket group.
- **Verify:** Read UI strings vs generated Nix.

### BUG-035 — Science ARM note mentions RStudio; catalog has no RStudio

- **Severity:** P2
- **Evidence:** `actions.rs:614` `arm_note`: “RStudio binary not available; Julia has limited ARM support”. Packages: octave, julia, gnuplot only.
- **Expected:** Note matches catalog.
- **Verify:** Bundles page on ARM.

### BUG-036 — Bundle expander allows package toggles while the bundle is disabled

- **Severity:** P2
- **Evidence:** Expand button only needs `sensitive` (`bundle_expander.rs:59–60`). Disable does not collapse. `ToggleBundlePackage` creates an empty map if missing (`apply.rs:137–142`). Disabled bundles are omitted from Apply (`nix_gen.rs:45–48`) but `bundle_packages` can linger if disable didn’t run (toggle packages after disable: disable **does** `bundle_packages.remove`). After disable, expander still expanded; toggling a checkbox **re-inserts** `bundle_packages` without `enabled_bundles`.
- **Expected:** Ignore package toggles when disabled, or disable checkboxes.
- **Actual:** Summary “Selected Packages” can list ids for a disabled bundle (`bundles.rs:86–94` iterates all maps).
- **Verify:** Enable, expand, disable, check a package; summary shows it; Apply omits the bundle.

### BUG-037 — Network.nix / hardware.nix / services.nix assignments are not `mkDefault`

- **Severity:** P2
- **Evidence:** Hostname/DNS use `lib.mkDefault` (`nix.rs:281–320`). Firewall/SSH/PipeWire/NVIDIA do not. Enabling any network setting writes `networking.firewall.enable` unconditionally (`nix.rs:496–502`).
- **Expected:** Same precedence story as hostname, or a loud warning.
- **Actual:** Toolkit can override or collide with user `configuration.nix`.
- **Verify:** Set `networking.firewall.enable = false` in user config; enable SSH in toolkit; eval conflict/`mkForce` needed.

### BUG-038 — Polkit policy `exec.path` vs `/run/current-system/sw/bin` (*suspected*)

- **Severity:** P2
- **Evidence:** Helper package substitutes policy to `$out/bin/nixos-toolkit-helper` (`flake.nix:154–159`). GUI wrapper sets `NIXOS_TOOLKIT_HELPER` to that store path (`flake.nix:133–136`) — **OK for `nixos-toolkit` from the module**. Unpackaged/cargo GUI discovers `/run/current-system/sw/bin/nixos-toolkit-helper` (`spawn.rs:8–11`). pkexec matches `exec.path` exactly.
- **Expected:** Policy accepts the path the GUI actually execs (or `pkexec --action-id`).
- **Actual:** Cargo/dev launches may get a generic pkexec action or denial. Three actions share one path (`data/polkit/org.nixos-toolkit.helper.policy`).
- **Verify:** `cargo run -p gui` on a module-installed host; Apply; `pkexec` path vs `grep exec.path` on the installed policy.

### BUG-039 — NixOS module does not install a polkit extraConfig snippet; metainfo still “skeleton”

- **Severity:** P2/P3
- **Evidence:** `flake.nix:269–275` only `security.polkit.enable` + `systemPackages`. Relies on `share/polkit-1/actions` from the helper package. Metainfo `flatpak/...metainfo.xml:34–37` “Initial libcosmic skeleton.”
- **Expected:** Module explicitly wires policy; Appstream describes current features.
- **Verify:** Read module; gnome-software Flatpak listing.

### BUG-040 — `can_write_managed_dir` is dead and wrong on Unix

- **Severity:** P2 (dead code)
- **Evidence:** `integration.rs:184–197` uses `Permissions::readonly()` (not write bit / euid). Never called.
- **Expected:** Remove or use a real probe (helper `CheckPermissions`).
- **Verify:** Grep callers.

### BUG-041 — Helper `validate` hostname charset ≠ GUI

- **Severity:** P2
- **Evidence:** GUI `hostname_charset_error` ASCII alphanumeric + `-` (`apply.rs:1068–1078`). Helper `h.chars().all(|c| c.is_alphanumeric() || c == '-')` (`commands.rs:167`) allows Unicode. Apply never calls Validate.
- **Expected:** One charset.
- **Verify:** Unit vs helper validate with `höst`.

### BUG-042 — IPv6 DNS cannot be set

- **Severity:** P2 (limitation vs “DNS servers”)
- **Evidence:** `is_ipv4` only (`state.rs:659–682`). Docs say IPv4. UI placeholder `1.1.1.1, 8.8.8.8` does not mention IPv6.
- **Expected:** Accept IPv6 or say IPv4-only in the field description (format note is comma-separated, not IPv4-only — `system-dns-format-desc`).
- **Verify:** Enter `2606:4700:4700::1111`.

### BUG-043 — `HelperResponse::{Permissions,SystemInfo,ValidationResult,GenerationResult}` swallowed

- **Severity:** P2
- **Evidence:** `apply_helper_response` `_ =>` (`apply.rs:770–775`) ignores unknown terminals (sets helper Idle unless Apply).
- **Expected:** If a future helper sends them, surface or log.
- **Verify:** fake-helper script `Permissions` for ReadState; GUI ignores.

### BUG-044 — `Message::AddCustomPackages` / `ClearProfile` have no controls

- **Severity:** P2 (completeness; ClearProfile documented as test-only)
- **Evidence:** `message.rs:37, 58`. Profiles radios cannot deselect.
- **Expected:** Way to clear desktop profile without empty-apply nuking everything.
- **Verify:** Profiles page; cannot uncheck.

### BUG-045 — Unfree allowlist misses some catalog unfree attrs (*suspected*)

- **Severity:** P2
- **Evidence:** `attr_needs_unfree` (`nix.rs:958–974`) list. Catalog also has `brave`, `veracrypt`, `google-chrome` (listed), `vscode` (listed). `heroic`/`lutris`/`steam` covered via gaming bundle short-circuit. `bitwarden-desktop` free. `nvidia` covered via driver. Unknown unfree custom packages fail eval (`docs/reference.md` limitation 5).
- **Expected:** Custom `spotify` etc. get `unfree.nix` or a GUI warning.
- **Actual:** Apply proceeds; nixos-rebuild fails later.
- **Verify:** Add `spotify`; preview has no `unfree.nix`; rebuild errors.

### BUG-046 — `selected.nix.template` is unused

- **Severity:** P3/P2 completeness
- **Evidence:** `docs/development.md` — template exists, generation uses format strings in `nix.rs`. `nix/templates/state/selected.nix.template`.
- **Expected:** Use or delete.
- **Verify:** Grep `selected.nix.template` in Rust (no hits).

### BUG-047 — Flake template copy uses `2>/dev/null || true`

- **Severity:** P2
- **Evidence:** `flake.nix:111–113` `cp -r ... 2>/dev/null || true`.
- **Expected:** Fail the build if templates missing.
- **Verify:** Empty templates dir; `nix build .#templates` still succeeds.

### BUG-048 — DismissToast recreates the entire toaster

- **Severity:** P2
- **Evidence:** `app.rs:562–564` `self.toasts = widget::Toasts::new(...)`.
- **Expected:** Dismiss one toast.
- **Actual:** May drop stacked toasts (in-bundle + added).
- **Verify:** Add a package that is both duplicate and in-bundle in one submit (`git, git` vs `git, htop`); dismiss one toast.

### BUG-049 — `RequestApply` / privileged actions silently no-op when busy

- **Severity:** P3/P2
- **Evidence:** `apply.rs:430–432`, `845–847`. No toast “already running”.
- **Verify:** Click Apply twice quickly.

### BUG-050 — Profiles page ignores `arm_compat` / `arm_note`

- **Severity:** P2
- **Evidence:** `view/profiles.rs` no ARM handling; bundles/hardware have banners.
- **Verify:** aarch64; all 13 profiles look fully supported.

---

## P3 — polish / dead surface

### BUG-051 — English-only i18n; catalog names hardcoded English

- **Evidence:** `docs/reference.md` limitation 1; `actions.rs` name strings.
- **Verify:** `LANG=de_DE.UTF-8`; UI still English except maybe libcosmic chrome.

### BUG-052 — About / metainfo screenshot is the SVG icon

- **Evidence:** `metainfo.xml:26–30`.
- **Verify:** Flathub/Appstream screenshot.

### BUG-053 — `install.sh` desktop icon is `preferences-system`, not the app SVG

- **Evidence:** README + `install.sh` (documented).
- **Verify:** User-level desktop file.

### BUG-054 — No `--help` / CLI; env vars undocumented in `--help` (none)

- **Evidence:** `crates/gui/src/main.rs` no clap. Env listed in docs/reference only.
- **Verify:** `nixos-toolkit --help`.

### BUG-055 — `page-onboarding` vs `page-onboarding-title` duplicate keys

- **Evidence:** FTL lines 21 vs 33.
- **Verify:** Grep `fl!("page-onboarding")` — unused.

### BUG-056 — Rebuild log placeholder vs live `code_view` wrapping

- **Evidence:** Long Nix attr paths wrap (`Wrapping::WordOrGlyph`); harder to copy as “code”.
- **Verify:** Preview pane.

### BUG-057 — Helper tracing directive `helper=info` vs binary crate name

- **Evidence:** `helper/src/main.rs:23` `helper=info`. Package may log as `nixos_toolkit_helper`.
- **Verify:** `RUST_LOG` default; helper stderr volume.

### BUG-058 — `GpuDetected` does not set `has_changes` while mutating driver

- **Evidence:** `apply.rs:317–327` vs other hardware setters.
- **Verify:** Related to BUG-001; dirty flag stays false after auto NVIDIA.

### BUG-059 — `seed_username_from_host` does not set `has_changes`

- **Evidence:** `app.rs:208–216`. Usually correct (not a user edit). Combined with WriteState, username appears in state.json after apply even if user never touched System.
- **Verify:** Apply with only a profile; `state.json` username = `$USER`.

### BUG-060 — Three polkit actions for one binary

- **Evidence:** `org.nixos-toolkit.helper.policy` manage-system / write-config / rebuild. rebuild `allow_active` is `auth_admin` (no keep); others `auth_admin_keep`. pkexec picks by path, not GUI op.
- **Expected:** One action, or `--action-id` per op so rebuild can require a fresh password.
- **Verify:** Apply twice; auth_admin_keep vs rebuild annotation unused.

---

## Counts check (CLAUDE.md)

| Claim | Actual | Notes |
|---|---|---|
| 13 profiles | 13 | OK |
| 16 bundle defs | 16 | OK |
| 15 bundle templates | 15 + missing `ai-tools.nix` | OK |
| 21 service toggles | 21 | OK |
| 13 HelperRequest | 13 | 4 unused by GUI |
| 5 system actions | 5 in catalog; UI 3 groups + hostname/DNS widgets | OK |
| 5 maintenance catalog | 5 | OK |
| 6 allowlist strings | 6 | OK |
| 11 nav pages | 11 | OK |
| Apply dropdown 4 + Dry Run | OK | Test/Build semantics BUG-017 |
| Network wireguard fields | present | BUG-011 |

---

## Suggested verification order

1. NVIDIA host: launch → Apply preview (BUG-001/002).
2. Large store: Maintenance disk + GC (BUG-004).
3. Flake hostname mismatch (BUG-005).
4. Enable Gaming, uncheck steam, preview (BUG-007).
5. Packages: `_1password-gui`, `git` (BUG-008/009).
6. Empty-apply on a previously applied tree (BUG-003) — destructive.
7. WireGuard port change (BUG-011).
8. Restart GUI with saved profile (BUG-012).

# Feature matrix (reconnaissance)

Counts from current code (matches `CLAUDE.md` unless noted):

| Catalog | Count | Source |
|---|---|---|
| Desktop profiles | 13 | `default_profiles()` + `nix/templates/profiles/*.nix` |
| Bundle definitions | 16 | `default_bundles()` |
| Bundle template files | 15 | `nix/templates/bundles/` (`ai-tools` has none) |
| Service toggles | 21 | `ServicesConfig` / `view/services.rs` |
| System catalog actions | 5 | `default_system_actions()` (UI uses 3 group toggles + dedicated hostname/DNS) |
| Maintenance catalog actions | 5 | `default_maintenance_actions()` |
| Maintenance allowlist strings | 6 | `ALLOWED_MAINTENANCE_COMMANDS` |
| Nav pages | 11 | `Page::ALL` |
| `HelperRequest` variants | 13 | `crates/common/src/ipc.rs` |
| Apply rebuild dropdown | 4 + Dry Run button | Switch / Boot / Test / Build + `DryBuild` |

Status: **working** (end-to-end wired), **partial** (UI or backend incomplete / misleading), **broken** (user action fails or silently no-ops), **missing** (declared/docs/IPC/UI string but no path).

---

## Chrome, lifecycle, i18n

| feature | where exposed | expected behavior | implementation path | status | test coverage | verification method |
|---|---|---|---|---|---|---|
| Window + 11-page nav | sidebar `nav_bar` | Switch pages; title `NixOS Toolkit — <page>` | `app.rs` `Page::ALL` → `view/mod.rs` → `NavSelect` | working | `apply.rs` nav tests | Launch GUI; click each nav item |
| About drawer | View → About | Context drawer with version, license, links | `ToggleAbout` → `context_drawer::about` | working | reducer match exhaustive | View → About; click repository/issues |
| Quit | View → Quit, Ctrl+Q | Exit | `Message::Quit` → `Intent::Exit` | working | reducer | Ctrl+Q / menu Quit |
| Refresh host probes | header refresh, F5, Ctrl+R | Re-run `detect_system` + `detect_gpu` (not ReadState) | `RefreshSystem` → `DetectSystem`/`DetectGpu` | partial | unit: GpuDetected | F5; banner/onboarding status updates; saved toolkit state does **not** reload |
| Menu Refresh item | FTL `menu-refresh`, `MenuAction::Refresh` | Refresh from View menu | Keybind exists; **menu items are only About + Quit** | missing | none | View menu has no Refresh; header button works |
| Theme System/Light/Dark | System page | Persist prefs; apply libcosmic theme | `ColorSchemeChanged` → `SavePrefs` + `ApplyTheme` | working | `config.rs` JSON round-trip | Change style; restart; prefs file `~/.config/nixos-toolkit/preferences.json` |
| Live prefs reload | `Message::UpdateConfig` | Reload when prefs file/cosmic-config changes | Reducer applies theme; **no cosmic-config/fs subscription** | missing | reducer smoke only | Edit `preferences.json` while running; UI does not update |
| Toasts | apply/packages/maintenance | Show then dismiss | `Intent::ShowToast` / `DismissToast` | working | apply.rs toast tests | Add a package; toast appears |
| Clipboard failure toast | `ClipboardCopied { ok: false }` | Show `toast-clipboard-failed` | Copy always maps `ok: true` | missing | none | No path produces `ok: false` |
| English Fluent strings | `i18n/en/*.ftl` | All user-visible copy localized | `fl!` + `i18n-embed` | partial | none for unused keys | Duplicate `gui.ftl` + `nixos_toolkit.ftl`; several keys unused (`menu-file`, `apply-complete`, `banner-state-helper-comm`) |
| Non-English i18n | docs claim Fluent | Other locales | Only `en/` | missing | n/a | No other locale dirs |
| Dirty-state indicator | `AppState.has_changes` | Show unsaved/unapplied edits | Flag is set/cleared; **never read by views** | missing | state_mutations | Change a toggle; no dirty chrome |
| `last_applied` timestamp | `IpcAppState.last_applied` | Record last successful apply | `to_ipc_state()` always writes `None` | missing | none | Inspect `state.json` after apply |

---

## Getting Started / integration

| feature | where exposed | expected behavior | implementation path | status | test coverage | verification method |
|---|---|---|---|---|---|---|
| NixOS detection | onboarding + banner | `/etc/NIXOS`; Flatpak via `flatpak-spawn --host` | `integration::detect_system` | working | host_path helpers (non-Flatpak) | On NixOS: “NixOS Detected”; elsewhere error banner |
| Config mode flake vs classic | onboarding subtitle + snippet | `flake.nix` vs `configuration.nix` | `detect_config_mode` | working | none (FS) | Host with flake vs classic |
| Integration status | onboarding + banner | Substring markers in uncommented Nix | shared `detect_integration_status` | working | common config tests (if present) | Add/remove import; Verify Integration |
| Copy snippet | Copy Snippet | Clipboard classic/flake snippet | `CopyIntegrationSnippet` | working | reducer | Button; paste |
| Open `/etc/nixos` | Open /etc/nixos | `xdg-open` / Flatpak host spawn | `OpenEtcNixos` → `open_path` | partial | none | Errors only `tracing::warn` |
| Verify Integration | Verify Integration | Re-probe + toast | `VerifyIntegration` → DetectSystem | working | reducer | Button; toast verified/missing/unknown |
| Helper-missing banner | startup if helper not found | Warn; disable privileged buttons | `SpawnSpec.helper_available` | working | spawn.rs unit | Flatpak without host helper; cargo GUI without helper on PATH |
| Not-NixOS banner | non-NixOS host | Error banner; tool still opens | `refresh_banner` priority | working | skip_host_probes hides it | Run off NixOS |
| `CheckPermissions` IPC | helper | Report read/write/rebuild | helper `check_permissions`; **GUI never sends** | partial | helper unit for write probe? | Manual JSON to helper |
| `GetSystemInfo` IPC | helper | Authoritative host info | helper `get_system_info`; **GUI never sends** (local probes instead) | partial | none | Manual JSON to helper |
| `can_write_managed_dir` | `integration.rs` | Detect write access | Function exists; **never called**; uses `readonly()` not Unix write | missing | none | Dead code |

---

## Desktop profiles (13)

| feature | where exposed | expected behavior | implementation path | status | test coverage | verification method |
|---|---|---|---|---|---|---|
| List 13 profiles | Profiles page radio list | gnome, kde, xfce, mate, cinnamon, pantheon, cosmic, hyprland, sway, i3, budgie, lxqt, enlightenment | `default_profiles()` + `view/profiles.rs` | working | catalog length via common tests | Count radios = 13 |
| Select one profile | radio | Exclusive selection; local template preview | `SelectProfile` → `LocalProfilePreview` | working | apply.rs | Select; preview pane fills |
| Clear profile | `Message::ClearProfile` | Deselect | Reducer clears; **no UI control** (intentional vs GTK) | missing | reducer | Cannot uncheck from UI |
| Profile preview after ReadState | Profiles page | Show template for restored selection | ReadState does **not** emit `LocalProfilePreview`; nav to Profiles does not either | partial | none | Restart with saved profile; preview stays placeholder until re-click |
| Copy template on apply | helper | Copy `nix/templates/profiles/<id>.nix` | `nix_gen::generate_all_files` | working | helper nix_gen tests | Apply; file under `/etc/nixos/nixos-toolkit/profiles/` |
| Missing template fallback | apply vs preview | Preview omits; apply writes helper fallback | documented limitation | partial | helper tests | Point templates dir at empty; compare preview vs apply |
| ARM notes on profiles | ProfileDef.arm_* | Warn on ARM | Fields exist; **Profiles page does not show ARM notes** | missing | none | ARM host: no profile ARM banner (bundles/hardware have banners) |

---

## Software bundles (16)

| feature | where exposed | expected behavior | implementation path | status | test coverage | verification method |
|---|---|---|---|---|---|---|
| List 16 bundles | Bundles page | Including `ai-tools` (no template) | `default_bundles()` | working | common nix tests | Count expanders = 16 |
| Enable bundle | toggler | Insert catalog packages into `bundle_packages` | `ToggleBundle` | working | state_mutations | Enable gaming; all catalog ids selected |
| Disable bundle | toggler | Remove id + package map | `disable_bundle` | working | state_mutations | Disable; summary updates |
| Expand + per-package toggles | chevron | Customize catalog set | `ToggleBundlePackage` | working | state_mutations | Uncheck steam; preview/apply omit attr but **stub still enables Steam** |
| ARM x86-only disable | gaming, virtualbox | Insensitive toggler | `ArmCompat::None` + `sensitive` | working | none (needs aarch64) | ARM: gaming/vbox greyed |
| ARM partial warning | banner + subtitle | Warn unavailable packages | `banner-arm-bundles` | working | none | ARM: banner |
| Selected-packages summary | bottom card | List selected catalog ids | iterates `bundle_packages` | partial | none | Shows ids not display names; leftover map if toggled while disabled |
| Catalog → Nix attr mapping | apply/preview | `bitwarden`→`bitwarden-desktop`, `julia`→`julia-bin` | `PackageDef.nix_attr` / `resolve_nix_attr` | working | common nix tests | Enable security; generated Nix has `bitwarden-desktop` |
| Bundle row subtitle `pkgs.<id>` | expander body | Show nixpkgs attr | Uses `package.id` not `resolved_nix_attr()` | partial | none | Bitwarden row says `pkgs.bitwarden` |
| Fallback stub vs template | apply/preview | GUI enable always fills `bundle_packages` → stub not template copy | `generate_fallback_bundle` + `bundle_module_stub` | working (by design) | common + helper | Richer template packages (Go, ripgrep, …) **not** installed from GUI |
| `ai-tools` | bundles | ollama only, no template | fallback always | working | none specific | Enable; preview `ollama` |
| Stub always enables modules | generated `bundles/*.nix` | Unchecking steam/docker should disable modules | stubs hardcoded steam/docker/libvirt/vbox | partial | stub snapshots | Uncheck docker in devtools; Nix still `virtualisation.docker.enable` |

---

## Custom packages

| feature | where exposed | expected behavior | implementation path | status | test coverage | verification method |
|---|---|---|---|---|---|---|
| Add names (comma/space/Nix list) | Packages page | Parse, add, toast | `parse_package_input` → `AddPackagesFromInput` | working | packages.rs extensive | Type `htop, fd`; Add |
| Remove package | trash icon | Remove from set | `RemoveCustomPackage` | working | state_mutations | Trash; empty placeholder |
| Duplicate toast | add existing | `toast-already-added` | `classify_new_packages` | working | apply.rs | Add twice |
| In-bundle block | add `git` etc. | Toast; do **not** add even if bundle off | classify against **all** catalog bundles | partial | packages.rs | Add `git` without enabling devtools; blocked |
| Invalid names | `_1password`, `1password`, empty | Surface `field_errors.packages` | Parser drops silently; `FieldErrors.packages` **never set** | broken | packages.rs invalid names | Type `_1password-gui` (catalog id); input clears, no error |
| `AddCustomPackages` message | tests/API | Bulk add | Reducer only; no UI | missing | none | Dead UI path |
| Generate `custom-packages.nix` | apply/preview | `environment.systemPackages` | `generate_custom_packages_nix` | working | helper tests | Apply with htop |

---

## System settings

| feature | where exposed | expected behavior | implementation path | status | test coverage | verification method |
|---|---|---|---|---|---|---|
| Hostname live edit | System identity | `[A-Za-z0-9-]`, max 63; no-op vs current host | `HostnameChanged`; `lib.mkDefault` | working | apply.rs field_errors | Type invalid `_`; error; matching `/etc/hostname` stores `None` |
| Hostname draft vs widget | text_input bound to state/host | Invalid keystrokes should remain visible | Invalid input **not stored**; widget reverts | partial | charset tests | Type `foo_`; `_` never appears |
| DNS IPv4 list | DNS field | Comma-separated IPv4 | `parse_and_set_dns` | working | apply.rs | `1.1.1.1, 8.8.8.8` |
| DNS IPv6 / hostnames | same field | Reject with error | IPv4-only; error shown | partial (by design vs docs) | apply.rs invalid | `::1` errors |
| DNS invalid keeps last good | live parse | Do not apply garbage | Input string kept; `dns_servers` last valid | partial | apply.rs | Type `1.1.1.`; apply still uses previous |
| Username + groups | username + 3 togglers | Write `users.nix` extraGroups | `generate_user_groups_nix` only if username **and** groups | partial | helper reconstruct | Groups without username: `apply_is_empty` false but **no users.nix**; production seeds USER |
| Catalog hostname/DNS actions | `default_system_actions` | Used by System page | View skips non-`UserGroup` | partial | none | Dead catalog entries (UI has dedicated fields) |
| Appearance | see chrome | Local only, not Nix | prefs JSON | working | config.rs | Confirm no `selected.nix` theme |

---

## Hardware

| feature | where exposed | expected behavior | implementation path | status | test coverage | verification method |
|---|---|---|---|---|---|---|
| GPU label | Hardware graphics | `lspci` parse; NVIDIA wins hybrid | `detect_gpu` / `gpu_from_lspci` | working | integration.rs samples | Hybrid Intel+NVIDIA shows NVIDIA |
| NVIDIA widgets | when vendor contains `nvidia`, not ARM | Driver combo + 3 togglers | `view/hardware.rs` | working | apply.rs nvidia tests | NVIDIA host: widgets visible |
| Auto-select NVIDIA stable | GpuDetected / `hardware_for_nix` | Default unwritten driver to stable | Mutates `nvidia_driver = Some(0)` | partial | apply.rs `nvidia_gpu_apply_defaults_unwritten_driver_to_stable` | Fresh NVIDIA machine: apply is **not empty**; writes proprietary NVIDIA + PipeWire |
| nvidia-open dropdown vs open modules | combo index 2 vs `nvidia_open` toggle | Index 2 = `nvidiaPackages.latest`; `open` is separate bool | `generate_hardware_nix` | partial | nix.rs snapshots | Pick “Open Source (nvidia-open)” without toggling Open Kernel Modules → `open = false` |
| Audio PipeWire/Pulse/None | dropdown | Write matching Nix | `audio_server` 0/1/2 | partial | nix.rs | **Any** `hardware.nix` (bluetooth, TLP, NVIDIA) also emits default PipeWire |
| Low-latency audio | toggler | PipeWire extraConfig | only with audio 0 | working | nix.rs | Enable; preview `92-low-latency` |
| Bluetooth + autopower | togglers | `hardware.bluetooth` + blueman | sibling + `hardware_config` synced | working | state.rs bluetooth tests | Enable; preview |
| Power profile | dropdown | PPD oneshot unless balanced | balanced (0) emits nothing | working | nix.rs | Performance → PPD |
| TLP | toggler | TLP on, PPD off | UI allows TLP + performance together; gen prefers TLP | partial | nix.rs tlp tests | Enable both; Nix has TLP, no PPD |
| Thermald | toggler; ARM disabled | Intel thermald | ARM insensitive | working | none | ARM: disabled + note |
| Hardware apply | Apply | Full `HardwareConfig` on Apply/Generate/preview | `to_apply_request` + helper | working | ipc old-JSON + gui tests | Not the old bluetooth-only stub |
| ARM hardware banner | Hardware page | Hide NVIDIA/thermald | `banner-arm-hardware` | working | apply.rs ARM nvidia | ARM: banner |

---

## Network

| feature | where exposed | expected behavior | implementation path | status | test coverage | verification method |
|---|---|---|---|---|---|---|
| Firewall toggle | Network | Default on; off is a setting | `SetFirewallEnabled`; `has_settings` true if **off** | working | ipc tests | Disable firewall; preview `enable = false` |
| Preset TCP/UDP chips | 22/80/443/8080 and 53/123/443/51820 | Toggle ports | `ToggleTcpPort`/`ToggleUdpPort` | working | state_mutations | Click SSH chip |
| Custom TCP/UDP | text fields | Parse extras; keep presets | `parse_and_set_custom_*` | partial | state_mutations | Invalid `abc`: **debug log only**, no field error; last good ports apply |
| SSH server + port/password/root | SSH section | OpenSSH settings | `generate_network_nix` | working | nix.rs | Enable SSH; preview |
| Fail2Ban | toggler | With sshd jail iff SSH on | generate_network_nix | working | nix.rs | Enable both |
| Tailscale | toggler + after note | `services.tailscale.enable` | network.nix | working | reconstruct | Enable; note to run `tailscale up` |
| WireGuard enable + listen port | VPN section | `networking.wireguard.enable` + UDP open; **no peers** | marker comment for reconstruct | partial | ipc + reconstruct | Enable; documented limitation |
| WG port change / disable | spin + toggle | Open new UDP; do not remove old | `SetWireguardEnabled`/`SetWireguardListenPort` | partial | none | Enable, change 51820→51821; both UDP ports remain |
| `has_settings` vs listen port only | apply_is_empty | Port-only change ignored if WG off | `NetworkConfig.has_settings` | working (by design) | ipc tests | Change port with WG off; no network.nix |
| Network.nix without mkDefault | apply | May override user firewall/SSH | unconditional assignments | partial | none | User `networking.firewall` in configuration.nix vs toolkit |

---

## Services (21)

| feature | where exposed | expected behavior | implementation path | status | test coverage | verification method |
|---|---|---|---|---|---|---|
| 21 toggles in 7 groups | Services page | printing, avahi, fwupd, upower, networkmanager, resolved, rustdesk, syncthing, locate, flatpak, gnome_keyring, gnome_tweaks, dconf, docker, libvirtd, postgresql, redis, earlyoom, auto_upgrade, auto_gc, store_optimize | `ToggleService` → `set_service` | working | state set_service | Count togglers = 21 |
| Unknown id | reducer | debug log | `_ => tracing::debug` | working | none | N/A |
| Printing also enables avahi | generated Nix | CUPS + avahi | `generate_services_nix` printing arm | partial | nix.rs | Enable printing only; Nix has avahi; reconstruct may mark avahi on |
| RustDesk / GNOME Tweaks | “services” | Packages not daemons | FTL + extra info row | working | none | Tooltip shows package |
| Syncthing copy | FTL “user service” | `services.syncthing.enable` is **system** | mismatch | partial | none | Read FTL vs Nix |
| Docker group copy | System “rootless Docker” | docker group is rootful socket | mismatch | partial | none | Read description |
| `services.nix` on apply | helper | Write enabled list | generate_services_nix | working | helper tests | Enable printing; file exists |

---

## Apply / preview / rebuild

| feature | where exposed | expected behavior | implementation path | status | test coverage | verification method |
|---|---|---|---|---|---|---|
| Local preview | Apply page + Refresh Preview | `generate_preview_full_from`; no helper | nav Apply → `LocalPreview` | working | state.rs preview hardware | Open Apply; preview text |
| Preview not live | other pages | Must refresh or re-enter Apply | only on nav/button | partial | none | Change hostname, stay on Apply without refresh: stale until Refresh |
| Rebuild dropdown | Switch/Boot/Test/Build | Sets `rebuild_type` | `SetRebuildType`; ignores DryBuild | working | apply.rs | Each option |
| Apply confirm | Apply Changes | Dialog empty/packages/normal | `RequestApply` → `ConfirmApply` | working | apply.rs + gui_state.rs | Empty vs profile vs packages-only |
| Empty apply | destructive dialog | Warns then **deletes** managed snippets/profiles/bundles via cleanup | `cleanup_stale_managed_files` | partial (dangerous by design) | dialog tests | Confirm empty apply on a machine with prior toolkit files |
| Dry Run | Dry Run button | `DryBuild`; snapshot/restore; no WriteState | session `run_apply_chain` | working | session.rs + helper dry_build tests | Dry run; tree restored |
| WriteState after success | Switch/Boot/Test/Build | Persist `state.json` | session after ApplyComplete | partial | session tests | WriteState failure only logs; GUI still `mark_applied` |
| Test rebuild | dropdown | “Temporary activation” | Files **are** written + state saved; only activation is temporary | partial | then_write_state includes Test | Test apply; `/etc/nixos/nixos-toolkit` changed |
| Build rebuild | dropdown | Build only, no activate | Files still written to managed tree | partial | apply.rs rebuild loop | Build; files persist |
| Helper apply chain | pkexec helper | EnsureDirectories → Apply → WriteState | `helper/session.rs` | working | session python fake-helper | Apply with fake-helper |
| `Validate` before apply | helper exists | Catch bad hostname/profile | **GUI never sends Validate** | missing | helper validate unit | Apply unknown profile id from hand-edited state |
| `Generate` IPC | helper | Dry generate without rebuild | GUI uses local preview instead | missing (by design) | helper generate | Manual JSON |
| Streaming logs | Apply log pane | Helper `Log` lines | `HelperResponse::Log` | working | apply.rs | Apply; log fills |
| Log auto-scroll | code_view | Follow tail | scrollable, no stick-to-bottom | partial | none | Long rebuild; view stays at top |
| Manual apply hint | spawn fail | Print `sudo nixos-rebuild …` | SpawnFailed Apply | working | none | Kill helper path |
| Flake `nixos-rebuild --flake /etc/nixos#<hostname>` | helper rebuild | Match flake output name | Uses `/etc/hostname` or `nixos` | broken (common flake setups) | none | Flake attr ≠ hostname; apply fails |
| Busy disable | apply/dry-run | Ignore extra clicks | `busy != Idle` | working | apply.rs | Double-click Apply |
| Helper missing disables apply | buttons | Tooltip; no spawn | `helper_missing` | working | none | No helper: buttons inert |

---

## Generations

| feature | where exposed | expected behavior | implementation path | status | test coverage | verification method |
|---|---|---|---|---|---|---|
| List generations | nav Generations / Refresh | `nix-env --list-generations` + metadata | `ListGenerations` | working | parse tests | Open page; rows |
| Current marker | row | No switch/delete on current | `generation.current` | working | none | Current has caption |
| Switch now / next boot | dialog | `RollbackGeneration` activate switch/boot | ConfirmRollback | working | apply.rs rollback intents | Switch; confirm |
| Delete generation | trash | Confirm then `DeleteGenerations` | helper nix-env | working | apply.rs | Delete non-current |
| Rollback to previous | suggested button | Max number < current | `RollbackToPrevious` | working | apply.rs cases | Button; empty/first/unknown logs |
| Refresh with helper missing | Refresh still pressable when idle | Should no-op or disable | `on_press` if idle **even if helper_missing** | partial | none | No helper: Refresh click does nothing |
| `config_rev` | Generation struct | Flake rev | Always `None` | missing | none | UI never shows rev |
| Stream rollback logs | log pane | Helper uses `.output()` not streaming | only GUI log lines | partial | none | Rollback; little live output |
| 60s timeout | ListGenerations | Fail visibly | Timeout: busy Idle, **no toast**, empty list | broken | none | Slow `nix-env`; “No generations found” / installation issue |

---

## Maintenance

| feature | where exposed | expected behavior | implementation path | status | test coverage | verification method |
|---|---|---|---|---|---|---|
| 5 actions | Maintenance list | gc, gc -d, optimise, verify, update channels/flake | `RequestMaintenance` | working | apply.rs confirm/run | Click each |
| Confirm destructive GC -d | dialog | Warning then run | catalog `warning` | working | apply.rs | Delete Old Generations |
| Allowlist | helper | Exact 6 strings; flake rewrite channel→flake update | `maintenance_command_to_run` | working | helper tests | Send disallowed command |
| Flake-aware update label | UI | Flake vs channel copy | `config_mode == Flake` | working | none | Flake host: “Update Flake Inputs” |
| Disk usage | section + Refresh | `du -sh /nix/store` + gen count | `GetDiskUsage` | partial | dummy DiskUsageLoaded | Large store: **60s timeout**; UI stuck on “Loading”; `error` field unused in UI except empty size |
| Maintenance 60s timeout | GC/verify/optimise | Commands often >60s | `timeout_for` 60s; Timeout swallowed | broken | none | `nix-collect-garbage -d` on large store |
| Log pane | output | stdout/stderr after completion | `MaintenanceOutput` | working | apply.rs | Run gc; log |
| Streaming | log | Live output | `.output()` waits for exit | partial | none | No live lines |

---

## Helper / IPC / security / packaging

| feature | where exposed | expected behavior | implementation path | status | test coverage | verification method |
|---|---|---|---|---|---|---|
| pkexec spawn | host GUI | `pkexec <helper>` + remove SHELL | `SpawnSpec::from_env` | working | spawn.rs | Apply prompts polkit |
| Flatpak spawn | Flatpak GUI | `flatpak-spawn --host --forward-fd=0/1 -- pkexec <probed>` | spawn.rs | working | spawn.rs unit | Flatpak with host helper |
| Env overrides | `NIXOS_TOOLKIT_*` | HELPER, NO_PKEXEC, SPAWN, TEMPLATES_DIR, SKIP_* | Flags/spawn | working | flags.rs, spawn.rs | `NIXOS_TOOLKIT_NO_PKEXEC=1` |
| CLI flags | desktop Exec | No clap; env only | `main.rs` | working (by design) | none | `--help` not implemented |
| ReadState / reconstruct | startup | JSON then Nix reconstruct | helper `read_state` | working | helper reconstruct tests | Delete state.json; GUI restores from Nix |
| Atomic write jail | helper | Refuse paths outside managed dir | `atomic_write` | working | nix_gen tests | (unit) |
| Maintenance allowlist | helper | No shell interpolation | split_whitespace argv | working | helper | Injection attempt |
| Polkit policy | helper package | Install actions; exec.path substituted to store helper | flake `postInstall` | partial | none | Module does not extra-install policy beyond `systemPackages`; unpackaged `pkexec /run/current-system/sw/bin/...` may not match store `exec.path` |
| NixOS module | `programs.nixos-toolkit.enable` | GUI+helper+polkit | flake.nix module | partial | none | Enables polkit + packages; does **not** create managed dir or import |
| Desktop file | data/ + package | `nixos-toolkit` | installed | working | none | Menu entry |
| Metainfo | Flatpak | Appstream | `releases` still “Initial libcosmic skeleton” | partial | none | gnome-software description |
| fake-helper | tests | JSONL double | `crates/fake-helper` | working | fake-helper tests | `FAKE_HELPER_SCRIPT` |
| `Intent::SendOnSession` / `CloseSession` | apply intents | Session chaining | **no-ops** (`// Session chaining is task 12`) | missing | comment | Dead intents; real chaining is in `session.rs` |
| `HelperOp::{CheckPermissions,GetSystemInfo,Validate,Generate}` | message.rs | GUI ops | Never constructed | missing | none | Dead variants |
| Helper stdout parse errors | client | Surface to UI | `tracing::error` only; line dropped | broken | none | Helper prints non-JSON; GUI hangs until timeout |

---

## Test coverage summary

| Area | What exists | Gaps |
|---|---|---|
| GUI reducer | Large `apply.rs` module tests; `gui_state.rs`, `state_mutations.rs`, `packages.rs`, `flags.rs` | No widget/UI tests; no pkexec/Flatpak runtime |
| Helper | reconstruct, dry-build restore, rollback activate, allowlist, atomic_write, cleanup | No real `nixos-rebuild`; no timeout tests |
| common nix/ipc | Hardware/network snapshots, unfree, old JSON compat | No eval of generated Nix against nixpkgs |
| Session | Python fake-helper apply chain | WriteState warning paths only |
| i18n | none | Unused keys, duplicate catalogs |

Verification methods in the matrix are the intended manual or unit checks; items marked **none** have no automated coverage of the user-visible path.

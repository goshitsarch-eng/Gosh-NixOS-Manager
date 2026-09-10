# Migration decisions

Lead decisions for the GTK4 → libcosmic rewrite. Each row records the question, options considered, the choice, and why. Priority order when options conflict:

1. Feature parity with the original GTK app
2. Working correctly in the Flatpak sandbox
3. Accessibility
4. Simplicity and maintainability
5. COSMIC conventions

Sources: `docs/migration/ux.md`, `architecture.md`, `packaging.md`, `review-phase1.md`.

---

## C1. Application ID

**Question:** What is `cosmic::Application::APP_ID`, the Flatpak id, desktop `Icon=` / `StartupWMClass`, and cosmic-config directory?

**Options:**
- Keep GTK `org.nixos-toolkit.app`
- `org.nixos_toolkit.NixosToolkit`
- `io.github.goshitsarch_eng.NixosToolkit`

**Choice:** `io.github.goshitsarch_eng.NixosToolkit`

**Why:** The GTK id is not a valid D-Bus well-known name (hyphen in a non-last component) and Flathub rejects generic `.app` suffixes. Working in the Flatpak (priority 2) requires a legal id. The GitHub-hosted form is Flathub-verifiable against `goshitsarch-eng/Gosh-NixOS-Manager`.

Polkit actions stay `org.nixos-toolkit.helper.*` (independent of the GUI id). Binary command stays `nixos-toolkit`.

Prefs load order: new path under the new id, then migrate-once from `~/.config/nixos-toolkit/preferences.json` if present.

---

## C2. Message enum shape

**Question:** Nested `Page(PageMsg)` (UX Appendix B) vs flat `Message` (architecture §3)? Missing variants?

**Options:** Nested page enums mapped with `.map`; one flat enum.

**Choice:** Architecture owns a **flat** `Message` in `crates/gui/src/message.rs`. Views emit those names (page modules may keep private enums that convert 1:1). Additions and drops:

| Variant | Decision |
|---|---|
| `ExpandBundle { id, expanded }` | **Add.** Architecture stores `expanded_bundles: HashSet<String>`. |
| `ClearProfile` | **Keep in enum for tests; no UI control** (GTK has none). |
| `EnableBundleAll` | **Drop.** `ToggleBundle { id, enabled: true }` means enable + check all packages. |
| `SetNvidiaDriver(Option<u8>)` | **Keep Option.** `None` = NVIDIA widgets absent (non-NVIDIA / ARM). |
| `SetSshRootLogin(u8)` | **Index**, matching the combo (0/1/2 → `"no"` / `"prohibit-password"` / `"yes"` in apply). |
| `RequestMaintenance { id }` | **Action id only.** Architecture looks up command/name/warning from `default_maintenance_actions()`. |
| Apply dialog | `Dialog::ConfirmApply { heading, body, destructive }` covering GTK’s three copies (Empty / PackagesOnly / Normal). |
| `*Apply` on hostname/DNS/username/ports | **Do not add.** Live `*Changed` matches GTK `connect_changed`. |
| Toasts | `Intent::ShowToast { text, timeout_ms }` from `apply()`. `AppModel` holds libcosmic `Toasts<Message>` (not a `Vec`). Duration 3000 ms default, 4000 ms for in-bundle duplicate. |

**Why:** One exhaustive match for tests (priority 4). Missing expander state would break the bundles page (priority 1).

---

## C3. HardwareConfig apply

**Question:** Keep the Hardware page as a `state.json` toy (GTK behaviour) or actually generate Nix?

**Options:** Silent stub; display-only copy; rewrite generator and IPC.

**Choice:** **In-scope Phase 2 bugfix.** Rewrite `generate_hardware_nix` to take `&HardwareConfig`, add `hardware_config` to `Generate`/`Apply` with `#[serde(default)]`, import `hardware.nix` when any non-default hardware setting is set, mirror `bluetooth_enabled` from `hardware_config`.

**Why:** Priority 1 “parity” would preserve a lying page. The page’s purpose is to configure hardware; shipping it again as theatre after a rewrite is a product defect (review B3). Correctness of a feature the GTK UI already showed outranks strict bug-for-bug backend behaviour.

Architecture owns `crates/common` + `crates/helper` for this patch.

---

## C4. Rollback “Set for Next Boot”

**Question:** GTK dialog has Switch Now / Set for Next Boot. Helper always `switch`.

**Options:** Map both to switch (silent change); drop Boot button; extend helper.

**Choice:** Extend `HelperRequest::RollbackGeneration { generation, activate }` where `activate` is `"switch"` | `"boot"`, default `"switch"` for old JSON. GUI keeps the three-button dialog.

**Why:** Visible regression if the button disappears; silent wrong action if both map to switch. Small helper patch, same bucket as C3.

Also restore NixOS/kernel version in helper `ListGenerations` when cheap (read generation metadata). If that lands in the same helper pass, do it; if not, poorer subtitles are accepted and listed in REPORT.md.

---

## C5. Theme store

**Question:** cosmic-config primary vs JSON canonical?

**Choice:** JSON canonical at `$XDG_CONFIG_HOME/nixos-toolkit/preferences.json` (GTK path, also works in Flatpak). Cosmic-config mirror best-effort. Load: flags path → JSON (new) → JSON (legacy) → cosmic-config → default System.

**Why:** Priority 1 (GTK path) and 2 (Flatpak xdg-config). Do not require cosmic-comp.

---

## C6. libcosmic features

**Choice:**

```toml
libcosmic = { git = "https://github.com/pop-os/libcosmic.git", rev = "<pinned SHA>", default-features = false, features = [
  "winit", "wayland", "x11", "wgpu", "tokio", "xdg-portal", "about", "a11y",
] }
```

- `default-features = false` (no `dbus-config` / CosmicSettingsDaemon)
- **no** `single-instance`
- **no** `multi-window` unless the pin requires it for dialogs
- Pin a **commit SHA** in Cargo.toml (not floating master)

**Why:** Must run outside COSMIC (priority 2) and headless smoke cannot take a session bus name (C22). `a11y` and `x11` are required (priority 3 and smoke/Xwayland).

---

## C7. Per-task definition of done

**Question:** Must every task run `flatpak-builder` + smoke?

**Choice:** Split.

| Gate | When |
|---|---|
| `cargo build --workspace --all-targets` | every implementation task |
| `cargo clippy --workspace --all-targets -- -D warnings` | every implementation task |
| `cargo test --workspace --all-targets` | every implementation task |
| `flatpak-builder` + weston smoke | Task 1 (skeleton) once it exists; after that **CI on the branch** and named packaging slices — not every UX string change |
| `scripts/verify.sh` (full) | CI, packaging tasks that touch `flatpak/`, and the Phase 3 gate |

Task 1 still produces a working (minimal) Flatpak so smoke exists. Subsequent tasks must not rebuild iced to change a Fluent string.

**Why:** Review B6. Hours of libcosmic rebuilds would stall the project or cause the gate to be ignored.

---

## C8. Binary and library names

**Choice:**

```toml
[lib]
name = "nixos_toolkit_gui"
path = "src/lib.rs"

[[bin]]
name = "nixos-toolkit"
path = "src/main.rs"
```

Architecture task 1 includes this. Flake `mv $out/bin/gui` is deleted when flake is updated (C14).

---

## C9. Helper spawn inside Flatpak

**Choice:** `SpawnSpec::from_env()` implements the packaging probe:

1. If `NIXOS_TOOLKIT_NO_PKEXEC` → spawn helper path directly.
2. If `NIXOS_TOOLKIT_SPAWN` set → that program + `NIXOS_TOOLKIT_SPAWN_ARGS` + helper.
3. If `FLATPAK_ID` or `/.flatpak-info` exists:
   - Probe host helper with `flatpak-spawn --host -- test -x PATH` for `$NIXOS_TOOLKIT_HELPER`, `/run/current-system/sw/bin/nixos-toolkit-helper`, `/usr/local/bin/…`, `/usr/bin/…`.
   - **Never** `Path::exists` on those paths from inside the sandbox.
   - If none exist: **do not pkexec**. Surface the helper-missing banner.
   - If found: `flatpak-spawn --host --forward-fd=0 --forward-fd=1 -- pkexec <absolute-host-path>`.
4. Else (host install): today’s pkexec + path discovery, `.env_remove("SHELL")`.

Manifest **must** set `NIXOS_TOOLKIT_TEMPLATES_DIR=/app/share/nixos-toolkit/templates`. Optional `/app/libexec` wrapper is sugar; the Rust client is the source of truth.

---

## C10. Smoke vs startup ReadState

**Choice:** Both.

- Production NixOS: `init` still `ReadState` (GTK parity, one pkexec on start).
- Smoke/CI: `NIXOS_TOOLKIT_SKIP_PRIVILEGED_INIT=1`.
- Spawn also refuses pkexec when the host helper is not executable (C9), so a forgotten skip flag still cannot hang on a polkit dialog.

---

## C11. Open `/etc/nixos`

**Choice:** `Intent::OpenPath`. If `FLATPAK_ID` set → `flatpak-spawn --host -- xdg-open /etc/nixos`. Else `open::that_detached`. Toast on failure.

---

## C12. Clipboard

**Choice:** `iced::clipboard::write` via `Intent::CopyClipboard`. libcosmic `xdg-portal` is enabled so iced can use the portal. No `wl-copy` / `xclip`.

---

## C13. i18n

**Choice:** Fluent from day one. Architecture owns `crates/gui/src/i18n.rs` + `i18n-embed` + `rust-embed` in task 1. UX owns `i18n/en/nixos_toolkit.ftl`. A unit test fails if a `fl!` key used in views is missing from `en`. Catalog strings in `actions.rs` stay English (option 1).

Not a GTK regression (GTK was hardcoded English). Empty catalogs are still a defect relative to the UX contract.

---

## C14. `flake.nix`

**Choice:** Architecture updates `flake.nix` in the same task that deletes GTK (`wrapGAppsHook4`, gtk4, libadwaita). Helper package stays green. GUI package uses libcosmic native deps (libxkbcommon, wayland, expat, fontconfig, freetype, mesa). This host has no `nix` on PATH; cargo/Flatpak remains the gate. `nix build .#gui` must not be left obviously GTK-shaped.

---

## C15. Polkit policy in the Flatpak

**Choice:** **Not** installed by the Flatpak. Host NixOS module only. Onboarding copy must say Flatpak users still need `programs.nixos-toolkit.enable` (or a host `nixos-toolkit-helper`).

---

## C16. Icon theme

**Choice:** Flatpak adds `org.freedesktop.Platform.Icontheme.Adwaita` (and Cosmic if available without pulling Cosmic.BaseApp). Views use `icon::from_name` **and** a text label so a missing icon never blanks a nav row. App icon is the existing SVG installed as `${FLATPAK_ID}.svg`.

---

## C17. Tests layout

**Choice:**

- Crate tests: `crates/common`, `crates/helper`, `crates/gui/src/core` (next to `apply.rs`).
- GUI integration: `crates/gui/tests/*.rs` once `[lib]` exists.
- Fake helper: `crates/fake-helper` workspace member, **no** libcosmic, speaks real IPC JSON.
- Exhaustive `match Message` lives in `crates/gui` so it cannot drift.

No fourth `tests/` package that depends on libcosmic (would unify features and break helper-only `cargo test` on thin hosts). Packaging still owns the test files listed above.

---

## C18. rust-version

**Choice:** Workspace `rust-version = "1.93"`. Our crates stay `edition = "2021"` unless a file needs 2024. Host/SDK 1.98 is fine.

---

## C19. UDP ports UI

**Choice:** No widget. `allowed_udp_ports` still round-trips in `NetworkConfig` / `state.json`. No `SetUdpPorts` message. No WireGuard UI.

---

## C20. Preview path

**Choice:** Local `generate_preview_full` + `$NIXOS_TOOLKIT_TEMPLATES_DIR`. No helper, no pkexec. GTK `apply.rs` already does this; packaging’s “preview goes through Generate” risk row is false.

---

## C21. Toaster / dialog types

**Choice:** libcosmic `Toasts<Message>` in `AppModel`. Dialog enum in `message.rs` with precomputed apply heading/body. `apply()` returns intents; it does not mention toaster widgets.

---

## C22. single-instance

**Choice:** Disabled. GTK used `ApplicationFlags::FLAGS_NONE`. Headless smoke cannot take a session bus name reliably.

---

## Other lead calls (from review §2 / §4)

| ID | Choice |
|---|---|
| N1 List generations | Prefer helper `ListGenerations` inside Flatpak (sandbox cannot see `/nix`). On the host, helper already runs privileged; accept a pkexec to *view* the list as the cost of one privileged path. Document vs GTK’s unprivileged walk. |
| N5 Empty-apply check | Keep GTK’s profile+bundles+packages only, **plus** non-default hardware after C3 (otherwise a hardware-only apply looks “empty” and warns about removing software incorrectly). Record in REPORT.md. |
| N6 bluetooth sibling | `SetBluetoothEnabled` writes both; `from_ipc_state` prefers `hardware_config.bluetooth_enabled` if they diverge. |
| Fail2ban with SSH off | Keep GTK’s silent drop (generated only when SSH is on). |
| Live hostname `on_input` | Preserve GTK live mutate. |
| In-bundle duplicate | Check **all** bundles, not only enabled (GTK). |
| Bundle empty package set | Does **not** disable the bundle. |
| F5 refresh | Re-run local `detect_system` only. Do **not** ReadState. |
| Nav start | Always Getting Started. Do not persist last page. |
| Banner priority | Not NixOS > helper-missing / state-load warning > integration. Intentional a11y improvement vs GTK last-writer-wins. |
| About acknowledgements | “libcosmic, iced, Rust, Nix” (not GTK4/libadwaita). View → About is an intentional improvement (GTK about was unwired). |
| `ai-tools` | Keep in UI (16 bundles). Do not invent a template in this migration. |
| `parse_generation_line` | Do not port (dead code). |
| Integration detection | Preserve GUI vs helper string mismatch; do not unify in this rewrite. |
| wgpu smoke | Accept tiny-skia. Try `WGPU_BACKEND=gl` then software before failing. |
| Generator pin | First `cargo-sources.json` generation records a SHA of flatpak-builder-tools, not `master`. |
| lspci in Flatpak | Unknown GPU fallback is parity. Optional later: host-spawn `lspci` without pkexec. |
| Channel update | Helper `RunMaintenance` always privileged; drop GTK’s unprivileged `nix-channel` retry. Document in REPORT.md. |
| Power profile vs TLP | If TLP on, disable power-profiles-daemon (existing generator). `power_profile` index maps to `services.power-profiles-daemon` profile when TLP is off. |

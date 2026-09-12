# Packaging, platform, and QA audit

Reconnaissance only (branch `audit-hardening`). Product: **NixOS Toolkit**. This note covers the Nix flake, NixOS module, Flatpak GUI, desktop/AppStream/icons, polkit, `install.sh`, and the cargo/flake/Flatpak verification gate.

Sources: `flake.nix`, `Cargo.toml` / crate manifests / `Cargo.lock`, `flatpak/*`, `data/*`, `scripts/*`, `.github/workflows/verify.yml`, `install.sh`, `crates/gui/src/{helper/spawn.rs,integration.rs,app.rs,config.rs,lib.rs}`, `docs/{development,architecture,reference}.md`, `README.md`.

Severity: **High** = wrong permissions, broken privileged install, or CI that cannot catch ship blockers. **Medium** = Flathub/desktop/polkit correctness or user-visible packaging bugs. **Low** = hygiene, missing metadata, incomplete matrices. **Info** = confirmed-good or out of scope.

---

## Inventory (what exists)

| Artifact | Path / identity |
|---|---|
| Workspace | `0.1.0`, edition 2021, `rust-version = "1.93"`, `GPL-3.0-or-later` |
| Release profile | `lto = true`, `codegen-units = 1`, `strip = true` (no `panic = "abort"`) |
| libcosmic | git rev `7cc116803b18d7b888eb511b009f775f036c3da7`; features `winit, wayland, x11, wgpu, tokio, xdg-portal, about, a11y` (no `dbus-config`, `applet`, `multi-window`, `rfd`) |
| App ID | `io.github.goshitsarch_eng.NixosToolkit` |
| Nix packages | `gui` (`nixos-toolkit`), `helper`, `templates`, `full` (symlinkJoin) |
| NixOS module | `programs.nixos-toolkit.enable` / `.package` / `.helperPackage` |
| Flatpak | GUI-only, Freedesktop Platform/SDK **25.08** + `rust-stable` |
| Desktop | `data/nixos-toolkit.desktop` (Nix); `flatpak/*.desktop` (App ID named) |
| Metainfo | Flatpak only (`flatpak/*.metainfo.xml`) |
| Icon | `data/icons/nixos-toolkit.svg` (48×48 viewBox, scalable) |
| Polkit | `data/polkit/org.nixos-toolkit.helper.policy` (host helper, not Flatpak) |
| MIME / URL handlers | **None** (correct; this is not a document handler) |
| Verify | `scripts/verify.sh` → cargo build/clippy/test + Flatpak + weston smoke |
| CI | `.github/workflows/verify.yml`: cargo + flake fmt/audit always; Flatpak `workflow_dispatch` only |

---

## Finding index

| ID | Sev | Title |
|---|---|---|
| [PKG-001](#pkg-001) | High | Flatpak `--filesystem=/etc/nixos:ro` is redundant and over-broad |
| [PKG-002](#pkg-002) | High | Flatpak/smoke never run on PR; `cargo-sources.json` drift is invisible |
| [PKG-003](#pkg-003) | Medium | GUI wrap `--set` pins helper/templates; module `helperPackage` does not rewrap GUI |
| [PKG-004](#pkg-004) | Medium | Three polkit actions, one `exec.path`, GUI never passes `--action-id` |
| [PKG-005](#pkg-005) | Medium | Polkit `exec.path` store substitute vs Flatpak probe of `/run/current-system/sw/bin` |
| [PKG-006](#pkg-006) | Medium | AppStream screenshot is the icon SVG; Flathub-unready |
| [PKG-007](#pkg-007) | Medium | No `desktop-file-validate` / `appstreamcli` in verify or CI |
| [PKG-008](#pkg-008) | Medium | No icon theme in Nix wrap or Flatpak; nav uses Freedesktop symbolic names |
| [PKG-009](#pkg-009) | Medium | `install.sh` does not install the product; desktop/alias are inaccurate |
| [PKG-010](#pkg-010) | Medium | About repository URL is empty (`CARGO_PKG_REPOSITORY`) |
| [PKG-011](#pkg-011) | Low | Flake `install … \|\| true` and template `2>/dev/null \|\| true` hide failures |
| [PKG-012](#pkg-012) | Low | MSRV not inherited on helper/common; toolchain is `stable.latest`, not 1.93 |
| [PKG-013](#pkg-013) | Low | Flake `openssl` buildInputs unused |
| [PKG-014](#pkg-014) | Low | Nix GUI wrap does not provide `xdg-utils` for Open URL / Open path |
| [PKG-015](#pkg-015) | Low | Nix package ships no AppStream metainfo |
| [PKG-016](#pkg-016) | Low | Smoke skips privileged init **and** host probes |
| [PKG-017](#pkg-017) | Low | `verify.sh` requires unused `python3`; no rustfmt |
| [PKG-018](#pkg-018) | Info | `cargo-sources.json` matches `Cargo.lock` at audit time (no CI guard) |
| [PKG-019](#pkg-019) | Low | `nix profile install` / `packages.full` do not load polkit into the system |
| [PKG-020](#pkg-020) | Low | `eachDefaultSystem` exposes Darwin attrs for a Linux-only GUI |
| [PKG-021](#pkg-021) | Info | COSMIC services are optional; JSON prefs + `xdg-portal` are the real path |
| [PKG-022](#pkg-022) | Info | No MIME, notifications, or file-chooser portal use (toasts + spawn-host open) |
| [PKG-023](#pkg-023) | Low | `OpenUrl` uses `open::that_detached` in-sandbox; `OpenPath` correctly spawn-hosts |
| [PKG-024](#pkg-024) | Low | Clipboard is iced/winit, not the portal; GNOME Wayland may no-op |
| [PKG-025](#pkg-025) | Low | Push CI branches omit `audit-hardening`; flake checks omit `cargo test` |
| [PKG-026](#pkg-026) | Low | LICENSE not installed into Nix/Flatpak share |
| [PKG-027](#pkg-027) | Info | Finish-args otherwise follow least privilege (no network, no host fs, no `device=all`) |
| [PKG-028](#pkg-028) | Low | Three desktop files already drifted (`data/`, `flatpak/`, `install.sh`) |
| [PKG-029](#pkg-029) | Low | `org.freedesktop.Flatpak` talk-name is a full host escape (justified; linter skip is a comment) |
| [PKG-030](#pkg-030) | Low | NixOS module enables polkit but does not ship an authentication agent |

---

## Findings

### PKG-001

**Severity:** High  
**Title:** Flatpak `--filesystem=/etc/nixos:ro` is redundant and over-broad

**Evidence:** Manifest finish-args (`flatpak/io.github.goshitsarch_eng.NixosToolkit.yml`) grant:

- `--filesystem=/etc/NIXOS:ro`
- `--filesystem=/etc/nixos:ro`
- `--filesystem=/etc/hostname:ro`

GUI detection does **not** read those paths from the sandbox. `crates/gui/src/integration.rs` uses `flatpak-spawn --host -- test` / `cat` / `hostname` / `lspci` whenever `in_flatpak()` is true. Opening `/etc/nixos` uses `flatpak-spawn --host -- xdg-open` (`app.rs` `open_path`). The same talk-name is required for the host helper.

Host `/etc/nixos` commonly contains secrets (Wi-Fi PSKs, age keys, tokens) in `configuration.nix` or adjacent files. A read-only bind of the whole tree is a sandbox hole the app does not need.

**Expected:** Only permissions the process actually uses. Detection and apply already require `org.freedesktop.Flatpak`. Prefer portals / spawn-host of specific commands over binding host config.

**Proposed fix:** Drop the three `--filesystem=…` binds. Keep `--talk-name=org.freedesktop.Flatpak` and the spawn-host probes. If a no-spawn fallback is desired, bind only `/etc/NIXOS:ro` and `/etc/hostname:ro` — never the whole `/etc/nixos` tree.

---

### PKG-002

**Severity:** High  
**Title:** Flatpak/smoke never run on PR; `cargo-sources.json` drift is invisible

**Evidence:** `.github/workflows/verify.yml`:

- `cargo` job: build, clippy `-D warnings`, test (always on PR / push `main`+`cosmic-migration`).
- `flake` job: `checks.x86_64-linux.fmt` and `.audit` only (comment: do not run `nix flake check`).
- `flatpak` job: `if: github.event_name == 'workflow_dispatch'`.

`scripts/verify.sh` is the full gate (cargo + Flatpak + weston smoke) but is not invoked in CI. `scripts/generate-cargo-sources.sh` is also not invoked; `verify.sh` only checks the file exists.

A lockfile bump that forgets `flatpak/cargo-sources.json` still goes green on GitHub. The first failure is a local `verify.sh` or a manual dispatch on a disk-heavy runner.

**Expected:** Either run Flatpak smoke on a schedule / labeled PR / nightly, or add a cheap job that fails if `cargo-sources.json` git SHAs / crate checksums do not cover `Cargo.lock` (even without a full builder).

**Proposed fix:**

1. Add `scripts/check-cargo-sources.sh` (compare lock package checksums + libcosmic rev to `cargo-sources.json`) on every PR.
2. Keep full Flatpak+weston as `workflow_dispatch` **and** a weekly cron, with runtime cache, until a larger runner exists.
3. Document that merge to `main` still requires a green local `scripts/verify.sh` when Flatpak files change.

---

### PKG-003

**Severity:** Medium  
**Title:** GUI wrap `--set` pins helper/templates; module `helperPackage` does not rewrap GUI

**Evidence:** `flake.nix` GUI `preFixup`:

```text
wrapProgram $out/bin/nixos-toolkit \
  --set NIXOS_TOOLKIT_TEMPLATES_DIR "$out/share/nixos-toolkit/templates" \
  --set NIXOS_TOOLKIT_HELPER "${nixos-toolkit-helper}/bin/nixos-toolkit-helper"
```

`makeWrapper --set` overwrites the environment; users cannot point a nix-built GUI at `fake-helper` or a local debug helper without wrapping again.

The NixOS module exposes `programs.nixos-toolkit.helperPackage` but still installs `programs.nixos-toolkit.package` (the GUI already wrapped against the flake’s `nixos-toolkit-helper`). Overriding only `helperPackage` puts a different helper on `PATH` and a different polkit `exec.path`, while the GUI still `pkexec`s the build-time helper.

**Expected:** `--set-default` so env overrides work. Module should wrap the GUI against `cfg.helperPackage`, or document that `package` and `helperPackage` must be overridden together.

**Proposed fix:** Switch to `--set-default`. In the module, `wrapProgram` the GUI with `cfg.helperPackage` (or drop `helperPackage` as a separate knob).

---

### PKG-004

**Severity:** Medium  
**Title:** Three polkit actions, one `exec.path`, GUI never passes `--action-id`

**Evidence:** `data/polkit/org.nixos-toolkit.helper.policy` defines:

- `org.nixos-toolkit.helper.manage-system` — active `auth_admin_keep`
- `org.nixos-toolkit.helper.write-config` — active `auth_admin_keep`
- `org.nixos-toolkit.helper.rebuild` — active `auth_admin` (no keep)

All three annotate the same `org.freedesktop.policykit.exec.path`. GUI spawn is `pkexec <helper>` with no `--action-id` (`crates/gui/src/helper/spawn.rs`, `client.rs`).

pkexec selects the action by binary path. Multiple actions with the same path are ambiguous: keep vs no-keep for rebuild never applies as written, and some polkit versions error if more than one action matches.

**Expected:** One action for the helper (it already multiplexes Apply / generations / maintenance over stdin), **or** `pkexec --action-id …` per operation class.

**Proposed fix:** Collapse to a single action (recommend `manage-system` + `auth_admin_keep` for Apply chains; rebuild-without-keep can be a later hardening pass with `--action-id`). Keep `allow_gui=true` only if the helper must speak to the session; the helper is headless — `false` is enough if the auth agent still prompts.

---

### PKG-005

**Severity:** Medium  
**Title:** Polkit `exec.path` store substitute vs Flatpak probe of `/run/current-system/sw/bin`

**Evidence:** Policy source annotates `/run/current-system/sw/bin/nixos-toolkit-helper`. Helper `postInstall` substitutes that string to `$out/bin/nixos-toolkit-helper` (store path). Flatpak probe order (`spawn.rs` `SYSTEM_HELPER_PATHS`) is:

1. `$NIXOS_TOOLKIT_HELPER` (unset in the sandbox)
2. `/run/current-system/sw/bin/nixos-toolkit-helper`
3. `/usr/local/bin/…`, `/usr/bin/…`

Native NixOS GUI uses the wrapped store path (PKG-003), which matches the substituted annotation. Flatpak `pkexec`s the profile symlink. Matching then depends on pkexec realpath vs string compare.

**Expected:** The path pkexec executes is exactly the annotation (or a documented realpath). Flatpak and the module should use one canonical path.

**Proposed fix:** Prefer **not** substituting to `$out` when the module puts the helper in `environment.systemPackages`; keep `/run/current-system/sw/bin/nixos-toolkit-helper` as `exec.path`. Have the GUI (including Flatpak probe) pkexec that path on NixOS. Keep store-path substitute only for exotic non-module installs, or install a tiny `/run/wrappers`-style stable path.

---

### PKG-006

**Severity:** Medium  
**Title:** AppStream screenshot is the icon SVG; Flathub-unready

**Evidence:** `flatpak/io.github.goshitsarch_eng.NixosToolkit.metainfo.xml` `<screenshot>` points at `data/icons/nixos-toolkit.svg` on `main`. AppStream/Flathub want a window capture (PNG/JPEG, typically ≥1080px on the long edge), not the application icon. Release text is still “Initial libcosmic skeleton.” (`0.1.0` / `2026-09-10`).

Otherwise the file is structurally decent: `id`, `launchable`, `GPL-3.0-or-later`, `developer id`, OARS 1.1, homepage/bugtracker, System/Settings categories. No `<provides>com.system76.CosmicApplication</provides>` (correct).

**Expected:** Real UI screenshot(s); release notes that match the current feature set; `appstreamcli validate --strict` clean.

**Proposed fix:** Capture weston/GNOME screenshots, host them in-repo or on a stable URL, refresh `<releases>`. Add `appstreamcli validate` to `scripts/verify.sh`.

---

### PKG-007

**Severity:** Medium  
**Title:** No desktop/AppStream validation in the gate

**Evidence:** `scripts/verify.sh` runs cargo + `build-flatpak.sh` + `smoke-flatpak.sh`. It does not call `desktop-file-validate` or `appstreamcli`. CI cargo job has no those packages. `python3` is required but unused.

Spot checks of the committed files:

- Both desktop files look spec-legal (`Type=Application`, trailing semicolons on Categories/Keywords, `StartupNotify=true`).
- Nix `Icon=nixos-toolkit` vs Flatpak `Icon=io.github.goshitsarch_eng.NixosToolkit` is intentional.
- `StartupWMClass=io.github.goshitsarch_eng.NixosToolkit` matches `Application::APP_ID` (good for both installs).
- No `MimeType=`, no `x-scheme-handler` (good).

**Expected:** Validate both `.desktop` files and the metainfo on every `verify.sh` / cargo-adjacent job (does not need a Flatpak runtime).

**Proposed fix:** Add `desktop-file-validate data/*.desktop flatpak/*.desktop` and `appstreamcli validate --no-net flatpak/*.metainfo.xml` (non-strict until PKG-006 screenshots land, then `--strict`).

---

### PKG-008

**Severity:** Medium  
**Title:** No icon theme in Nix wrap or Flatpak; nav uses Freedesktop symbolic names

**Evidence:** UI icons are `icon::from_name("go-home-symbolic")` etc. (`message.rs` `Page::icon_name`, generations/maintenance buttons). The only shipped SVG is the **app** icon. Flake wrap does not add `hicolor-icon-theme`, `adwaita-icon-theme`, or `cosmic-icons` to `XDG_DATA_DIRS`. Flatpak does not add `org.freedesktop.Platform.Icontheme.Adwaita` or bundle cosmic-icons. Migration UX (historical) asked packaging to ship a theme; that did not land.

On COSMIC, system icons exist. On weston smoke / minimal NixOS / Fedora toolbox, nav rows can render without icons (labels remain). Not a crash, but a packaging gap vs the stated contract.

**Expected:** Named symbolic icons resolve on the supported desktops, including Flatpak on GNOME/KDE/weston, without requiring COSMIC.

**Proposed fix:** Nix: wrap `XDG_DATA_DIRS` with `adwaita-icon-theme` (or `cosmic-icons` if that is the intended look). Flatpak: add the Adwaita icontheme extension **or** vendor the ~15 symbolic SVGs used in-tree under `hicolor`. Do not add Cosmic.BaseApp.

---

### PKG-009

**Severity:** Medium  
**Title:** `install.sh` does not install the product; desktop/alias are inaccurate

**Evidence:** `install.sh` (advertised in README as a curl|bash convenience):

- Does not build or install `nixos-toolkit` / `nixos-toolkit-helper`.
- Optionally writes `alias nixos-toolkit='nix run github:goshitsarch-eng/Gosh-NixOS-Manager'` (or a fish function). That is a full flake eval on every launch, GUI-only, no helper on `PATH`, no polkit policy.
- Writes `~/.local/share/applications/nixos-toolkit.desktop` with `Icon=preferences-system`, no `StartupWMClass`, `Exec=nix run github:…` (unquoted extra-experimental-features variant can break as a desktop Exec).
- Never mentions `programs.nixos-toolkit.enable` or the one-time `selected.nix` import.
- Always targets GitHub `main`, even when run from a local checkout / other branch.

README already warns it “does not install packages,” but the script still says “Installation complete!”

**Expected:** Either delete the script in favor of the NixOS module, or make it: (1) refuse non-NixOS without a hard stop, (2) print the module snippet, (3) optionally `nix profile install …#full` with a polkit warning, (4) install `data/nixos-toolkit.desktop` + the real SVG.

**Proposed fix:** Replace the success copy with “launcher helper only.” Use `data/nixos-toolkit.desktop` as the template, `Icon` pointing at a copied SVG under `~/.local/share/icons`. Point Exec at a wrapped `nix run` **or** refuse to write a desktop file if the helper is missing.

---

### PKG-010

**Severity:** Medium  
**Title:** About repository URL is empty

**Evidence:** `crates/gui/src/app.rs` `const REPOSITORY: &str = env!("CARGO_PKG_REPOSITORY");`. Workspace `Cargo.toml` sets `repository`, but `crates/gui/Cargo.toml` does **not** set `repository.workspace = true`. Debug `.d` files record `CARGO_PKG_REPOSITORY=` empty. About “Repository” link is therefore blank.

**Expected:** About links to `https://github.com/goshitsarch-eng/Gosh-NixOS-Manager`.

**Proposed fix:** Add `repository.workspace = true` (and `license`/`version` already inherited) on `gui` / `helper` / `common`. Optionally hardcode the same URL as a fallback.

---

### PKG-011

**Severity:** Low  
**Title:** Flake install/copy errors are swallowed

**Evidence:** GUI `postInstall`: `install -Dm644 …desktop … || true` and the same for the SVG. `templateFiles` uses `cp -r … 2>/dev/null || true`. A missing `data/` file still produces a “successful” GUI package without a desktop entry or templates.

**Expected:** Fail the derivation if desktop, icon, or templates are missing.

**Proposed fix:** Drop `|| true` / `2>/dev/null`. Use `cp -r` without hiding errors.

---

### PKG-012

**Severity:** Low  
**Title:** MSRV inheritance and toolchain pin

**Evidence:** Workspace `rust-version = "1.93"`. Only `gui` and `fake-helper` set `rust-version.workspace = true`. `helper` and `common` do not. Flake uses `pkgs.rust-bin.stable.latest` (comment: “1.93+”), not a 1.93 pin. CI uses `dtolnay/rust-toolchain@stable`.

**Expected:** Every member inherits MSRV. Flake/CI either pin 1.93 or document that “stable.latest” is the supported compiler.

**Proposed fix:** Inherit `rust-version` on all crates. Pin rust-overlay (and CI) to 1.93 if MSRV is a real floor; otherwise state “stable ≥ 1.93”.

---

### PKG-013

**Severity:** Low  
**Title:** Flake `openssl` buildInputs unused

**Evidence:** `guiBuildInputs` and `helperBuildInputs` both include `openssl`. `Cargo.lock` has no `openssl` / `openssl-sys` / `native-tls`. Helper does not use TLS. Leftover from an older stack.

**Expected:** Native inputs match the crate graph (libxkbcommon, wayland, mesa, fontconfig, X11 for GUI; nothing extra for helper).

**Proposed fix:** Remove `openssl` (and helper `pkg-config` if nothing needs it). Re-add only if a future dep requires it.

---

### PKG-014

**Severity:** Low  
**Title:** Nix GUI wrap does not provide `xdg-utils`

**Evidence:** `OpenUrl` → `open::that_detached`; host `OpenPath` → same (`app.rs`). Flake wrap does not prefix `PATH` with `xdg-utils`. `nix run` / a NixOS system without `xdg-open` on PATH fails About links and “Open /etc/nixos” with only a tracing warning.

**Expected:** Desktop OpenURI works from the wrapped binary.

**Proposed fix:** `--prefix PATH : ${lib.makeBinPath [ pkgs.xdg-utils ]}` on the GUI (not the helper).

---

### PKG-015

**Severity:** Low  
**Title:** Nix package ships no AppStream metainfo

**Evidence:** Flatpak installs `flatpak/*.metainfo.xml`. Nix `postInstall` installs desktop + SVG only. GNOME Software / KDE Discover on a NixOS module install will not show the app description, license, or screenshots.

**Expected:** `$out/share/metainfo/io.github.goshitsarch_eng.NixosToolkit.metainfo.xml` (or `nixos-toolkit.metainfo.xml` with a matching `launchable` for `nixos-toolkit.desktop`).

**Proposed fix:** Share one metainfo template; substitute `launchable`/icon for Nix vs Flatpak, or install the Flatpak metainfo and add a second launchable.

---

### PKG-016

**Severity:** Low  
**Title:** Smoke skips privileged init and host probes

**Evidence:** `scripts/smoke-flatpak.sh` runs:

```text
--env=NIXOS_TOOLKIT_SKIP_PRIVILEGED_INIT=1
--env=NIXOS_TOOLKIT_SKIP_HOST_PROBES=1
```

plus `WGPU_BACKEND=gl` with a retry. It greps panic/adapter/pkexec and requires the process to still be alive at timeout (124/143). That is a good **window** smoke. It does **not** assert: helper-absent banner without the skip flags, spawn-host `test -e /etc/NIXOS` without pkexec, Wayland-unset X11 fallback, or desktop/metainfo inside `/app`.

**Expected:** At least one run **without** `SKIP_HOST_PROBES` on a non-NixOS host (Fedora CI) proving detection uses spawn-host and still does not invoke pkexec. Optional second run with `WAYLAND_DISPLAY` unset and `DISPLAY` set.

**Proposed fix:** Default smoke: drop `SKIP_HOST_PROBES`; keep `SKIP_PRIVILEGED_INIT` so ReadState does not pkexec. Fail if logs contain `pkexec`. Add `SMOKE_X11=1` optional path.

---

### PKG-017

**Severity:** Low  
**Title:** `verify.sh` tool list vs what it runs

**Evidence:** Requires `python3` but never calls it (`generate-cargo-sources.sh` is the Python consumer). Does not run `cargo fmt` (flake job does). Does not validate desktop/metainfo (PKG-007).

**Expected:** `need` only tools that are used; fmt either in verify or clearly CI-only.

**Proposed fix:** Drop `python3` from verify; add desktop/appstream validation; optionally `cargo fmt --check`.

---

### PKG-018

**Severity:** Info  
**Title:** `cargo-sources.json` is not stale vs `Cargo.lock` at audit time

**Evidence (sampled):**

| Item | `Cargo.lock` | `flatpak/cargo-sources.json` |
|---|---|---|
| libcosmic | rev `7cc116803b18d7b888eb511b009f775f036c3da7` | git commit + vendor dest `libcosmic-7cc1168` |
| serde 1.0.228 | checksum `9a8e94ea7f378bd32cbbd37198a4a91436180c5bb472411e48b5ec2e2124ae9e` | same sha256 |
| thiserror 2.0.17 and 1.0.69 | both present | both vendored |
| accesskit | `f0599eed…` | git dest `accesskit-f0599ee` |

Generator is pinned (`scripts/generate-cargo-sources.sh` `TOOLS_COMMIT=1fc32195e3e60fe5c97f0af646dec7a99df5962b`), `--git-tarballs` correctly avoided, `cargo/config` copied to `.cargo/config.toml` in the manifest. **No automated check** (see PKG-002).

**Expected:** Keep committing the generator output; add the cheap lock vs sources check.

**Proposed fix:** None for content right now; add the check from PKG-002.

---

### PKG-019

**Severity:** Low  
**Title:** Profile / `packages.full` installs do not load polkit

**Evidence:** README: `nix profile install` installs `packages.default` (GUI only); wrapper points at a store helper but does not put the helper on `PATH` or install policy. `packages.full` symlinkJoins GUI+helper+templates, so `~/.nix-profile/share/polkit-1/actions/` may contain the policy. System polkit does not search the user profile. NixOS module is the only path that both enables polkit and links the helper into `/run/current-system/sw`.

**Expected:** Docs (already mostly honest) plus `packages.full` meta/description warning. Optional: module-only polkit, refuse to treat `nix profile` as a privileged install.

**Proposed fix:** `meta.longDescription` on `full` and GUI. README is fine; `install.sh` is not (PKG-009).

---

### PKG-020

**Severity:** Low  
**Title:** `eachDefaultSystem` includes Darwin

**Evidence:** `flake-utils.lib.eachDefaultSystem` yields x86_64/aarch64 darwin + linux. `meta.platforms = platforms.linux`. GUI `buildInputs` are X11/Wayland/Mesa. `nix flake check` on Darwin (or Hydra `eachDefaultSystem`) will try to evaluate/build a Linux GUI.

**Expected:** `eachSystem [ "x86_64-linux" "aarch64-linux" ]` (and later other Linux if wanted).

**Proposed fix:** Restrict systems; keep `nixosModules` / `overlays` at flake top-level.

---

### PKG-021

**Severity:** Info  
**Title:** COSMIC-specific services are not required

**Evidence:**

- No Cosmic.BaseApp, no `--filesystem=xdg-config/cosmic`, no `com.system76.CosmicSettingsDaemon` talk-name.
- libcosmic `dbus-config` / `applet` features off.
- Prefs: JSON at `$XDG_CONFIG_HOME/nixos-toolkit/preferences.json` is canonical; `cosmic-config` under `APP_ID` is best-effort (`config.rs`). Missing daemon → `Ok`/`Err` logged at debug, theme still applies via `Theme::light/dark` or `system_preference()` + `xdg-portal`.
- `Application` single-instance is **off** (`main.rs`), so the App ID is not a required well-known D-Bus name.
- Desktop profile `cosmic` is a **NixOS target**, not a runtime dependency of the GUI.

**Expected:** App runs on weston/GNOME/KDE without cosmic-comp. Matches the migration packaging plan.

**Proposed fix:** None. Keep cosmic-config best-effort; do not add daemon talk-names “just in case.”

---

### PKG-022

**Severity:** Info  
**Title:** No MIME, notifications, or file-chooser usage

**Evidence:** No `MimeType` / URL handlers. No `org.freedesktop.Notifications` (toasts are in-app). No `rfd` / file portal. “Open /etc/nixos” is spawn-host `xdg-open`. Clipboard is iced (`Intent::CopyClipboard`). No `--socket=session-bus`, no `--share=network`, no `--device=all`.

**Expected:** Do not add those permissions.

**Proposed fix:** None.

---

### PKG-023

**Severity:** Low  
**Title:** `OpenUrl` does not use spawn-host; `OpenPath` does

**Evidence:** `Intent::OpenPath` → `open_path()` → Flatpak: `flatpak-spawn --host -- xdg-open`. `Intent::OpenUrl` → `open::that_detached(url)` in the sandbox. Freedesktop runtime `xdg-open` usually hits the OpenURI portal (no extra talk-name). If `open` falls back to a browser binary inside the sandbox, it fails (no network share, no browser).

**Expected:** Same host/portal path for URLs and filesystem paths.

**Proposed fix:** In Flatpak, `flatpak-spawn --host -- xdg-open <url>` (or ashpd OpenURI). Keep `open` on the host.

---

### PKG-024

**Severity:** Low  
**Title:** Clipboard may fail on GNOME Wayland

**Evidence:** Copy snippet uses `iced::clipboard::write`, not ashpd. libcosmic `xdg-portal` is enabled but not used for this path. GNOME does not implement `wlr-data-control`; iced/winit clipboard can silently fail. There is a Fluent `toast-clipboard-failed` path in `core/apply.rs` if the write reports failure.

**Expected:** Copy works on GNOME/KDE/COSMIC/weston.

**Proposed fix:** If field reports fail on GNOME, switch copy to the portal (`ashpd` / libcosmic portal helper). No extra finish-args.

---

### PKG-025

**Severity:** Low  
**Title:** CI branch list and flake checks vs local gate

**Evidence:** Push triggers only `main` and `cosmic-migration` (not `audit-hardening`). PRs still run cargo+flake. Flake `checks` inherit GUI/helper packages, clippy (gui artifacts, `--all-targets -- --deny warnings`, no `-p gui`), fmt, audit. **No** flake `cargo test`. GitHub cargo job is the test gate. Clippy in flake may compile more than `-p gui` while using GUI native inputs (works, but is heavier than the comment implies).

**Expected:** Either add `audit-hardening` to push branches while this work lands, or rely on PRs only (document). Decide whether flake clippy is workspace-wide on purpose.

**Proposed fix:** Add a crane `cargoTest` check on a nightly/Hydra job, not necessarily every PR. Align clippy with `cargo clippy --workspace`.

---

### PKG-026

**Severity:** Low  
**Title:** LICENSE not installed into prefixes

**Evidence:** Root `LICENSE` is GPL-3. Flake `meta.license = gpl3Plus` (correct vs `GPL-3.0-or-later`). Neither Nix `postInstall` nor Flatpak `install` copies `LICENSE` to `$out/share/licenses` / `/app/share/licenses`. Metainfo `project_license` is set.

**Expected:** Common packaging policy: install the license text next to the binary.

**Proposed fix:** `install -Dm644 LICENSE …/share/licenses/nixos-toolkit/LICENSE` in both Nix and Flatpak.

---

### PKG-027

**Severity:** Info  
**Title:** Finish-args otherwise match least privilege

**Evidence:** Present: `--socket=wayland`, `--socket=fallback-x11`, `--share=ipc` (X11 SHM), `--device=dri` (wgpu), `--talk-name=org.freedesktop.Flatpak`, templates env. Absent: `--share=network` (channel update is host-side), `--filesystem=host`, `--filesystem=home`, `--device=all`, `--socket=pulse`, `--socket=session-bus`. GPU: dri only. Helper is **not** in the Flatpak. `NIXOS_TOOLKIT_TEMPLATES_DIR=/app/share/nixos-toolkit/templates` so preview works offline.

**Expected:** Keep this set, minus PKG-001 binds.

**Proposed fix:** After dropping `/etc/nixos:ro`, re-review whether `--share=ipc` is still wanted for X11-only sessions (yes, if `fallback-x11` stays).

---

### PKG-028

**Severity:** Low  
**Title:** Three desktop files already drifted

**Evidence:**

| File | Exec | Icon | StartupWMClass |
|---|---|---|---|
| `data/nixos-toolkit.desktop` | `nixos-toolkit` | `nixos-toolkit` | App ID |
| `flatpak/*.desktop` | `nixos-toolkit` | App ID | App ID |
| `install.sh` generated | `nix run github:…` | `preferences-system` | missing |

**Expected:** Two committed files (Nix vs Flatpak naming). `install.sh` must not invent a third dialect.

**Proposed fix:** PKG-009; optionally generate the Flatpak desktop from the Nix one with a small substitute (`Icon=`).

---

### PKG-029

**Severity:** Low  
**Title:** `org.freedesktop.Flatpak` is a full host escape; linter skip is a comment

**Evidence:** `--talk-name=org.freedesktop.Flatpak` lets the app `flatpak-spawn --host` any host command (`pkexec`, `cat`, `lspci`, `xdg-open`). That is required for this product. Manifest comment `# flathub-linter: skip=finish-args-flatpak-spawn-access` is **not** a Flathub skip file.

**Expected:** When/if submitted to Flathub, a real skip + review note: system manager, GUI-only, host helper via pkexec, no `--filesystem=host`.

**Proposed fix:** Add `flathub.json` / documented exception when publishing. Do not replace spawn-host with `--filesystem=host`.

---

### PKG-030

**Severity:** Low  
**Title:** NixOS module enables polkit but not an agent

**Evidence:** Module sets `security.polkit.enable = true` and installs both packages. It does not enable a DE-specific agent (`polkit-gnome`, `pantheon-agent-polkit`, KDE’s agent). COSMIC/GNOME/KDE sessions usually already have one; sway/i3/weston users hit “Unexpected response from helper” (`README` troubleshooting, `progress.md`).

**Expected:** Document “run a polkit agent” next to the module snippet. Optional: `lib.mkIf` a warning in the module description.

**Proposed fix:** README module section one-liner; do not hard-depend on `polkit_gnome` (wrong for KDE/COSMIC).

---

## QA / scripts (adequacy)

| Script | Adequacy |
|---|---|
| `scripts/verify.sh` | Correct local **full** gate. Not used in GitHub Actions. Unused `python3`. No desktop/AppStream/fmt. |
| `scripts/build-flatpak.sh` | Solid: requires `cargo-sources.json`, `--disable-rofiles-fuse`, retargets the local remote (avoids leftover `file:///tmp` remotes). |
| `scripts/smoke-flatpak.sh` | Adequate as a “window stays up 10s, no panic, no pkexec” test. Weak as a sandbox/helper-absent test (PKG-016). Headless weston + pixman is the right compositor choice. |
| `scripts/generate-cargo-sources.sh` | Pinned tools commit, submodule init, no `--git-tarballs`. Right design. Network + aiohttp/tomlkit required; correctly **not** called from verify. |
| Flake checks | fmt+audit in CI; GUI/helper/clippy only if someone runs `nix flake check`. No test. |
| GitHub `cargo` | Good native dep list for libcosmic on ubuntu-24.04. `rust-cache` present. No rustfmt (flake job covers fmt). |

Unit tests that **do** cover packaging-adjacent behavior: `crates/gui/src/helper/spawn.rs` (Flatpak no-pkexec / spawn-host / `NO_PKEXEC`), `crates/gui/tests/flags.rs` (`Flags::for_tests` helper unavailable). There is no integration test that execs `flatpak-spawn`.

---

## Feature flags and release profile

- **Crate features:** none on `gui` / `helper` / `common`. libcosmic features are explicit and appropriate for non-COSMIC (wayland+x11+wgpu+xdg-portal). `rfd` off → no file chooser dep. `dbus-config` off → no Cosmic settings daemon requirement.
- **Workspace `tokio` `features = ["full"]`:** helper crate overrides to a smaller set; GUI uses libcosmic’s tokio. Harmless but noisy.
- **Release:** LTO + single codegen unit + strip is a reasonable size/perf profile for a GUI. `strip = true` makes production `RUST_BACKTRACE` less useful; smoke sets `RUST_BACKTRACE=1` on a debug-unrelated Flatpak **release** build (limited help). Consider `split-debuginfo` later, not blocking.

---

## Proposed fix order

1. **PKG-001** — drop `/etc/nixos:ro` (and likely the other two binds).
2. **PKG-010** — inherit `repository` (one-line, About link).
3. **PKG-003 / PKG-005 / PKG-004** — wrap `--set-default`, one polkit action, stable `exec.path` aligned with Flatpak probe.
4. **PKG-002 / PKG-007 / PKG-018** — lock vs `cargo-sources` check + desktop/appstream validate on PR; keep full Flatpak as dispatch/cron.
5. **PKG-009 / PKG-028** — stop shipping a third desktop dialect; honest `install.sh`.
6. **PKG-008 / PKG-014 / PKG-015 / PKG-006** — icons, xdg-utils, metainfo, real screenshots (Flathub-shaped).
7. **PKG-011 / PKG-012 / PKG-013 / PKG-020 / PKG-026** — flake hygiene.

Do not add `--share=network`, `--filesystem=home`, Cosmic.BaseApp, or MIME handlers.

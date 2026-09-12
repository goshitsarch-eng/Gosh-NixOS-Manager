# COSMIC / libcosmic UX audit

Reconnaissance only. No product UI was changed in this pass.

**Product:** NixOS Toolkit (`nixos-toolkit`)
**GUI:** `crates/gui/` (libcosmic Application on `AppModel`)
**libcosmic pin:** `7cc116803b18d7b888eb511b009f775f036c3da7`
**Features:** `winit, wayland, x11, wgpu, tokio, xdg-portal, about, a11y`
**APP_ID:** `io.github.goshitsarch_eng.NixosToolkit`
**References:** [libcosmic widgets](https://github.com/pop-os/libcosmic/tree/master/src/widget), [cosmic-app-template](https://github.com/pop-os/cosmic-app-template), [cosmic-settings](https://github.com/pop-os/cosmic-settings), `docs/migration/ux.md`

**Method:** Full widget-tree review of `app.rs`, `view/*.rs`, `widget/*.rs`, `core/apply.rs`, Fluent catalogs, and the pinned libcosmic APIs (`settings::{view_column,section,item}`, `flex_item`, `dialog`, `toaster`, `about`, `warning`, `spin_button`, `nav_bar`). A live visual pass was **not** completed in this session (no interactive walk of every page on the COSMIC session). Verification steps below are what a follow-up visual pass should run.

---

## 1. What is already COSMIC-correct

The shell is a real libcosmic app, not a GTK clone with iced paint:

- `cosmic::Application` owns header, nav bar, context drawer, dialog overlay, and condensed nav-toggle.
- Nav is `nav_bar::Model` with symbolic icons and `Page` data; Getting Started is activated on init.
- About is `widget::about::About` in `context_drawer::about` (not a second window).
- Modals go through `Application::dialog()` → `widget::dialog()` with `suggested` / `destructive` / `standard` actions.
- Most settings pages use `settings::section` + `settings::item::builder` with `.toggler()`, `.radio()`, `.checkbox()`, `.toggler_maybe()`.
- Spacing generally comes from `theme::spacing()` rather than raw `24`.
- Toaster wraps the page (`widget::toaster`); toast timeout is 3s (GTK parity; libcosmic Short is 5s).
- Theme is applied in `Settings::theme(...)` before the first frame (avoids a light/dark flash).
- Default size 1000×700, min 640×480.
- Window / header title is `NixOS Toolkit — {page}`.
- `a11y` feature is on; SSH/WireGuard `spin_button` passes the a11y `name` (required when the feature is enabled).
- Helper-missing and busy states disable privileged actions and attach tooltips.
- Escape dismisses dialogs, then the About drawer.
- Exclusive profile selection uses `.radio()`, not independent checkboxes.

These are the right primitives. The gaps are **how** they are composed, not missing widgets.

---

## 2. Highest-impact deviations

Ranked by user-visible harm vs COSMIC convention.

| Pri | Issue | Why it matters |
|-----|--------|----------------|
| P1 | Toast close/timeout **rebuilds the entire `Toasts` map** | Closing or timing out one toast drops every other toast. `Toasts::new` takes `fn(ToastId) -> Message`; `DismissToast` ignores the id. |
| P1 | Status banner lives **inside the page scrollable** | COSMIC/Adwaita banners stay pinned under the header. Integration / helper warnings scroll away. |
| P1 | Nested scrollables (`view::root` + `code_view`) | Wheel events stick in 150–280px preview/log cards; logs do not auto-scroll to the end. |
| P1 | `Busy::LoadingState` has **no UI** | Init `ReadState` can overwrite in-progress edits with no spinner, overlay, or disabled controls. |
| P1 | Almost no `settings::flex_item` / `.flex_control()` | COSMIC Settings wraps value controls. Dropdowns, text fields, port chips, and spin buttons will overflow or squash under ~800px / condensed nav. |
| P1 | Services rows are **not** `.toggler()` list buttons | Entire COSMIC switch-row is clickable. Services wrap a standalone `toggler` in a `tooltip`, so only the switch hit-target works and AT gets a tooltip instead of a row name. |
| P2 | Apply success/fail chrome is missing | `apply-complete` / `apply-failed` FTL is unused. Result is a 3s toast; the Apply page returns to idle with no persistent status. |
| P2 | No dirty / unsaved affordance | `AppState.has_changes` is tracked and never shown. Apply is just another nav row. |
| P2 | Double `settings::view_column` | Root wraps the page column; every page builds another. Extra vertical rhythm vs a single settings column. |
| P2 | View menu contains **Quit**; Refresh is header-only | COSMIC apps: View → About (app-template). Quit belongs on the window control or a File menu, not View. |
| P2 | Dependent controls stay enabled | SSH port / root / password, WireGuard port, Bluetooth autopower remain active when the parent switch is off. |
| P2 | Theme “System” does not follow the desktop live | No `watch_config` / `ThemeMode` subscription. `Message::UpdateConfig` is dead. JSON is canonical; cosmic-config is a write-only mirror. |
| P3 | Custom widgets that duplicate list/card patterns | `bundle_expander` cards sit **outside** `settings::section`. `info_item` uses a dummy horizontal spacer as a “control”. `port_chip` is a full suggested/standard button, not a compact chip. |
| P3 | Duplicate Fluent files | `i18n/en/gui.ftl` and `i18n/en/nixos_toolkit.ftl` are the same catalog. Duplicate IDs across Fluent resources. |
| P3 | Catalog strings still English in `common::actions` | Profiles, bundles, most maintenance actions bypass Fluent. |

---

## 3. Chrome

### 3.1 Header bar

**Current**

- `header_start`: one `menu::bar` tree labeled `view` with About + Quit (`menu::Item::Button(..., None, ...)` — no icons).
- `header_center`: runtime title (`set_header_title`).
- `header_end`: `button::icon("view-refresh-symbolic")` → `RefreshSystem` (local detect, **not** `ReadState`).
- Condensed nav-toggle: runtime (not custom).
- `footer`: default `None`.

**COSMIC pattern**

- cosmic-app-template: View → About only; window chrome quits.
- Header icon buttons for global actions; **page-specific** actions swap `header_end`.
- Menu items often include symbolic icons.

**Problems**

- Quit under View is the wrong group.
- Refresh is unlabeled in the menu; FTL `menu-file` / `menu-refresh` / `menu-about` are unused.
- Header Refresh is always enabled, including during apply/maintenance, and is easy to confuse with Generations/Maintenance “Refresh”.
- No page-local header actions (Apply suggested button, Generations rollback, etc.).
- No dirty-state or busy indicator in the header.

**Proposed**

- Keep View → About (label `menu-about` = “About NixOS Toolkit”).
- Move Quit to window close, or a File menu with Quit (Ctrl+Q) if a menu entry is required.
- Keep header Refresh; tooltip should say it re-detects the host, not reloads saved state.
- Optional: when `page == Apply` and `has_changes`, put `button::suggested(apply-changes)` in `header_end`.
- Optional: header spinner when `busy != Idle`.

**Verify:** condensed width (runtime breakpoint ~648px + scale), About open, Ctrl+Q, header Refresh vs Generations Refresh.

### 3.2 Navigation

**Current:** 11 flat `Page::ALL` rows, symbolic icons, no `divider_above`, no headings, no search.

| Page | Icon |
|------|------|
| Getting Started | `go-home-symbolic` |
| Desktop Profiles | `user-desktop-symbolic` |
| Software Bundles | `package-x-generic-symbolic` |
| Custom Packages | `list-add-symbolic` |
| System Settings | `preferences-system-symbolic` |
| Hardware | `video-display-symbolic` |
| Network | `network-workgroup-symbolic` |
| Services | `system-run-symbolic` |
| Generations | `document-open-recent-symbolic` |
| Maintenance | `user-trash-symbolic` |
| Apply Changes | `emblem-synchronizing-symbolic` |

**COSMIC pattern:** COSMIC Settings groups nav (Desktop, Input, System, …) with section semantics and search. App-template uses a short ungrouped list. `segmented_button` supports `divider_above()`.

**Problems**

- Eleven destinations is long for a condensed overlay.
- Maintenance uses the trash icon (reads as “delete”, not “maintain”). Prefer `applications-system-symbolic`, `utilities-system-monitor-symbolic`, or `drive-harddisk-symbolic`.
- Apply is a **commit action**, not a settings category. COSMIC would keep it as a suggested header/footer action or a clearly separated last nav item with `divider_above()`.
- All profile rows in the Profiles **page** use catalog icon `desktop-symbolic` (not in cosmic-icons; Settings uses `user-desktop-symbolic` / per-DE names).

**Proposed**

- Insert `divider_above()` before Generations (operations) and/or Apply.
- Change Maintenance icon.
- Keep 11 pages for now (IA is frozen); do not invent a second nav.
- Consider `search_input` only if lists grow; not required at current catalog size.

**Verify:** keyboard nav in the bar, condensed overlay, icon presence on COSMIC vs GNOME icon themes.

### 3.3 Context drawer (About)

**Current:** `ContextPage::About` only. Toggle matches app-template (same page closes). Links: Repository, Issues. Author, comments, license, SVG app icon, version. No `developers`, `license_url`, or acknowledgements.

**Pattern:** `examples/about` uses Website / Repository / Support, `developers([...])`, `license_url`.

**Problems**

- Comments string names implementation (“Built with libcosmic, iced, Rust, Nix”) — product copy, not About metadata.
- No license URL; license is a bare `GPL-3.0-or-later` string.
- Close message is `ToggleAbout` (fine).

**Proposed:** add `license_url`, keep two links, shorten comments to `app-comment`. Optional developers list.

**Verify:** open About, activate both links, Escape, toggle twice.

### 3.4 Dialogs

**Current** (`Application::dialog`)

| Dialog | Primary | Secondary | Tertiary |
|--------|---------|-----------|----------|
| Confirm apply (empty) | destructive Apply | Cancel | — |
| Confirm apply (normal / packages-only) | suggested Apply | Cancel | — |
| Rollback generation | suggested Switch Now | Set for Next Boot | Cancel |
| Delete generation | destructive Delete | Cancel | — |
| Maintenance with warning | destructive Run Anyway | Cancel | — |

Escape / `DismissDialog` / apply Cancel all clear the slot. No dialog icons. Maintenance **without** warning runs immediately (GTK parity).

**Pattern:** `widget::dialog` max three actions; primary at end; optional `icon()`. COSMIC destructive = red, suggested = accent.

**Problems**

- Rollback puts **Cancel on tertiary**. Usable, but Cancel is usually secondary; the two forward actions can be primary + secondary with a less prominent tertiary, or Boot as tertiary.
- Empty-apply body is a long WARNING paragraph; dialogs prefer short body + optional `control()` for extra detail.
- No dialog icon (`dialog-warning-symbolic` / `dialog-error-symbolic`).
- Dialog focus without a text field is a known libcosmic limitation; buttons should still be tabbable.

**Proposed:** add warning/destructive icons; keep empty-apply destructive; consider shortening bodies; do not add extra windows.

**Verify:** each dialog via the real buttons; Escape; Tab cycle; empty config vs packages-only vs normal apply.

### 3.5 Toasts

**Current:** `Toasts::new(|_| Message::DismissToast)`. `update` on `DismissToast` **replaces** `self.toasts` with a new empty map. Push uses `Toast::new(text).duration(3s)` (4s for in-bundle). No toast actions.

**Pattern:** `on_close: fn(ToastId) -> Message` then `toasts.remove(id)`. Close button is `window-close-symbolic`. Limit 5.

**Problems**

- Any close or timeout wipes **all** toasts (package add can emit added + duplicate + in-bundle together).
- `DismissToast` cannot remove a single id.
- Apply/helper errors often toast the raw helper string (not Fluent, not truncated).

**Proposed:**

```rust
Message::CloseToast(widget::toaster::ToastId)
// update:
self.toasts.remove(id);
```

Keep 3s / 4s durations. Do not add action buttons unless there is an undo.

**Verify:** add several packages in one submit; close one toast; confirm others remain. Apply failure toast.

### 3.6 Status banner

**Current:** custom `widget::status_banner` (error/warning/success/info) using theme destructive/warning/success/accent containers, symbolic icons, **no close**. Priority: not NixOS > helper missing > state-load warning > integration. ARM banners are **page-local**.

**Pattern:** `widget::warning` is warning-only and always draws a close button. Custom four-tone non-dismissible bar is the right substitute (also specified in `docs/migration/ux.md`).

**Problems**

- Banner is the first child of the **scrollable** settings column → it leaves the viewport.
- Success “Integrated” uses a full-width accent/success bar on every page, which is loud for a steady-state.
- Helper-missing copy is a long paragraph in a banner.

**Proposed:** pin banner above `scrollable` in `view::root` (column: banner, then scrollable page). After first-run, consider demoting “Integrated” to onboarding-only or a quiet caption. Keep ARM banners on Bundles/Hardware.

**Verify:** not NixOS, not integrated, integrated, helper missing, failed ReadState; scroll a long page and confirm the global banner stays put.

---

## 4. Shared page layout

### 4.1 Column, padding, typography

**Current** (`view/mod.rs`):

```text
settings::view_column([banner?, page])
  .padding(space_m)
  .scrollable(Fill)
```

Every page then:

```text
settings::view_column([
  text::title2(page-…-title),
  text::body(page-…-desc),
  sections / custom widgets
])
```

**Pattern**

- One `settings::view_column` per page.
- COSMIC Settings: section titles are `text::heading` via `section().title()`; page title often lives in the **header**, not a 29px `title2` in the body.
- App-template uses `title1` + `title3` in content because it has no settings IA.
- Contract asked for `title2` + body (GTK title-1 parity).

**Problems**

- Header already shows the page name; `title2` repeats it at 29px and eats space at 640×480.
- Root + page `view_column` doubles `space_m` around the page stack.
- Contract mentioned `space_l` page margins; code uses `space_m` (this actually matches libcosmic `view_column` better — keep `space_m`).
- `section_header()` reimplements heading + caption because `section()` has title but **no description API**. That custom header is acceptable.

**Proposed**

- Root: `column[ pinned banner, scrollable(padding(space_m, page)) ]` — page returns a single `view_column`.
- Demote page title to `text::title3` or drop it and keep the body caption only.
- Keep `section_header` for titled+described groups.

**Verify:** first screen of each page at 1000×700 and 640×480; no clipped titles; spacing vs COSMIC Settings Appearance.

### 4.2 Settings rows

**Current**

- Toggles (hardware, network, system groups): `.toggler()` / `.toggler_maybe()` — **correct** (list button, click anywhere).
- Profiles: `.radio()` — **correct**.
- Bundle packages: `.checkbox()` — **correct**.
- Dropdowns, text inputs, spin buttons, chips, icon buttons: `.control(...)` on a non-clickable row.
- Info rows: `info_item` → `.control(space::horizontal())`.
- **Zero** uses of `settings::flex_item` / `.flex_control()`.

**Pattern (cosmic-settings About, hostname, hardware info):**

- Values that can wrap: `settings::flex_item(title, text::body(value))` or `.flex_control(widget)`.
- Hostname: `editable_input` (click-to-edit, submit on unfocus), not a permanently wide `text_input`.
- Info rows do **not** need a dummy spacer.

**Problems**

- `.control(text_input)` / `.control(dropdown)` / `.control(row_of_chips)` fights the settings row at narrow widths (label + control on one non-flex row).
- Dummy spacer on info rows can steal width.
- Field errors **replace** the description instead of an error caption / destructive text class.
- `text_input` `on_submit` for hostname/DNS/username/ports is the **same** message as `on_input` (live apply). There is no explicit apply control (`object-select-symbolic`). Live apply is OK if validation is visible; currently invalid hostname keeps the previous committed value while the widget still shows the typed string from the parent state — hostname invalid path **does not update** `state.hostname`, but the input is bound to `state.hostname` **or** system hostname, so the field can snap back or refuse to show the invalid draft.

**Proposed**

- Use `.flex_control()` for dropdown, text_input, spin_button, chip rows.
- Info: `flex_item(title, caption(description))` or builder without dummy space.
- Keep a draft string for hostname (like `dns_input`) so invalid input remains visible with `error-hostname-*`.
- Optional: `editable_input` for hostname to match Settings → About.

**Verify:** 640px width on System, Hardware, Network; invalid hostname/DNS; ARM thermald disabled row still readable.

### 4.3 Buttons

**Current:** `suggested` (apply, add, rollback previous, copy snippet), `standard` (secondary), `destructive` (dialogs, delete generation icon class), `icon` for trash/switch/run.

**Problems**

- Loose buttons (onboarding actions, apply toolbar, generations toolbar, refresh-preview) sit as raw `view_column` children or as `section.add(button)` list rows — they pick up **List** background and look like settings rows, or float with no alignment.
- Package trash is not `Button::Destructive`.
- Port chips use full-size suggested/standard buttons (heavy vs COSMIC compact toggles).

**Proposed:** put toolbars in a `row` with `space_s`, not inside `settings::section` unless they are row controls. Destructive class on package remove. Chips: `button::standard` + selected style, or a compact custom class; wrap with `flex_row`.

### 4.4 Empty / loading / error

| State | Current | Gap |
|-------|---------|-----|
| Packages empty | `empty_placeholder` title+caption in the list | No icon; COSMIC empty states usually icon + title + caption |
| Generations empty | same | same; also used when helper missing (reads as “installation issue”) |
| Generations loading | circular + “Loading...” in the list | Good |
| Disk loading | subtitle “Calculating…” / “Loading...” | Good |
| Init ReadState | `Busy::LoadingState` unused in views | **No spinner** |
| Apply running | circular + “Building/Validating” on the button row | Good; Complete/Failed label missing |
| Helper missing | banner + disabled buttons + tooltip | Good |
| Field errors | description swap | No error color |

**Proposed:** helper-missing generations empty copy; global or page spinner for `LoadingState`; use `apply-complete` / `apply-failed` on the Apply row after `ApplyFinished`.

### 4.5 Code / logs

**Current:** `selectable_text::monotext` in `Container::Card` + inner `scrollable` + fixed height. Not `text_editor`. Selectable: yes. Auto-scroll: **no**. Horizontal overflow: wrap WordOrGlyph.

**Proposed:** keep custom `code_view` (justified). Prefer **no inner scrollable** if the page already scrolls (profiles preview), or give logs an `Id` + `scroll_to` end on append (apply/generations/maintenance). Slightly taller logs on Apply.

---

## 5. Per-page

Each page: implementation → pattern → problems → proposed → verify.

### 5.1 Getting Started (`view/onboarding.rs`)

**Implementation:** title2/body; System Status section with info row; One-Time Setup with `code_view` (280px) + row of Copy (suggested) / Open /etc/nixos / Verify.

**Pattern:** settings sections; suggested = the one next-step action.

**Problems**

- Three buttons stuffed into one list row; will overflow condensed; Open/Verify compete with Copy.
- Snippet is not labeled “Classic” vs “Flake” beyond whatever the status row says.
- Verify has no busy state (`verify_pending` is model-only).
- View does not show Unknown integration except via global banner (hidden).

**Proposed:** put buttons **under** the section (not `section.add(row)`); disable Verify while pending; optional caption under snippet naming the import style.

**Verify:** classic vs flake snippet, copy toast, open directory, verify toasts, not-NixOS status row.

### 5.2 Desktop Profiles (`view/profiles.rs`)

**Implementation:** radio list of 13 catalog profiles; preview `code_view` 200px. Exclusive. No clear control (correct). ARM notes on `ProfileDef` unused (GTK parity — do not invent).

**Problems**

- Every profile icon is `desktop-symbolic` (weak on COSMIC icon theme).
- Catalog name/description not Fluent.
- Preview is a nested scroll in the page scroll.
- Long list with no grouping (GNOME/KDE vs tiling).

**Proposed:** per-DE symbolic icons if they exist (`preferences-desktop-symbolic` fallback); optional `divider_above` between desktop-environment and compositor rows in the **list**, not nav. Keep radios.

**Verify:** select each radio, preview updates, only one selected, keyboard activate row.

### 5.3 Software Bundles (`view/bundles.rs` + `widget/bundle_expander.rs`)

**Implementation:** ARM banner; loose `section_header`; 16 custom Card expanders (icon, title, caption, count, toggler, chevron); nested `settings::section` of checkboxes; summary card of selected ids.

**Pattern:** libcosmic has **no** expander. Custom header + body is the agreed substitute. COSMIC still expects expanders to **look like** settings list cards in one section, not a stack of independent cards with a floating header.

**Problems**

- Expanders are not children of `settings::section` / `list_column` → inconsistent padding vs every other page.
- Chevron does not expand on header click (only the icon button). Adwaita ExpanderRow toggles expansion on row click; switch stays separate — good split, but the row itself should expand.
- ARM subtitle concatenates `⚠️` **and** a compat icon (color + emoji). Drop the emoji.
- Incompatible bundles (`ArmCompat::None`): toggler and chevron disabled; body hidden even if expanded. Fine.
- Enabling a bundle still does not auto-expand (user may not see checkboxes).
- Summary is a comma-separated id dump, not a list.

**Proposed:** wrap expanders in one `list_column` or make each expander a section card with list styling; header click → expand; remove emoji; optional expand-on-enable.

**Verify:** ARM (or force arch in tests), enable/disable all packages, expand keyboard, summary updates.

### 5.4 Custom Packages (`view/packages.rs`)

**Implementation:** add `text_input` + suggested Add; Enter submits; list of id + `pkgs.{id}` + trash; empty placeholder.

**Problems**

- Input is not `text_input::search` (search.nixos.org is mentioned in copy but there is no search/browse).
- Add row is one list item (input + button) — OK if flex, currently not.
- Trash not destructive-styled; tooltip only (no a11y name API beyond tooltip).
- No confirmation on remove (OK — reversible until apply).

**Proposed:** `.flex_control` on the input; destructive trash; keep paste parser in architecture.

**Verify:** paste `pkgs.htop, pkgs.ripgrep`, duplicates, in-bundle toast, empty state.

### 5.5 System Settings (`view/system.rs`)

**Implementation:** Appearance dropdown (System/Light/Dark); hostname; DNS; username + group togglers from catalog; notes as info rows.

**Problems**

- Appearance is an **app** preference on a page named like OS settings. COSMIC Settings puts appearance under Desktop; app-template uses cosmic-config. Acceptable if the section title stays “Appearance” / “Style” and copy says **application** color scheme (it does).
- `view` reads `std::env::var("USER")` — views should not probe the environment (architecture already seeds username).
- Hostname widget is bound to committed state, so validation UX is poor (see §4.2).
- No `editable_input`.
- Group names/descriptions from catalog English.

**Proposed:** bind username like other drafts; flex controls; consider `editable_input` for hostname.

**Verify:** System/Light/Dark instant, restart persistence (`preferences.json`), invalid hostname/DNS/username captions.

### 5.6 Hardware (`view/hardware.rs`)

**Implementation:** ARM banner; GPU info; NVIDIA block iff vendor string contains `nvidia` and not ARM; audio dropdown + low-latency; bluetooth; power profile, TLP, thermald `toggler_maybe`.

**Problems**

- NVIDIA options appear only after detect; unknown GPU has no “I have NVIDIA” override.
- `power-profile-balanced-symbolic` / `sensors-temperature-symbolic` may be missing on COSMIC icon theme (fallback empty).
- Power profile vs TLP overlap is unexplained (both can be on).
- Bluetooth autopower enabled while bluetooth is off.

**Proposed:** disable autopower unless bluetooth on; fallback icons (`battery-symbolic`, `dialog-information-symbolic`); keep NVIDIA gating.

**Verify:** NVIDIA machine, non-NVIDIA, ARM thermald disabled, dropdown indices 0..n.

### 5.7 Network (`view/network.rs`)

**Implementation:** firewall; TCP chips + custom TCP; UDP chips + custom UDP; SSH + spin port + password + root dropdown + fail2ban + warning info; Tailscale; WireGuard + listen port. Format note.

**Problems**

- Chip rows in `.control(row)` overflow (four buttons).
- SSH and WireGuard child rows not disabled when parent is off.
- Enabling WireGuard also opens the UDP port (architecture) with no extra copy on the chip row.
- Warning info row is not a `warning` banner (OK as info, but it is a security recommendation).
- `spin_button` label is the numeric value **and** a11y name is the field title — correct for this pin.

**Proposed:** `flex_row` chips; `toggler_maybe` / disable spin when parent off; keep UDP UI (current code, not the old GTK gap).

**Verify:** presets vs custom split, WG port 51820 chip vs listen spin, condensed wrap.

### 5.8 Services (`view/services.rs`)

**Implementation:** 21 services in 7 groups; each row is `tooltip(settings::item.control(toggler), caption(nix option))`. Extra RustDesk client info row.

**This is the largest pattern miss on an otherwise settings-like page.**

**Problems**

- Not `.toggler()` → row click does nothing.
- Tooltip for the Nix option hides on unhover and is bad for a11y; COSMIC would put `NixOS option: …` in `description` or a context drawer.
- `gnome_tweaks` is a package, not a service; still a switch (product issue, but UX-wise it looks like a daemon).
- Note section is a one-row list.

**Proposed:** `.toggler()`; put nix option in `.description(...)` (or caption under title). Keep groups.

**Verify:** click row padding (not just the switch); tooltip/description for a sample of 21; keyboard.

### 5.9 Generations (`view/generations.rs`)

**Implementation:** Refresh + suggested Rollback; list with current caption vs switch/delete icon buttons; boot info; log 150px. Loading spinner. Delete is destructive-class icon.

**Problems**

- Toolbar not in header; Refresh here vs header Refresh is a naming collision.
- Empty state blames “installation” even when helper is missing or list failed.
- Current generation has no badge styling (plain caption).
- Switch icon `system-switch-user-symbolic` is “switch user”, not “switch generation”. Prefer `system-reboot-symbolic` / `document-open-recent-symbolic`.
- Log does not auto-scroll; height 150 is tight.
- Rollback previous disabled when busy, but the reason is not shown.

**Proposed:** distinct strings (“Reload generations”); helper-missing empty copy; keep confirm dialogs.

**Verify:** loading, empty, current vs other, both dialogs, log append.

### 5.10 Maintenance (`view/maintenance.rs`)

**Implementation:** action rows + play icon; disk size + generation count; refresh button **inside** the list; log 200px. Flake hosts relabel channel update.

**Problems**

- Play icon for destructive GC is the same as verify-store.
- `section.add(refresh)` makes Refresh Disk Info look like a setting row.
- Concurrent-run copy was specified as log text; reducer just no-ops when busy (no “already running” line).
- Nav icon (trash) biases the whole page toward delete.

**Proposed:** standard button under the disk section, not `section.add`; destructive styling only on `gc_all`; log “already running” if we keep silent ignore.

**Verify:** warning dialog for delete-old-generations; no dialog for GC; disk refresh label “Calculating…”.

### 5.11 Apply (`view/apply.rs`)

**Implementation:** preview 200px; **loose** Refresh Preview button; rebuild-type dropdown (Switch/Boot/Test/Build); suggested Apply + Dry Run + spinner/status while busy; log 200px. Confirm dialog before apply; dry-run skips dialog.

**Problems**

- `apply-complete` / `apply-failed` never rendered; `ApplyFinished` only toasts.
- Rebuild dropdown + Apply + Dry Run relationship is unclear (Dry Run ignores the dropdown and uses DryBuild — correct — but the dropdown still looks like it applies to Dry Run).
- Refresh Preview is an orphan button between sections.
- No `has_changes` hint; empty preview placeholder is easy to miss.
- Busy disables buttons but user can still change rebuild type? Dropdown stays enabled (`on_press` not gated). **Rebuild type can change during apply** in the UI; reducer will set `rebuild_type` even while busy.

**Proposed:** disable the dropdown while busy; persistent Complete/Failed text; put Refresh Preview in the preview section header row; caption on Dry Run “ignores Rebuild Mode”.

**Verify:** empty/packages/normal dialogs; dry-run no dialog; busy labels; rebuild Switch vs Boot copy in the dialog (`nixos-rebuild {rebuild}`).

---

## 6. Custom widgets vs libcosmic

| Widget | Justified? | Notes |
|--------|------------|--------|
| `status_banner` | Yes | `warning` is one tone + close button |
| `code_view` | Yes | `text_editor` is for editing |
| `bundle_expander` | Yes | expander removed from libcosmic |
| `port_chip` | Yes as a concept | Implementation is too large; wrap |
| `empty_placeholder` | Weak | Prefer icon + title + caption; or a disabled settings item |
| `section_header` | Yes | `section().title()` has no description |
| `info_item` | Workaround | Dummy spacer; use `flex_item` |

Do **not** reintroduce GTK-style custom nav, custom modal windows, or a hand-rolled settings list. The failures are composition, not missing frameworks.

Unused libcosmic pieces that would help:

- `settings::flex_item` / `.flex_control`
- `text_input::search` (packages)
- `editable_input` (hostname)
- `widget::warning` only if we want a closeable ARM bar
- `nav.insert().divider_above()`
- `toaster` close-by-id
- `dialog().icon(...)`
- `button::destructive` on in-list deletes
- `core.watch_config` / theme mode subscription
- `segmented_control` for rebuild mode (4 options) instead of a dropdown

---

## 7. Interaction, a11y, visual system

### 7.1 Keyboard and focus

| Binding | Wired | Notes |
|---------|-------|--------|
| Ctrl+Q | Menu + `subscription` | Duplicate path; OK if one captures |
| Ctrl+R / F5 | Menu keybind + subscription + header | RefreshSystem only |
| Escape | `on_escape` | Dialog, then About |
| Enter in inputs | `on_submit` | Packages add; others same as `on_input` |
| Nav bar | runtime | |
| Row toggles | hardware/system yes; **services no** | |
| Profile radios | list button | |

Missing: no Apply shortcut; no search; no skip-link; dialog focus depends on libcosmic.

### 7.2 Accessibility

- Feature `a11y` on.
- Icon-only buttons have tooltips; AccessKit names may still be empty if only `tooltip` is set — prefer `.tooltip` **and** ensure the icon button label/name is set where the API allows.
- Banner is text + icon (not color-only) — good.
- ARM `⚠️` is color+emoji; drop emoji.
- Services tooltip is the opposite of a stable accessible description.
- `spin_button` name = “SSH Port” / “Listen Port” — good.
- High contrast: app does not expose HighContrast Light/Dark (`ThemeType` variants exist in libcosmic demo). System theme should follow COSMIC HC if `system_preference()` honors it — **unverified**; no HC option in the Style dropdown.

### 7.3 Responsive / HiDPI

- Min 640×480; condensed nav at ~648px × scale (`core.rs` 280+8+360).
- Scale factor is handled by libcosmic; app does not custom-scale (correct).
- Chip rows, onboarding buttons, apply toolbar, hostname+dropdown rows will fail first.
- Nested scrollables are worse on short heights (480).

### 7.4 Theme

- Style dropdown: System / Light / Dark; instant `set_theme`.
- Saved to `$XDG_CONFIG_HOME/nixos-toolkit/preferences.json`; cosmic-config `color_scheme` mirrored on save.
- Load order: override → JSON → cosmic-config → System.
- No live follow when the desktop theme changes (`UpdateConfig` never subscribed).
- No high-contrast explicit values.

**Proposed:** if Style is System, subscribe to COSMIC theme mode; keep JSON canonical if that remains a packaging decision, but then document that COSMIC Settings theme changes will not move the app until restart unless subscribed.

### 7.5 i18n

- `i18n-embed` + `fl!` from day one — good.
- Duplicate `gui.ftl` / `nixos_toolkit.ftl`.
- Unused keys: `menu-file`, `menu-view`, `menu-refresh`, `menu-quit`, `menu-about`, `apply-complete`, `apply-failed`, several `page-*` vs `nav-*` overlaps.
- Catalog (`ProfileDef.name`, bundle names, maintenance except flake/channel override) still English.

**Proposed:** one FTL file; Fluent for catalog display strings; do not translate package ids, nix options, commands, paths.

---

## 8. Proposed change backlog (do not implement in this phase)

Ordered for a later UX implementation pass.

1. **Fix toasts:** `CloseToast(ToastId)` + `toasts.remove`.
2. **Pin the global banner** above the scrollable; keep page-local ARM banners.
3. **Single settings column** in the page; root only pins banner + scroll + padding.
4. **`flex_control` / `flex_item`** on every dropdown, text field, spin, chip row, info row.
5. **Services → `.toggler()`**; nix option as description.
6. **Loading/busy chrome:** spinner for `LoadingState`; disable rebuild dropdown while applying; render apply complete/fail.
7. **Toast/dialog copy:** no raw helper dumps; dialog icons.
8. **Menu:** View → About only (or File → Quit); stop putting Quit in View.
9. **Nav:** divider before operations/Apply; Maintenance icon; optional header Apply when dirty.
10. **Dependent rows** insensitive when parent switch is off.
11. **`code_view`:** auto-scroll logs; avoid nested scroll on short previews.
12. **Bundle expander** in a list/section; header click expands; no emoji.
13. **Theme System** live subscription.
14. **Draft hostname** field; optional `editable_input`.
15. **One Fluent catalog**; translate catalog names.
16. **Empty states** with symbolic icon; helper-missing copy on Generations.
17. **Rebuild mode** as `segmented_control` (four options) — more COSMIC than a dropdown for a primary mode.

Out of scope unless product asks: collapsing 11 pages, search, putting theme in a context drawer, COSMIC Settings-style subpages.

---

## 9. Verification plan (follow-up visual pass)

Run on COSMIC (Wayland), then once on another session if possible:

```bash
nix develop
export NIXOS_TOOLKIT_TEMPLATES_DIR=$PWD/nix/templates
# optional: NIXOS_TOOLKIT_SKIP_PRIVILEGED_INIT=1 for chrome-only
cargo run -p gui
```

Checklist:

1. First frame theme matches Style (no flash).
2. Resize 1000×700 → 640×480 → condensed nav overlay; About + condensed together.
3. Keyboard: Tab through System and Services; Space on a services **row**; Escape dialog/About; Ctrl+Q; F5.
4. Toasts: add multiple packages; close one.
5. Scroll Hardware/Apply: global banner stays; code views vs page scroll.
6. Light / Dark / System; restart; COSMIC desktop theme change while System is selected.
7. Helper missing: Apply/Generations/Maintenance disabled, preview still works.
8. Each dialog; empty apply destructive.
9. NVIDIA vs no NVIDIA; thermald on ARM if testable.
10. Icon audit: missing symbolic names show as empty boxes (profiles `desktop-symbolic`, power-profile, sensors-temperature, system-switch-user).

Automated (already in tree): `cargo test -p gui --all-targets` covers reducer dialogs/toasts intents, not pixels.

---

## 10. Screen × pattern matrix

| Surface | Uses libcosmic primitive correctly? | Main gap |
|---------|--------------------------------------|----------|
| Header | Yes | Quit in View; refresh semantics |
| Nav | Yes | Ungrouped; trash icon; Apply not separated |
| About drawer | Yes | Metadata completeness |
| Dialogs | Yes | Cancel placement; no icons |
| Toasts | API used, **handler wrong** | Reset-all on dismiss |
| Banner | Custom OK | Scrolls away |
| Onboarding | Partial | Buttons as list row |
| Profiles | Yes (radios) | Icons; nested preview scroll |
| Bundles | Custom OK, placement off | Not in settings list; emoji |
| Packages | Partial | flex; empty icon |
| System | Partial | flex; hostname draft; env in view |
| Hardware | Yes for togglers | Dependent rows; icons |
| Network | Partial | chips overflow; dependent rows |
| Services | **No** (toggler pattern) | Tooltip + non-row switch |
| Generations | Partial | empty copy; icons; toolbar |
| Maintenance | Partial | refresh as list row |
| Apply | Partial | no complete/fail; dropdown while busy |

---

## 11. Runtime note

This audit is from the widget tree and the pinned libcosmic source. It was not a click-through on the live COSMIC session. The P1 items (toast reset, banner in scroll, nested scroll, loading state, flex overflow, services hit-target) are visible in code without a screenshot; overflow and missing icons still need the visual pass in §9.

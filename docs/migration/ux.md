# NixOS Toolkit — GTK4 → libcosmic UX contract

This document is the **view-layer contract** for the COSMIC migration. It inventories every window, dialog, widget, shortcut, setting, and user flow in the current GTK4/libadwaita GUI (`crates/gui`) and maps each surface to a libcosmic equivalent following COSMIC design conventions (nav bar, settings sections, context drawer, modal dialog, toaster).

**Ownership**

| Layer | Owner | Lives in |
|---|---|---|
| Views (layout, widgets, emit `Message`) | UX | `crates/gui/src/view/*` |
| Reusable composed widgets | UX | `crates/gui/src/widget/*` |
| English Fluent catalog (every user-visible string) | UX | `i18n/en/nixos_toolkit.ftl` |
| `Message` enum, `update`, `AppState`, helper IPC, subscriptions | Architecture | `crates/gui/src/app.rs`, `state.rs`, `helper/` |
| Crate features, icons, desktop entry, flake wrapping | Packaging | `Cargo.toml`, `data/`, `flake.nix` |

Views **borrow** state and **emit** messages. They do not spawn `pkexec`, talk to the helper, write preferences, or downcast a window. Architecture owns those side effects.

GTK sources of truth (do not trust this file over the code):

- Shell: [`crates/gui/src/window.rs`](../../crates/gui/src/window.rs), [`crates/gui/src/app.rs`](../../crates/gui/src/app.rs), [`crates/gui/src/main.rs`](../../crates/gui/src/main.rs)
- Pages: [`crates/gui/src/pages/`](../../crates/gui/src/pages/)
- Prefs: [`crates/gui/src/preferences.rs`](../../crates/gui/src/preferences.rs)
- Local state: [`crates/gui/src/state.rs`](../../crates/gui/src/state.rs)
- Detection: [`crates/gui/src/integration.rs`](../../crates/gui/src/integration.rs)
- Catalogs: [`crates/common/src/actions.rs`](../../crates/common/src/actions.rs), [`crates/common/src/ipc.rs`](../../crates/common/src/ipc.rs)

References: [libcosmic book](https://pop-os.github.io/libcosmic-book/), [cosmic-app-template](https://github.com/pop-os/cosmic-app-template), [libcosmic widgets](https://github.com/pop-os/libcosmic/tree/master/src/widget), [cosmic-settings](https://github.com/pop-os/cosmic-settings).

---

## 1. Complete inventory: GTK surface → libcosmic equivalent

### 1.1 Application chrome

| GTK surface | Source | libcosmic equivalent | Notes / gaps |
|---|---|---|---|
| `adw::Application` (`APP_ID = org.nixos-toolkit.app`) | `main.rs`, `app.rs` | `cosmic::Application` with `const APP_ID: &str = "org.nixos-toolkit.app"` | Keep the ID. COSMIC window chrome (header bar, nav bar, context drawer, dialog popover) comes from the trait. |
| `adw::ApplicationWindow` 1000×700, title `"NixOS Toolkit"` | `window.rs:76-83` | `cosmic::app::Settings::default().size(Size::new(1000.0, 700.0))` + `set_window_title` | Title becomes `"NixOS Toolkit — {page}"` (COSMIC app-template convention). Min size: 640×480. |
| `adw::NavigationSplitView` sidebar 220–320px | `window.rs:189-193` | `nav_bar::Model` + `Application::nav_model` / `on_nav_select` | Runtime caps nav at 280px when not condensed (`libcosmic` `nav_bar()`). |
| `adw::Breakpoint` max-width 600sp → `collapsed` | `window.rs:196-202` | `Core::is_condensed()` + header nav-toggle | No Adwaita breakpoint API. Compact mode already collapses the nav bar; must also work outside COSMIC (winit/X11). |
| Sidebar `gtk::ListBox.navigation-sidebar` + `adw::ActionRow` + symbolic prefix | `window.rs:90-116` | `nav_bar::Model::insert().text().icon().data::<PageId>()` | First item activated on init (Getting Started). |
| Sidebar `adw::HeaderBar` title `"NixOS Toolkit"` | `window.rs:158-172` | COSMIC header bar (automatic) | Sidebar header is runtime-owned; do not draw a second title. |
| Content `adw::HeaderBar` (empty) | `window.rs:181-183` | `header_start` (menu), `header_end` (refresh) | GTK header is empty; COSMIC fills it. This is an **intentional improvement**, not a regression. |
| Content `gtk::Stack` Crossfade | `window.rs:130` | `view()` match on `nav.active_data::<PageId>()` | No crossfade. Instant page swap is COSMIC-native. |
| `adw::ToastOverlay` timeout 3s | `window.rs:204-206`, `show_toast` | `widget::toaster` wrapping the page column; `Toasts<Message>` in the model | libcosmic `Duration::Short` is **5s**, `Long` is 15s. Use `Duration::Custom(3s)` for GTK parity. |
| `adw::Banner` (status) as extra top bar | `window.rs:177-179`, `detect_and_update` | Custom `widget::status_banner` **above** page content, below header | `widget::warning` covers warning/error only. Success (“Integrated”) needs a custom container (accent / success color). Optional dismiss is **not** in GTK; do not add one. |
| `gio::Action` `app.quit` | `app.rs:64-80` | `menu::Item::Button` + `KeyBind` Ctrl+Q → `Message::Quit` | Also keep COSMIC window-close. |
| `gio::Action` `app.about` | `app.rs:71-75, 84-108` | Context drawer `ContextPage::About` via `widget::about::About` | GTK about is **not** in any visible menu. COSMIC **View → About** is the faithful place to put it. |
| `gio::Action` `win.refresh` | `window.rs:214-221`, `app.rs:81` | Header icon button + Ctrl+R / F5 → `Message::RefreshSystem` | |
| `adw::AboutWindow` | `app.rs:87-107` | `context_drawer::about(&self.about, Message::LaunchUrl, Message::CloseContext)` | Fields: name NixOS Toolkit, icon app SVG (not `preferences-system`), developer “NixOS Toolkit Contributors”, version `CARGO_PKG_VERSION`, website + issues links, license GPL-3.0, comments, “Built with” acknowledgements. |
| Implicit GNOME app menu (quit/about) | none visible | `menu::bar` in `header_start` | File / View / Help — see §4. |
| `gtk::ScrolledWindow` per page (v-auto, h-never, 24px margins, 24px spacing) | every `pages/*.rs` | `widget::scrollable` + `settings::view_column` + `theme::spacing().space_l` | Use COSMIC spacing tokens, not hard-coded 24. |
| Page `gtk::Label.title-1` + `dim-label` description | every page | `widget::text::title2` + `widget::text::body` (or caption) | |
| `adw::PreferencesGroup` | every page | `widget::settings::section().title().description()` | |
| `adw::ActionRow` title/subtitle + prefix icon | many | `settings::item::builder(title).description(sub).icon(icon)` | Control is a suffix; info-only rows omit a control. |
| `adw::SwitchRow` | system/hardware/network/services/bundles | `settings::item::builder(...).toggler(active, Message::…)` | Entire row is clickable (list button). |
| `adw::ComboRow` + `gtk::StringList` | system/hardware/network | `widget::dropdown(&options, selected_idx, Message::…)` inside `settings::item` | |
| `adw::EntryRow` + `show_apply_button` | system, network | `widget::text_input(placeholder, value).on_input.on_submit` | **No EntryRow apply button.** Submit on Enter; optional trailing `button::icon("object-select-symbolic")` for hostname/DNS/username/ports. Live `on_input` matches GTK `connect_changed`. |
| `adw::SpinRow` SSH port 1–65535 step 1 | `network.rs:193-200` | `widget::spin_button(label, value, 1, 1, 65535, Message::SshPort)` | Display the numeric label; a11y `name` = “SSH Port”. |
| `adw::ExpanderRow` + `show_enable_switch` | `bundles.rs:175-180` | **No expander widget in current libcosmic** (removed). Custom `widget::bundle_expander` — see §8. | |
| `gtk::CheckButton` (profile radio-style, `.selection-mode`) | `profiles.rs:148-152` | `settings::item::builder(...).radio(id, selected, Message::SelectProfile)` | GTK is a fake radio (independent checks, then force-uncheck). COSMIC `radio` is exclusive. |
| `gtk::CheckButton` (bundle packages) | `bundles.rs:217-240` | `settings::item::builder(display).description("pkgs.{id}").checkbox(on, Message::TogglePackage)` | |
| `gtk::ToggleButton` port presets | `network.rs:132-144` | Custom `widget::port_chip` (`button::standard` with selected class) | `segmented_button` is a poor fit (variable count, independent toggles). |
| `gtk::Entry` + Add button | `packages.rs:105-127` | `widget::text_input::search` or `text_input` + `button::suggested(fl!("add"))` | Enter submits (`on_submit`). |
| `gtk::ListBox.boxed-list` | packages, generations | `widget::list_column` / `settings::section` | |
| `gtk::TextView` editable=false monospace wrap=word, `.card` | onboarding, profiles, apply, generations, maintenance | Custom `widget::code_view` = `text::body` + `font::mono` in `scrollable` `container` (Card) | `widget::text_editor` is for editing. Logs append; previews replace. Selectable text via `selectable_text` if available. Height: ~200–280px. Auto-scroll logs to end. |
| `gtk::Button.suggested-action` | copy snippet, add pkg, apply, rollback | `widget::button::suggested` | |
| `gtk::Button.pill` | apply / dry-run | `button::suggested` / `button::standard` | COSMIC does not use pill CSS; standard COSMIC buttons are already rounded. |
| `gtk::Button.destructive` / `.error` | delete generation | `widget::button::destructive` or `button::icon` with destructive class | |
| `gtk::Button.flat.circular` trash | remove package | `button::icon(icon::from_name("user-trash-symbolic"))` + tooltip | |
| `gtk::Spinner` | apply, generations loading | `widget::progress_bar::indeterminate_circular` (or `circular`) | Hide when idle. |
| `gtk::Image::from_icon_name` | everywhere | `widget::icon::from_name(...).size(16)` (symbolic) | See §10 for non-COSMIC icon themes. |
| `gtk::gdk::Display.clipboard().set_text` | `onboarding.rs:229-231` | `iced::clipboard::write(text)` Task, **or** xdg-portal (`xdg-portal` feature) | Architecture decides; view emits `Message::CopySnippet`. |
| `xdg-open /etc/nixos` | `onboarding.rs:149-153` | `open::that_detached("/etc/nixos")` (portal-friendly) | View emits `Message::OpenNixosDir`. |
| `adw::MessageDialog` modal | apply, generations, maintenance | `Application::dialog()` returning `widget::dialog()` | Modal popover over the window. Escape / secondary = cancel. |
| `adw::StyleManager` color scheme | `app.rs:35-36`, `system.rs:113-129` | `cosmic::theme` + cosmic-config `ThemeMode`; explicit Light/Dark via `ThemeType` | See §5. Apply **before first frame** to avoid flash. |
| `UserPreferences` JSON | `preferences.rs` | Prefer cosmic-config `org.nixos-toolkit.app` v1; keep System/Light/Dark | Path today: `~/.config/nixos-toolkit/preferences.json` (`glib::user_config_dir()`). |
| No i18n (hardcoded English) | all pages | `i18n-embed` + `fl!` from day one | Not a regression. English catalog is required and complete. |
| `data/nixos-toolkit.desktop` | `data/nixos-toolkit.desktop` | Keep; `StartupWMClass` may need the COSMIC/iced class | Packaging. |
| App icon `data/icons/nixos-toolkit.svg` | data | `icon::from_svg_bytes` for About; freedesktop name `nixos-toolkit` for the window | GTK About used `preferences-system` — switch to the real app icon. |

### 1.2 Dialogs

| GTK dialog | Source | Heading / body / responses | libcosmic | Appearance |
|---|---|---|---|---|
| Apply (empty config) | `apply.rs:275-284` | “Apply Empty Configuration?” / WARNING will remove ALL managed software / Cancel, Apply | `widget::dialog().title().body().secondary_action(cancel).primary_action(destructive Apply)` | Destructive primary |
| Apply (no profile/bundles, has packages) | `apply.rs:285-292` | “Apply Configuration?” / only custom packages / Cancel, Apply | same | Suggested primary |
| Apply (normal) | `apply.rs:293-299` | “Apply Configuration?” / will run `nixos-rebuild switch` / Cancel, Apply | same | Suggested primary |
| Switch generation | `generations.rs:444-474` | “Switch to Generation {n}?” / rebuilds and activates / Cancel, “Set for Next Boot”, “Switch Now” | `dialog` with **tertiary** Cancel, **secondary** Boot, **primary** Switch Now | Primary suggested |
| Delete generation | `generations.rs:529-556` | “Delete Generation {n}?” / permanent / Cancel, Delete | secondary Cancel, primary Delete | Destructive primary |
| Maintenance with `warning` | `maintenance.rs:218-247` | “Run {name}?” / warning body / Cancel, “Run Anyway” | secondary Cancel, primary Run Anyway | Destructive primary |
| Maintenance without warning | `maintenance.rs:251` | *(no dialog — run immediately)* | keep: emit `RunMaintenance` with no dialog | — |

Default response is always Cancel. Close / Escape = cancel. Do not run the action until the primary (or Boot) message is received.

### 1.3 Toasts (3s unless noted)

| Trigger | Source | Message |
|---|---|---|
| Copy snippet | `onboarding.rs:235` | `Snippet copied to clipboard` |
| Verify → Integrated | `onboarding.rs:260` | `Integration verified! You're ready to use the toolkit.` |
| Verify → NotIntegrated | `onboarding.rs:261` | `Integration not detected. Please add the import and run 'nixos-rebuild switch'.` |
| Verify → Unknown | `onboarding.rs:262` | `Could not verify integration. Please check manually.` |
| Add 1 custom package | `packages.rs:333-337` | `Added: {pkg}` |
| Add N custom packages | same | `Added {n} packages` |
| Duplicate custom package | `packages.rs:347` | `Already added: {list}` |
| Package already in a bundle | `packages.rs:355-361` | `'{pkg}' is already in '{bundle}' bundle` (4s in GTK) |

State-load failures are **banners**, not toasts (`window.rs:387-393`).

### 1.4 Status banner

| Condition | Source | Title | Tone |
|---|---|---|---|
| `!is_nixos` | `window.rs:233-236` | `Not running on NixOS` | error |
| `Integrated` | `window.rs:239-244` | `Integrated - Ready to apply changes` | success |
| `NotIntegrated` | `window.rs:246-251` | `Setup required - See Getting Started` | warning |
| `Unknown` | `window.rs:253-255` | hidden | — |
| Helper spawn / read / timeout fail | `window.rs:387-393` | e.g. `Could not load saved state - using defaults` | warning (overrides integration banner) |

Page-local ARM banners (not the global one):

| Page | Copy |
|---|---|
| Bundles | `Running on ARM64 - some packages may not be available` |
| Hardware | `ARM64: NVIDIA drivers and Intel Thermald are not available` |

---

## 2. Navigation IA and COSMIC layout

### 2.1 Shell (top to bottom, start to end)

```
┌─ header_bar ──────────────────────────────────────────────────────────┐
│ header_start: menu::bar   header_center: title   header_end: refresh  │
│                         [nav-toggle when condensed]                   │
├─ nav_bar (or overlay when condensed) ─┬─ content ─────────────────────┤
│ 11 PageId rows with icon + label      │ status_banner (if revealed)   │
│                                       │ scrollable page view          │
│                                       │   title2 + body description   │
│                                       │   settings::section…          │
├───────────────────────────────────────┴───────────────────────────────┤
│ toaster overlay (bottom of window)                                    │
│ dialog popover (modal, centered) when Dialog != None                  │
│ context drawer (end edge) when ContextPage shown                      │
└───────────────────────────────────────────────────────────────────────┘
```

Default window **1000×700**. Nav visible ≥ condensed breakpoint; below that, `nav_bar_toggle` in the header (COSMIC runtime).

`Application::footer` stays `None` (GTK has no footer).

### 2.2 Nav bar pages (order is the IA)

Activate **Getting Started** on init (`window.rs:209-211`).

| # | `PageId` | GTK label | GTK icon | Fluent key | COSMIC icon name |
|---|---|---|---|---|---|
| 1 | `Onboarding` | Getting Started | `go-home-symbolic` | `nav-onboarding` | `go-home-symbolic` |
| 2 | `Profiles` | Desktop Profiles | `user-desktop-symbolic` | `nav-profiles` | `user-desktop-symbolic` |
| 3 | `Bundles` | Software Bundles | `package-x-generic-symbolic` | `nav-bundles` | `package-x-generic-symbolic` |
| 4 | `Packages` | Custom Packages | `list-add-symbolic` | `nav-packages` | `list-add-symbolic` |
| 5 | `System` | System Settings | `preferences-system-symbolic` | `nav-system` | `preferences-system-symbolic` |
| 6 | `Hardware` | Hardware | `video-display-symbolic` | `nav-hardware` | `video-display-symbolic` |
| 7 | `Network` | Network | `network-workgroup-symbolic` | `nav-network` | `network-workgroup-symbolic` |
| 8 | `Services` | Services | `system-run-symbolic` | `nav-services` | `system-run-symbolic` |
| 9 | `Generations` | Generations | `document-open-recent-symbolic` | `nav-generations` | `document-open-recent-symbolic` |
| 10 | `Maintenance` | Maintenance | `user-trash-symbolic` | `nav-maintenance` | `user-trash-symbolic` |
| 11 | `Apply` | Apply Changes | `emblem-synchronizing-symbolic` | `nav-apply` | `emblem-synchronizing-symbolic` |

Store `PageId` as `nav.insert().data::<PageId>(...)`. `on_nav_select` activates the id and updates the window title.

### 2.3 Context drawer

| `ContextPage` | When | Content |
|---|---|---|
| `About` | View → About | `widget::about::About` (see §1.1) |
| *(none else)* | — | Do **not** park settings here; System page already owns theme. |

Toggle: same page closes the drawer (`cosmic-app-template` pattern).

### 2.4 Dialog slot

Architecture holds `Option<DialogKind>`. `Application::dialog()` returns `Some(widget::dialog(…))` when set. Views never construct a separate window.

```
enum DialogKind {
    Apply { mode: ApplyWarn },          // Empty | PackagesOnly | Normal
    SwitchGeneration { number: u32 },
    DeleteGeneration { number: u32 },
    RunMaintenance { id: String, name: String, warning: String },
}
```

### 2.5 Toaster

`Toasts<Message>` in the model. `view()` wraps content with `widget::toaster(&self.toasts, page)`. Views emit `Message::Toast(ToastRequest::CopiedSnippet | …)`; architecture pushes and times out.

### 2.6 Header actions

| Slot | Widget | Message |
|---|---|---|
| `header_start` | `menu::bar` (File, View) | menu actions |
| `header_center` | (runtime title) | — |
| `header_end` | `button::icon("view-refresh-symbolic")` tooltip Refresh | `RefreshSystem` |
| condensed | runtime `nav_bar_toggle` | COSMIC runtime |

---

## 3. Per-page widget mapping

Shared page chrome (every page):

```text
settings::view_column([
    text::title2(fl!("page-…-title")),
    text::body(fl!("page-…-desc")),          // wrap
    /* optional page banner */
    /* sections */
])
.apply(scrollable)
```

### 3.1 Getting Started — `pages/onboarding.rs` → `view/onboarding.rs`

**Title:** Welcome to NixOS Toolkit  
**Description:** This tool helps you manage your NixOS configuration declaratively. Follow the steps below to complete the one-time setup.

| GTK control | Mapping | Emits |
|---|---|---|
| `PreferencesGroup` “System Status” | `settings::section().title(fl!("onboarding-status"))` | — |
| `ActionRow` title/subtitle, prefix `emblem-system-symbolic`, CSS error/warning | `settings::item::builder(status_title).description(status_sub).icon(...)` (no control) | — |
| Status copy when `!is_nixos` | title `Not NixOS`, sub `This tool only works on NixOS systems` | — |
| Status copy when NixOS | title `NixOS Detected ({mode})`, sub `Integration: {status} \| Version: {ver}` | — |
| `PreferencesGroup` “One-Time Setup” + description | `settings::section().title.description` | — |
| `TextView` 280px, snippet from `classic_integration_snippet` / `flake_integration_snippet` | `widget::code_view` height 280, not editable | — |
| `Button` “Copy Snippet” suggested | `button::suggested(fl!("copy-snippet"))` | `Onboarding(CopySnippet)` |
| `Button` “Open /etc/nixos” | `button::standard(fl!("open-nixos-dir"))` | `Onboarding(OpenNixosDir)` |
| `Button` “Verify Integration” | `button::standard(fl!("verify-integration"))` | `Onboarding(VerifyIntegration)` |

Snippet selection: `ConfigMode::Flake` → flake snippet; `Classic | Unknown` → classic (`onboarding.rs:215-218`). Architecture re-runs `detect_system` on Verify and Refresh.

### 3.2 Desktop Profiles — `pages/profiles.rs` → `view/profiles.rs`

**Title:** Desktop Environment  
**Description:** Select a desktop environment profile. You can only have one active at a time.

Data: `common::actions::default_profiles()` — **13** profiles: gnome, kde, xfce, mate, cinnamon, pantheon, cosmic, hyprland, sway, i3, budgie, lxqt, enlightenment. Icons in catalog are all `desktop-symbolic`.

| GTK control | Mapping | Emits |
|---|---|---|
| `PreferencesGroup` “Available Profiles” | `settings::section().title(fl!("profiles-available"))` | — |
| Per profile `ActionRow` title=name, subtitle=description, prefix=icon, suffix `CheckButton.selection-mode` | `settings::item::builder(name).description(desc).icon(icon).radio(id, selected, …)` | `Profiles(Select(id))` |
| `PreferencesGroup` “Preview” / “Nix configuration that will be generated” | section title + description | — |
| `TextView` 200px, placeholder `# Select a profile to see preview` | `code_view`; content from `common::nix::read_template` or fallback “Template file not found…” | — |

**Exclusive selection.** Selecting a profile replaces any previous one (`AppState::select_profile`). GTK does not offer “clear profile” in the UI — do not add one.

ARM notes exist on `ProfileDef` but the GTK page does **not** show them. Do not invent ARM UI here.

### 3.3 Software Bundles — `pages/bundles.rs` → `view/bundles.rs`

**Title:** Software Bundles  
**Description:** Enable bundles and expand to customize individual packages.

Data: `default_bundles()` — **16** bundles (devtools, ai-tools, gaming, virtualization, virtualbox, containers, flatpak, multimedia, office, security, communication, browsers, science, cad, utilities, fonts). Prompt said 15; **code has 16** including `ai-tools`.

| GTK control | Mapping | Emits |
|---|---|---|
| ARM `adw::Banner` warning | `widget::status_banner` warning, only if `CpuArch::detect().is_arm()` | — |
| `PreferencesGroup` “Available Bundles” / “Click to expand and customize packages” | section | — |
| `ExpanderRow` title, subtitle (ARM note with ⚠️), prefix icon, optional ARM compat icon, suffix `"{n} packages"`, enable switch, `enable_expansion` | **`widget::bundle_expander`** (§8) | `Bundles(Toggle { id, enabled })`, `Bundles(Expand { id, expanded })` |
| Inner `ActionRow` per package: display_name, `pkgs.{id}`, prefix checkbox, `title_lines(0)` | nested `settings::item::builder.checkbox` | `Bundles(TogglePackage { bundle, package, enabled })` |
| ARM `ArmCompat::None` → row insensitive | expander `enabled=false` | no toggle |
| ARM compat icons | `emblem-ok-symbolic` / `dialog-warning-symbolic` / `action-unavailable-symbolic` | — |
| `PreferencesGroup` “Selected Packages” | section | — |
| Summary `Label` in `Frame.card`: joined package ids or `No packages selected` | `text::body` in card container | — |

Enable-bundle behavior (must preserve): turning the switch **on** checks **all** packages in the bundle and writes `bundle_packages`; turning **off** unchecks all and `disable_bundle` (drops package set). Individual package toggles update the set without disabling the bundle even if the set becomes empty (GTK does this; keep it).

Sync flag `is_syncing` is architecture’s problem (ignore programmatic updates). Views are stateless w.r.t. GTK signals.

### 3.4 Custom Packages — `pages/packages.rs` → `view/packages.rs`

**Title:** Custom Packages  
**Description:** Add individual packages from nixpkgs. Paste package names from search.nixos.org.

| GTK control | Mapping | Emits |
|---|---|---|
| Group “Add Package” + long description of paste formats | section title + description | — |
| `gtk::Entry` placeholder `Package name (e.g., zed-editor, htop, neofetch)` | `text_input` `on_input` + `on_submit` | `Packages(Input(String))`, `Packages(Submit)` |
| `Button` “Add” suggested | `button::suggested` | `Packages(Submit)` |
| Group “Installed Custom Packages” / “Packages will be installed when you click Apply” | section | — |
| Empty `ActionRow` “No custom packages” / “Add packages above to get started” `.dim-label` | `widget::empty_placeholder` | — |
| Package row: title=id, subtitle=`pkgs.{id}`, prefix `package-x-generic-symbolic`, suffix trash `user-trash-symbolic` tooltip “Remove package” | settings item + icon button | `Packages(Remove(id))` |

**Parsing stays in architecture** (or a pure function in `common`), not in the view. Formats to preserve (`parse_package_input`):

- `environment.systemPackages = [ pkgs.foo pkgs.bar ];` (bracket extract)
- comma-separated
- newline-separated
- `with pkgs; …`
- space-separated (if not `pkgs.` prefix)
- single token
- strip `pkgs.`, `nixpkgs#`, trailing `;` `]` `[`
- valid: starts with ASCII letter; rest `[A-Za-z0-9._-]`; length 1–128

Duplicate vs already-in-bundle feedback is **toasts** (see §1.3). Do not add the in-bundle package.

### 3.5 System Settings — `pages/system.rs` → `view/system.rs`

**Title:** System Settings  
**Description:** Configure system-level settings. These will be applied through NixOS configuration.

| GTK control | Mapping | Emits |
|---|---|---|
| Group “Appearance” | section | — |
| `ComboRow` “Style” / “Choose application color scheme”, prefix `weather-clear-symbolic`, model System, Light, Dark | `dropdown` 3 options | `System(SetColorScheme(System\|Light\|Dark))` |
| Group “Network Identity” | section | — |
| `EntryRow` “Hostname”, prefix `computer-symbolic`, apply button, prefilled `/etc/hostname` or `"nixos"` | `text_input` + optional apply icon | `System(HostnameChanged)`, `System(HostnameApply)` |
| Info row “Note” / hostname rebuild+reboot | info item, `dialog-information-symbolic` | — |
| Group “DNS Configuration” / “Set custom DNS resolvers…” | section | — |
| `EntryRow` “DNS Servers”, prefix `network-server-symbolic` | `text_input` | `System(DnsChanged)`, `System(DnsApply)` |
| Info “Format” / commas e.g. `1.1.1.1, 8.8.8.8` | info item | — |
| Group “User Group Membership” | section | — |
| `EntryRow` “Username”, prefix `avatar-default-symbolic`, prefilled `$USER` or `"user"` | `text_input` | `System(UsernameChanged)`, `System(UsernameApply)` |
| Three `SwitchRow`s from `default_system_actions()` UserGroup: **libvirtd**, **docker**, **vboxusers** | togglers with catalog name/description/icon | `System(ToggleGroup { group, enabled })` |
| Info “Note” / rebuild + log out | info item | — |

Validation (architecture, but views should not invent extra rules):

- Hostname: non-empty, `[A-Za-z0-9-]`. Unchanged vs `/etc/hostname` is a no-op.
- DNS: comma-split, trim, **IPv4 only** (`u8.u8.u8.u8`) — GTK rejects IPv6. Keep that limitation unless architecture expands it.
- Username: non-empty, `[A-Za-z0-9_-]`.

Theme change is **instant** and saved locally (not helper state).

`default_system_actions()` also defines hostname/DNS `TextInput` actions; the GTK page **hard-codes** those rows instead of iterating. Keep the hard-coded layout (Appearance + Identity + DNS + Groups).

### 3.6 Hardware — `pages/hardware.rs` → `view/hardware.rs`

**Title:** Hardware Configuration  
**Description:** Configure graphics drivers, audio, bluetooth, and power management settings.

| GTK control | Mapping | Emits |
|---|---|---|
| ARM banner | status_banner warning | — |
| Group “Graphics Drivers” / “Configure GPU drivers for your system” | section | — |
| `ActionRow` “Detected GPU” subtitle=`lspci` parse or `Unknown GPU (lspci not available)`, prefix `video-display-symbolic` | info item | — |
| **If** detected string contains `"nvidia"` **and not ARM**: | | |
| `ComboRow` “NVIDIA Driver” / “Select which NVIDIA driver package to use”: Stable (nvidia), Beta (nvidia-beta), Open Source (nvidia-open), Nouveau (open-source, limited); default 0; prefix `application-x-firmware-symbolic` | dropdown | `Hardware(NvidiaDriver(u8))` |
| Switch “Modesetting” / Wayland recommended, default **on**, `preferences-desktop-display-symbolic` | toggler | `Hardware(NvidiaModesetting(bool))` |
| Switch “Power Management” / experimental, default off, `battery-symbolic` | toggler | `Hardware(NvidiaPowerManagement(bool))` |
| Switch “Open Kernel Modules” / Turing+, default off, `emblem-system-symbolic` | toggler | `Hardware(NvidiaOpen(bool))` |
| Group “Audio” | section | — |
| `ComboRow` “Audio Server”: PipeWire (recommended), PulseAudio, None; default 0; `audio-speakers-symbolic` | dropdown | `Hardware(AudioServer(u8))` |
| Switch “Low Latency Audio”, default off, `audio-input-microphone-symbolic` | toggler | `Hardware(AudioLowLatency(bool))` |
| Group “Bluetooth” | section | — |
| Switch “Enable Bluetooth”, default off, `bluetooth-symbolic` | toggler | `Hardware(Bluetooth(bool))` |
| Switch “Power on at Boot”, default off, `system-restart-symbolic` | toggler | `Hardware(BluetoothAutopower(bool))` |
| Group “Power Management” | section | — |
| `ComboRow` “Power Profile”: Balanced, Performance, Power Saver; default 0; `power-profile-balanced-symbolic` | dropdown | `Hardware(PowerProfile(u8))` |
| Switch “TLP Power Management”, default off, `battery-full-symbolic` | toggler | `Hardware(Tlp(bool))` |
| Switch “Thermald”: Intel-only subtitle on ARM, **insensitive on ARM**, default off, `sensors-temperature-symbolic` | toggler_maybe | `Hardware(Thermald(bool))` |
| Note group: rebuild / reboot | info item | — |

GPU detect is a side effect (`lspci`) — architecture runs it on refresh/init and passes `detected_gpu: String` into the view. View does not spawn processes.

Indices must match `HardwareConfig` (`ipc.rs:360-396`): nvidia 0..3, audio 0..2, power 0..2.

### 3.7 Network — `pages/network.rs` → `view/network.rs`

**Title:** Network & Security  
**Description:** Configure firewall rules, SSH access, and VPN settings.

| GTK control | Mapping | Emits |
|---|---|---|
| Group “Firewall” / “Configure NixOS firewall rules” | section | — |
| Switch “Enable Firewall” / “Block incoming…”, **default on**, `security-high-symbolic` | toggler | `Network(Firewall(bool))` |
| `ActionRow` “Quick Open Ports” / “Common service ports”, suffix toggle buttons SSH 22, HTTP 80, HTTPS 443, Alt HTTP 8080 | info label + row of `port_chip` | `Network(TogglePresetPort { port: u16, enabled })` |
| `EntryRow` “Additional TCP Ports”, `network-wired-symbolic` | `text_input` | `Network(CustomPortsChanged)`, `Network(CustomPortsApply)` |
| Info “Format” / `3000, 5432, 6379` | info item | — |
| Group “SSH Server” | section | — |
| Switch “Enable SSH Server”, default off, `utilities-terminal-symbolic` | toggler | `Network(Ssh(bool))` |
| `SpinRow` “SSH Port” 22, 1–65535, `network-wired-symbolic` | `spin_button` | `Network(SshPort(u16))` |
| Switch “Password Authentication”, default off, `dialog-password-symbolic` | toggler | `Network(SshPasswordAuth(bool))` |
| `ComboRow` “Root Login”: Disabled (recommended), Prohibit Password (keys only), Enabled (not recommended) → `"no"` / `"prohibit-password"` / `"yes"` | dropdown | `Network(SshRootLogin(u8))` |
| Switch “Fail2Ban”, default off, `security-medium-symbolic` | toggler | `Network(Fail2ban(bool))` |
| Warning row “Security Recommendation” / keys not passwords, disable root | info item, `dialog-warning-symbolic` | — |
| Group “VPN” | section | — |
| Switch “Tailscale”, default off, `network-vpn-symbolic` | toggler | `Network(Tailscale(bool))` |
| Info “After enabling” / `sudo tailscale up` | info item | — |
| Note: rebuild required | info item | — |

**GTK gap, do not invent UI:** `allowed_udp_ports` exists on `NetworkConfig` with **no widget**. Leave it unused.

Preset ports vs custom: on load, custom entry shows TCP ports **excluding** `{22,80,443,8080}` (`network.rs:358-367`). Chips reflect whether those four are in the vec. Preserve that split.

### 3.8 Services — `pages/services.rs` → `view/services.rs`

**Title:** System Services  
**Description:** Enable or disable common system services. Changes require a system rebuild.

**21** `SwitchRow`s in 7 groups, plus 2 info rows. Tooltip on each switch: `NixOS option: {nix_option}`.

| Group | id | Title | Description | Icon | Nix option |
|---|---|---|---|---|---|
| Hardware Services | `printing` | Printing (CUPS) | Enable printing support via CUPS | `printer-symbolic` | `services.printing.enable` |
| | `avahi` | Avahi/mDNS | Enable network service discovery (Bonjour compatible) | `network-workgroup-symbolic` | `services.avahi.enable` |
| | `fwupd` | Firmware Updates | Enable fwupd for firmware updates (LVFS) | `software-update-available-symbolic` | `services.fwupd.enable` |
| | `upower` | UPower | Power management service for laptops | `battery-symbolic` | `services.upower.enable` |
| Network Services | `networkmanager` | NetworkManager | Modern network configuration manager | `network-wired-symbolic` | `networking.networkmanager.enable` |
| | `resolved` | systemd-resolved | System DNS resolver with caching | `network-server-symbolic` | `services.resolved.enable` |
| Remote Access | `rustdesk` | RustDesk | Open-source remote desktop (like TeamViewer/AnyDesk) | `computer-symbolic` | `services.rustdesk-server.enable` |
| | *(info)* | RustDesk Client | Install rustdesk package for the client app | `dialog-information-symbolic` | — |
| Sync & Backup | `syncthing` | Syncthing | Continuous file synchronization (runs as user service) | `emblem-synchronizing-symbolic` | `services.syncthing.enable` |
| | `locate` | Locate Database | Enable mlocate/plocate for fast file searching | `system-search-symbolic` | `services.locate.enable` |
| Desktop Services | `flatpak` | Flatpak | Enable Flatpak application support | `package-x-generic-symbolic` | `services.flatpak.enable` |
| | `gnome_keyring` | GNOME Keyring | Secure storage for passwords and keys | `channel-secure-symbolic` | `services.gnome.gnome-keyring.enable` |
| | `gnome_tweaks` | GNOME Tweaks | Advanced GNOME desktop customization tool | `preferences-other-symbolic` | `environment.systemPackages.gnome-tweaks` |
| | `dconf` | dconf | Configuration system for GNOME/GTK apps | `preferences-system-symbolic` | `programs.dconf.enable` |
| Development | `docker` | Docker | Container runtime daemon | `application-x-executable-symbolic` | `virtualisation.docker.enable` |
| | `libvirtd` | libvirtd | Virtualization management daemon for KVM/QEMU | `computer-symbolic` | `virtualisation.libvirtd.enable` |
| | `postgresql` | PostgreSQL | PostgreSQL database server | `drive-harddisk-symbolic` | `services.postgresql.enable` |
| | `redis` | Redis | In-memory data structure store | `drive-harddisk-symbolic` | `services.redis.servers."".enable` |
| System | `earlyoom` | Early OOM | Kill processes early when system runs low on memory | `dialog-warning-symbolic` | `services.earlyoom.enable` |
| | `auto_upgrade` | Auto Upgrade | Automatically upgrade NixOS (use with caution) | `software-update-available-symbolic` | `system.autoUpgrade.enable` |
| | `auto_gc` | Auto Garbage Collect | Automatically clean up old Nix store paths | `user-trash-symbolic` | `nix.gc.automatic` |
| | `store_optimize` | Store Optimization | Automatically optimize Nix store (deduplication) | `drive-harddisk-symbolic` | `nix.settings.auto-optimise-store` |

Emits: `Services(Toggle { id, enabled })`.  
Trailing note row: service changes need rebuild.

GTK group title uses Pango `"Sync &amp; Backup"` — display **Sync & Backup**.

### 3.9 Generations — `pages/generations.rs` → `view/generations.rs`

**Title:** System Generations  
**Description:** View, manage, and rollback NixOS system generations. Each generation represents a complete system configuration that you can boot into.

| GTK control | Mapping | Emits |
|---|---|---|
| `Button` “Refresh” `view-refresh-symbolic` | `button::standard` + icon | `Generations(Refresh)` |
| `Button` “Rollback to Previous” suggested `edit-undo-symbolic` | `button::suggested` | `Generations(RollbackPrevious)` |
| Group “Available Generations” | section | — |
| Loading row + spinner | placeholder + circular progress | — |
| Empty: “No generations found” / installation issue, warning icon | empty_placeholder | — |
| Row title `Generation {n}` or `Generation {n} (current)` | settings item | — |
| Subtitle `{date} \| NixOS {v} \| Kernel {k}` (omit missing parts) | description | — |
| Current: prefix `emblem-ok-symbolic`, suffix badge “Current” success | caption label, no buttons | — |
| Other: prefix `document-open-recent-symbolic`; icon button `system-switch-user-symbolic` tooltip “Switch to this generation”; icon button `user-trash-symbolic` tooltip “Delete this generation” | icon buttons | `Generations(RequestSwitch(n))`, `Generations(RequestDelete(n))` |
| Group “Boot Menu” info about GRUB/systemd-boot | info item | — |
| Group “Operation Log” | section | — |
| `TextView` 150px, placeholder `# Generation operations will be logged here` | `code_view` auto-scroll | — |

Rollback with unknown/first generation only appends log text (no dialog). Switch/delete **always** confirm (§1.2).

**GTK gap:** this page shells `pkexec nix-env` itself instead of helper `ListGenerations` / `RollbackGeneration` / `DeleteGenerations`. Views must **not** spawn processes; architecture should use helper IPC. UX still shows the same buttons, dialogs, and log.

Load on first view (GTK `connect_realize`). Architecture can refresh when `PageId::Generations` becomes active.

### 3.10 Maintenance — `pages/maintenance.rs` → `view/maintenance.rs`

**Title:** System Maintenance  
**Description:** Clean up disk space and maintain your NixOS system.

Actions from `default_maintenance_actions()`:

| id | Name | Description | Icon | Warning | Command |
|---|---|---|---|---|---|
| `gc_unreachable` | Garbage Collect | Delete unreachable store objects to free disk space | `user-trash-symbolic` | — | `nix-collect-garbage` |
| `gc_all` | Delete Old Generations | Delete all old system generations (keeps current only) | `user-trash-full-symbolic` | “This will delete all old configurations. You won't be able to roll back to previous generations!” | `nix-collect-garbage -d` |
| `optimize_store` | Optimize Store | Deduplicate files in the Nix store to save space | `drive-harddisk-symbolic` | — | `nix-store --optimise` |
| `verify_store` | Verify Store | Check Nix store for integrity issues | `security-high-symbolic` | — | `nix-store --verify --check-contents` |
| `update_channels` | Update Channels | Update Nix channels to latest versions | `software-update-available-symbolic` | — | `nix-channel --update` |

| GTK control | Mapping | Emits |
|---|---|---|
| Group “Maintenance Actions” | section | — |
| `ActionRow` + suffix play `media-playback-start-symbolic` tooltip “Run this action” | settings item + icon button | `Maintenance(RequestRun { id })` — architecture looks up warning |
| Group “Disk Usage” | section | — |
| “Nix Store Size” subtitle Loading/Calculating/size/`Error - see log`, `drive-harddisk-symbolic` | info item | — |
| “System Generations” subtitle `{n} generation(s)`, `document-open-recent-symbolic` | info item | — |
| `Button` “Refresh Disk Info”; while running label “Calculating…” disabled | `button::standard` | `Maintenance(RefreshDisk)` |
| Group “Output Log” 200px | `code_view` | — |

Busy: if an action is already running, append `Another action is already running. Please wait.` to the log (toast optional; GTK uses log). Disable Run buttons while `is_running`.

Auto-load disk info when the page is first shown (GTK `realize`).

### 3.11 Apply Changes — `pages/apply.rs` → `view/apply.rs`

**Title:** Apply Changes  
**Description:** Review your configuration and apply changes to the system.

| GTK control | Mapping | Emits |
|---|---|---|
| Group “Configuration Preview” / “Nix files that will be written” | section | — |
| `TextView` 200px placeholder `# No changes to preview…` | `code_view` | — |
| `Button` “Refresh Preview” | `button::standard` | `Apply(RefreshPreview)` |
| `Button` “Apply Changes” suggested+pill | `button::suggested` — disabled + label “Applying…” / “Building…” while busy | `Apply(RequestApply)` |
| `Button` “Dry Run” tooltip “Build configuration without activating” | `button::standard` | `Apply(DryRun)` |
| `Spinner` hidden unless busy | `indeterminate_circular` | — |
| Status label hidden unless finishing: `✓ Complete` success / `✗ Failed` error; during run “Building configuration…” / “Validating configuration…” | `text::body` with success/error class | — |
| Group “Build Log” / “Output from nixos-rebuild” 200px | `code_view` auto-scroll | — |

Apply confirm dialog **before** `do_apply` (§1.2). Dry run skips the dialog (GTK does). Dry run does **not** `WriteState`. Successful apply **does** `WriteState` on the **same helper session**.

Busy: both buttons insensitive; ignore re-entry (`Already running`).

Preview uses `generate_preview_full` with profile, bundles, bundle_packages, hostname, DNS, groups, username, bluetooth, custom packages, network, services. **GTK preview omits `hardware_config`.** Call this out to architecture — UX still shows whatever preview string it is given.

---

## 4. Keyboard shortcuts and menu

GTK today (`app.rs:79-81`):

| Accelerator | Action | Visible affordance |
|---|---|---|
| Ctrl+Q | `app.quit` | none (GNOME hidden app menu) |
| Ctrl+R | `win.refresh` → `detect_and_update` | none |
| F5 | same refresh | none |
| *(none)* | `app.about` | **not wired to any widget** |

COSMIC menu (`header_start` `menu::bar`) — this is the faithful place for About/Quit, not a new feature:

**File**

| Item | Icon | Shortcut | Message |
|---|---|---|---|
| Refresh | `view-refresh-symbolic` | Ctrl+R | `RefreshSystem` |
| — | | | |
| Quit | `window-close-symbolic` | Ctrl+Q | `Quit` |

**View**

| Item | Icon | Shortcut | Message |
|---|---|---|---|
| About NixOS Toolkit | — | — | `ToggleContextPage(About)` |

Also bind **F5** to `RefreshSystem` (not shown in menu, matches GTK).

`key_binds: HashMap<menu::KeyBind, MenuAction>`:

```
Ctrl+Q  → MenuAction::Quit
Ctrl+R  → MenuAction::Refresh
F5      → MenuAction::Refresh
```

Do **not** add Ctrl+, / Settings: theme already lives on System Settings.  
Do **not** steal Super+Q (COSMIC desktop close). In-app Ctrl+Q still quits.

Escape: COSMIC runtime closes the context drawer; `on_escape` should also dismiss `DialogKind` as Cancel.

---

## 5. Settings and theme

### 5.1 What GTK stores

[`crates/gui/src/preferences.rs`](../../crates/gui/src/preferences.rs):

```json
{ "color_scheme": "system" | "light" | "dark" }
```

Path: `~/.config/nixos-toolkit/preferences.json` (`glib::user_config_dir()`). **Not** privileged helper state. Default: System.

Applied in `ApplicationImpl::startup` **before** the window (`app.rs:34-36`) via `adw::StyleManager`:

| Pref | adw |
|---|---|
| System | `ColorScheme::Default` |
| Light | `ForceLight` |
| Dark | `ForceDark` |

Changing the System page combo applies immediately and writes JSON.

### 5.2 COSMIC mapping

| Pref | libcosmic |
|---|---|
| System | Follow desktop: subscribe to `ThemeMode` (cosmic-config / settings daemon). `ThemeType::System` / dark-mode flag from `cosmic_theme::ThemeMode`. |
| Light | `ThemeType::Light` (force) |
| Dark | `ThemeType::Dark` (force) |

COSMIC **prefers** following the desktop theme; keep explicit Light/Dark for GTK parity.

Persist via **cosmic-config** key under `org.nixos-toolkit.app` / v1 (e.g. `color_scheme`). Architecture owns the struct. On first launch, if cosmic-config is empty, migrate from `~/.config/nixos-toolkit/preferences.json` if present.

Apply theme in `init` before the first `view` to avoid a flash.

Do **not** put theme in helper `AppState`. Do **not** require pkexec to change theme.

### 5.3 Other local vs privileged state

| Data | Storage | Privileged? |
|---|---|---|
| Color scheme | cosmic-config / JSON | no |
| Window size | COSMIC/iced (optional; GTK did not persist) | no |
| Selected profile, bundles, packages, hostname, DNS, groups, network, services, hardware | helper `state.json` via `ReadState`/`WriteState` | yes |
| Nav page | session only (GTK always starts on Getting Started) | no — **keep first page = Getting Started** |

---

## 6. User-flow storyboards

Message names below are **proposals**. Architecture owns the enum. Views emit these (or nested `PageMsg` mapped with `.map`).

### 6.1 First run / onboarding

1. App `init` → `RefreshSystem` + `LoadState`. Nav activates `Onboarding`.
2. Banner: Not NixOS / Setup required / Integrated / hidden.
3. Status row + snippet (classic vs flake) bind to `SystemInfo`.
4. User clicks Copy Snippet → `Onboarding(CopySnippet)` → clipboard Task + toast `CopiedSnippet` (3s).
5. User clicks Open /etc/nixos → `Onboarding(OpenNixosDir)` → `open::that_detached`.
6. User pastes snippet, rebuilds outside the app.
7. User clicks Verify Integration → `Onboarding(VerifyIntegration)` → re-detect → toast + status/banner update.

### 6.2 Restore state

1. `LoadState` spawns helper `ReadState` (pkexec prompt — same as GTK).
2. Success → architecture replaces `AppState` → views re-render from the new model (no `sync_from_state` mutation).
3. Failure → banner warning, defaults. App remains usable.

### 6.3 Select a desktop profile (exclusive)

1. Nav → Profiles.
2. User activates a radio row → `Profiles(Select("kde"))`.
3. Preview `code_view` shows template (or missing-template fallback).
4. `AppState.selected_profile = Some("kde")`, `has_changes = true`.

### 6.4 Enable a bundle and customize packages

1. Nav → Bundles. ARM banner if aarch64. Incompatible bundles (`ArmCompat::None`) disabled.
2. User turns expander switch on → `Bundles(Toggle { id, enabled: true })` → all package checkboxes on.
3. User expands → `Bundles(Expand { id, true })` (view-only UI state; architecture may keep `HashSet` of expanded ids).
4. User unchecks a package → `Bundles(TogglePackage { … enabled: false })`.
5. Summary card lists remaining package ids (sorted, unique).

### 6.5 Add / remove custom packages

1. Nav → Packages.
2. User pastes `pkgs.htop, pkgs.ripgrep` → `Packages(Input)` (controlled field).
3. Enter or Add → `Packages(Submit)` → parse → toasts for added / duplicate / in-bundle.
4. List rows appear; empty placeholder hides.
5. Trash → `Packages(Remove("htop"))`. If last item, placeholder returns.

### 6.6 System hostname / DNS / groups / theme

1. Nav → System.
2. Style dropdown → `System(SetColorScheme(Dark))` → instant theme + save prefs.
3. Hostname input → `HostnameChanged` (validate; ignore invalid). Enter/apply → `HostnameApply`.
4. DNS comma list → `DnsChanged` / `DnsApply` (IPv4 only).
5. Username → `UsernameChanged` / `UsernameApply`.
6. Group togglers → `ToggleGroup { group: "docker", enabled: true }`.

### 6.7 Hardware GPU / audio / BT / power

1. Nav → Hardware. Detected GPU row already filled.
2. If NVIDIA (x86): driver dropdown + three switches.
3. Audio server dropdown; low-latency switch.
4. Bluetooth + autopower.
5. Power profile; TLP; Thermald (disabled on ARM).
6. Each control emits the corresponding `Hardware(…)` message. Note row reminds: rebuild / possibly reboot.

### 6.8 Network firewall / SSH / VPN

1. Nav → Network. Firewall on by default.
2. Chip “HTTPS (443)” → `TogglePresetPort { port: 443, enabled: true }`.
3. Additional TCP field → parsed u16s merged (architecture).
4. Enable SSH → port spin 22 → 2222 (`SshPort(2222)`), password off, root Disabled, optional Fail2ban.
5. Tailscale on → info row still visible.
6. UDP ports remain untouched (no UI).

### 6.9 Toggle services

1. Nav → Services.
2. Any of 21 togglers → `Services(Toggle { id, enabled })`.
3. Tooltips show Nix option (accessibility + power users).

### 6.10 Generations list / switch / delete

1. Nav → Generations → architecture `Generations(Refresh)` (or auto on select).
2. Loading spinner, then rows newest-first.
3. Rollback to Previous → if current > 1, same as switch to `current - 1` (dialog); else log-only.
4. Switch icon → dialog Cancel / Set for Next Boot / Switch Now → `ConfirmSwitch { n, mode: Switch|Boot }`.
5. Delete icon → destructive dialog → `ConfirmDelete(n)`.
6. Log appends streaming result; list refreshes on success.

### 6.11 Maintenance

1. Nav → Maintenance → `RefreshDisk` (store size + generation count).
2. Run Garbage Collect → no dialog → `RunMaintenance("gc_unreachable")` → log stdout/stderr.
3. Run Delete Old Generations → dialog “Run Anyway” → then run.
4. Concurrent run: log “already running”, no second pkexec.

### 6.12 Preview, dry-run, apply (pkexec + streaming log + save state)

1. Nav → Apply.
2. Refresh Preview → `Apply(RefreshPreview)` → `code_view` from `generate_preview_full`.
3. Dry Run → no dialog → `Apply(DryRun)` → spinner “Validating…” / button “Building…” → stream `HelperResponse::Log` into build log → no `WriteState`.
4. Apply Changes → dialog (empty / packages-only / normal) → `Apply(Confirm)` → `EnsureDirectories` then `Apply { rebuild_type: Switch }` on one helper session → stream logs → on success `WriteState` same session → status “Complete” / “Failed”.
5. Failure to spawn helper: log “Failed to start helper” + manual `sudo nixos-rebuild switch` hint (keep that copy).

### 6.13 Theme change instant

System page Style → `SetColorScheme` → `theme::set_theme` + persist. No rebuild, no toast (GTK is silent).

### 6.14 Refresh system detection

Ctrl+R / F5 / header refresh / File → Refresh → `RefreshSystem` → re-detect NixOS, mode, integration, hostname, GPU; update banner + onboarding status. Does **not** re-read helper state (GTK `detect_and_update` does not).

### 6.15 Toast feedback

Any `Message::Toast(…)` / page-level success copies listed in §1.3. Architecture `toasts.push(Toast::new(fl!(…)).duration(3s))`.

---

## 7. Accessibility and i18n

### 7.1 Accessibility

libcosmic feature `a11y` (iced accessibility) **on**.

| Rule | How |
|---|---|
| Every settings row has a title (and description where GTK had a subtitle) | `settings::item::builder(title).description(sub)` |
| Icon-only buttons have tooltips **and** accessible names | Refresh, Remove package, Switch generation, Delete generation, Run action, dialog buttons |
| `spin_button` SSH port | `name` = “SSH Port” |
| Radio group | one selected profile; labels are profile names |
| Togglers | row click + switch; state in the title |
| Dialogs | `title` + `body`; primary/secondary labeled; Escape cancels |
| Status banner | text, not color-only (error/warning/success **and** copy) |
| Code/log views | selectable; not editable; wrap |
| Contrast | COSMIC theme; high-contrast follows desktop |
| Keyboard | nav bar (runtime), menu accelerators, Enter submits inputs, Escape closes dialog/drawer |
| Focus | dialog should receive focus (known libcosmic issue if no text field — buttons still tabbable) |

Do not rely on color alone for ARM warnings (icon + text).

### 7.2 i18n plan

GTK has **zero** gettext. COSMIC convention is Fluent (`i18n-embed` + `rust-embed` + `fl!`). **Add Fluent from day one.** English catalog is mandatory and must contain every user-visible string (including toasts, dialogs, placeholders, tooltips, log *user-facing* lines, combo options).

Layout:

```
i18n/en/nixos_toolkit.ftl
crates/gui/src/i18n.rs    (architecture may own the loader; UX owns keys + English text)
```

Identifier: `nixos_toolkit`. Fallback: `en`.

**Key convention:** `{page}-{control}[-qualifier]`. Placeholders use Fluent variables (`{ $name }`, `{ $count }`).

Profile/bundle/service **catalog strings** currently live in `common::actions` as English. Phase 2 options (architecture decides):

1. Keep catalog English and wrap in `fl!` only for chrome (fast, incomplete i18n).
2. **Preferred:** move display names/descriptions to Fluent (`profile-gnome-name`, `bundle-gaming-desc`, `service-docker-name`, …) keyed by id. `common` keeps ids/icons/templates only.

Until (2) lands, the English FTL still lists chrome + dialogs + toasts + page titles; catalog strings stay English in `actions.rs`.

Do not translate:

- Package ids (`pkgs.zed-editor`)
- Nix options (`services.printing.enable`)
- Commands (`nixos-rebuild switch`, `sudo tailscale up`)
- Generation numbers, versions, hostnames, ports
- File paths (`/etc/nixos`)

### 7.3 English catalog — chrome and flows (minimum)

```fluent
app-title = NixOS Toolkit
app-comment = A declarative NixOS system management tool

nav-onboarding = Getting Started
nav-profiles = Desktop Profiles
nav-bundles = Software Bundles
nav-packages = Custom Packages
nav-system = System Settings
nav-hardware = Hardware
nav-network = Network
nav-services = Services
nav-generations = Generations
nav-maintenance = Maintenance
nav-apply = Apply Changes

menu-file = File
menu-view = View
menu-refresh = Refresh
menu-quit = Quit
menu-about = About NixOS Toolkit

about-developer = NixOS Toolkit Contributors
about-website = Website
about-issues = Report an issue
about-built-with = Built with
# license shown as GPL-3.0 via About.license

banner-not-nixos = Not running on NixOS
banner-integrated = Integrated - Ready to apply changes
banner-setup-required = Setup required - See Getting Started
banner-state-helper-comm = Failed to communicate with helper
banner-state-read-error = State read error: { $message }
banner-state-timeout = State read timeout
banner-state-spawn = Could not load saved state - using defaults
banner-arm-bundles = Running on ARM64 - some packages may not be available
banner-arm-hardware = ARM64: NVIDIA drivers and Intel Thermald are not available

toast-copied-snippet = Snippet copied to clipboard
toast-verify-ok = Integration verified! You're ready to use the toolkit.
toast-verify-missing = Integration not detected. Please add the import and run 'nixos-rebuild switch'.
toast-verify-unknown = Could not verify integration. Please check manually.
toast-pkg-added = Added: { $pkg }
toast-pkg-added-n = Added { $count } packages
toast-pkg-duplicate = Already added: { $list }
toast-pkg-in-bundle = '{ $pkg }' is already in '{ $bundle }' bundle

dialog-cancel = Cancel
dialog-apply = Apply
dialog-delete = Delete
dialog-run-anyway = Run Anyway
dialog-switch-now = Switch Now
dialog-boot-next = Set for Next Boot
# plus heading/body keys per dialog in §1.2

copy-snippet = Copy Snippet
open-nixos-dir = Open /etc/nixos
verify-integration = Verify Integration
add = Add
refresh = Refresh
refresh-preview = Refresh Preview
refresh-disk = Refresh Disk Info
refresh-disk-busy = Calculating…
apply-changes = Apply Changes
dry-run = Dry Run
rollback-previous = Rollback to Previous
remove-package = Remove package
run-action = Run this action
switch-generation = Switch to this generation
delete-generation = Delete this generation
generation-current = Current
loading = Loading...
```

Page title/description keys, combo option keys (`style-system`, `nvidia-stable`, `audio-pipewire`, `root-login-no`, …), placeholders, notes, and log *chrome* (`# Select a profile to see preview`, `✓ Complete`, `✗ Failed`) all go in the same file. UX fills the FTL when implementing views.

---

## 8. Faithful alternatives (no 1:1 widget)

| Missing GTK widget | Closest COSMIC | How we use it |
|---|---|---|
| `adw::Banner` | `widget::warning` (warning/error, optional close) | **Custom `widget::status_banner`**: full-width bar, three tones (error / warning / success), **no close button** (GTK banners are not dismissible). Page-local ARM banners use the same widget. Success uses accent/success container style, not `warning`’s orange. |
| `adw::ToastOverlay` | `widget::toaster` + `Toasts` | Wrap the page. `Duration::Custom(3s)` to match GTK (default Short is 5s). Optional action button unused (GTK toasts had none). |
| `adw::ExpanderRow` + enable switch | **Removed** from libcosmic. `widget::cards` is a notification stack, not a settings expander. | Custom `widget::bundle_expander`: header row = icon + title + subtitle + package count + `toggler` + chevron `button::icon("pan-down-symbolic" / "pan-up-symbolic")`. Body = nested `settings::section` of checkboxes, shown iff expanded. Chevron does not enable the bundle; the toggler does. |
| `adw::SpinRow` | `widget::spin_button` | Put as the control of a settings item. Label is the numeric value; item title is “SSH Port”. |
| `adw::ComboRow` | `widget::dropdown` | Control of a settings item. |
| `adw::EntryRow` + apply button | `text_input` + `on_submit` + optional trailing `object-select-symbolic` | Live `on_input` matches GTK `changed`. Apply button is optional parity, not required if Enter submits. |
| `adw::SwitchRow` | `settings::item::builder.toggler` | Row is a `list::button` (click anywhere). |
| `adw::ActionRow` | `settings::item::builder` ± control | Info rows have no control. |
| `adw::PreferencesGroup` | `settings::section` | |
| `adw::AboutWindow` | `widget::about::About` in context drawer | Not a separate window. |
| `adw::MessageDialog` | `widget::dialog` via `Application::dialog()` | Modal popover. Three actions max: tertiary / secondary / primary. Generation switch uses all three. |
| `adw::NavigationSplitView` + `Breakpoint` | `nav_bar::Model` + `Core::is_condensed` | Collapse is runtime. Extra: header nav toggle. Target condensed ~600px equivalent. |
| `gtk::TextView` (read-only monospace) | Custom `widget::code_view` | `scrollable` + `container` (Card) + `text::body` with mono font. Fixed min height. Logs: append + scroll to end (`operation::scroll_to` or keep a `widget::Id` and `Task`). Previews: replace string. **Not** `text_editor` (that is for editing). |
| `gtk::StackTransitionType::Crossfade` | none | Instant swap. |
| `gtk::Spinner` | `progress_bar::indeterminate_circular` | |
| `gtk::CheckButton` radio CSS | `settings::item::builder.radio` | True exclusive group. |
| `gtk::ToggleButton` chips | Custom `widget::port_chip` | `button::standard` with selected/accent class when on. |
| `gtk::ListBox.boxed-list` | `settings::section` / `list_column` | |
| `.suggested-action` / `.destructive` / `.pill` / `.success` / `.error` | `button::suggested` / `button::destructive` / theme text classes | No pill. |
| Pango markup escape on bundle titles | not needed | COSMIC text is not Pango; pass plain strings. |
| `adw::Toast` timeout 3 vs 4s (in-bundle) | `Duration::Custom` | 3s default; 4s for in-bundle toast to match GTK. |

### 8.1 `widget::bundle_expander` sketch

```
row[
  icon(bundle.icon),
  column[ title(name), caption(subtitle) ]  // fill
  caption("{n} packages"),
  toggler(enabled) → ToggleBundle,
  icon_button(chevron) → ExpandBundle,     // disabled if ArmCompat::None
]
if expanded && sensitive:
  settings::section of checkbox rows
```

Do not use `widget::cards` (wrong metaphor: stacked notifications with “clear all”).

### 8.2 `widget::status_banner` sketch

```
container(row[ optional icon, text::body(message) ])
  .width(Fill)
  .padding(space_xs)
  .class(BannerError | BannerWarning | BannerSuccess)
```

Revealed iff `Option<BannerState>` is `Some`. Architecture sets it from `SystemInfo` / load errors.

### 8.3 `widget::code_view` sketch

```
scrollable(
  container(text::body(contents).font(mono))
    .class(Card)
    .padding(12)
    .width(Fill)
)
.height(Length::Fixed(h))   // 150 / 200 / 280 as in GTK
```

---

## 9. Files UX owns in Phase 2

Do **not** create `app.rs` / `state.rs` / helper client. Architecture owns those. UX adds:

```
crates/gui/src/view/
    mod.rs              // PageId, view_page() dispatcher, shared page chrome helper
    onboarding.rs       // Getting Started
    profiles.rs         // radios + preview
    bundles.rs          // expander list + summary
    packages.rs         // add field + list
    system.rs           // appearance, hostname, DNS, groups
    hardware.rs         // GPU/audio/bt/power
    network.rs          // firewall/SSH/VPN
    services.rs         // 21 togglers in 7 sections
    generations.rs      // toolbar, rows, log
    maintenance.rs      // actions, disk, log
    apply.rs            // preview, actions, build log
    dialog.rs           // dialog() element for DialogKind (layout only)
    about.rs            // About builder (or inline in app header_start context)
    menu.rs             // menu::bar + MenuAction → Message (if architecture agrees)

crates/gui/src/widget/
    mod.rs
    status_banner.rs    // error/warning/success bar
    code_view.rs        // read-only monospace card
    bundle_expander.rs  // ExpanderRow+switch replacement
    port_chip.rs        // preset port toggle
    empty_placeholder.rs
    // optional thin helpers:
    page_header.rs      // title2 + body
    info_row.rs         // icon + title + subtitle, no control

i18n/en/nixos_toolkit.ftl
```

Each `view/*.rs` exports:

```rust
pub fn view<'a>(&'a PageModel, &'a AppState, &'a UiBits) -> Element<'a, Message>;
```

or, if architecture keeps page models inside `App`:

```rust
impl BundlesPage {
    pub fn view(&self, state: &AppState, arm: bool) -> Element<'_, Message> { … }
}
```

Views take **borrowed** `AppState` / `SystemInfo` / page-local UI flags (`expanded_bundles`, `package_input`, `is_applying`, `preview_text`, `log_text`). They return `Element<Message>` only.

`view/mod.rs` maps `PageId` → page view. Architecture’s `Application::view` wraps: banner + page + toaster.

### 9.1 What each view contains (and does not)

| File | Contains | Does not |
|---|---|---|
| `onboarding.rs` | status item, snippet view, 3 buttons | clipboard, xdg-open, detect_system |
| `profiles.rs` | radio list, preview | `read_template` (architecture supplies preview string) |
| `bundles.rs` | ARM banner, expanders, summary | mutating HashSets |
| `packages.rs` | input row, list, placeholder | parser (call into `common` or emit raw string) |
| `system.rs` | appearance + identity + DNS + groups | writing preferences.json |
| `hardware.rs` | groups; NVIDIA block iff `show_nvidia` | `lspci` |
| `network.rs` | firewall chips + SSH + VPN | merging port vecs (can be view-local display of state) |
| `services.rs` | 7 sections of togglers | ServicesConfig struct writes |
| `generations.rs` | toolbar, rows, boot info, log | pkexec / filesystem |
| `maintenance.rs` | action rows, disk rows, log | helper spawn |
| `apply.rs` | preview, buttons, spinner, status, log | helper session / timeouts |
| `dialog.rs` | widget::dialog tree for current `DialogKind` | presenting windows |
| `menu.rs` | File/View trees | key handling (runtime) |

### 9.2 Widget module rules

- Short, factual comments only for non-obvious constraints.
- Generic over `Message`.
- No helper IPC, no `AppState` mutation.
- Icons via `icon::from_name`; sizes 16 unless About (128).

---

## 10. Open questions (architecture / packaging)

These block or shape view implementation. UX default is in **bold**.

1. **Clipboard**  
   GTK uses the GDK clipboard. COSMIC options: `iced::clipboard::write` vs `xdg-portal` (`ashpd`) vs `wl-clipboard` binary.  
   **Prefer `iced::clipboard::write` (works X11/Wayland); enable `xdg-portal` for file/folder portals. Do not shell out to `wl-copy`.**

2. **Open `/etc/nixos`**  
   GTK `xdg-open`. Template uses `open::that_detached`. Portal may deny opening `/etc`.  
   **Use `open` crate; on failure toast `Could not open /etc/nixos`.** Need a Fluent string.

3. **Icon theme on non-COSMIC**  
   `icon::from_name("…-symbolic")` needs a freedesktop theme (Pop, Adwaita, Papirus, Cosmic). NixOS + KDE/sway may lack those names (`user-desktop-symbolic`, `emblem-synchronizing-symbolic`, `power-profile-balanced-symbolic`, `sensors-temperature-symbolic`, `system-switch-user-symbolic`).  
   **Packaging: depend on `adwaita-icon-theme` (or `cosmic-icons`) in the Nix wrapper. Bundle a fallback SVG set for the 11 nav icons + app icon. Missing icons must not blank the row — show the text label.**

4. **Symbolic icons availability**  
   Confirm the wrapper installs an icon theme and `XDG_DATA_DIRS` includes it when running under `nix run` / `nix develop`.

5. **App icon in About**  
   GTK used `preferences-system`. **Use `data/icons/nixos-toolkit.svg` via `icon::from_svg_bytes`.**

6. **Theme persistence**  
   cosmic-config vs keep JSON. **cosmic-config + one-time JSON migrate.**

7. **Forced Light/Dark outside COSMIC**  
   `ThemeType::Light/Dark` should still work on GNOME/KDE via libcosmic’s own theme. Verify in packaging.

8. **Helper / pkexec prompts**  
   Load-state on startup still prompts (GTK does). **Keep it.** Alternative (lazy load) is architecture’s call.

9. **Generations IPC**  
   GTK bypasses helper. **Views assume architecture uses helper messages; UI stays the same.**

10. **Hardware in Apply preview/IPC**  
    GTK `Apply` request and preview omit `hardware_config` even though the page writes it into `AppState`. **Architecture should include it; UX will render whatever preview string it gets.**

11. **UDP ports**  
    No GTK UI. **Do not add.**

12. **IPv6 DNS**  
    GTK validates IPv4 only. **Keep unless architecture expands.**

13. **`text_editor` vs static text for logs**  
    Streaming append is easier with a `String` + `code_view`. `text_editor::Content` is heavier. **Use `String`.**

14. **Auto-scroll logs**  
    Need a `widget::Id` + iced operation, or always render the full string in a scrollable (user can drag). **Best-effort scroll-to-end Task on log append.**

15. **Condensed breakpoint**  
    libcosmic condensed ≠ 600sp. **Accept runtime condensed; if we must match 600px, architecture can set size limits / listen to window resize.**

16. **Nav icons vs COSMIC icon names**  
    Some Adwaita names differ in cosmic-icons. **Keep the GTK names first; packaging maps aliases if needed.**

17. **Window class / desktop file**  
    `StartupWMClass=nixos-toolkit` vs iced class. **Packaging updates `.desktop` after first COSMIC run.**

18. **MenuAction vs app Message**  
    Template maps `MenuAction` → `Message`. **UX writes `view/menu.rs` assuming that.**

19. **Page-local UI state** (expanded bundles, package input draft, log strings, preview string, applying flag, selected dialog)  
    **Architecture stores it on `App` / page structs; views borrow it.**

20. **i18n of `actions.rs` catalogs**  
    **Preferred: Fluent keys per id in Phase 2. Fallback: English from `common` until architecture splits metadata.**

21. **`a11y` + `xdg-portal` + `wgpu` features**  
    **Request `a11y`, `tokio`, `winit`, `wayland`, `xdg-portal`, `about`. `wgpu` is packaging performance.**

22. **First-run pkexec + not-NixOS**  
    On non-NixOS, `LoadState` will fail and show the warning banner **and** “Not running on NixOS”. GTK overwrites the banner with the last writer. **Priority: Not NixOS > state-load warning > integration.** Architecture should not clobber `!is_nixos`.

---

## Appendix A — GTK widget census (by file)

| File | Lines (approx) | Groups / primary widgets |
|---|---|---|
| `window.rs` | 467 | split view, 11 nav rows, banner, toast overlay, breakpoint |
| `app.rs` | 115 | about window, quit/about/refresh actions |
| `preferences.rs` | 61 | color scheme JSON |
| `pages/onboarding.rs` | 278 | 1 status row, snippet, 3 buttons |
| `pages/profiles.rs` | 243 | 13 radio rows, preview |
| `pages/bundles.rs` | 473 | 16 expanders + N checkboxes, ARM banner, summary |
| `pages/packages.rs` | 515 | entry+add, dynamic list |
| `pages/system.rs` | 473 | style combo, 3 entries, 3 group switches |
| `pages/hardware.rs` | 400 | GPU info, 4 NVIDIA, 2 audio, 2 BT, 3 power |
| `pages/network.rs` | 411 | firewall, 4 chips, ports entry, 5 SSH, 1 VPN |
| `pages/services.rs` | 476 | 21 switches, 2 info rows |
| `pages/generations.rs` | 647 | 2 buttons, dynamic rows, log, 2 dialogs |
| `pages/maintenance.rs` | 523 | 5 actions, 2 disk rows, log, 1 dialog type |
| `pages/apply.rs` | 707 | preview, 3 buttons, spinner, status, log, 1 dialog |

## Appendix B — Proposed `PageId` / `Message` sketch

Architecture may nest or flatten. Views will match whatever lands; this is the UX vocabulary.

```
enum PageId {
    Onboarding, Profiles, Bundles, Packages, System, Hardware,
    Network, Services, Generations, Maintenance, Apply,
}

enum Message {
    Nav(PageId), // if we emit from custom widgets; runtime uses on_nav_select
    Quit,
    RefreshSystem,
    ToggleContextPage(ContextPage),
    LaunchUrl(String),
    ToastDismiss(ToastId),

    Onboarding(OnboardingMsg),
    Profiles(ProfilesMsg),
    Bundles(BundlesMsg),
    Packages(PackagesMsg),
    System(SystemMsg),
    Hardware(HardwareMsg),
    Network(NetworkMsg),
    Services(ServicesMsg),
    Generations(GenerationsMsg),
    Maintenance(MaintenanceMsg),
    Apply(ApplyMsg),

    DialogCancel,
    DialogConfirm, // meaning depends on DialogKind
}

enum OnboardingMsg { CopySnippet, OpenNixosDir, VerifyIntegration }
enum ProfilesMsg { Select(String) }
enum BundlesMsg {
    Toggle { id: String, enabled: bool },
    Expand { id: String, expanded: bool },
    TogglePackage { bundle: String, package: String, enabled: bool },
}
enum PackagesMsg { Input(String), Submit, Remove(String) }
enum SystemMsg {
    SetColorScheme(ColorSchemePreference),
    HostnameChanged(String), HostnameApply,
    DnsChanged(String), DnsApply,
    UsernameChanged(String), UsernameApply,
    ToggleGroup { group: String, enabled: bool },
}
enum HardwareMsg {
    NvidiaDriver(u8), NvidiaModesetting(bool),
    NvidiaPowerManagement(bool), NvidiaOpen(bool),
    AudioServer(u8), AudioLowLatency(bool),
    Bluetooth(bool), BluetoothAutopower(bool),
    PowerProfile(u8), Tlp(bool), Thermald(bool),
}
enum NetworkMsg {
    Firewall(bool),
    TogglePresetPort { port: u16, enabled: bool },
    CustomPortsChanged(String), CustomPortsApply,
    Ssh(bool), SshPort(u16), SshPasswordAuth(bool), SshRootLogin(u8),
    Fail2ban(bool), Tailscale(bool),
}
enum ServicesMsg { Toggle { id: String, enabled: bool } }
enum GenerationsMsg {
    Refresh, RollbackPrevious, RequestSwitch(u32), RequestDelete(u32),
    ConfirmSwitch { number: u32, mode: SwitchMode }, ConfirmDelete(u32),
}
enum SwitchMode { Switch, Boot }
enum MaintenanceMsg { RequestRun { id: String }, ConfirmRun { id: String }, RefreshDisk }
enum ApplyMsg { RefreshPreview, RequestApply, Confirm, DryRun }
```

---

*End of UX contract. Phase 2 implements `crates/gui/src/view/*` and `crates/gui/src/widget/*` against this mapping; it does not invent pages or drop controls listed here.*

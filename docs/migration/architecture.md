# NixOS Toolkit — libcosmic migration architecture

This document is the architecture contract for the GTK4/libadwaita → libcosmic rewrite of the GUI crate. It maps current GObject pages, signals, `RefCell` state, and helper IPC onto libcosmic’s `Application` model (`Message`, `update`, `view`, `Task`, `subscription`).

**Scope of this phase:** design only. Implementation of `crates/gui` (and a narrowly-scoped hardware IPC completeness patch in `common`/`helper` if the lead assigns it) happens in Phase 2.

**Non-goals:** rewriting Nix generation except to close the HardwareConfig apply gap; changing the helper’s JSON protocol shape except where completeness requires it; pixel-level UI (owned by UX).

**Priority order** (when trade-offs conflict):

1. Feature parity with the original GTK app
2. Working correctly in Flatpak
3. Accessibility
4. Simplicity
5. COSMIC conventions

---

## 1. Current vs target architecture

### 1.1 Current (GTK4 / libadwaita)

```
 ┌──────────────────────────────────────────────────────────────────┐
 │  gui binary (unprivileged)                                       │
 │                                                                  │
 │  NixosToolkitApp (adw::Application)                              │
 │    activate → MainWindow (adw::ApplicationWindow)                │
 │      RefCell<AppState>     // incomplete source of truth         │
 │      RefCell<SystemInfo>                                         │
 │      gtk::Stack of GObject pages                                 │
 │        each page: glib::object_subclass gtk::Box                 │
 │        constructed → setup_ui                                    │
 │        own RefCell/widget state                                  │
 │        callbacks: connect_clicked / toggled / notify /           │
 │                   row_selected / realize / glib timeout          │
 │        pages.root().downcast::<MainWindow>()                     │
 │          → update_app_state()  OR  widgets scraped by            │
 │            get_app_state() at Apply time                         │
 │                                                                  │
 │  HelperClient: std::process + mpsc reader thread                 │
 │    spawn hardcoded: pkexec <helper>  (.env_remove("SHELL"))      │
 │    GTK pages poll try_recv() via glib::timeout_add_local(100ms)  │
 │    OR glib::spawn_future_local + std::thread + pkexec nix-env    │
 │       (generations/maintenance bypass the helper protocol)       │
 └───────────────────────────────┬──────────────────────────────────┘
                                 │ JSON lines stdin/stdout
                                 ▼
 ┌──────────────────────────────────────────────────────────────────┐
 │  nixos-toolkit-helper (privileged via pkexec / polkit)           │
 │    crates/helper/{main,commands,nix_gen,rebuild}.rs              │
 │    writes /etc/nixos/nixos-toolkit/**                            │
 │    nixos-rebuild, generations, maintenance allowlist             │
 └──────────────────────────────────────────────────────────────────┘
                                 ▲
                                 │ shared types
 ┌──────────────────────────────────────────────────────────────────┐
 │  crates/common  {actions, ipc, config, nix}                      │
 │  nix/templates/{profiles,bundles,state}                          │
 └──────────────────────────────────────────────────────────────────┘
```

**What is broken about the current state model**

`MainWindow::get_app_state()` (`crates/gui/src/window.rs`) clones `RefCell<AppState>` then **overwrites** `network_config`, `services_config`, and `hardware_config` by reading live widgets. Network / services / hardware pages do **not** push into window state on toggle. Profiles / bundles / packages / system **do** push via `update_app_state`. So there are two sources of truth:

- Window `AppState` (profiles, bundles, packages, hostname, DNS, groups)
- Page widgets (network, services, hardware — and also duplicate bundle/package/profile UI state)

Apply then scrapes widgets. That cannot exist in a pure `view(&self)` world.

Additionally, `AppState.bluetooth_enabled` is a sibling of `hardware_config.bluetooth_enabled`. Apply/Generate send the sibling bool, not `HardwareConfig`. Hardware UI is persisted in `state.json` via ReadState/WriteState but is **not** applied to Nix except a bluetooth-only stub (see §8).

### 1.2 Target (libcosmic / iced)

```
 ┌──────────────────────────────────────────────────────────────────┐
 │  gui binary (unprivileged, iced runtime, wgpu)                   │
 │                                                                  │
 │  cosmic::Application for AppModel                                │
 │    type Message = crate::message::Message                        │
 │    type Flags   = crate::app::Flags                              │
 │    fn init  → (AppModel, Task)   // detect + ReadState Task      │
 │    fn update(&mut self, Message) → Task                          │
 │         └── delegates to AppModel::apply (pure, unit-testable)   │
 │    fn view(&self) → Element      // dispatch view::{page}        │
 │    fn subscription(&self)        // config watch; optional       │
 │                                  // helper stdout only while     │
 │                                  // a session is live            │
 │    fn nav_model / on_nav_select                                  │
 │    fn context_drawer             // About                        │
 │    fn header_start               // menu (About, Quit)           │
 │                                                                  │
 │  AppState is the ONLY source of truth for selections.            │
 │  Views are pure: view(state) → widgets that emit Message.        │
 │  No widget types in update().                                    │
 │                                                                  │
 │  HelperClient: injectable SpawnSpec (not hardcoded pkexec)       │
 │    one session per privileged op                                 │
 │    Apply + WriteState share that session                         │
 │    stdout lines → Message::Helper(HelperEvent) via Task::stream  │
 └───────────────────────────────┬──────────────────────────────────┘
                                 │ same JSON IPC
                                 ▼
                    crates/helper + crates/common
                    (unchanged except HardwareConfig IPC, §8)
```

### 1.3 GTK pattern → libcosmic mapping

| GTK / current | libcosmic / target |
|---|---|
| `glib::object_subclass` page `Box`, `constructed` → `setup_ui` | Stateless `view::*_page(state) -> Element<Message>` |
| `connect_clicked` / `toggled` / `notify` / `row_selected` | Widget `on_press` / `on_toggle` emits `Message` |
| `connect_realize` (load generations / disk info) | `on_nav_select` or first view of page emits `Message::…Requested` → Task |
| `RefCell<AppState>` on `MainWindow` | `AppModel.state: AppState` |
| `page.root().downcast::<MainWindow>()` + `update_app_state` | Message handled in `update` mutates `AppModel.state` |
| `get_app_state()` scraping widgets | **Deleted.** Apply reads `self.state` only |
| `adw::ToastOverlay` / `adw::Banner` | `AppModel.toasts` / `AppModel.banner`; UX renders them |
| `adw::MessageDialog` | `AppModel.dialog: Option<Dialog>` rendered by UX (modal overlay) |
| `glib::timeout_add_local` polling helper | `Task::stream` yielding `HelperEvent` |
| `glib::spawn_future_local` + `std::thread` + raw `pkexec` | Helper IPC Task (generations & maintenance **must** stop bypassing helper) |
| `adw::StyleManager` + `glib::user_config_dir` prefs | `config.rs`: JSON + cosmic-config; iced/cosmic appearance |
| `adw::AboutWindow` | `context_drawer` + `cosmic::widget::about::About` |
| `adw::NavigationSplitView` + `gtk::ListBox` sidebar | `nav_bar::Model` via `nav_model` / `on_nav_select` |
| `gio::Action` quit / about / refresh + accels | `header_start` menu + `key_binds`; `Message::Quit` / `ToggleAbout` / `RefreshSystem` |
| APP_ID `org.nixos-toolkit.app` on `adw::Application` | `cosmic::Application::APP_ID` (see §12) |

---

## 2. Crate / module map and ownership

### 2.1 Workspace crates (unchanged layout)

| Crate | Path | Owner | Role |
|---|---|---|---|
| `common` | `crates/common` | Architecture (shared); lead may assign IPC completeness here | Profiles/bundles/actions, IPC types, Nix gen, `SystemInfo`, configs |
| `helper` | `crates/helper` | Architecture (shared); **not** UX, **not** packaging | Privileged JSON server: writes `/etc/nixos/nixos-toolkit/`, rebuild, generations, maintenance allowlist |
| `gui` | `crates/gui` | Architecture owns core/state/config/helper; UX owns `view/` and `widget/` | Unprivileged libcosmic app |

Templates stay at `nix/templates/{profiles,bundles,state}`. Runtime path is `$NIXOS_TOOLKIT_TEMPLATES_DIR` (`common::config::paths::templates_dir()`).

### 2.2 Target `crates/gui/src` layout

```
crates/gui/src/
  main.rs                 # Architecture. tracing, Flags::from_env(), cosmic::app::run
  app.rs                  # Architecture. Application impl: init, update, view dispatch,
                          #   subscription, nav_model, on_nav_select, context_drawer, header_start
  message.rs              # Architecture. Message + HelperEvent + Dialog + Page
  state.rs                # Architecture. AppState + mutations (pure). IPC convert.
  config.rs               # Architecture. UserPreferences, load/save, cosmic-config
  integration.rs          # Architecture. detect_system(), snippets, GPU detect (no GTK)
  helper/
    mod.rs
    spawn.rs              # SpawnSpec discovery (env, exe-relative, system paths, Flatpak)
    client.rs             # Line-delimited JSON client (sync core; used by async bridge)
    session.rs            # Async stream: spawn → send → yield HelperEvent → drop
  core/                   # Architecture. Testable business logic, no iced/libcosmic types
    mod.rs
    apply.rs              # AppModel::apply / reduce: (state, Message) → (patch, Vec<Intent>)
    packages.rs           # parse_package_input, clean_package_name, is_valid_package_name
    preview.rs            # AppState → NixGenOptions → generate_preview_full
    validate.rs           # hostname / DNS / username / empty-apply checks
  view/                   # UX. One module per page + chrome (banner, dialog, toast)
  widget/                 # UX. Reusable cosmic widgets (settings rows, log view, …)
```

Packaging owns `tests/` (unit + integration + Flatpak smoke). Architecture’s job is to make every `Message` variant exercisable without a display.

### 2.3 What UX emits vs what Architecture handles

- UX widgets emit the `Message` names in §3. They do not spawn processes, touch `/etc/nixos`, or clone helper clients.
- Architecture `update` / `AppModel::apply` mutates `AppState`, opens/closes dialogs, and returns `Intent`s that `app.rs` turns into iced `Task`s.
- Packaging drives `AppModel::apply` and a fake helper process that speaks the existing IPC.

### 2.4 First skeleton (must compile)

`view/` pages may return an empty `column` / “coming soon” placeholder. `nav_bar::Model` still lists all 11 pages. Helper client and `AppState` exist. `cargo build -p gui` succeeds after every subsequent task.

---

## 3. Complete `Message` enum

File: `crates/gui/src/message.rs`.

UX may copy these names. Keep the enum `Clone` + `Debug`. Avoid putting `HelperClient` or widget types in variants.

```rust
use common::ipc::{
    AppState as IpcAppState, DiskUsageInfo, GeneratedFile, Generation, HardwareConfig,
    HelperResponse, LogLevel, NetworkConfig, Permissions, RebuildType, ServicesConfig,
    SystemInfo as IpcSystemInfo,
};
use common::{IntegrationStatus, SystemInfo};

/// Top-level application message. Every user action and every helper event.
#[derive(Debug, Clone)]
pub enum Message {
    // ── Chrome / nav / app lifecycle ──────────────────────────────────────
    /// Nav bar selection (also sent from `on_nav_select` after activate).
    NavSelect(Page),
    ToggleAbout,
    ToggleContextPage(ContextPage),
    LaunchUrl(String),
    Quit,
    /// Re-run local `detect_system()` (was `win.refresh`, Ctrl+R / F5).
    RefreshSystem,
    SystemDetected(SystemInfo),
    /// User-facing toast dismissed or requested.
    Toast(Toast),
    DismissToast,
    DismissDialog,
    /// cosmic-config file changed on disk.
    UpdateConfig(UserPreferences),
    /// Clipboard write finished (success/fail for toasts).
    ClipboardCopied { ok: bool },

    // ── Onboarding ────────────────────────────────────────────────────────
    CopyIntegrationSnippet,
    OpenEtcNixos,
    VerifyIntegration,

    // ── Profiles ──────────────────────────────────────────────────────────
    SelectProfile(String),
    ClearProfile,
    /// Local template preview for the selected profile (no helper).
    RefreshProfilePreview,

    // ── Bundles ───────────────────────────────────────────────────────────
    ToggleBundle { id: String, enabled: bool },
    ToggleBundlePackage { bundle_id: String, package: String, enabled: bool },
    /// Enable bundle and select all of its packages (expander switch on).
    EnableBundleAll { id: String, packages: Vec<String> },

    // ── Custom packages ───────────────────────────────────────────────────
    PackageInputChanged(String),
    AddPackagesFromInput,
    /// Parsed names after `parse_package_input` (tests inject this directly).
    AddCustomPackages(Vec<String>),
    RemoveCustomPackage(String),

    // ── System settings (hostname / DNS / groups / app theme) ─────────────
    HostnameChanged(String),
    DnsServersChanged(String),          // raw comma-separated; parsed in apply()
    UsernameChanged(String),
    ToggleUserGroup { group: String, enabled: bool },
    ColorSchemeChanged(ColorSchemePreference),

    // ── Hardware (state only; Nix apply is §8) ────────────────────────────
    SetHardwareConfig(HardwareConfig),
    SetNvidiaDriver(Option<u8>),
    SetNvidiaModesetting(bool),
    SetNvidiaPowerManagement(bool),
    SetNvidiaOpen(bool),
    SetAudioServer(u8),                 // 0=PipeWire, 1=PulseAudio, 2=None
    SetAudioLowLatency(bool),
    SetBluetoothEnabled(bool),
    SetBluetoothAutoPower(bool),
    SetPowerProfile(u8),                // 0=Balanced, 1=Performance, 2=Power Saver
    SetTlpEnabled(bool),
    SetThermaldEnabled(bool),
    GpuDetected(String),

    // ── Network ───────────────────────────────────────────────────────────
    SetNetworkConfig(NetworkConfig),
    SetFirewallEnabled(bool),
    ToggleTcpPort { port: u16, enabled: bool },
    CustomTcpPortsChanged(String),      // comma-separated; parsed in apply()
    SetSshEnabled(bool),
    SetSshPort(u16),
    SetSshPasswordAuth(bool),
    SetSshRootLogin(String),            // "no" | "prohibit-password" | "yes"
    SetFail2banEnabled(bool),
    SetTailscaleEnabled(bool),

    // ── Services ──────────────────────────────────────────────────────────
    SetServicesConfig(ServicesConfig),
    ToggleService { id: String, enabled: bool },
    // ids: printing, avahi, fwupd, upower, networkmanager, resolved, rustdesk,
    //      syncthing, locate, flatpak, gnome_keyring, gnome_tweaks, dconf,
    //      docker, libvirtd, postgresql, redis, earlyoom, auto_upgrade,
    //      auto_gc, store_optimize

    // ── Apply / preview / dry-run ─────────────────────────────────────────
    RefreshPreview,                     // local generate_preview_full; no helper
    PreviewReady(String),
    /// Open confirm dialog (empty-config warning if nothing selected).
    RequestApply,
    ConfirmApply,                       // dialog "Apply"
    CancelApply,
    RequestDryRun,                      // RebuildType::DryBuild; no confirm, no WriteState
    ApplyFinished { success: bool, message: String },

    // ── Generations ───────────────────────────────────────────────────────
    LoadGenerations,
    GenerationsLoaded(Vec<Generation>),
    RequestRollback { generation: u32 },
    ConfirmRollback { generation: u32, mode: RollbackMode }, // SwitchNow | SetForNextBoot
    RequestDeleteGeneration { generation: u32 },
    ConfirmDeleteGeneration { generation: u32 },
    RollbackToPrevious,

    // ── Maintenance ───────────────────────────────────────────────────────
    LoadDiskUsage,
    DiskUsageLoaded(DiskUsageInfo),
    RequestMaintenance { command: String, name: String, warning: Option<String> },
    ConfirmMaintenance { command: String, name: String },

    // ── Helper session (never emitted by widgets; Tasks/subscriptions) ────
    Helper(HelperEvent),
}

/// How generation activation should run after `nix-env --switch-generation`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RollbackMode {
    /// `switch-to-configuration switch`
    SwitchNow,
    /// `switch-to-configuration boot`
    SetForNextBoot,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Page {
    Onboarding,
    Profiles,
    Bundles,
    Packages,
    System,
    Hardware,
    Network,
    Services,
    Generations,
    Maintenance,
    Apply,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ContextPage {
    #[default]
    About,
}

/// Events from a live helper process. Mapped 1:1 from `HelperResponse`
/// plus session lifecycle. `session` ties events to the op that spawned them.
#[derive(Debug, Clone)]
pub enum HelperEvent {
    SpawnFailed { op: HelperOp, error: String },
    Spawned { op: HelperOp },
    /// Parsed stdout line.
    Response { op: HelperOp, response: HelperResponse },
    /// Helper stdout closed / process exited before a terminal response.
    Closed { op: HelperOp, error: Option<String> },
    /// ReadState timed out (preserve GTK 5s behaviour).
    Timeout { op: HelperOp },
}

/// Why a helper was spawned. Drives post-response behaviour
/// (e.g. Apply success → WriteState on the SAME session).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HelperOp {
    ReadState,
    WriteState,
    CheckPermissions,
    GetSystemInfo,
    EnsureDirectories,
    Validate,
    Generate,
    /// `then_write_state` is true for Switch/Boot/Test/Build; false for DryBuild.
    Apply { rebuild: RebuildType, then_write_state: bool },
    ListGenerations,
    Rollback { generation: u32, mode: RollbackMode },
    DeleteGenerations { generations: Vec<u32> },
    RunMaintenance { command: String },
    GetDiskUsage,
}

#[derive(Debug, Clone)]
pub enum Toast {
    Info(String),
    Warning(String),
    Error(String),
}
```

`UserPreferences` / `ColorSchemePreference` live in `config.rs` and are re-exported.

### 3.1 GTK callback → Message cheat sheet (for UX)

| Page (current file) | GTK signal | Message |
|---|---|---|
| `window.rs` sidebar | `row_selected` | `NavSelect(Page::…)` |
| `onboarding.rs` | Copy / Open / Verify | `CopyIntegrationSnippet` / `OpenEtcNixos` / `VerifyIntegration` |
| `profiles.rs` | check toggled | `SelectProfile(id)` |
| `bundles.rs` | expander `enable_expansion` | `ToggleBundle` / `EnableBundleAll` |
| `bundles.rs` | package check | `ToggleBundlePackage` |
| `packages.rs` | Add / Enter | `AddPackagesFromInput` |
| `packages.rs` | trash | `RemoveCustomPackage` |
| `system.rs` | style combo | `ColorSchemeChanged` |
| `system.rs` | hostname/DNS/user apply+changed | `HostnameChanged` / `DnsServersChanged` / `UsernameChanged` |
| `system.rs` | group switch | `ToggleUserGroup` |
| `hardware.rs` | combo/switch | `SetNvidia*` / `SetAudio*` / `SetBluetooth*` / `SetTlp*` / … |
| `network.rs` | firewall/SSH/VPN widgets | `SetFirewallEnabled` / `ToggleTcpPort` / `SetSsh*` / … |
| `services.rs` | switch | `ToggleService { id, enabled }` |
| `apply.rs` | Refresh / Apply / Dry Run | `RefreshPreview` / `RequestApply` / `RequestDryRun` |
| `generations.rs` | Refresh / rollback / switch / delete | `LoadGenerations` / `RollbackToPrevious` / `RequestRollback` / `RequestDeleteGeneration` |
| `maintenance.rs` | run / refresh disk | `RequestMaintenance` / `LoadDiskUsage` |

---

## 4. `AppModel` fields

File: `crates/gui/src/app.rs` (struct) + `state.rs` (selection model).

```rust
pub struct AppModel {
    /// libcosmic runtime (nav collapse, window, theme).
    core: cosmic::Core,

    /// Startup flags (spawn spec, paths). Immutable after init.
    flags: Flags,

    nav: cosmic::widget::nav_bar::Model,
    context_page: ContextPage,
    about: cosmic::widget::about::About,
    key_binds: HashMap<menu::KeyBind, MenuAction>,

    /// Selection model — the only source of truth for what will be applied.
    state: AppState,

    /// Unprivileged detection (local, no pkexec).
    system_info: SystemInfo,
    gpu_vendor: Option<String>,
    cpu_arch: common::CpuArch,

    /// Unprivileged UI prefs (theme). See §6.
    prefs: UserPreferences,

    /// Ephemeral UI — not persisted to state.json.
    page: Page,                          // mirrors nav.active data
    dialog: Option<Dialog>,
    toasts: Vec<Toast>,
    banner: Option<Banner>,

    package_input: String,               // custom-packages entry text
    profile_preview: String,             // profiles page template text
    apply_preview: String,               // apply page full preview
    apply_log: String,
    generations_log: String,
    maintenance_log: String,
    disk_usage: Option<DiskUsageInfo>,
    generations: Vec<Generation>,
    current_generation: Option<u32>,

    /// In-flight privileged work. At most one helper session.
    helper: HelperStatus,
    /// True while Apply/DryRun is running (buttons disabled).
    busy: Busy,

    /// Transient parse/validation errors shown inline (hostname, DNS, packages).
    field_errors: FieldErrors,
}

/// Persisted selections. Same fields as today's `crates/gui/src/state.rs`.
/// Converted to `common::ipc::AppState` for ReadState/WriteState.
pub struct AppState {
    pub selected_profile: Option<String>,
    pub enabled_bundles: HashSet<String>,
    pub bundle_packages: HashMap<String, HashSet<String>>,
    pub hostname: Option<String>,
    pub dns_servers: Vec<String>,
    pub user_groups: HashSet<String>,
    pub username: Option<String>,
    /// Kept in sync with `hardware_config.bluetooth_enabled` (see §8).
    pub bluetooth_enabled: bool,
    pub custom_packages: HashSet<String>,
    pub network_config: NetworkConfig,
    pub services_config: ServicesConfig,
    pub hardware_config: HardwareConfig,
    pub has_changes: bool,
    pub last_applied: Option<String>,
}

pub enum HelperStatus {
    Idle,
    /// Process running; stdout is being streamed as HelperEvent.
    Active { op: HelperOp },
}

pub enum Busy {
    Idle,
    Applying,
    DryRun,
    LoadingState,
    LoadingGenerations,
    LoadingDisk,
    Maintenance,
    RollingBack,
    DeletingGeneration,
}

pub enum Dialog {
    ConfirmApply { empty_config: bool, heading: String, body: String },
    ConfirmRollback { generation: u32 },
    ConfirmDeleteGeneration { generation: u32 },
    ConfirmMaintenance { command: String, name: String, warning: String },
}

pub struct Banner {
    pub kind: BannerKind, // Error / Warning / Success / Info
    pub text: String,
}

pub struct FieldErrors {
    pub hostname: Option<String>,
    pub dns: Option<String>,
    pub username: Option<String>,
    pub packages: Option<String>,
}
```

`AppState` mutation methods already in `crates/gui/src/state.rs` (`select_profile`, `toggle_bundle`, `set_network_config`, `to_ipc_state`, `from_ipc_state`, `summary`, …) stay. Add:

- `fn sync_bluetooth_from_hardware(&mut self)` so the duplicate bool cannot drift
- `fn set_service(&mut self, id: &str, enabled: bool)` instead of reconstructing `ServicesConfig` in views
- `fn parse_and_set_dns(&mut self, raw: &str) -> Result<(), String>`
- `fn parse_and_add_tcp_ports(&mut self, raw: &str) -> Result<(), String>`

### 4.1 Testable core: `AppModel::apply`

`cosmic::Application::update` **must not** contain widget types. It should be a thin wrapper:

```rust
fn update(&mut self, msg: Message) -> Task<cosmic::Action<Message>> {
    let intents = self.apply(msg);
    intents_to_tasks(self, intents)
}
```

```rust
/// Pure-enough reducer. No iced/libcosmic/GTK types.
/// Packaging unit-tests this for every Message variant.
impl AppModel {
    pub fn apply(&mut self, msg: Message) -> Vec<Intent> { /* … */ }
}

/// Side effects the iced layer knows how to run. Tests inspect these
/// instead of spawning processes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Intent {
    None,
    SpawnHelper { op: HelperOp, request: HelperRequest },
    /// Continue an existing helper session (ApplyComplete → WriteState).
    SendOnSession { request: HelperRequest },
    CloseSession,
    DetectSystem,                    // threadpool → Message::SystemDetected
    DetectGpu,                       // lspci → Message::GpuDetected
    LocalPreview,                    // generate_preview_full → PreviewReady
    LocalProfilePreview { id: String },
    CopyClipboard(String),
    OpenPath(PathBuf),               // xdg-open / cosmic portal
    SavePrefs,                       // JSON + cosmic-config
    SetWindowTitle(String),
    Exit,
}
```

Rules:

- `apply` never blocks on helper I/O.
- `apply` never reads widgets.
- Helper stdout arrives later as `Message::Helper`.
- `Intent::SpawnHelper` is refused if `helper != Idle` (except `SendOnSession` while `Active`).

---

## 5. Helper I/O → Task / Subscription

### 5.1 Protocol (unchanged)

Line-delimited JSON. Types in `crates/common/src/ipc.rs`.

**Requests:** `CheckPermissions`, `GetSystemInfo`, `Validate`, `Generate`, `Apply`, `EnsureDirectories`, `ReadState`, `WriteState`, `ListGenerations`, `RollbackGeneration`, `DeleteGenerations`, `RunMaintenance`, `GetDiskUsage`.

**Responses:** `Ok`, `Error`, `Permissions`, `SystemInfo`, `ValidationResult`, `GenerationResult`, `Log`, `ApplyComplete`, `State`, `Generations`, `MaintenanceOutput`, `DiskUsage`.

`Apply` / `Generate` currently omit `HardwareConfig` — see §8.

### 5.2 Spawn is injectable (not hardcoded `pkexec`)

Today: `HelperClient::spawn_privileged()` always runs `pkexec <helper>` (`crates/gui/src/helper/client.rs`). Path discovery:

1. `$NIXOS_TOOLKIT_HELPER`
2. next to current exe (`nixos-toolkit-helper`)
3. `/run/current-system/sw/bin/nixos-toolkit-helper`, `/usr/local/bin/…`, `/usr/bin/…`
4. `PATH` name `nixos-toolkit-helper`

Always `.env_remove("SHELL")` (pkexec rejects nix-develop shells not in `/etc/shells`).

**Target:** `Flags.spawn: SpawnSpec` built once in `Flags::from_env()` / tests.

```rust
#[derive(Debug, Clone)]
pub struct SpawnSpec {
    /// e.g. "pkexec", "flatpak-spawn", path to helper, or a fake binary in tests
    pub program: PathBuf,
    /// e.g. [helper_path]  or  ["--host", "--", "pkexec", helper_path]
    pub args: Vec<String>,
    pub extra_env: Vec<(String, String)>,
    /// Always includes "SHELL" unless tests say otherwise.
    pub remove_env: Vec<String>,
    /// Forwarded to the helper process.
    pub templates_dir: Option<PathBuf>,
}

impl SpawnSpec {
    pub fn from_env() -> Self { /* see §7 */ }
    pub fn for_tests(fake_helper: PathBuf) -> Self { /* program = fake, args = [] */ }
}
```

`HelperClient::spawn(spec: &SpawnSpec)` is the only spawn function. `spawn_privileged()` is deleted.

### 5.3 Session lifetime

Do **not** keep a root helper alive for the whole app. One process per `HelperOp` (or op-chain). Drop = kill + wait (same as today’s `Drop`).

**Invariant (parity with `pages/apply.rs` after Bug 5):** `Apply` success **must** `WriteState` on the **same** stdin, then close. Never spawn a second pkexec/flatpak-spawn for the save. Dry-run does **not** WriteState.

Suggested session machine in `helper/session.rs`:

```
Idle
  --Intent::SpawnHelper-->  spawn(spec)
      send(request)
      if op == Apply: first send EnsureDirectories, wait Ok, then send Apply
      stream stdout lines
        Log            → HelperEvent::Response(Log)          (keep streaming)
        ApplyComplete  → if success && then_write_state:
                            send WriteState { state }
                            wait Ok | Error
                         then HelperEvent::Response(ApplyComplete)
                         close
        State/Ok/Error/Generations/MaintenanceOutput/DiskUsage
                       → HelperEvent::Response(...) ; close
        EOF            → HelperEvent::Closed
        timeout        → HelperEvent::Timeout ; kill
```

Timeouts (preserve current numbers):

| Op | Timeout |
|---|---|
| ReadState (startup) | 5 s |
| EnsureDirectories | 10 s |
| WriteState after Apply | 3 s |
| Apply / rebuild | none (stream until `ApplyComplete` or EOF) |
| ListGenerations / DiskUsage / Maintenance | 60 s (new; GTK currently blocks on process exit) |

### 5.4 Mapping to iced

Prefer **`Task::stream`** (or `Task::perform` for one-shot) over a process-lifetime `subscription`. Subscriptions are for things that should start/stop with view state (config watch). A helper child should be owned by the Task so dropping the Task can kill the child.

```rust
fn spawn_helper_task(spec: SpawnSpec, op: HelperOp, request: HelperRequest, state: IpcAppState)
    -> Task<Message>
{
    Task::stream(async_stream::stream! {
        match session::run(spec, op.clone(), request, state).await {
            // run() yields a stream of HelperEvent
            // …
        }
    })
    .map(Message::Helper)
}
```

`subscription()`:

```rust
fn subscription(&self) -> Subscription<Message> {
    let mut subs = vec![
        self.core().watch_config::<UserPreferences>(Self::APP_ID)
            .map(|u| Message::UpdateConfig(u.config)),
    ];
    // Do NOT subscribe to helper stdout here.
    Subscription::batch(subs)
}
```

Optional: `iced::keyboard` for Ctrl+Q / Ctrl+R / F5 if not covered by `key_binds`.

### 5.5 Apply flow (parity)

GTK (`crates/gui/src/pages/apply.rs`):

1. User clicks Apply → confirm dialog (destructive if no profile/bundles/packages)
2. Spawn pkexec helper
3. `EnsureDirectories` → wait Ok
4. `Apply { rebuild_type: Switch, … }` (no `hardware_config` today)
5. Poll `Log` lines → text view
6. `ApplyComplete` → if success, `WriteState` on same client → close
7. Dry run: `RebuildType::DryBuild`, skip WriteState

Target `apply()`:

1. `RequestApply` → if `busy != Idle` ignore; else set `dialog = ConfirmApply { empty_config }`
2. `ConfirmApply` → `busy = Applying`; `apply_log` reset; `Intent::SpawnHelper { op: Apply { Switch, then_write_state: true }, request }`
3. Session internally sends EnsureDirectories then Apply
4. `Helper(Response(Log))` appends to `apply_log`
5. `Helper(Response(ApplyComplete { success }))` → if success, session sends WriteState (session layer, not a second Intent::SpawnHelper); `has_changes = false`; toast; `busy = Idle`
6. `RequestDryRun` → same with `DryBuild`, `then_write_state: false`, no dialog

Preview: `RefreshPreview` → `Intent::LocalPreview` → `common::nix::generate_preview_full` using `$NIXOS_TOOLKIT_TEMPLATES_DIR`. **No helper.** Same as today.

### 5.6 Generations and maintenance currently bypass IPC — stop that

| Current GTK | Problem | Target |
|---|---|---|
| `generations.rs` reads `/nix/var/nix/profiles` itself | Flatpak cannot see host `/nix` | `ListGenerations` via helper |
| switch/delete via `pkexec nix-env` + `switch-to-configuration` | Second privileged path; no allowlist | `RollbackGeneration` / `DeleteGenerations` |
| `maintenance.rs` `pkexec <command>` | Bypasses helper allowlist | `RunMaintenance` |
| disk usage already uses helper `GetDiskUsage` | Spawn logic duplicated | Same `HelperClient` |

`RollbackMode::SetForNextBoot` is extra vs helper’s `rollback_generation()`, which always activates with `switch`. **Parity:** keep Switch Now vs Set for Next Boot in the UI. **Correctness (lead decision):** extend helper `RollbackGeneration { generation, activate: Switch \| Boot }` **or** map both to current `RollbackGeneration` (Switch only) until the helper grows a field. Recommendation: small helper patch, same ownership as §8 — optional activate argument defaulting to `"switch"` so old clients keep working. If the lead defers it, GUI “Set for Next Boot” is documented as a known gap vs GTK.

Helper `RollbackGeneration` does not currently stream `Log` lines (it returns a single `Ok`/`Error`). GUI still appends a short local log (“Switching to generation N…” / result).

### 5.7 Startup

`init`:

1. Build nav (11 pages, Onboarding active)
2. `prefs = UserPreferences::load(&flags)`
3. Apply color scheme from prefs
4. `Intent::DetectSystem` (unprivileged; no pkexec)
5. `Intent::SpawnHelper { ReadState }` (5 s timeout) unless `flags.skip_privileged_on_init`
6. `Intent::DetectGpu` (can wait until Hardware page; GTK did it in `HardwarePage::setup_ui`)

Failed ReadState → banner warning, defaults. Same as `show_state_load_warning`.

On `Helper(Response(State(ipc)))` → `AppState::from_ipc_state`. Views just re-render. **No `sync_from_state` into widgets.**

---

## 6. Persistence

Two stores, never mixed.

### 6.1 Privileged system selections — `state.json` via helper

| | |
|---|---|
| Path | `/etc/nixos/nixos-toolkit/state/state.json` (`common::config::paths::STATE_JSON`) |
| Who writes | helper `write_state` only |
| Who reads | helper `read_state` (falls back to reconstructing from Nix files if missing) |
| When load | app `init` → `ReadState` |
| When save | after successful **non-dry** Apply, same helper session |
| Type | `common::ipc::AppState` |

GUI `AppState` converts via existing `to_ipc_state` / `from_ipc_state`. `has_changes` is GUI-only (not in IPC). `last_applied` is IPC-only; set by GUI when Apply succeeds before WriteState.

**Do not** write `state.json` from the GUI process. Flatpak sandbox must not gain `/etc/nixos` write access.

### 6.2 Unprivileged UI prefs — JSON canonical, cosmic-config mirror

Today: `crates/gui/src/preferences.rs` writes `glib::user_config_dir()/nixos-toolkit/preferences.json` (`ColorSchemePreference = System \| Light \| Dark`). No privilege.

**Decision:** JSON is the portable source of truth; cosmic-config is a COSMIC-native mirror for `watch_config` and desktop integration.

Justification:

- The app **must work outside the COSMIC desktop** (GNOME/KDE/Sway/Fedora). cosmic-config is just files under `$XDG_CONFIG_HOME/cosmic/<APP_ID>/vN/` and does not need cosmic-comp, but Flatpak and tests already understand a single JSON file.
- Flatpak xdg-config portability: persist `xdg-config/nixos-toolkit`. Packaging does not have to special-case the cosmic-config directory for basic prefs.
- GTK already used JSON; keeping it is feature parity for theme restoration.
- `CosmicConfigEntry` still lets COSMIC users get live reload via `watch_config`.

Load order:

1. `Flags.prefs_path` if set (tests)
2. `$XDG_CONFIG_HOME/nixos-toolkit/preferences.json` if present
3. cosmic-config `Config::get_entry(APP_ID)` if present
4. defaults (`ColorScheme::System`)

Save: write JSON **and** cosmic-config (best-effort; log errors, do not fail the UI).

```rust
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, CosmicConfigEntry)]
#[version = 1]
pub struct UserPreferences {
    pub color_scheme: ColorSchemePreference, // system | light | dark
}
```

Theme application: map to iced/cosmic appearance in `init` / `ColorSchemeChanged`. Do **not** call `adw::StyleManager`. Do **not** require cosmic-comp.

Path helper: `dirs::config_dir()` (or `xdg`) — not `glib::user_config_dir`.

### 6.3 What is not persisted

Dialogs, logs, preview text, `busy`, helper session, toasts, package entry buffer, nav page (always start at Onboarding, same as GTK selecting row 0).

---

## 7. `Flags` / environment

```rust
#[derive(Debug, Clone)]
pub struct Flags {
    pub spawn: SpawnSpec,
    pub templates_dir: PathBuf,
    pub prefs_path: Option<PathBuf>,
    /// Tests / headless CI: do not pkexec on startup.
    pub skip_privileged_on_init: bool,
    /// Tests: skip lspci / /etc/NIXOS probes.
    pub skip_host_probes: bool,
}

impl Flags {
    pub fn from_env() -> Self { /* … */ }
}
```

### 7.1 Environment variables

| Variable | Meaning |
|---|---|
| `NIXOS_TOOLKIT_HELPER` | Absolute path to helper binary (already used by Nix wrapper in `flake.nix`) |
| `NIXOS_TOOLKIT_TEMPLATES_DIR` | Templates root (already used by `common::config::paths::templates_dir`) |
| `NIXOS_TOOLKIT_SPAWN` | Optional. If set, the **program** used instead of `pkexec` (e.g. `flatpak-spawn`) |
| `NIXOS_TOOLKIT_SPAWN_ARGS` | Optional. JSON array of args **before** the helper path. Example: `["--host","--","pkexec"]` |
| `NIXOS_TOOLKIT_NO_PKEXEC` | If `1`/`true`, spawn the helper directly (dev, already-root, tests) |
| `NIXOS_TOOLKIT_SKIP_PRIVILEGED_INIT` | Skip ReadState on startup |
| `FLATPAK_ID` | If present and `NIXOS_TOOLKIT_SPAWN` unset, default spawn to `flatpak-spawn --host -- pkexec <helper>` |

Discovery algorithm for `SpawnSpec::from_env()`:

1. Resolve helper path (same 4-step list as today).
2. If `NIXOS_TOOLKIT_NO_PKEXEC` → `program = helper`, `args = []`.
3. Else if `NIXOS_TOOLKIT_SPAWN` set → `program = that`, `args = SPAWN_ARGS + [helper]`.
4. Else if `FLATPAK_ID` set → `program = flatpak-spawn`, `args = ["--host", "--", "pkexec", helper]`.
5. Else → `program = pkexec`, `args = [helper]`.
6. Always `remove_env` includes `SHELL`.
7. If `NIXOS_TOOLKIT_TEMPLATES_DIR` set, pass it through `extra_env` so the helper sees it after pkexec (pkexec may strip env; helper also has its own wrapper in `flake.nix`).

Packaging (Flatpak) is expected to set `NIXOS_TOOLKIT_SPAWN` / `NIXOS_TOOLKIT_SPAWN_ARGS` / `NIXOS_TOOLKIT_HELPER` in the wrapper rather than relying on `FLATPAK_ID` heuristics. The heuristic is a fallback.

`main.rs`:

```rust
fn main() -> cosmic::iced::Result {
    // tracing init (keep)
    let flags = Flags::from_env();
    let settings = cosmic::app::Settings::default()
        .size_limits(/* ~1000x700, matching GTK default */);
    // Do NOT enable single-instance (see §11).
    cosmic::app::run::<AppModel>(settings, flags)
}
```

Executor: `type Executor = cosmic::executor::Default;` (tokio). Features on libcosmic git dep: `about`, `wgpu`, `xdg-portal`, `winit`, `tokio`, `wayland`. **Do not** enable `single-instance` by default.

Must run on GNOME/KDE/Sway/Fedora without cosmic-comp: wgpu + Wayland + X11 fallback via winit. No cosmic-comp APIs.

---

## 8. HardwareConfig apply gap — decision for the lead

### 8.1 Evidence

- UI: `crates/gui/src/pages/hardware.rs` exposes NVIDIA driver/modesetting/powermgmt/open, audio server/low-latency, bluetooth + autopower, power profile, TLP, thermald. ARM hides NVIDIA + thermald.
- Persistence: `HardwareConfig` is a field on `common::ipc::AppState` and is saved/loaded via WriteState/ReadState. `MainWindow::get_app_state()` copies it from the page at Apply time.
- **Generate/Apply requests do not include `HardwareConfig`** (`crates/common/src/ipc.rs` lines 24–53). They only have `bluetooth_enabled: bool`.
- Helper `generate_all_files` (`crates/helper/src/nix_gen.rs` ~109–127) writes `hardware.nix` **only if `bluetooth_enabled`**, calling `generate_hardware_nix(false, false, false, true, false, false)` — nvidia/audio/tlp/thermald **hardcoded false**.
- Preview path is the same: `common::nix::generate_preview_full` only emits hardware.nix when `options.bluetooth_enabled` (`crates/common/src/nix.rs` ~99–102, 600–610).
- `NixGenOptions` has no `HardwareConfig`.
- `generate_hardware_nix` itself already accepts nvidia/audio/bluetooth/tlp/thermald, but ignores driver index, modesetting, powermgmt, low-latency, autopower, power profile.

So: the Hardware page is **feature-parity UI** that **does not change the system**, except a bluetooth stub that also ignores `bluetooth_autopower`.

### 8.2 Recommendation

**Keep the Hardware UI (parity) AND actually apply `HardwareConfig` in Generate/Apply (correctness).** This is a small `common` + `helper` change. Architecture should own it if the lead assigns; UX should not; packaging should not. `crates/common` and `crates/helper` stay shared.

Priority: (1) parity would leave a lying UI; (2) Flatpak does not change this; correctness outranks simplicity and COSMIC conventions.

### 8.3 Proposed patch (not in this phase unless assigned)

1. Add `hardware_config: HardwareConfig` to `HelperRequest::Generate` and `HelperRequest::Apply`. Keep `bluetooth_enabled` for one release; helper treats `hardware_config.bluetooth_enabled || bluetooth_enabled`.
2. Add `hardware_config: HardwareConfig` to `NixGenOptions`.
3. Import `./hardware.nix` when `hardware_config` has any non-default setting (not only bluetooth).
4. Expand `generate_hardware_nix` to consume the full struct:
   - NVIDIA: driver package from `nvidia_driver` (0 stable / 1 beta / 2 open / 3 nouveau); `modesetting`, `powerManagement`, `open`
   - Audio: PipeWire vs PulseAudio vs none; low-latency extras
   - Bluetooth: `enable` + `powerOnBoot` from `bluetooth_autopower` (today always `powerOnBoot = true`)
   - Power: TLP / thermald; power-profiles-daemon vs TLP mutual exclusion already sketched
5. Thread the field through `helper/src/{main,commands,nix_gen}.rs`.
6. GUI Apply/preview use `state.hardware_config`. Keep `state.bluetooth_enabled` mirrored.

Serde: new field `#[serde(default)]` so old helpers/GUI still deserialize `state.json`.

If the lead **rejects** applying hardware (strict GTK-behaviour parity): keep the UI, document in-app that only bluetooth is generated, and do not expand Nix gen. That is worse UX; do not do it silently.

### 8.4 Related helper gap (generations boot mode)

GTK “Set for Next Boot” vs helper always `switch` — see §5.6. Same “small shared-crate patch” bucket.

---

## 9. What stays in `common` / `helper` unchanged

Leave alone unless §8 (or §5.6 activate mode) is assigned:

- Profile and bundle **definitions** and templates (`default_profiles` 13 DEs, `default_bundles` — 16 defs / 15 templates; `ai-tools` has no `nix/templates/bundles/ai-tools.nix` and already uses helper fallback. Out of scope.)
- `default_maintenance_actions` allowlist strings — GUI must send these exact command strings to `RunMaintenance`
- `default_system_actions` group names (`libvirtd`, `docker`, `vboxusers`)
- IPC envelope: tagged JSON, `RebuildType`, `Log`, `ApplyComplete`, `Generation`, `DiskUsageInfo`, `NetworkConfig`, `ServicesConfig`
- Helper main loop (stdin lines → `handle_request` → one JSON response, plus extra `Log` lines during rebuild)
- `atomic_write` + read-back verify
- `lib.mkDefault` for hostname/DNS (progress.md Bug 6)
- Paths in `common::config::paths`
- Polkit policy `data/polkit/org.nixos-toolkit.helper.policy` (packaging may wrap it for Flatpak; Architecture does not redesign polkit)
- Helper binary name `nixos-toolkit-helper`

GUI **must** stop calling `pkexec nix-env` / `pkexec nix-collect-garbage` directly so the allowlist remains the security boundary.

Move **into** `common` (or `gui::core` if we want to avoid touching common):

- `parse_package_input` / `clean_package_name` / `is_valid_package_name` from `crates/gui/src/pages/packages.rs` — currently methods on a GTK page, untestable without a display. Prefer `common::packages` so helper validation could reuse later; `gui::core::packages` is acceptable if we want zero common churn.

Optional tidy (only with §8): extract `generate_network_nix(&NetworkConfig)` from the duplicated strings in `helper/src/nix_gen.rs` and `common::nix::generate_preview_full`.

---

## 10. Incremental build plan (compile after every task)

Constraint: **the app stays compilable after every task.** First skeleton has empty pages. No giant-bang rewrite of all GTK pages in one commit.

| Task | Deliverable | Still works |
|---|---|---|
| 0 | This document | n/a |
| 1 | libcosmic git dep; `main.rs` + `AppModel` with empty `view`; 11 nav entries; `Message` stub; `Flags::from_env`; **no GTK** | `cargo build -p gui` opens a window with sidebar |
| 2 | `state.rs` + `core/apply.rs` + unit tests for mutations (no window) | `cargo test -p gui` |
| 3 | `config.rs` JSON load/save + theme message | prefs round-trip tests |
| 4 | `helper/{spawn,client,session}` + fake-helper integration test | ReadState/WriteState/Log stream without display |
| 5 | `init` ReadState + `integration.rs` banner | skeleton shows NixOS/integration banner |
| 6 | UX: Onboarding view (copy/open/verify messages already handled) | |
| 7 | UX: Profiles + Bundles + Packages (Messages already in enum) | |
| 8 | UX: System + Network + Services + Hardware | state-only; hardware Nix still stub until §8 |
| 9 | Apply preview (local) + Apply/DryRun session + WriteState-on-same-session | |
| 10 | Generations + Maintenance via helper IPC (delete GTK bypass) | |
| 11 | §8 HardwareConfig IPC patch (if assigned) | preview/apply write real hardware.nix |
| 12 | Packaging: unit tests every Message; fake-helper flows; Flatpak spawn smoke | |

After task 1, `pages/*.rs` GTK modules are gone from the crate. Do not keep a GTK and a cosmic GUI in the same crate.

`crates/helper` and `crates/common` keep compiling throughout; `nix build .#helper` remains the privileged backend.

---

## 11. Risks

### 11.1 pkexec / polkit inside Flatpak

The GTK app assumes a host polkit agent and `pkexec <helper>`. Inside a Flatpak:

- `pkexec` in the sandbox does not see host polkit correctly
- helper must run on the **host** (`flatpak-spawn --host`) then elevate
- `NIXOS_TOOLKIT_HELPER` inside the sandbox is the wrong binary (no nixos-rebuild, no write to `/etc/nixos`)
- Packaging must install the helper **on the host** (system package / NixOS module) or document that the Flatpak is a GUI-only frontend

Mitigation: `SpawnSpec` is data, not `pkexec` literals. Packaging sets env. Architecture never calls `pkexec` except as the default `SpawnSpec` on non-Flatpak.

### 11.2 Async helper lifetime

GTK leaked complexity: `Rc<RefCell<Option<HelperClient>>>` plus 100 ms poll, plus a second spawn in unused `save_state()`. Risks to avoid:

- Two sessions (double password prompt) — forbidden for Apply+WriteState
- Task cancelled on nav change killing nixos-rebuild — **do not** cancel Apply stream on `NavSelect`
- Drop of `AppModel` during rebuild — `Drop` kills the child (same as GTK); acceptable
- stdout JSON interleaved with nixos-rebuild logs — helper already wraps rebuild lines as `Log` JSON; client must ignore non-JSON lines (log and continue)

### 11.3 State split (the bug we are fixing)

If any view keeps a local copy of switches and Apply reads the copy, Hardware/Network/Services will silently diverge again. Rule: **no page-local selection state.** Widgets are bound to `AppModel.state`.

`bluetooth_enabled` vs `hardware_config.bluetooth_enabled`: `SetBluetoothEnabled` writes both.

### 11.4 libcosmic git API churn

libcosmic is a git dependency (`https://github.com/pop-os/libcosmic`). `Application::update` return type, `Task` vs `Command`, `context_drawer` signatures, and `about` widget have moved before. Mitigation:

- Pin a commit in `Cargo.toml` / `Cargo.lock` (packaging owns lockfile policy)
- Keep iced types out of `core/apply.rs` so churn is confined to `app.rs` / `view/`
- Avoid `single-instance` (D-Bus; breaks headless smoke and multi-instance tests; Flatpak needs extra talk-name permissions)
- Avoid features we do not use (`markdown`, `qr_code`)

### 11.5 Headless / CI

`wgpu` + no display fails. Packaging should run unit tests (`AppModel::apply`) without opening a window. Optional smoke: `COSMIC`/`iced` with `WGPU_BACKEND=empty` or xvfb — packaging’s call. Architecture keeps `update` display-free.

### 11.6 Integration detection split

GUI `integration.rs` string-matches `"nixos-toolkit"` in `configuration.nix`. Helper `detect_integration` requires `nixos-toolkit/state/selected.nix`. Preserve GUI local detect for the banner (no pkexec) but prefer helper `GetSystemInfo` when a session is already up. Do not “fix” detection in this migration unless it is a one-line parity bug.

### 11.7 Duplicate privileged paths

Until tasks 9–10 land, do not reintroduce `Command::new("pkexec")` in views. `OpenEtcNixos` may use `xdg-open` or the xdg-portal (unprivileged). GPU detect uses `lspci` unprivileged; in Flatpak packaging may need `--device=all` or host spawn — if `lspci` fails, show “Unknown GPU” (GTK already does).

### 11.8 Empty apply is destructive

GTK warns when profile+bundles+packages are all empty. Preserve that dialog. `core/validate.rs` implements `fn apply_is_empty(state) -> bool`.

---

## 12. APP_ID

Current:

| Location | Value |
|---|---|
| `crates/gui/src/main.rs` | `org.nixos-toolkit.app` |
| Polkit actions | `org.nixos-toolkit.helper.manage-system` / `.write-config` / `.rebuild` |
| Desktop file | `Name=NixOS Toolkit`, `Exec=nixos-toolkit`, `StartupWMClass=nixos-toolkit` |
| Nix wrapper | binary renamed `nixos-toolkit` |

**Proposal: keep `org.nixos-toolkit.app`.**

Reasons:

- Matches existing polkit vendor prefix `org.nixos-toolkit.*`
- Reverse-DNS, valid as libcosmic `APP_ID` and as a Flatpak id
- Changing it splits cosmic-config + JSON prefs + D-Bus well-known names
- Flathub “made-for-COSMIC” is a metainfo `<provides><id>com.system76.CosmicApplication</id></provides>`, **not** a reason to change APP_ID

If packaging needs a Flathub-legal ID tied to GitHub (`goshitsarch-eng/Gosh-NixOS-Manager`), the alternative is `io.github.goshitsarch_eng.NixosToolkit`. That is a packaging/metainfo decision. If they switch, Architecture will rename `APP_ID`, prefs directory, and document a one-time prefs migration. Polkit action ids can stay `org.nixos-toolkit.helper.*`.

Coordinate with packaging before any rename. Until then, `const APP_ID: &'static str = "org.nixos-toolkit.app"`.

---

## Appendix A — `core/packages.rs` (extract from GTK)

Move these as pure functions (signatures UX and tests can share):

```rust
pub fn parse_package_input(input: &str) -> Vec<String> { /* existing logic */ }
pub fn clean_package_name(name: &str) -> String { /* strip pkgs. / nixpkgs# / ; */ }
pub fn is_valid_package_name(name: &str) -> bool { /* ASCII ident, len≤128 */ }

/// Returns (added, already_custom, in_bundle_name).
pub fn classify_new_packages(
    requested: &[String],
    existing: &HashSet<String>,
    bundles: &[BundleDef],
) -> (Vec<String>, Vec<String>, Vec<(String, String)>)
```

`AddPackagesFromInput` uses `package_input`, then `AddCustomPackages` / toasts for duplicates and bundle collisions — same behaviour as `PackagesPage::add_packages`.

---

## Appendix B — `AppState` → `NixGenOptions` (preview / apply payload)

```rust
impl AppState {
    pub fn to_nix_gen_options<'a>(
        &'a self,
        profiles: &'a [ProfileDef],
        bundles: &'a [BundleDef],
    ) -> NixGenOptions<'a> { /* … */ }

    pub fn to_apply_request(&self, rebuild: RebuildType) -> HelperRequest { /* … */ }
}
```

Until §8 lands, `to_apply_request` matches today’s `HelperRequest::Apply { bluetooth_enabled: self.hardware_config.bluetooth_enabled \|\| self.bluetooth_enabled, network_config, services_config, … }` — still no `hardware_config` field, so behaviour matches GTK. After §8, include `hardware_config`.

---

## Appendix C — Nav pages (order = GTK sidebar)

1. Getting Started → `Page::Onboarding` → `view::onboarding`
2. Desktop Profiles → `Page::Profiles` → `view::profiles`
3. Software Bundles → `Page::Bundles` → `view::bundles`
4. Custom Packages → `Page::Packages` → `view::packages`
5. System Settings → `Page::System` → `view::system`
6. Hardware → `Page::Hardware` → `view::hardware`
7. Network → `Page::Network` → `view::network`
8. Services → `Page::Services` → `view::services`
9. Generations → `Page::Generations` → `view::generations` (on select: `LoadGenerations` if cache empty)
10. Maintenance → `Page::Maintenance` → `view::maintenance` (on select: `LoadDiskUsage` if cache empty)
11. Apply Changes → `Page::Apply` → `view::apply` (on select: `RefreshPreview`)

Icons: reuse GTK symbolic names where cosmic icon theme has them; UX can substitute.

---

## Appendix D — Ownership recap (Phase 2 files)

| File | Owner |
|---|---|
| `crates/gui/src/app.rs` | Architecture |
| `crates/gui/src/message.rs` | Architecture |
| `crates/gui/src/state.rs` | Architecture |
| `crates/gui/src/config.rs` | Architecture |
| `crates/gui/src/helper/**` | Architecture |
| `crates/gui/src/integration.rs` | Architecture |
| `crates/gui/src/main.rs` | Architecture |
| `crates/gui/src/core/**` | Architecture |
| `crates/gui/src/view/**` | UX |
| `crates/gui/src/widget/**` | UX |
| `tests/**` | Packaging |
| `crates/common`, `crates/helper` | Shared; Architecture patches only for §8 / §5.6 if assigned |
| Flatpak manifest, spawn env, CI | Packaging |

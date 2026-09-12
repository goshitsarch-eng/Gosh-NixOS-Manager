//! libcosmic application: nav, chrome, init, update, view dispatch.

use crate::config::UserPreferences;
use crate::helper::session::{self, helper_missing_message};
use crate::helper::spawn::{env_flag, in_flatpak, SpawnSpec};
use crate::integration::detect_system;
use crate::message::{
    ContextPage, Dialog, HelperEvent, HelperOp, Intent, Message, Page, RollbackMode,
};
use crate::state::AppState;
use common::actions::{default_bundles, default_profiles, CpuArch};
use common::config::SystemInfo;
use common::ipc::{DiskUsageInfo, Generation, HelperRequest, RebuildType};
use cosmic::app::{context_drawer, Core, Task};
use cosmic::iced::event::{self, Event};
use cosmic::iced::keyboard::{key::Named, Key};
use cosmic::iced::Subscription;
use cosmic::widget::menu::action::MenuAction as MenuActionTrait;
use cosmic::widget::menu::key_bind::{KeyBind, Modifier};
use cosmic::widget::{self, about::About, icon, menu, nav_bar};
use cosmic::{Application, ApplicationExt, Apply, Element};
use futures::StreamExt;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

const APP_ICON: &[u8] = include_bytes!("../../../data/icons/nixos-toolkit.svg");
const REPOSITORY: &str = env!("CARGO_PKG_REPOSITORY");

/// Startup flags (spawn spec, paths). Immutable after init.
#[derive(Debug, Clone)]
pub struct Flags {
    pub spawn: SpawnSpec,
    pub templates_dir: PathBuf,
    pub prefs_path: Option<PathBuf>,
    /// Tests / headless CI: do not spawn a privileged helper on startup.
    pub skip_privileged_on_init: bool,
    /// Tests: skip lspci / /etc/NIXOS probes.
    pub skip_host_probes: bool,
}

impl Flags {
    #[must_use]
    pub fn from_env() -> Self {
        let spawn = SpawnSpec::from_env();
        let templates_dir = spawn
            .templates_dir
            .clone()
            .unwrap_or_else(common::config::paths::templates_dir);
        Self {
            spawn,
            templates_dir,
            prefs_path: None,
            skip_privileged_on_init: env_flag("NIXOS_TOOLKIT_SKIP_PRIVILEGED_INIT"),
            skip_host_probes: env_flag("NIXOS_TOOLKIT_SKIP_HOST_PROBES"),
        }
    }

    #[must_use]
    pub fn for_tests() -> Self {
        Self {
            spawn: SpawnSpec::unavailable(None, Vec::new(), vec!["SHELL".into()]),
            templates_dir: PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../nix/templates"),
            prefs_path: None,
            skip_privileged_on_init: true,
            skip_host_probes: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HelperStatus {
    Idle,
    Active { op: HelperOp },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BannerKind {
    Error,
    Warning,
    Success,
    Info,
}

#[derive(Debug, Clone)]
pub struct Banner {
    pub kind: BannerKind,
    pub text: String,
}

#[derive(Debug, Clone, Default)]
pub struct FieldErrors {
    pub hostname: Option<String>,
    pub dns: Option<String>,
    pub username: Option<String>,
    pub packages: Option<String>,
    pub tcp_ports: Option<String>,
    pub udp_ports: Option<String>,
}

/// Application model. Views borrow this; `apply()` mutates it.
pub struct AppModel {
    core: Core,
    pub(crate) flags: Flags,
    pub(crate) nav: nav_bar::Model,
    pub(crate) context_page: ContextPage,
    pub(crate) context_open: bool,
    about: About,
    key_binds: HashMap<KeyBind, MenuAction>,
    pub(crate) state: AppState,
    pub(crate) system_info: SystemInfo,
    pub(crate) gpu_vendor: Option<String>,
    pub(crate) cpu_arch: CpuArch,
    pub(crate) prefs: UserPreferences,
    pub(crate) page: Page,
    pub(crate) dialog: Option<Dialog>,
    pub(crate) toasts: widget::Toasts<Message>,
    pub(crate) banner: Option<Banner>,
    pub(crate) helper_missing: bool,
    pub(crate) state_load_warning: Option<String>,
    pub(crate) package_input: String,
    pub(crate) dns_input: String,
    pub(crate) custom_tcp_input: String,
    pub(crate) custom_udp_input: String,
    pub(crate) rebuild_type: RebuildType,
    pub(crate) profile_preview: String,
    pub(crate) apply_preview: String,
    pub(crate) apply_log: String,
    pub(crate) generations_log: String,
    pub(crate) maintenance_log: String,
    pub(crate) disk_usage: Option<DiskUsageInfo>,
    pub(crate) generations: Vec<Generation>,
    pub(crate) current_generation: Option<u32>,
    pub(crate) helper: HelperStatus,
    pub(crate) busy: Busy,
    pub(crate) field_errors: FieldErrors,
    pub(crate) verify_pending: bool,
    pub(crate) helper_cancel: Arc<AtomicBool>,
    /// UDP port opened automatically for WireGuard (moved/removed with the toggle).
    pub(crate) wg_auto_udp: Option<u16>,
}

impl AppModel {
    /// Display-free model for integration tests (`Core::default()`).
    #[must_use]
    pub fn test_model() -> Self {
        Self::init(Core::default(), Flags::for_tests()).0
    }

    #[must_use]
    pub fn page(&self) -> Page {
        self.page
    }

    #[must_use]
    pub fn state(&self) -> &AppState {
        &self.state
    }

    #[must_use]
    pub fn pending_dialog(&self) -> Option<&Dialog> {
        self.dialog.as_ref()
    }

    #[must_use]
    pub fn busy(&self) -> Busy {
        self.busy
    }

    #[must_use]
    pub(crate) fn banner(&self) -> Option<&Banner> {
        self.banner.as_ref()
    }

    #[must_use]
    pub(crate) fn cpu_arch(&self) -> CpuArch {
        self.cpu_arch
    }

    #[must_use]
    pub(crate) fn generations_log(&self) -> &str {
        &self.generations_log
    }

    #[must_use]
    pub(crate) fn maintenance_log(&self) -> &str {
        &self.maintenance_log
    }

    fn sync_nav(&mut self, page: Page) {
        let entities: Vec<_> = self.nav.iter().collect();
        for entity in entities {
            if self.nav.data::<Page>(entity) == Some(&page) {
                self.nav.activate(entity);
                break;
            }
        }
    }

    pub(crate) fn seed_username_from_host(&mut self) {
        if self.state.username.is_some() {
            return;
        }
        self.state.username = default_username(
            self.flags.skip_host_probes,
            None,
            std::env::var("USER").ok().as_deref(),
        );
    }

    pub(crate) fn sync_draft_inputs(&mut self) {
        self.dns_input = self.state.dns_servers.join(", ");
        self.custom_tcp_input =
            crate::state::custom_tcp_input_from_ports(&self.state.network_config.allowed_tcp_ports);
        self.custom_udp_input =
            crate::state::custom_udp_input_from_ports(&self.state.network_config.allowed_udp_ports);
    }

    #[must_use]
    pub(crate) fn helper_can_spawn(&self) -> bool {
        self.flags.spawn.helper_available && !self.helper_missing
    }

    pub(crate) fn startup_intents(&mut self) -> Vec<Intent> {
        let mut intents = vec![
            Intent::SetWindowTitle(self.window_title()),
            Intent::ApplyTheme,
            Intent::DetectSystem,
            Intent::DetectGpu,
        ];
        if !self.flags.skip_privileged_on_init && self.flags.spawn.helper_available {
            self.busy = Busy::LoadingState;
            intents.push(Intent::SpawnHelper {
                op: HelperOp::ReadState,
                request: HelperRequest::ReadState,
            });
        }
        intents
    }
}

#[must_use]
pub(crate) fn default_username(
    skip_host_probes: bool,
    existing: Option<&str>,
    user_env: Option<&str>,
) -> Option<String> {
    if let Some(user) = existing {
        return Some(user.to_string());
    }
    if skip_host_probes {
        return None;
    }
    user_env
        .map(str::trim)
        .filter(|s| is_valid_username(s))
        .map(ToString::to_string)
}

#[must_use]
pub(crate) fn is_valid_username(s: &str) -> bool {
    common::validate::username_is_valid(s)
}

impl Application for AppModel {
    type Executor = cosmic::executor::Default;
    type Flags = Flags;
    type Message = Message;

    const APP_ID: &'static str = crate::APP_ID;

    fn core(&self) -> &Core {
        &self.core
    }

    fn core_mut(&mut self) -> &mut Core {
        &mut self.core
    }

    fn init(core: Core, flags: Self::Flags) -> (Self, Task<Self::Message>) {
        let mut nav = nav_bar::Model::default();
        for page in Page::ALL {
            let item = nav
                .insert()
                .text(page.title())
                .data(page)
                .icon(icon::from_name(page.icon_name()));
            if page == Page::Onboarding {
                item.activate();
            }
        }

        let about = About::default()
            .name(crate::fl!("app-title"))
            .icon(widget::icon::from_svg_bytes(APP_ICON))
            .version(env!("CARGO_PKG_VERSION"))
            .author(crate::fl!("about-author"))
            .comments(crate::fl!("about-comments"))
            .links([
                (crate::fl!("repository"), REPOSITORY.to_string()),
                (
                    crate::fl!("issues"),
                    "https://github.com/goshitsarch-eng/Gosh-NixOS-Manager/issues".to_string(),
                ),
            ])
            .license(env!("CARGO_PKG_LICENSE"));

        let prefs = UserPreferences::load(flags.prefs_path.as_deref());
        let helper_missing = !flags.spawn.helper_available;
        let system_info = if flags.skip_host_probes {
            SystemInfo::default()
        } else {
            detect_system()
        };

        let mut key_binds = HashMap::new();
        key_binds.insert(
            KeyBind {
                modifiers: vec![Modifier::Ctrl],
                key: Key::Character("q".into()),
            },
            MenuAction::Quit,
        );
        key_binds.insert(
            KeyBind {
                modifiers: vec![Modifier::Ctrl],
                key: Key::Character("r".into()),
            },
            MenuAction::Refresh,
        );
        key_binds.insert(
            KeyBind {
                modifiers: Vec::new(),
                key: Key::Named(Named::F5),
            },
            MenuAction::Refresh,
        );

        let mut app = AppModel {
            core,
            flags,
            nav,
            context_page: ContextPage::About,
            context_open: false,
            about,
            key_binds,
            state: AppState::default(),
            system_info,
            gpu_vendor: None,
            cpu_arch: CpuArch::detect(),
            prefs,
            page: Page::Onboarding,
            dialog: None,
            toasts: widget::Toasts::new(Message::DismissToast),
            banner: None,
            helper_missing,
            state_load_warning: None,
            package_input: String::new(),
            dns_input: String::new(),
            custom_tcp_input: String::new(),
            custom_udp_input: String::new(),
            rebuild_type: RebuildType::Switch,
            profile_preview: String::new(),
            apply_preview: String::new(),
            apply_log: String::new(),
            generations_log: String::new(),
            maintenance_log: String::new(),
            disk_usage: None,
            generations: Vec::new(),
            current_generation: None,
            helper: HelperStatus::Idle,
            busy: Busy::Idle,
            field_errors: FieldErrors::default(),
            verify_pending: false,
            helper_cancel: Arc::new(AtomicBool::new(false)),
            wg_auto_udp: None,
        };

        app.seed_username_from_host();
        app.sync_draft_inputs();
        if helper_missing {
            app.banner = Some(Banner {
                kind: BannerKind::Warning,
                text: crate::fl!("helper-missing-banner"),
            });
        }
        let intents = app.startup_intents();
        app.refresh_banner();
        let task = app.intents_to_task(intents);
        (app, task)
    }

    fn header_start(&self) -> Vec<Element<'_, Self::Message>> {
        let menu_bar = menu::bar(vec![menu::Tree::with_children(
            menu::root(crate::fl!("view")).apply(Element::from),
            menu::items(
                &self.key_binds,
                vec![
                    menu::Item::Button(crate::fl!("about"), None, MenuAction::About),
                    menu::Item::Button(crate::fl!("refresh"), None, MenuAction::Refresh),
                ],
            ),
        )]);
        vec![menu_bar.into()]
    }

    fn header_end(&self) -> Vec<Element<'_, Self::Message>> {
        let refresh = widget::button::icon(icon::from_name("view-refresh-symbolic"))
            .on_press(Message::RefreshSystem)
            .tooltip(crate::fl!("refresh"));
        vec![refresh.into()]
    }

    fn nav_model(&self) -> Option<&nav_bar::Model> {
        Some(&self.nav)
    }

    fn context_drawer(&self) -> Option<context_drawer::ContextDrawer<'_, Self::Message>> {
        if !self.core.window.show_context {
            return None;
        }
        Some(match self.context_page {
            ContextPage::About => context_drawer::about(
                &self.about,
                |url| Message::LaunchUrl(url.to_string()),
                Message::ToggleAbout,
            ),
        })
    }

    fn dialog(&self) -> Option<Element<'_, Self::Message>> {
        let dialog = self.dialog.as_ref()?;
        Some(match dialog {
            Dialog::ConfirmApply {
                heading,
                body,
                destructive,
            } => {
                let primary = if *destructive {
                    widget::button::destructive(crate::fl!("dialog-apply"))
                        .on_press(Message::ConfirmApply)
                } else {
                    widget::button::suggested(crate::fl!("dialog-apply"))
                        .on_press(Message::ConfirmApply)
                };
                widget::dialog()
                    .title(heading.as_str())
                    .body(body.as_str())
                    .primary_action(primary)
                    .secondary_action(
                        widget::button::standard(crate::fl!("dialog-cancel"))
                            .on_press(Message::CancelApply),
                    )
                    .into()
            }
            Dialog::ConfirmRollback { generation } => {
                let generation_n: u32 = *generation;
                widget::dialog()
                    .title(crate::fl!(
                        "dialog-rollback-title",
                        generation = generation_n
                    ))
                    .body(crate::fl!("dialog-rollback-body"))
                    .primary_action(
                        widget::button::suggested(crate::fl!("dialog-switch-now")).on_press(
                            Message::ConfirmRollback {
                                generation: *generation,
                                mode: RollbackMode::SwitchNow,
                            },
                        ),
                    )
                    .secondary_action(
                        widget::button::standard(crate::fl!("dialog-set-for-next-boot")).on_press(
                            Message::ConfirmRollback {
                                generation: *generation,
                                mode: RollbackMode::SetForNextBoot,
                            },
                        ),
                    )
                    .tertiary_action(
                        widget::button::standard(crate::fl!("dialog-cancel"))
                            .on_press(Message::DismissDialog),
                    )
                    .into()
            }
            Dialog::ConfirmDeleteGeneration { generation } => {
                let generation_n: u32 = *generation;
                widget::dialog()
                    .title(crate::fl!("dialog-delete-title", generation = generation_n))
                    .body(crate::fl!("dialog-delete-body"))
                    .primary_action(
                        widget::button::destructive(crate::fl!("dialog-delete")).on_press(
                            Message::ConfirmDeleteGeneration {
                                generation: *generation,
                            },
                        ),
                    )
                    .secondary_action(
                        widget::button::standard(crate::fl!("dialog-cancel"))
                            .on_press(Message::DismissDialog),
                    )
                    .into()
            }
            Dialog::ConfirmMaintenance { id, name, warning } => widget::dialog()
                .title(crate::fl!("dialog-maintenance-title", name = name.as_str()))
                .body(warning.as_str())
                .primary_action(
                    widget::button::destructive(crate::fl!("dialog-run-anyway"))
                        .on_press(Message::ConfirmMaintenance { id: id.clone() }),
                )
                .secondary_action(
                    widget::button::standard(crate::fl!("dialog-cancel"))
                        .on_press(Message::DismissDialog),
                )
                .into(),
        })
    }

    fn view(&self) -> Element<'_, Self::Message> {
        let content = crate::view::root(self);
        widget::toaster(&self.toasts, content)
    }

    fn subscription(&self) -> Subscription<Self::Message> {
        event::listen_with(|event, status, _id| {
            if status == event::Status::Captured {
                return None;
            }
            match event {
                Event::Keyboard(cosmic::iced::keyboard::Event::KeyPressed {
                    key,
                    modifiers,
                    ..
                }) => match key {
                    Key::Named(Named::F5) => Some(Message::RefreshSystem),
                    Key::Character(c) if c.eq_ignore_ascii_case("q") && modifiers.control() => {
                        Some(Message::Quit)
                    }
                    Key::Character(c) if c.eq_ignore_ascii_case("r") && modifiers.control() => {
                        Some(Message::RefreshSystem)
                    }
                    _ => {
                        let _ = modifiers;
                        None
                    }
                },
                _ => None,
            }
        })
    }

    fn update(&mut self, message: Self::Message) -> Task<Self::Message> {
        if let Message::DismissToast(id) = message {
            self.toasts.remove(id);
            return Task::none();
        }
        if let Message::NavSelect(page) = message {
            self.sync_nav(page);
        }

        let intents = self.apply(message);
        self.set_show_context(self.context_open);
        self.intents_to_task(intents)
    }

    fn on_nav_select(&mut self, id: nav_bar::Id) -> Task<Self::Message> {
        self.nav.activate(id);
        let page = self.nav.active_data::<Page>().copied().unwrap_or(self.page);
        self.update(Message::NavSelect(page))
    }

    fn on_escape(&mut self) -> Task<Self::Message> {
        if self.dialog.is_some() {
            return self.update(Message::DismissDialog);
        }
        if self.context_open {
            return self.update(Message::ToggleAbout);
        }
        Task::none()
    }
}

impl AppModel {
    fn intents_to_task(&mut self, intents: Vec<Intent>) -> Task<Message> {
        let mut tasks = Vec::new();
        for intent in intents {
            match intent {
                Intent::None => {}
                Intent::SpawnHelper { op, request } => {
                    self.helper_cancel
                        .store(false, std::sync::atomic::Ordering::SeqCst);
                    tasks.push(spawn_helper_task(
                        self.flags.spawn.clone(),
                        op,
                        request,
                        self.helper_cancel.clone(),
                    ));
                }
                Intent::SendOnSession { .. } | Intent::CloseSession => {
                    // Session chaining is task 12.
                }
                Intent::DetectSystem => {
                    let skip = self.flags.skip_host_probes;
                    tasks.push(cosmic::task::future(async move {
                        if skip {
                            Message::SystemDetected(SystemInfo::default())
                        } else {
                            Message::SystemDetected(detect_system())
                        }
                    }));
                }
                Intent::DetectGpu => {
                    if !self.flags.skip_host_probes {
                        tasks.push(cosmic::task::future(async {
                            Message::GpuDetected(crate::integration::detect_gpu())
                        }));
                    }
                }
                Intent::LocalPreview => {
                    let preview = generate_local_preview(self);
                    tasks.push(cosmic::task::message(Message::PreviewReady(preview)));
                }
                Intent::LocalProfilePreview { id } => {
                    self.profile_preview = generate_profile_preview(&id, &self.flags.templates_dir);
                }
                Intent::CopyClipboard(text) => {
                    tasks.push(
                        cosmic::iced::clipboard::write(text)
                            .map(|()| cosmic::Action::App(Message::ClipboardCopied { ok: true })),
                    );
                }
                Intent::OpenPath(path) => {
                    tasks.push(cosmic::task::future(async move {
                        match open_path(&path) {
                            Ok(()) => cosmic::Action::None,
                            Err(err) => {
                                tracing::warn!(?err, path = %path.display(), "failed to open path");
                                cosmic::Action::App(Message::Notify {
                                    text: crate::fl!("toast-open-failed"),
                                })
                            }
                        }
                    }));
                }
                Intent::OpenUrl(url) => {
                    tasks.push(cosmic::task::future(async move {
                        match open::that_detached(&url) {
                            Ok(()) => cosmic::Action::None,
                            Err(err) => {
                                tracing::warn!(?err, url, "failed to open url");
                                cosmic::Action::App(Message::Notify {
                                    text: crate::fl!("toast-open-failed"),
                                })
                            }
                        }
                    }));
                }
                Intent::SavePrefs => {
                    let result = if let Some(path) = &self.flags.prefs_path {
                        self.prefs.save_to(path)
                    } else {
                        self.prefs.save()
                    };
                    if let Err(err) = result {
                        tracing::warn!(?err, "failed to save preferences");
                    }
                }
                Intent::ApplyTheme => {
                    tasks.push(cosmic::command::set_theme(
                        self.prefs.color_scheme.to_theme(),
                    ));
                }
                Intent::SetWindowTitle(title) => {
                    self.set_header_title(title.clone());
                    if let Some(id) = self.core.main_window_id() {
                        tasks.push(self.set_window_title(title, id));
                    }
                }
                Intent::ShowToast { text, timeout_ms } => {
                    let toast = widget::Toast::new(text)
                        .duration(std::time::Duration::from_millis(timeout_ms));
                    tasks.push(self.toasts.push(toast).map(cosmic::Action::App));
                }
                Intent::Exit => {
                    tasks.push(cosmic::iced::exit());
                }
            }
        }
        Task::batch(tasks)
    }
}

fn generate_local_preview(app: &AppModel) -> String {
    let profiles = default_profiles();
    let bundles = default_bundles();
    let mut options = app.state.to_nix_gen_options(&profiles, &bundles);
    options.hardware_config = app.hardware_for_nix();
    common::nix::generate_preview_full_from(&options, &app.flags.templates_dir)
}

fn generate_profile_preview(id: &str, templates_dir: &Path) -> String {
    match default_profiles().into_iter().find(|profile| profile.id == id) {
        Some(profile) => match common::nix::read_template_from(templates_dir, &profile.template) {
            Ok(content) => content,
            Err(_) => format!(
                "# Profile: {}\n# Template: {}\n# (Template file not found - will be available after installation)",
                profile.name, profile.template
            ),
        },
        None => format!(
            "# Profile: {id}\n# (Template file not found - will be available after installation)"
        ),
    }
}

fn spawn_helper_task(
    spec: SpawnSpec,
    op: HelperOp,
    request: HelperRequest,
    cancel: Arc<AtomicBool>,
) -> Task<Message> {
    if !spec.helper_available {
        return cosmic::task::message(Message::Helper(HelperEvent::SpawnFailed {
            op,
            error: helper_missing_message().to_string(),
        }));
    }
    cosmic::task::stream(session::stream(spec, op, request, cancel).map(Message::Helper))
}

fn open_path(path: &PathBuf) -> std::io::Result<()> {
    if in_flatpak() {
        Command::new("flatpak-spawn")
            .args(["--host", "--", "xdg-open"])
            .arg(path)
            .spawn()?;
        Ok(())
    } else {
        open::that_detached(path)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum MenuAction {
    About,
    Quit,
    Refresh,
}

impl MenuActionTrait for MenuAction {
    type Message = Message;

    fn message(&self) -> Self::Message {
        match self {
            Self::About => Message::ToggleAbout,
            Self::Quit => Message::Quit,
            Self::Refresh => Message::RefreshSystem,
        }
    }
}

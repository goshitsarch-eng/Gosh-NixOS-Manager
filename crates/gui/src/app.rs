//! libcosmic application: nav, chrome, init, update, view dispatch.

use crate::config::UserPreferences;
use crate::helper::session::{self, helper_missing_message};
use crate::helper::spawn::{env_flag, in_flatpak, SpawnSpec};
use crate::integration::detect_system;
use crate::message::{
    ContextPage, Dialog, HelperEvent, HelperOp, Intent, Message, Page, RollbackMode,
};
use crate::state::AppState;
use common::actions::CpuArch;
use common::config::SystemInfo;
use common::ipc::{DiskUsageInfo, Generation, HelperRequest};
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
use std::path::PathBuf;
use std::process::Command;

const APP_ICON: &[u8] = include_bytes!("../../../data/icons/nixos-toolkit.svg");
const REPOSITORY: &str = env!("CARGO_PKG_REPOSITORY");

/// Startup flags (spawn spec, paths). Immutable after init.
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
            templates_dir: PathBuf::from("./nix/templates"),
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
}

impl AppModel {
    #[must_use]
    pub fn page(&self) -> Page {
        self.page
    }

    #[must_use]
    pub fn state(&self) -> &AppState {
        &self.state
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
            .author("NixOS Toolkit Contributors")
            .comments("A declarative NixOS system management tool. Built with libcosmic, iced, Rust, Nix.")
            .links([
                (crate::fl!("repository"), REPOSITORY.to_string()),
                (
                    "Issues".to_string(),
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
            toasts: widget::Toasts::new(|_| Message::DismissToast),
            banner: None,
            helper_missing,
            state_load_warning: None,
            package_input: String::new(),
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
        };

        if helper_missing {
            app.banner = Some(Banner {
                kind: BannerKind::Warning,
                text: helper_missing_message().to_string(),
            });
        }
        app.refresh_banner();

        let mut intents = vec![
            Intent::SetWindowTitle(app.window_title()),
            Intent::ApplyTheme,
        ];
        if !app.flags.skip_privileged_on_init && app.flags.spawn.helper_available {
            app.busy = Busy::LoadingState;
            intents.push(Intent::SpawnHelper {
                op: HelperOp::ReadState,
                request: HelperRequest::ReadState,
            });
        }

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
                    menu::Item::Button(crate::fl!("quit"), None, MenuAction::Quit),
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
                    widget::button::destructive("Apply").on_press(Message::ConfirmApply)
                } else {
                    widget::button::suggested("Apply").on_press(Message::ConfirmApply)
                };
                widget::dialog()
                    .title(heading.as_str())
                    .body(body.as_str())
                    .primary_action(primary)
                    .secondary_action(
                        widget::button::standard("Cancel").on_press(Message::CancelApply),
                    )
                    .into()
            }
            Dialog::ConfirmRollback { generation } => widget::dialog()
                .title(format!("Switch to Generation {generation}?"))
                .body("This rebuilds and activates the selected generation.")
                .primary_action(widget::button::suggested("Switch Now").on_press(
                    Message::ConfirmRollback {
                        generation: *generation,
                        mode: RollbackMode::SwitchNow,
                    },
                ))
                .secondary_action(widget::button::standard("Set for Next Boot").on_press(
                    Message::ConfirmRollback {
                        generation: *generation,
                        mode: RollbackMode::SetForNextBoot,
                    },
                ))
                .tertiary_action(
                    widget::button::standard("Cancel").on_press(Message::DismissDialog),
                )
                .into(),
            Dialog::ConfirmDeleteGeneration { generation } => widget::dialog()
                .title(format!("Delete Generation {generation}?"))
                .body("This permanently deletes the selected generation.")
                .primary_action(widget::button::destructive("Delete").on_press(
                    Message::ConfirmDeleteGeneration {
                        generation: *generation,
                    },
                ))
                .secondary_action(
                    widget::button::standard("Cancel").on_press(Message::DismissDialog),
                )
                .into(),
            Dialog::ConfirmMaintenance { id, name, warning } => widget::dialog()
                .title(format!("Run {name}?"))
                .body(warning.as_str())
                .primary_action(
                    widget::button::destructive("Run Anyway")
                        .on_press(Message::ConfirmMaintenance { id: id.clone() }),
                )
                .secondary_action(
                    widget::button::standard("Cancel").on_press(Message::DismissDialog),
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
                    Key::Named(Named::Escape) => Some(Message::DismissDialog),
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
        if matches!(message, Message::DismissToast) {
            self.toasts = widget::Toasts::new(|_| Message::DismissToast);
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
        self.update(Message::DismissDialog)
    }
}

impl AppModel {
    fn intents_to_task(&mut self, intents: Vec<Intent>) -> Task<Message> {
        let mut tasks = Vec::new();
        for intent in intents {
            match intent {
                Intent::None => {}
                Intent::SpawnHelper { op, request } => {
                    tasks.push(spawn_helper_task(self.flags.spawn.clone(), op, request));
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
                    tasks.push(cosmic::task::future(async {
                        Message::GpuDetected(crate::integration::detect_gpu())
                    }));
                }
                Intent::LocalPreview | Intent::LocalProfilePreview { .. } => {
                    // Local preview is task 12 / 5.
                }
                Intent::CopyClipboard(text) => {
                    tasks.push(
                        cosmic::iced::clipboard::write(text)
                            .map(|()| cosmic::Action::App(Message::ClipboardCopied { ok: true })),
                    );
                }
                Intent::OpenPath(path) => {
                    tasks.push(cosmic::task::future(async move {
                        if let Err(err) = open_path(&path) {
                            tracing::warn!(?err, path = %path.display(), "failed to open path");
                        }
                        cosmic::Action::None
                    }));
                }
                Intent::OpenUrl(url) => {
                    tasks.push(cosmic::task::future(async move {
                        if let Err(err) = open::that_detached(&url) {
                            tracing::warn!(?err, url, "failed to open url");
                        }
                        cosmic::Action::None
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

fn spawn_helper_task(spec: SpawnSpec, op: HelperOp, request: HelperRequest) -> Task<Message> {
    if !spec.helper_available {
        return cosmic::task::message(Message::Helper(HelperEvent::SpawnFailed {
            op,
            error: helper_missing_message().to_string(),
        }));
    }
    cosmic::task::stream(session::stream(spec, op, request).map(Message::Helper))
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

//! Pure-enough reducer: `AppModel::apply` returns side-effect [`Intent`]s.

use crate::app::{AppModel, Banner, BannerKind, Busy, HelperStatus};
use crate::core::packages::{classify_new_packages, parse_package_input};
use crate::helper::session::helper_missing_message;
use crate::integration::{classic_integration_snippet, flake_integration_snippet};
use crate::message::{ContextPage, Dialog, HelperEvent, HelperOp, Intent, Message};
use common::actions::default_bundles;
use common::config::{ConfigMode, IntegrationStatus};
use common::ipc::HelperResponse;
use std::collections::HashSet;
use std::path::PathBuf;

const TOAST_MS: u64 = 3000;
const IN_BUNDLE_TOAST_MS: u64 = 4000;

const SSH_ROOT_LOGIN: [&str; 3] = ["no", "prohibit-password", "yes"];

impl AppModel {
    /// Reducer. No iced/libcosmic widget types. Exhaustive over [`Message`].
    pub fn apply(&mut self, msg: Message) -> Vec<Intent> {
        match msg {
            Message::NavSelect(page) => {
                self.page = page;
                vec![Intent::SetWindowTitle(self.window_title())]
            }
            Message::ToggleAbout => {
                if self.context_page == ContextPage::About && self.context_open {
                    self.context_open = false;
                } else {
                    self.context_page = ContextPage::About;
                    self.context_open = true;
                }
                Vec::new()
            }
            Message::LaunchUrl(url) => vec![Intent::OpenUrl(url)],
            Message::Quit => vec![Intent::Exit],
            Message::RefreshSystem => vec![Intent::DetectSystem],
            Message::SystemDetected(info) => {
                self.system_info = info;
                self.refresh_banner();
                Vec::new()
            }
            Message::DismissToast => Vec::new(),
            Message::DismissDialog => {
                self.dialog = None;
                Vec::new()
            }
            Message::UpdateConfig(prefs) => {
                self.prefs = prefs;
                vec![Intent::ApplyTheme]
            }
            Message::ClipboardCopied { ok } => {
                if ok {
                    Vec::new()
                } else {
                    vec![Intent::ShowToast {
                        text: "Failed to copy to clipboard".into(),
                        timeout_ms: TOAST_MS,
                    }]
                }
            }

            Message::CopyIntegrationSnippet => {
                let snippet = match self.system_info.config_mode {
                    ConfigMode::Flake => flake_integration_snippet(),
                    _ => classic_integration_snippet(),
                };
                vec![
                    Intent::CopyClipboard(snippet),
                    Intent::ShowToast {
                        text: "Snippet copied to clipboard".into(),
                        timeout_ms: TOAST_MS,
                    },
                ]
            }
            Message::OpenEtcNixos => vec![Intent::OpenPath(PathBuf::from("/etc/nixos"))],
            Message::VerifyIntegration => vec![Intent::DetectSystem],

            Message::SelectProfile(id) => {
                self.state.select_profile(id);
                Vec::new()
            }
            Message::ClearProfile => {
                self.state.clear_profile();
                Vec::new()
            }
            Message::RefreshProfilePreview => {
                if let Some(id) = self.state.selected_profile.clone() {
                    vec![Intent::LocalProfilePreview { id }]
                } else {
                    self.profile_preview.clear();
                    Vec::new()
                }
            }

            Message::ToggleBundle { id, enabled } => {
                if enabled {
                    self.state.enable_bundle(&id);
                    if let Some(bundle) = default_bundles().iter().find(|b| b.id == id) {
                        let packages: HashSet<String> =
                            bundle.packages.iter().map(|p| p.id.clone()).collect();
                        self.state.set_bundle_packages(id, packages);
                    }
                } else {
                    self.state.disable_bundle(&id);
                }
                Vec::new()
            }
            Message::ToggleBundlePackage {
                bundle_id,
                package,
                enabled,
            } => {
                if self.state.get_bundle_packages(&bundle_id).is_none() {
                    self.state.set_bundle_packages(&bundle_id, HashSet::new());
                }
                self.state
                    .toggle_bundle_package(&bundle_id, package, enabled);
                Vec::new()
            }
            Message::ExpandBundle { id, expanded } => {
                self.state.set_bundle_expanded(id, expanded);
                Vec::new()
            }

            Message::PackageInputChanged(value) => {
                self.package_input = value;
                Vec::new()
            }
            Message::AddPackagesFromInput => {
                let packages = parse_package_input(&self.package_input);
                if packages.is_empty() {
                    return Vec::new();
                }
                self.package_input.clear();

                let (added, duplicates, in_bundle) = classify_new_packages(
                    &packages,
                    &self.state.custom_packages,
                    &default_bundles(),
                );
                for pkg in &added {
                    self.state.add_custom_package(pkg.clone());
                }

                let mut intents = Vec::new();
                if !added.is_empty() {
                    let text = if added.len() == 1 {
                        format!("Added: {}", added[0])
                    } else {
                        format!("Added {} packages", added.len())
                    };
                    intents.push(Intent::ShowToast {
                        text,
                        timeout_ms: TOAST_MS,
                    });
                }
                if !duplicates.is_empty() {
                    intents.push(Intent::ShowToast {
                        text: format!("Already added: {}", duplicates.join(", ")),
                        timeout_ms: TOAST_MS,
                    });
                }
                for (pkg, bundle) in &in_bundle {
                    intents.push(Intent::ShowToast {
                        text: format!("'{pkg}' is already in '{bundle}' bundle"),
                        timeout_ms: IN_BUNDLE_TOAST_MS,
                    });
                }
                intents
            }
            Message::AddCustomPackages(packages) => {
                for pkg in packages {
                    self.state.add_custom_package(pkg);
                }
                Vec::new()
            }
            Message::RemoveCustomPackage(pkg) => {
                self.state.remove_custom_package(&pkg);
                Vec::new()
            }

            Message::HostnameChanged(hostname) => {
                let trimmed = hostname.trim().to_string();
                if trimmed.is_empty() {
                    self.state.hostname = None;
                    self.state.has_changes = true;
                } else if self.system_info.hostname.as_deref() == Some(trimmed.as_str()) {
                    self.state.hostname = None;
                } else {
                    self.state.set_hostname(trimmed);
                }
                Vec::new()
            }
            Message::DnsServersChanged(raw) => {
                if let Err(err) = self.state.parse_and_set_dns(&raw) {
                    self.field_errors.dns = Some(err);
                } else {
                    self.field_errors.dns = None;
                }
                Vec::new()
            }
            Message::UsernameChanged(username) => {
                self.state.set_username(username);
                Vec::new()
            }
            Message::ToggleUserGroup { group, enabled } => {
                self.state.set_user_group(group, enabled);
                Vec::new()
            }
            Message::ColorSchemeChanged(scheme) => {
                self.prefs.color_scheme = scheme;
                vec![Intent::SavePrefs, Intent::ApplyTheme]
            }

            Message::SetNvidiaDriver(driver) => {
                self.state.hardware_config.nvidia_driver = driver;
                self.state.has_changes = true;
                Vec::new()
            }
            Message::SetNvidiaModesetting(enabled) => {
                self.state.hardware_config.nvidia_modesetting = enabled;
                self.state.has_changes = true;
                Vec::new()
            }
            Message::SetNvidiaPowerManagement(enabled) => {
                self.state.hardware_config.nvidia_powermanagement = enabled;
                self.state.has_changes = true;
                Vec::new()
            }
            Message::SetNvidiaOpen(enabled) => {
                self.state.hardware_config.nvidia_open = enabled;
                self.state.has_changes = true;
                Vec::new()
            }
            Message::SetAudioServer(server) => {
                self.state.hardware_config.audio_server = server;
                self.state.has_changes = true;
                Vec::new()
            }
            Message::SetAudioLowLatency(enabled) => {
                self.state.hardware_config.audio_lowlatency = enabled;
                self.state.has_changes = true;
                Vec::new()
            }
            Message::SetBluetoothEnabled(enabled) => {
                self.state.set_bluetooth_enabled(enabled);
                Vec::new()
            }
            Message::SetBluetoothAutoPower(enabled) => {
                self.state.hardware_config.bluetooth_autopower = enabled;
                self.state.has_changes = true;
                Vec::new()
            }
            Message::SetPowerProfile(profile) => {
                self.state.hardware_config.power_profile = profile;
                self.state.has_changes = true;
                Vec::new()
            }
            Message::SetTlpEnabled(enabled) => {
                self.state.hardware_config.tlp_enabled = enabled;
                self.state.has_changes = true;
                Vec::new()
            }
            Message::SetThermaldEnabled(enabled) => {
                self.state.hardware_config.thermald_enabled = enabled;
                self.state.has_changes = true;
                Vec::new()
            }
            Message::GpuDetected(gpu) => {
                self.gpu_vendor = Some(gpu);
                Vec::new()
            }

            Message::SetFirewallEnabled(enabled) => {
                self.state.network_config.firewall_enabled = enabled;
                self.state.has_changes = true;
                Vec::new()
            }
            Message::ToggleTcpPort { port, enabled } => {
                self.state.set_tcp_port(port, enabled);
                Vec::new()
            }
            Message::CustomTcpPortsChanged(raw) => {
                if let Err(err) = self.state.parse_and_add_tcp_ports(&raw) {
                    tracing::debug!(err, "invalid custom TCP ports");
                }
                Vec::new()
            }
            Message::SetSshEnabled(enabled) => {
                self.state.network_config.ssh_enabled = enabled;
                self.state.has_changes = true;
                Vec::new()
            }
            Message::SetSshPort(port) => {
                self.state.network_config.ssh_port = port;
                self.state.has_changes = true;
                Vec::new()
            }
            Message::SetSshPasswordAuth(enabled) => {
                self.state.network_config.ssh_password_auth = enabled;
                self.state.has_changes = true;
                Vec::new()
            }
            Message::SetSshRootLogin(index) => {
                let value = SSH_ROOT_LOGIN.get(index as usize).copied().unwrap_or("no");
                self.state.network_config.ssh_root_login = value.to_string();
                self.state.has_changes = true;
                Vec::new()
            }
            Message::SetFail2banEnabled(enabled) => {
                self.state.network_config.fail2ban_enabled = enabled;
                self.state.has_changes = true;
                Vec::new()
            }
            Message::SetTailscaleEnabled(enabled) => {
                self.state.network_config.tailscale_enabled = enabled;
                self.state.has_changes = true;
                Vec::new()
            }

            Message::ToggleService { id, enabled } => {
                self.state.set_service(&id, enabled);
                Vec::new()
            }

            Message::RefreshPreview => vec![Intent::LocalPreview],
            Message::PreviewReady(preview) => {
                self.apply_preview = preview;
                Vec::new()
            }
            Message::RequestApply => {
                if self.busy != Busy::Idle {
                    return Vec::new();
                }
                let empty = self.state.selected_profile.is_none()
                    && self.state.enabled_bundles.is_empty()
                    && self.state.custom_packages.is_empty();
                self.dialog = Some(if empty {
                    Dialog::ConfirmApply {
                        heading: "Apply Empty Configuration?".into(),
                        body: "This will remove ALL managed software from the generated NixOS configuration.".into(),
                        destructive: true,
                    }
                } else {
                    Dialog::ConfirmApply {
                        heading: "Apply Configuration?".into(),
                        body: "This will run nixos-rebuild switch.".into(),
                        destructive: false,
                    }
                });
                Vec::new()
            }
            Message::ConfirmApply => {
                self.dialog = None;
                // Apply session is task 12. Do not spawn pkexec here.
                Vec::new()
            }
            Message::CancelApply => {
                self.dialog = None;
                Vec::new()
            }
            Message::RequestDryRun => {
                // Dry-run session is task 12.
                Vec::new()
            }
            Message::ApplyFinished { success, message } => {
                self.busy = Busy::Idle;
                if success {
                    self.state.mark_applied();
                }
                vec![Intent::ShowToast {
                    text: message,
                    timeout_ms: TOAST_MS,
                }]
            }

            Message::LoadGenerations => {
                // Helper ListGenerations is task 13.
                Vec::new()
            }
            Message::GenerationsLoaded(list) => {
                self.current_generation = list.iter().find(|g| g.current).map(|g| g.number);
                self.generations = list;
                self.busy = Busy::Idle;
                Vec::new()
            }
            Message::RequestRollback { generation } => {
                self.dialog = Some(Dialog::ConfirmRollback { generation });
                Vec::new()
            }
            Message::ConfirmRollback { generation, mode } => {
                self.dialog = None;
                let _ = (generation, mode);
                // Rollback session is task 13.
                Vec::new()
            }
            Message::RequestDeleteGeneration { generation } => {
                self.dialog = Some(Dialog::ConfirmDeleteGeneration { generation });
                Vec::new()
            }
            Message::ConfirmDeleteGeneration { generation } => {
                self.dialog = None;
                let _ = generation;
                Vec::new()
            }
            Message::RollbackToPrevious => Vec::new(),

            Message::LoadDiskUsage => Vec::new(),
            Message::DiskUsageLoaded(info) => {
                self.disk_usage = Some(info);
                Vec::new()
            }
            Message::RequestMaintenance { id } => {
                if let Some(action) = common::actions::default_maintenance_actions()
                    .into_iter()
                    .find(|a| a.id == id)
                {
                    if let Some(warning) = action.warning {
                        self.dialog = Some(Dialog::ConfirmMaintenance {
                            id: action.id,
                            name: action.name,
                            warning,
                        });
                    }
                    // No warning: run immediately in task 14.
                }
                Vec::new()
            }
            Message::ConfirmMaintenance { id } => {
                self.dialog = None;
                let _ = id;
                Vec::new()
            }

            Message::Helper(event) => self.apply_helper(event),
        }
    }

    fn apply_helper(&mut self, event: HelperEvent) -> Vec<Intent> {
        match event {
            HelperEvent::SpawnFailed { op, error } => {
                self.helper = HelperStatus::Idle;
                self.busy = Busy::Idle;
                if matches!(op, HelperOp::ReadState) {
                    self.state_load_warning =
                        Some("Could not load saved state - using defaults".into());
                }
                if error.contains("not available") {
                    self.helper_missing = true;
                }
                self.refresh_banner();
                Vec::new()
            }
            HelperEvent::Spawned { op } => {
                self.helper = HelperStatus::Active { op };
                Vec::new()
            }
            HelperEvent::Response { op, response } => {
                match *response {
                    HelperResponse::State(ipc) => {
                        self.state = crate::state::AppState::from_ipc_state(ipc);
                        self.state_load_warning = None;
                        self.refresh_banner();
                    }
                    HelperResponse::Error { message, .. } => {
                        if matches!(op, HelperOp::ReadState) {
                            self.state_load_warning = Some(format!("State read error: {message}"));
                            self.refresh_banner();
                        }
                    }
                    HelperResponse::ApplyComplete { success, message } => {
                        return self.apply(Message::ApplyFinished { success, message });
                    }
                    HelperResponse::Generations(list) => {
                        return self.apply(Message::GenerationsLoaded(list));
                    }
                    HelperResponse::DiskUsage(info) => {
                        return self.apply(Message::DiskUsageLoaded(info));
                    }
                    HelperResponse::Log { message, .. } => {
                        self.apply_log.push_str(&message);
                        self.apply_log.push('\n');
                    }
                    _ => {}
                }
                if !matches!(op, HelperOp::Apply { .. }) {
                    self.helper = HelperStatus::Idle;
                }
                Vec::new()
            }
            HelperEvent::Closed { op, .. } | HelperEvent::Timeout { op } => {
                self.helper = HelperStatus::Idle;
                self.busy = Busy::Idle;
                if matches!(op, HelperOp::ReadState) {
                    self.state_load_warning = Some("State read timeout".into());
                    self.refresh_banner();
                }
                Vec::new()
            }
        }
    }

    pub(crate) fn refresh_banner(&mut self) {
        // Priority: Not NixOS > helper-missing / state-load warning > integration.
        if !self.system_info.is_nixos && !self.flags.skip_host_probes {
            self.banner = Some(Banner {
                kind: BannerKind::Error,
                text: "Not running on NixOS".into(),
            });
            return;
        }
        if self.helper_missing {
            self.banner = Some(Banner {
                kind: BannerKind::Warning,
                text: helper_missing_message().to_string(),
            });
            return;
        }
        if let Some(warning) = &self.state_load_warning {
            self.banner = Some(Banner {
                kind: BannerKind::Warning,
                text: warning.clone(),
            });
            return;
        }
        if self.flags.skip_host_probes {
            self.banner = None;
            return;
        }
        self.banner = match self.system_info.integration_status {
            IntegrationStatus::Integrated => Some(Banner {
                kind: BannerKind::Success,
                text: "Integrated - Ready to apply changes".into(),
            }),
            IntegrationStatus::NotIntegrated => Some(Banner {
                kind: BannerKind::Warning,
                text: "Setup required - See Getting Started".into(),
            }),
            IntegrationStatus::Unknown => None,
        };
    }

    pub(crate) fn window_title(&self) -> String {
        format!("{} — {}", crate::fl!("app-title"), self.page.title())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::Flags;
    use crate::config::ColorSchemePreference;
    use crate::message::{Dialog, HelperOp, Page, RollbackMode};
    use common::ipc::{
        AppState as IpcAppState, DiskUsageInfo, Generation, HardwareConfig, LogLevel,
        NetworkConfig, RebuildType, ServicesConfig,
    };
    use common::SystemInfo;
    use cosmic::Application;

    fn test_app() -> AppModel {
        AppModel::init(cosmic::Core::default(), Flags::for_tests()).0
    }

    fn dummy_generation() -> Generation {
        Generation {
            number: 1,
            date: String::new(),
            current: true,
            nixos_version: None,
            kernel_version: None,
            config_rev: None,
        }
    }

    fn dummy_disk() -> DiskUsageInfo {
        DiskUsageInfo {
            store_size: "0".into(),
            generation_count: 0,
            error: None,
        }
    }

    /// Fails to compile if a [`Message`] variant is added without updating this match.
    #[test]
    fn all_message_variants_mentioned() {
        fn mention(msg: Message) {
            match msg {
                Message::NavSelect(_)
                | Message::ToggleAbout
                | Message::LaunchUrl(_)
                | Message::Quit
                | Message::RefreshSystem
                | Message::SystemDetected(_)
                | Message::DismissToast
                | Message::DismissDialog
                | Message::UpdateConfig(_)
                | Message::ClipboardCopied { .. }
                | Message::CopyIntegrationSnippet
                | Message::OpenEtcNixos
                | Message::VerifyIntegration
                | Message::SelectProfile(_)
                | Message::ClearProfile
                | Message::RefreshProfilePreview
                | Message::ToggleBundle { .. }
                | Message::ToggleBundlePackage { .. }
                | Message::ExpandBundle { .. }
                | Message::PackageInputChanged(_)
                | Message::AddPackagesFromInput
                | Message::AddCustomPackages(_)
                | Message::RemoveCustomPackage(_)
                | Message::HostnameChanged(_)
                | Message::DnsServersChanged(_)
                | Message::UsernameChanged(_)
                | Message::ToggleUserGroup { .. }
                | Message::ColorSchemeChanged(_)
                | Message::SetNvidiaDriver(_)
                | Message::SetNvidiaModesetting(_)
                | Message::SetNvidiaPowerManagement(_)
                | Message::SetNvidiaOpen(_)
                | Message::SetAudioServer(_)
                | Message::SetAudioLowLatency(_)
                | Message::SetBluetoothEnabled(_)
                | Message::SetBluetoothAutoPower(_)
                | Message::SetPowerProfile(_)
                | Message::SetTlpEnabled(_)
                | Message::SetThermaldEnabled(_)
                | Message::GpuDetected(_)
                | Message::SetFirewallEnabled(_)
                | Message::ToggleTcpPort { .. }
                | Message::CustomTcpPortsChanged(_)
                | Message::SetSshEnabled(_)
                | Message::SetSshPort(_)
                | Message::SetSshPasswordAuth(_)
                | Message::SetSshRootLogin(_)
                | Message::SetFail2banEnabled(_)
                | Message::SetTailscaleEnabled(_)
                | Message::ToggleService { .. }
                | Message::RefreshPreview
                | Message::PreviewReady(_)
                | Message::RequestApply
                | Message::ConfirmApply
                | Message::CancelApply
                | Message::RequestDryRun
                | Message::ApplyFinished { .. }
                | Message::LoadGenerations
                | Message::GenerationsLoaded(_)
                | Message::RequestRollback { .. }
                | Message::ConfirmRollback { .. }
                | Message::RequestDeleteGeneration { .. }
                | Message::ConfirmDeleteGeneration { .. }
                | Message::RollbackToPrevious
                | Message::LoadDiskUsage
                | Message::DiskUsageLoaded(_)
                | Message::RequestMaintenance { .. }
                | Message::ConfirmMaintenance { .. }
                | Message::Helper(_) => {}
            }
        }

        let samples = [
            Message::NavSelect(Page::Onboarding),
            Message::ToggleAbout,
            Message::LaunchUrl(String::new()),
            Message::Quit,
            Message::RefreshSystem,
            Message::SystemDetected(SystemInfo::default()),
            Message::DismissToast,
            Message::DismissDialog,
            Message::UpdateConfig(crate::config::UserPreferences::default()),
            Message::ClipboardCopied { ok: true },
            Message::CopyIntegrationSnippet,
            Message::OpenEtcNixos,
            Message::VerifyIntegration,
            Message::SelectProfile(String::new()),
            Message::ClearProfile,
            Message::RefreshProfilePreview,
            Message::ToggleBundle {
                id: String::new(),
                enabled: true,
            },
            Message::ToggleBundlePackage {
                bundle_id: String::new(),
                package: String::new(),
                enabled: true,
            },
            Message::ExpandBundle {
                id: String::new(),
                expanded: true,
            },
            Message::PackageInputChanged(String::new()),
            Message::AddPackagesFromInput,
            Message::AddCustomPackages(Vec::new()),
            Message::RemoveCustomPackage(String::new()),
            Message::HostnameChanged(String::new()),
            Message::DnsServersChanged(String::new()),
            Message::UsernameChanged(String::new()),
            Message::ToggleUserGroup {
                group: String::new(),
                enabled: true,
            },
            Message::ColorSchemeChanged(ColorSchemePreference::System),
            Message::SetNvidiaDriver(None),
            Message::SetNvidiaModesetting(false),
            Message::SetNvidiaPowerManagement(false),
            Message::SetNvidiaOpen(false),
            Message::SetAudioServer(0),
            Message::SetAudioLowLatency(false),
            Message::SetBluetoothEnabled(false),
            Message::SetBluetoothAutoPower(false),
            Message::SetPowerProfile(0),
            Message::SetTlpEnabled(false),
            Message::SetThermaldEnabled(false),
            Message::GpuDetected(String::new()),
            Message::SetFirewallEnabled(true),
            Message::ToggleTcpPort {
                port: 22,
                enabled: true,
            },
            Message::CustomTcpPortsChanged(String::new()),
            Message::SetSshEnabled(false),
            Message::SetSshPort(22),
            Message::SetSshPasswordAuth(false),
            Message::SetSshRootLogin(0),
            Message::SetFail2banEnabled(false),
            Message::SetTailscaleEnabled(false),
            Message::ToggleService {
                id: String::new(),
                enabled: false,
            },
            Message::RefreshPreview,
            Message::PreviewReady(String::new()),
            Message::RequestApply,
            Message::ConfirmApply,
            Message::CancelApply,
            Message::RequestDryRun,
            Message::ApplyFinished {
                success: true,
                message: String::new(),
            },
            Message::LoadGenerations,
            Message::GenerationsLoaded(vec![dummy_generation()]),
            Message::RequestRollback { generation: 1 },
            Message::ConfirmRollback {
                generation: 1,
                mode: RollbackMode::SwitchNow,
            },
            Message::RequestDeleteGeneration { generation: 1 },
            Message::ConfirmDeleteGeneration { generation: 1 },
            Message::RollbackToPrevious,
            Message::LoadDiskUsage,
            Message::DiskUsageLoaded(dummy_disk()),
            Message::RequestMaintenance { id: String::new() },
            Message::ConfirmMaintenance { id: String::new() },
            Message::Helper(HelperEvent::Spawned {
                op: HelperOp::ReadState,
            }),
        ];
        for msg in samples {
            mention(msg);
        }
        let _ = (
            Dialog::ConfirmApply {
                heading: String::new(),
                body: String::new(),
                destructive: false,
            },
            HardwareConfig::default(),
            NetworkConfig::default(),
            ServicesConfig::default(),
        );
    }

    fn toast_text<'a>(intents: &'a [Intent], needle: &str) -> Option<&'a Intent> {
        intents
            .iter()
            .find(|intent| matches!(intent, Intent::ShowToast { text, .. } if text == needle))
    }

    fn toast_timeout(intent: &Intent) -> u64 {
        match intent {
            Intent::ShowToast { timeout_ms, .. } => *timeout_ms,
            _ => panic!("expected ShowToast"),
        }
    }

    #[test]
    fn add_packages_from_input_single_toast() {
        let (mut app, _) = AppModel::init(cosmic::Core::default(), Flags::for_tests());
        app.apply(Message::PackageInputChanged("neofetch".into()));
        let intents = app.apply(Message::AddPackagesFromInput);
        assert!(app.state.has_custom_package("neofetch"));
        assert!(app.package_input.is_empty());
        let toast = toast_text(&intents, "Added: neofetch").expect("added toast");
        assert_eq!(toast_timeout(toast), 3000);
    }

    #[test]
    fn add_packages_from_input_multiple_toast() {
        let (mut app, _) = AppModel::init(cosmic::Core::default(), Flags::for_tests());
        app.apply(Message::PackageInputChanged("neofetch, ripgrep".into()));
        let intents = app.apply(Message::AddPackagesFromInput);
        assert!(app.state.has_custom_package("neofetch"));
        assert!(app.state.has_custom_package("ripgrep"));
        let toast = toast_text(&intents, "Added 2 packages").expect("count toast");
        assert_eq!(toast_timeout(toast), 3000);
    }

    #[test]
    fn add_packages_from_input_duplicate_toast() {
        let (mut app, _) = AppModel::init(cosmic::Core::default(), Flags::for_tests());
        app.apply(Message::AddCustomPackages(vec!["neofetch".into()]));
        app.apply(Message::PackageInputChanged("neofetch".into()));
        let intents = app.apply(Message::AddPackagesFromInput);
        let toast = toast_text(&intents, "Already added: neofetch").expect("dup toast");
        assert_eq!(toast_timeout(toast), 3000);
        assert_eq!(app.state.custom_packages.len(), 1);
    }

    #[test]
    fn add_packages_from_input_in_bundle_toast_all_bundles() {
        let (mut app, _) = AppModel::init(cosmic::Core::default(), Flags::for_tests());
        app.apply(Message::PackageInputChanged("git".into()));
        let intents = app.apply(Message::AddPackagesFromInput);
        assert!(!app.state.has_custom_package("git"));
        let toast = toast_text(&intents, "'git' is already in 'Development Tools' bundle")
            .expect("in-bundle toast");
        assert_eq!(toast_timeout(toast), 4000);
    }

    #[test]
    fn add_packages_from_input_empty_is_noop() {
        let (mut app, _) = AppModel::init(cosmic::Core::default(), Flags::for_tests());
        app.apply(Message::PackageInputChanged("123invalid".into()));
        let intents = app.apply(Message::AddPackagesFromInput);
        assert!(intents.is_empty());
        assert_eq!(app.package_input, "123invalid");
    }

    #[test]
    fn quit_returns_exit() {
        let mut app = test_app();
        let intents = app.apply(Message::Quit);
        assert!(intents.iter().any(Intent::is_exit));
    }

    #[test]
    fn skip_privileged_init_does_not_read_state() {
        let flags = Flags::for_tests();
        assert!(flags.skip_privileged_on_init);
        let (_app, _task) = AppModel::init(cosmic::Core::default(), flags);
    }

    #[test]
    fn apply_select_profile_bundle_service_and_ports() {
        let mut app = test_app();
        app.apply(Message::SelectProfile("kde".into()));
        assert_eq!(app.state.selected_profile.as_deref(), Some("kde"));

        app.apply(Message::ToggleBundle {
            id: "devtools".into(),
            enabled: true,
        });
        assert!(app.state.is_bundle_enabled("devtools"));
        assert!(app
            .state
            .get_bundle_packages("devtools")
            .is_some_and(|pkgs| pkgs.contains("git")));

        app.apply(Message::SetBluetoothEnabled(true));
        assert!(app.state.bluetooth_enabled);
        assert!(app.state.hardware_config.bluetooth_enabled);

        app.apply(Message::ToggleTcpPort {
            port: 22,
            enabled: true,
        });
        app.apply(Message::CustomTcpPortsChanged("80, 443".into()));
        assert_eq!(
            app.state.network_config.allowed_tcp_ports,
            vec![22, 80, 443]
        );

        app.apply(Message::ToggleService {
            id: "printing".into(),
            enabled: true,
        });
        assert!(app.state.services_config.printing);

        app.apply(Message::AddCustomPackages(vec!["htop".into()]));
        assert!(app.state.has_custom_package("htop"));
    }

    #[test]
    fn request_apply_empty_is_destructive_confirm() {
        let mut app = test_app();
        app.state.network_config.ssh_enabled = true;
        let intents = app.apply(Message::RequestApply);
        assert!(intents.is_empty());
        match &app.dialog {
            Some(Dialog::ConfirmApply {
                heading,
                destructive: true,
                ..
            }) => assert!(heading.contains("Empty")),
            other => panic!("expected empty ConfirmApply, got {other:?}"),
        }
    }

    #[test]
    fn request_apply_with_profile_is_not_destructive() {
        let mut app = test_app();
        app.state.select_profile("gnome");
        app.apply(Message::RequestApply);
        match &app.dialog {
            Some(Dialog::ConfirmApply {
                heading,
                destructive: false,
                ..
            }) => assert!(!heading.contains("Empty")),
            other => panic!("expected non-empty ConfirmApply, got {other:?}"),
        }
    }

    #[test]
    fn request_apply_packages_only_shares_non_empty_dialog() {
        let mut app = test_app();
        app.state.add_custom_package("htop");
        app.apply(Message::RequestApply);
        match &app.dialog {
            Some(Dialog::ConfirmApply {
                destructive: false,
                heading,
                ..
            }) => assert_eq!(heading, "Apply Configuration?"),
            other => panic!("expected non-empty ConfirmApply, got {other:?}"),
        }
    }

    #[test]
    fn request_apply_ignored_when_busy() {
        let mut app = test_app();
        app.busy = Busy::Applying;
        app.apply(Message::RequestApply);
        assert!(app.dialog.is_none());
    }

    #[test]
    fn confirm_and_cancel_apply_dismiss_dialog_without_spawn() {
        let mut app = test_app();
        app.apply(Message::RequestApply);
        assert!(app.dialog.is_some());
        let intents = app.apply(Message::ConfirmApply);
        assert!(app.dialog.is_none());
        assert!(!intents
            .iter()
            .any(|intent| matches!(intent, Intent::SpawnHelper { .. })));

        app.apply(Message::RequestApply);
        let intents = app.apply(Message::CancelApply);
        assert!(app.dialog.is_none());
        assert!(intents.is_empty());
    }

    #[test]
    fn helper_state_log_and_apply_complete() {
        let mut app = test_app();
        app.state.has_changes = true;

        let ipc = IpcAppState {
            selected_profile: Some("xfce".into()),
            ..IpcAppState::default()
        };
        app.apply(Message::Helper(HelperEvent::Response {
            op: HelperOp::ReadState,
            response: Box::new(HelperResponse::State(ipc)),
        }));
        assert_eq!(app.state.selected_profile.as_deref(), Some("xfce"));

        app.apply(Message::Helper(HelperEvent::Response {
            op: HelperOp::Apply {
                rebuild: RebuildType::Switch,
                then_write_state: true,
            },
            response: Box::new(HelperResponse::Log {
                level: LogLevel::Info,
                message: "building".into(),
            }),
        }));
        assert!(app.apply_log.contains("building"));

        app.state.has_changes = true;
        let intents = app.apply(Message::Helper(HelperEvent::Response {
            op: HelperOp::Apply {
                rebuild: RebuildType::Switch,
                then_write_state: true,
            },
            response: Box::new(HelperResponse::ApplyComplete {
                success: true,
                message: "done".into(),
            }),
        }));
        assert!(!app.state.has_changes);
        assert!(intents.iter().any(|intent| matches!(
            intent,
            Intent::ShowToast { text, .. } if text == "done"
        )));
    }
}

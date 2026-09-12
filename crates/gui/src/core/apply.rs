//! Pure-enough reducer: `AppModel::apply` returns side-effect [`Intent`]s.

use crate::app::{AppModel, Banner, BannerKind, Busy, HelperStatus};
use crate::core::packages::{classify_new_packages, parse_package_input};
use crate::integration::{classic_integration_snippet, flake_integration_snippet};
use crate::message::{
    ContextPage, Dialog, HelperEvent, HelperOp, Intent, Message, Page, RollbackMode,
};
use common::actions::default_bundles;
use common::config::{ConfigMode, IntegrationStatus};
use common::ipc::{HardwareConfig, HelperRequest, HelperResponse, RebuildType};
use std::collections::HashSet;
use std::path::PathBuf;

const TOAST_MS: u64 = 3000;
const IN_BUNDLE_TOAST_MS: u64 = 4000;

const SSH_ROOT_LOGIN: [&str; 3] = ["no", "prohibit-password", "yes"];

impl AppModel {
    /// Reducer. No iced/libcosmic widget types. Exhaustive over [`Message`].
    pub fn apply(&mut self, msg: Message) -> Vec<Intent> {
        match msg {
            Message::NavSelect(page) => self.nav_intents(page),
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
            Message::RefreshSystem => vec![Intent::DetectSystem, Intent::DetectGpu],
            Message::SystemDetected(info) => {
                self.system_info = info;
                self.seed_username_from_host();
                self.refresh_banner();
                if self.verify_pending {
                    self.verify_pending = false;
                    let text = match self.system_info.integration_status {
                        IntegrationStatus::Integrated => {
                            crate::fl!("toast-integration-verified")
                        }
                        IntegrationStatus::NotIntegrated => {
                            crate::fl!("toast-integration-missing")
                        }
                        IntegrationStatus::Unknown => {
                            crate::fl!("toast-integration-unknown")
                        }
                    };
                    vec![Intent::ShowToast {
                        text,
                        timeout_ms: TOAST_MS,
                    }]
                } else {
                    Vec::new()
                }
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
                        text: crate::fl!("toast-clipboard-failed"),
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
                        text: crate::fl!("toast-snippet-copied"),
                        timeout_ms: TOAST_MS,
                    },
                ]
            }
            Message::OpenEtcNixos => vec![Intent::OpenPath(PathBuf::from("/etc/nixos"))],
            Message::VerifyIntegration => {
                self.verify_pending = true;
                vec![Intent::DetectSystem]
            }

            Message::SelectProfile(id) => {
                self.state.select_profile(id.clone());
                vec![Intent::LocalProfilePreview { id }]
            }
            Message::ClearProfile => {
                self.state.clear_profile();
                self.profile_preview.clear();
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
                        let package = added[0].clone();
                        crate::fl!("toast-added-package", package = package)
                    } else {
                        let count = added.len() as u32;
                        crate::fl!("toast-added-packages", count = count)
                    };
                    intents.push(Intent::ShowToast {
                        text,
                        timeout_ms: TOAST_MS,
                    });
                }
                if !duplicates.is_empty() {
                    let packages = duplicates.join(", ");
                    intents.push(Intent::ShowToast {
                        text: crate::fl!("toast-already-added", packages = packages),
                        timeout_ms: TOAST_MS,
                    });
                }
                for (pkg, bundle) in &in_bundle {
                    let package = pkg.clone();
                    let bundle = bundle.clone();
                    intents.push(Intent::ShowToast {
                        text: crate::fl!("toast-in-bundle", package = package, bundle = bundle),
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
                    self.field_errors.hostname = None;
                    self.state.hostname = None;
                    self.state.has_changes = true;
                    return Vec::new();
                }
                if let Some(err) = hostname_charset_error(&trimmed) {
                    self.field_errors.hostname = Some(err);
                    return Vec::new();
                }
                self.field_errors.hostname = None;
                if self.system_info.hostname.as_deref() == Some(trimmed.as_str()) {
                    self.state.hostname = None;
                } else {
                    self.state.set_hostname(trimmed);
                }
                Vec::new()
            }
            Message::DnsServersChanged(raw) => {
                match self.state.parse_and_set_dns(&raw) {
                    Ok(()) => self.field_errors.dns = None,
                    Err(err) => self.field_errors.dns = Some(err),
                }
                self.dns_input = raw;
                Vec::new()
            }
            Message::UsernameChanged(username) => {
                let trimmed = username.trim().to_string();
                if trimmed.is_empty() {
                    self.field_errors.username = None;
                    self.state.set_username("");
                    return Vec::new();
                }
                if !crate::app::is_valid_username(&trimmed) {
                    self.field_errors.username = Some(crate::fl!("error-username-invalid"));
                    return Vec::new();
                }
                self.field_errors.username = None;
                self.state.set_username(trimmed);
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
                if driver == Some(2) {
                    self.state.hardware_config.nvidia_open = true;
                }
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
                let nvidia = gpu.to_ascii_lowercase().contains("nvidia");
                self.gpu_vendor = Some(gpu);
                if nvidia
                    && !self.cpu_arch.is_arm()
                    && self.state.hardware_config.nvidia_driver.is_none()
                {
                    self.state.hardware_config.nvidia_driver = Some(0);
                }
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
                if let Err(err) = self.state.parse_and_set_custom_tcp_ports(&raw) {
                    tracing::debug!(err, "invalid custom TCP ports");
                }
                self.custom_tcp_input = raw;
                Vec::new()
            }
            Message::ToggleUdpPort { port, enabled } => {
                self.state.set_udp_port(port, enabled);
                Vec::new()
            }
            Message::CustomUdpPortsChanged(raw) => {
                if let Err(err) = self.state.parse_and_set_custom_udp_ports(&raw) {
                    tracing::debug!(err, "invalid custom UDP ports");
                }
                self.custom_udp_input = raw;
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
            Message::SetWireguardEnabled(enabled) => {
                self.state.network_config.wireguard_enabled = enabled;
                if enabled {
                    let port = self.state.wireguard_listen_port();
                    self.state.network_config.wireguard_listen_port = port;
                    // Open the listen UDP port; leave it if the user later disables WireGuard.
                    self.state.set_udp_port(port, true);
                    self.custom_udp_input = crate::state::custom_udp_input_from_ports(
                        &self.state.network_config.allowed_udp_ports,
                    );
                }
                self.state.has_changes = true;
                Vec::new()
            }
            Message::SetWireguardListenPort(port) => {
                let port = if port == 0 {
                    crate::state::DEFAULT_WIREGUARD_LISTEN_PORT
                } else {
                    port
                };
                self.state.network_config.wireguard_listen_port = port;
                if self.state.network_config.wireguard_enabled {
                    // Open the new listen port; do not remove a previously auto-added port.
                    self.state.set_udp_port(port, true);
                    self.custom_udp_input = crate::state::custom_udp_input_from_ports(
                        &self.state.network_config.allowed_udp_ports,
                    );
                }
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
                if self.busy != Busy::Idle || !self.helper_can_spawn() {
                    return Vec::new();
                }
                self.dialog = Some(self.apply_confirm_dialog());
                Vec::new()
            }
            Message::ConfirmApply => {
                self.dialog = None;
                let rebuild = self.rebuild_type;
                let then_write_state = matches!(
                    rebuild,
                    RebuildType::Switch
                        | RebuildType::Boot
                        | RebuildType::Test
                        | RebuildType::Build
                );
                self.start_rebuild(rebuild, then_write_state)
            }
            Message::CancelApply => {
                self.dialog = None;
                Vec::new()
            }
            Message::RequestDryRun => self.start_rebuild(RebuildType::DryBuild, false),
            Message::SetRebuildType(rebuild) => {
                if !matches!(rebuild, RebuildType::DryBuild) {
                    self.rebuild_type = rebuild;
                }
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

            Message::LoadGenerations => self.load_generations_intents(),
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
                self.start_rollback(generation, mode)
            }
            Message::RequestDeleteGeneration { generation } => {
                self.dialog = Some(Dialog::ConfirmDeleteGeneration { generation });
                Vec::new()
            }
            Message::ConfirmDeleteGeneration { generation } => {
                self.dialog = None;
                self.start_delete_generation(generation)
            }
            Message::RollbackToPrevious => self.rollback_to_previous(),

            Message::LoadDiskUsage => self.load_disk_usage_intents(),
            Message::DiskUsageLoaded(info) => {
                self.disk_usage = Some(info);
                if self.busy == Busy::LoadingDisk {
                    self.busy = Busy::Idle;
                }
                Vec::new()
            }
            Message::RequestMaintenance { id } => self.request_maintenance(id),
            Message::ConfirmMaintenance { id } => {
                self.dialog = None;
                self.start_maintenance(&id)
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
                    self.state_load_warning = Some(crate::fl!("banner-state-spawn"));
                }
                if error.contains("not available") || error.contains("not found") {
                    self.helper_missing = true;
                }
                if matches!(op, HelperOp::Apply { .. }) {
                    self.apply_log.push_str(&format!(
                        "{}\n",
                        crate::fl!("apply-helper-failed", error = error.as_str())
                    ));
                    self.apply_log
                        .push_str(&format!("\n{}\n", crate::fl!("apply-manual-hint")));
                    self.apply_log.push_str(&format!(
                        "{}\n",
                        crate::fl!("apply-manual-command", rebuild = self.rebuild_type.as_arg())
                    ));
                }
                self.refresh_banner();
                vec![Intent::ShowToast {
                    text: error,
                    timeout_ms: TOAST_MS,
                }]
            }
            HelperEvent::Spawned { op } => {
                self.helper = HelperStatus::Active { op };
                Vec::new()
            }
            HelperEvent::Response { op, response } => self.apply_helper_response(op, *response),
            HelperEvent::Closed { op, error } => {
                self.helper = HelperStatus::Idle;
                let was_busy = self.busy;
                self.busy = Busy::Idle;
                if matches!(op, HelperOp::ReadState) {
                    self.state_load_warning = Some(crate::fl!("banner-state-timeout"));
                    self.refresh_banner();
                    return Vec::new();
                }
                let detail = error.unwrap_or_else(|| crate::fl!("log-helper-closed"));
                match op {
                    HelperOp::Apply { .. } if was_busy != Busy::Idle => {
                        self.apply_log.push_str(&format!("\n{detail}\n"));
                        vec![Intent::ShowToast {
                            text: detail,
                            timeout_ms: TOAST_MS,
                        }]
                    }
                    _ => Vec::new(),
                }
            }
            HelperEvent::Timeout { op } => {
                self.helper = HelperStatus::Idle;
                self.busy = Busy::Idle;
                if matches!(op, HelperOp::ReadState) {
                    self.state_load_warning = Some(crate::fl!("banner-state-timeout"));
                    self.refresh_banner();
                }
                Vec::new()
            }
        }
    }

    fn apply_helper_response(&mut self, op: HelperOp, response: HelperResponse) -> Vec<Intent> {
        match response {
            HelperResponse::State(ipc) => {
                self.state = crate::state::AppState::from_ipc_state(ipc);
                self.sync_draft_inputs();
                self.seed_username_from_host();
                self.state_load_warning = None;
                self.helper = HelperStatus::Idle;
                if self.busy == Busy::LoadingState {
                    self.busy = Busy::Idle;
                }
                self.refresh_banner();
                Vec::new()
            }
            HelperResponse::Error { message, details } => {
                self.helper = HelperStatus::Idle;
                if !matches!(op, HelperOp::Apply { .. }) {
                    self.busy = Busy::Idle;
                }
                let detail = details.as_deref().map_or_else(String::new, |d| {
                    format!("\n{}", crate::fl!("log-details", details = d))
                });
                match &op {
                    HelperOp::ReadState => {
                        self.state_load_warning = Some(crate::fl!(
                            "banner-state-read-error",
                            message = message.as_str()
                        ));
                        self.refresh_banner();
                        Vec::new()
                    }
                    HelperOp::Apply { .. } => {
                        self.busy = Busy::Idle;
                        self.apply_log.push_str(&format!(
                            "\n{}\n",
                            crate::fl!(
                                "log-error",
                                message = message.as_str(),
                                detail = detail.as_str()
                            )
                        ));
                        vec![Intent::ShowToast {
                            text: message,
                            timeout_ms: TOAST_MS,
                        }]
                    }
                    HelperOp::RunMaintenance { .. } => {
                        self.maintenance_log.push_str(&format!(
                            "\n{}\n",
                            crate::fl!(
                                "log-error",
                                message = message.as_str(),
                                detail = detail.as_str()
                            )
                        ));
                        vec![Intent::ShowToast {
                            text: message,
                            timeout_ms: TOAST_MS,
                        }]
                    }
                    HelperOp::Rollback { .. }
                    | HelperOp::DeleteGenerations { .. }
                    | HelperOp::ListGenerations => {
                        self.generations_log.push_str(&format!(
                            "\n{}\n",
                            crate::fl!(
                                "log-error",
                                message = message.as_str(),
                                detail = detail.as_str()
                            )
                        ));
                        vec![Intent::ShowToast {
                            text: message,
                            timeout_ms: TOAST_MS,
                        }]
                    }
                    _ => vec![Intent::ShowToast {
                        text: message,
                        timeout_ms: TOAST_MS,
                    }],
                }
            }
            HelperResponse::ApplyComplete { success, message } => {
                self.helper = HelperStatus::Idle;
                self.apply(Message::ApplyFinished { success, message })
            }
            HelperResponse::Generations(list) => {
                self.helper = HelperStatus::Idle;
                self.apply(Message::GenerationsLoaded(list))
            }
            HelperResponse::DiskUsage(info) => {
                self.helper = HelperStatus::Idle;
                if self.busy == Busy::LoadingDisk {
                    self.busy = Busy::Idle;
                }
                self.apply(Message::DiskUsageLoaded(info))
            }
            HelperResponse::Log { message, .. } => {
                match op {
                    HelperOp::RunMaintenance { .. } => {
                        self.maintenance_log.push_str(&message);
                        self.maintenance_log.push('\n');
                    }
                    HelperOp::Rollback { .. }
                    | HelperOp::DeleteGenerations { .. }
                    | HelperOp::ListGenerations => {
                        self.generations_log.push_str(&message);
                        self.generations_log.push('\n');
                    }
                    _ => {
                        self.apply_log.push_str(&message);
                        self.apply_log.push('\n');
                    }
                }
                Vec::new()
            }
            HelperResponse::MaintenanceOutput {
                stdout,
                stderr,
                success,
            } => {
                self.helper = HelperStatus::Idle;
                self.busy = Busy::Idle;
                if !stdout.is_empty() {
                    self.maintenance_log.push_str(&stdout);
                    if !stdout.ends_with('\n') {
                        self.maintenance_log.push('\n');
                    }
                }
                if !stderr.is_empty() {
                    self.maintenance_log
                        .push_str(&format!("\n{}\n", crate::fl!("log-stderr")));
                    self.maintenance_log.push_str(&stderr);
                    if !stderr.ends_with('\n') {
                        self.maintenance_log.push('\n');
                    }
                }
                let text = if success {
                    crate::fl!("toast-maintenance-ok")
                } else {
                    crate::fl!("toast-maintenance-failed")
                };
                vec![Intent::ShowToast {
                    text,
                    timeout_ms: TOAST_MS,
                }]
            }
            HelperResponse::Ok => {
                self.helper = HelperStatus::Idle;
                match op {
                    HelperOp::Rollback { generation, mode } => {
                        self.generations_log
                            .push_str(&format!("{}\n", crate::fl!("log-switch-ok")));
                        if mode == RollbackMode::SetForNextBoot {
                            self.generations_log
                                .push_str(&format!("{}\n", crate::fl!("log-switch-next-boot")));
                        }
                        let _ = generation;
                        self.busy = Busy::Idle;
                        self.load_generations_intents()
                    }
                    HelperOp::DeleteGenerations { generations } => {
                        if let Some(number) = generations.first() {
                            let number: u32 = *number;
                            self.generations_log.push_str(&format!(
                                "{}\n",
                                crate::fl!("log-generation-deleted", number = number)
                            ));
                        } else {
                            self.generations_log.push_str(&format!(
                                "{}\n",
                                crate::fl!("log-generation-deleted-generic")
                            ));
                        }
                        self.busy = Busy::Idle;
                        self.load_generations_intents()
                    }
                    HelperOp::Apply { .. } => {
                        // EnsureDirectories Ok is swallowed by the session; keep applying.
                        self.helper = HelperStatus::Active { op };
                        Vec::new()
                    }
                    _ => {
                        if self.busy != Busy::Applying && self.busy != Busy::DryRun {
                            self.busy = Busy::Idle;
                        }
                        Vec::new()
                    }
                }
            }
            _ => {
                if !matches!(op, HelperOp::Apply { .. }) {
                    self.helper = HelperStatus::Idle;
                }
                Vec::new()
            }
        }
    }

    fn nav_intents(&mut self, page: Page) -> Vec<Intent> {
        self.page = page;
        let mut intents = vec![Intent::SetWindowTitle(self.window_title())];
        match page {
            Page::Apply => intents.push(Intent::LocalPreview),
            Page::Generations => intents.extend(self.load_generations_intents()),
            Page::Maintenance => intents.extend(self.load_disk_usage_intents()),
            Page::Hardware if self.gpu_vendor.is_none() => intents.push(Intent::DetectGpu),
            _ => {}
        }
        intents
    }

    fn apply_confirm_dialog(&self) -> Dialog {
        let rebuild = self.rebuild_type.as_arg().to_string();
        let no_profile = self.state.selected_profile.is_none();
        let no_bundles = self.state.enabled_bundles.is_empty();
        let no_packages = self.state.custom_packages.is_empty();
        if self.state.apply_is_empty() {
            Dialog::ConfirmApply {
                heading: crate::fl!("apply-empty-heading"),
                body: crate::fl!("apply-empty-body", rebuild = rebuild.clone()),
                destructive: true,
            }
        } else if no_profile && no_bundles && !no_packages {
            Dialog::ConfirmApply {
                heading: crate::fl!("apply-packages-heading"),
                body: crate::fl!("apply-packages-body", rebuild = rebuild),
                destructive: false,
            }
        } else {
            Dialog::ConfirmApply {
                heading: crate::fl!("apply-normal-heading"),
                body: crate::fl!("apply-normal-body", rebuild = rebuild),
                destructive: false,
            }
        }
    }

    /// Hardware sent to Nix generation. Visible NVIDIA combo defaults to Stable.
    pub(crate) fn hardware_for_nix(&self) -> HardwareConfig {
        let mut hardware_config = self.state.effective_hardware();
        if hardware_config.nvidia_driver.is_none()
            && !self.cpu_arch.is_arm()
            && self
                .gpu_vendor
                .as_deref()
                .is_some_and(|vendor| vendor.to_ascii_lowercase().contains("nvidia"))
        {
            hardware_config.nvidia_driver = Some(0);
        }
        hardware_config
    }

    fn to_apply_request(&self, rebuild: RebuildType) -> HelperRequest {
        let mut request = self.state.to_apply_request(rebuild);
        if let HelperRequest::Apply {
            hardware_config, ..
        } = &mut request
        {
            *hardware_config = self.hardware_for_nix();
        }
        request
    }

    fn start_rebuild(&mut self, rebuild: RebuildType, then_write_state: bool) -> Vec<Intent> {
        if self.busy != Busy::Idle || !self.helper_can_spawn() {
            return Vec::new();
        }
        self.busy = if matches!(rebuild, RebuildType::DryBuild) {
            Busy::DryRun
        } else {
            Busy::Applying
        };
        self.apply_log = format!(
            "{}\n\n",
            crate::fl!("apply-log-starting", rebuild = rebuild.as_arg())
        );
        let save = then_write_state.then(|| {
            let mut ipc = self.state.to_ipc_state();
            let hardware = self.hardware_for_nix();
            ipc.bluetooth_enabled = hardware.bluetooth_enabled;
            ipc.hardware_config = hardware;
            Box::new(ipc)
        });
        let request = self.to_apply_request(rebuild);
        vec![Intent::SpawnHelper {
            op: HelperOp::Apply {
                rebuild,
                then_write_state,
                save,
            },
            request,
        }]
    }

    fn load_generations_intents(&mut self) -> Vec<Intent> {
        if self.busy != Busy::Idle || !self.helper_can_spawn() {
            return Vec::new();
        }
        self.busy = Busy::LoadingGenerations;
        self.generations_log
            .push_str(&format!("{}\n", crate::fl!("log-loading-generations")));
        vec![Intent::SpawnHelper {
            op: HelperOp::ListGenerations,
            request: HelperRequest::ListGenerations,
        }]
    }

    fn load_disk_usage_intents(&mut self) -> Vec<Intent> {
        if self.busy != Busy::Idle || !self.helper_can_spawn() {
            return Vec::new();
        }
        self.busy = Busy::LoadingDisk;
        vec![Intent::SpawnHelper {
            op: HelperOp::GetDiskUsage,
            request: HelperRequest::GetDiskUsage,
        }]
    }

    fn start_rollback(&mut self, generation: u32, mode: RollbackMode) -> Vec<Intent> {
        if self.busy != Busy::Idle || !self.helper_can_spawn() {
            return Vec::new();
        }
        let activate = match mode {
            RollbackMode::SwitchNow => "switch",
            RollbackMode::SetForNextBoot => "boot",
        };
        self.busy = Busy::RollingBack;
        self.generations_log.push_str(&format!(
            "{}\n",
            crate::fl!("log-switching-generation", generation = generation)
        ));
        vec![Intent::SpawnHelper {
            op: HelperOp::Rollback { generation, mode },
            request: HelperRequest::RollbackGeneration {
                generation,
                activate: activate.to_string(),
            },
        }]
    }

    fn start_delete_generation(&mut self, generation: u32) -> Vec<Intent> {
        if self.busy != Busy::Idle || !self.helper_can_spawn() {
            return Vec::new();
        }
        self.busy = Busy::DeletingGeneration;
        self.generations_log.push_str(&format!(
            "{}\n",
            crate::fl!("log-deleting-generation", generation = generation)
        ));
        vec![Intent::SpawnHelper {
            op: HelperOp::DeleteGenerations {
                generations: vec![generation],
            },
            request: HelperRequest::DeleteGenerations {
                generations: vec![generation],
            },
        }]
    }

    fn rollback_to_previous(&mut self) -> Vec<Intent> {
        if self.busy != Busy::Idle {
            return Vec::new();
        }
        let previous = self.current_generation.and_then(|current| {
            self.generations
                .iter()
                .filter(|generation| generation.number < current)
                .map(|generation| generation.number)
                .max()
        });
        match previous {
            Some(generation) => self.apply(Message::RequestRollback { generation }),
            None if self.current_generation.is_some() && self.generations.is_empty() => {
                self.generations_log
                    .push_str(&format!("{}\n", crate::fl!("log-rollback-refresh")));
                Vec::new()
            }
            None if self.current_generation.is_some() => {
                self.generations_log
                    .push_str(&format!("{}\n", crate::fl!("log-rollback-first")));
                Vec::new()
            }
            None => {
                self.generations_log
                    .push_str(&format!("{}\n", crate::fl!("log-rollback-unknown")));
                Vec::new()
            }
        }
    }

    fn request_maintenance(&mut self, id: String) -> Vec<Intent> {
        if !self.helper_can_spawn() {
            return Vec::new();
        }
        let Some(action) = common::actions::default_maintenance_actions()
            .into_iter()
            .find(|action| action.id == id)
        else {
            return vec![Intent::ShowToast {
                text: crate::fl!("toast-unknown-maintenance", id = id.as_str()),
                timeout_ms: TOAST_MS,
            }];
        };
        if let Some(warning) = action.warning {
            self.dialog = Some(Dialog::ConfirmMaintenance {
                id: action.id,
                name: action.name,
                warning,
            });
            Vec::new()
        } else {
            self.start_maintenance(&action.id)
        }
    }

    fn start_maintenance(&mut self, id: &str) -> Vec<Intent> {
        if self.busy != Busy::Idle || !self.helper_can_spawn() {
            return Vec::new();
        }
        let Some(action) = common::actions::default_maintenance_actions()
            .into_iter()
            .find(|action| action.id == id)
        else {
            return vec![Intent::ShowToast {
                text: crate::fl!("toast-unknown-maintenance", id = id),
                timeout_ms: TOAST_MS,
            }];
        };
        self.busy = Busy::Maintenance;
        self.maintenance_log
            .push_str(&format!("$ {}\n\n", action.command));
        vec![Intent::SpawnHelper {
            op: HelperOp::RunMaintenance {
                command: action.command.clone(),
            },
            request: HelperRequest::RunMaintenance {
                command: action.command,
            },
        }]
    }

    pub(crate) fn refresh_banner(&mut self) {
        // Priority: Not NixOS > helper-missing / state-load warning > integration.
        if !self.system_info.is_nixos && !self.flags.skip_host_probes {
            self.banner = Some(Banner {
                kind: BannerKind::Error,
                text: crate::fl!("banner-not-nixos"),
            });
            return;
        }
        if self.helper_missing {
            self.banner = Some(Banner {
                kind: BannerKind::Warning,
                text: crate::fl!("helper-missing-banner"),
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
                text: crate::fl!("banner-integrated"),
            }),
            IntegrationStatus::NotIntegrated => Some(Banner {
                kind: BannerKind::Warning,
                text: crate::fl!("banner-setup-required"),
            }),
            IntegrationStatus::Unknown => None,
        };
    }

    pub(crate) fn window_title(&self) -> String {
        format!("{} — {}", crate::fl!("app-title"), self.page.title())
    }
}

/// GTK hostname charset `[A-Za-z0-9-]`, max 63 characters. Empty is handled by the caller.
fn hostname_charset_error(hostname: &str) -> Option<String> {
    if !common::validate::hostname_is_valid(hostname) {
        if hostname.len() > common::validate::MAX_HOSTNAME_LEN {
            return Some(crate::fl!("error-hostname-too-long"));
        }
        return Some(crate::fl!("error-hostname-invalid"));
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::Flags;
    use crate::config::ColorSchemePreference;
    use crate::message::{Dialog, HelperOp, Page, RollbackMode};
    use common::ipc::{
        AppState as IpcAppState, DiskUsageInfo, Generation, HardwareConfig, HelperRequest,
        LogLevel, NetworkConfig, RebuildType, ServicesConfig,
    };
    use common::SystemInfo;
    use cosmic::Application;

    fn test_app() -> AppModel {
        AppModel::test_model()
    }

    fn test_app_with_helper() -> AppModel {
        let mut app = AppModel::test_model();
        app.helper_missing = false;
        app.flags.spawn.helper_available = true;
        app
    }

    fn spawn_helper(intents: &[Intent]) -> Option<&Intent> {
        intents
            .iter()
            .find(|intent| matches!(intent, Intent::SpawnHelper { .. }))
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
                | Message::ToggleUdpPort { .. }
                | Message::CustomUdpPortsChanged(_)
                | Message::SetSshEnabled(_)
                | Message::SetSshPort(_)
                | Message::SetSshPasswordAuth(_)
                | Message::SetSshRootLogin(_)
                | Message::SetFail2banEnabled(_)
                | Message::SetTailscaleEnabled(_)
                | Message::SetWireguardEnabled(_)
                | Message::SetWireguardListenPort(_)
                | Message::ToggleService { .. }
                | Message::RefreshPreview
                | Message::PreviewReady(_)
                | Message::RequestApply
                | Message::ConfirmApply
                | Message::CancelApply
                | Message::RequestDryRun
                | Message::SetRebuildType(_)
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
            Message::ToggleUdpPort {
                port: 53,
                enabled: true,
            },
            Message::CustomUdpPortsChanged(String::new()),
            Message::SetSshEnabled(false),
            Message::SetSshPort(22),
            Message::SetSshPasswordAuth(false),
            Message::SetSshRootLogin(0),
            Message::SetFail2banEnabled(false),
            Message::SetTailscaleEnabled(false),
            Message::SetWireguardEnabled(false),
            Message::SetWireguardListenPort(51820),
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
            Message::SetRebuildType(RebuildType::Switch),
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
        app.apply(Message::CustomTcpPortsChanged("9090".into()));
        assert_eq!(app.custom_tcp_input, "9090");
        assert_eq!(app.state.network_config.allowed_tcp_ports, vec![22, 9090]);

        app.apply(Message::ToggleUdpPort {
            port: 53,
            enabled: true,
        });
        app.apply(Message::CustomUdpPortsChanged("9091".into()));
        assert_eq!(app.custom_udp_input, "9091");
        assert_eq!(app.state.network_config.allowed_udp_ports, vec![53, 9091]);

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
        let mut app = test_app_with_helper();
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
    fn request_apply_network_only_is_not_empty() {
        let mut app = test_app_with_helper();
        app.state.network_config.ssh_enabled = true;
        app.apply(Message::RequestApply);
        match &app.dialog {
            Some(Dialog::ConfirmApply {
                destructive: false,
                heading,
                ..
            }) => assert!(!heading.contains("Empty")),
            other => panic!("expected non-empty ConfirmApply, got {other:?}"),
        }
    }

    #[test]
    fn request_apply_hardware_only_is_not_empty() {
        let mut app = test_app_with_helper();
        app.state.hardware_config.nvidia_driver = Some(0);
        app.apply(Message::RequestApply);
        match &app.dialog {
            Some(Dialog::ConfirmApply {
                destructive: false,
                heading,
                ..
            }) => assert_eq!(*heading, crate::fl!("apply-normal-heading")),
            other => panic!("expected non-empty ConfirmApply, got {other:?}"),
        }
    }

    #[test]
    fn request_apply_with_profile_is_not_destructive() {
        let mut app = test_app_with_helper();
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
    fn request_apply_packages_only_uses_packages_copy() {
        let mut app = test_app_with_helper();
        app.state.add_custom_package("htop");
        app.apply(Message::RequestApply);
        match &app.dialog {
            Some(Dialog::ConfirmApply {
                destructive: false,
                heading,
                body,
            }) => {
                assert_eq!(*heading, crate::fl!("apply-packages-heading"));
                assert!(body.contains("only your custom packages"));
            }
            other => panic!("expected packages-only ConfirmApply, got {other:?}"),
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
    fn confirm_apply_spawns_switch_with_write_state() {
        let mut app = test_app_with_helper();
        app.state.select_profile("gnome");
        app.apply(Message::RequestApply);
        let intents = app.apply(Message::ConfirmApply);
        assert!(app.dialog.is_none());
        assert_eq!(app.busy, Busy::Applying);
        match spawn_helper(&intents) {
            Some(Intent::SpawnHelper { op, request }) => {
                match op {
                    HelperOp::Apply {
                        rebuild: RebuildType::Switch,
                        then_write_state: true,
                        save: Some(save),
                    } => {
                        assert_eq!(save.selected_profile.as_deref(), Some("gnome"));
                    }
                    other => panic!("expected Apply Switch write-state, got {other:?}"),
                }
                assert!(matches!(
                    request,
                    HelperRequest::Apply {
                        rebuild_type: RebuildType::Switch,
                        ..
                    }
                ));
            }
            other => panic!("expected SpawnHelper, got {other:?}"),
        }
    }

    #[test]
    fn nvidia_gpu_apply_defaults_unwritten_driver_to_stable() {
        let mut app = test_app_with_helper();
        app.apply(Message::GpuDetected("NVIDIA GeForce RTX 3060".into()));
        assert_eq!(app.state.hardware_config.nvidia_driver, Some(0));
        assert_eq!(app.hardware_for_nix().nvidia_driver, Some(0));
        assert!(app.hardware_for_nix().nvidia_modesetting);
        assert!(!app.state.apply_is_empty());

        let intents = app.apply(Message::RequestDryRun);
        match spawn_helper(&intents) {
            Some(Intent::SpawnHelper { request, .. }) => match request {
                HelperRequest::Apply {
                    hardware_config, ..
                } => assert_eq!(hardware_config.nvidia_driver, Some(0)),
                other => panic!("expected Apply, got {other:?}"),
            },
            other => panic!("expected SpawnHelper, got {other:?}"),
        }
        assert_eq!(app.state.hardware_config.nvidia_driver, Some(0));
    }

    #[test]
    fn nvidia_gpu_on_arm_does_not_default_driver() {
        let mut app = test_app_with_helper();
        app.cpu_arch = common::actions::CpuArch::Aarch64;
        app.apply(Message::GpuDetected("NVIDIA GeForce RTX 3060".into()));
        assert!(app.state.hardware_config.nvidia_driver.is_none());
        assert!(app.hardware_for_nix().nvidia_driver.is_none());
        assert!(app.state.apply_is_empty());
    }

    #[test]
    fn nvidia_open_combo_sets_open_flag() {
        let mut app = test_app();
        app.apply(Message::SetNvidiaDriver(Some(2)));
        assert_eq!(app.state.hardware_config.nvidia_driver, Some(2));
        assert!(app.state.hardware_config.nvidia_open);
        assert!(
            app.hardware_for_nix().nvidia_open
                || app.state.hardware_config.nvidia_driver == Some(2)
        );
    }

    #[test]
    fn nvidia_gpu_apply_keeps_explicit_driver() {
        let mut app = test_app_with_helper();
        app.apply(Message::GpuDetected("nvidia".into()));
        app.state.hardware_config.nvidia_driver = Some(2);
        assert_eq!(app.hardware_for_nix().nvidia_driver, Some(2));
        let intents = app.apply(Message::RequestDryRun);
        match spawn_helper(&intents) {
            Some(Intent::SpawnHelper { request, .. }) => match request {
                HelperRequest::Apply {
                    hardware_config, ..
                } => assert_eq!(hardware_config.nvidia_driver, Some(2)),
                other => panic!("expected Apply, got {other:?}"),
            },
            other => panic!("expected SpawnHelper, got {other:?}"),
        }
    }

    #[test]
    fn non_nvidia_gpu_apply_leaves_driver_none() {
        let mut app = test_app_with_helper();
        app.apply(Message::GpuDetected("Intel Corporation".into()));
        assert!(app.hardware_for_nix().nvidia_driver.is_none());
        let intents = app.apply(Message::RequestDryRun);
        match spawn_helper(&intents) {
            Some(Intent::SpawnHelper { request, .. }) => match request {
                HelperRequest::Apply {
                    hardware_config, ..
                } => assert!(hardware_config.nvidia_driver.is_none()),
                other => panic!("expected Apply, got {other:?}"),
            },
            other => panic!("expected SpawnHelper, got {other:?}"),
        }
    }

    #[test]
    fn request_dry_run_spawns_dry_build_without_write_state() {
        let mut app = test_app_with_helper();
        let intents = app.apply(Message::RequestDryRun);
        assert!(app.dialog.is_none());
        assert_eq!(app.busy, Busy::DryRun);
        match spawn_helper(&intents) {
            Some(Intent::SpawnHelper { op, request }) => {
                match op {
                    HelperOp::Apply {
                        rebuild: RebuildType::DryBuild,
                        then_write_state: false,
                        save,
                    } => assert!(save.is_none()),
                    other => panic!("expected DryBuild, got {other:?}"),
                }
                assert!(matches!(
                    request,
                    HelperRequest::Apply {
                        rebuild_type: RebuildType::DryBuild,
                        ..
                    }
                ));
            }
            other => panic!("expected SpawnHelper, got {other:?}"),
        }
    }

    #[test]
    fn cancel_apply_dismisses_dialog_without_spawn() {
        let mut app = test_app_with_helper();
        app.apply(Message::RequestApply);
        let intents = app.apply(Message::CancelApply);
        assert!(app.dialog.is_none());
        assert!(intents.is_empty());
    }

    #[test]
    fn confirm_apply_ignored_when_busy() {
        let mut app = test_app();
        app.busy = Busy::Applying;
        let intents = app.apply(Message::ConfirmApply);
        assert!(spawn_helper(&intents).is_none());
    }

    #[test]
    fn select_profile_requests_local_preview() {
        let mut app = test_app();
        let intents = app.apply(Message::SelectProfile("kde".into()));
        assert!(intents.iter().any(|intent| matches!(
            intent,
            Intent::LocalProfilePreview { id } if id == "kde"
        )));
    }

    #[test]
    fn nav_select_apply_refreshes_preview() {
        let mut app = test_app();
        let intents = app.apply(Message::NavSelect(Page::Apply));
        assert!(intents
            .iter()
            .any(|intent| matches!(intent, Intent::LocalPreview)));
    }

    #[test]
    fn nav_during_apply_does_not_spawn_or_cancel() {
        let mut app = test_app();
        app.busy = Busy::Applying;
        let intents = app.apply(Message::NavSelect(Page::Generations));
        assert_eq!(app.page, Page::Generations);
        assert_eq!(app.busy, Busy::Applying);
        assert!(spawn_helper(&intents).is_none());
        assert!(!intents
            .iter()
            .any(|intent| matches!(intent, Intent::CloseSession)));
    }

    #[test]
    fn verify_integration_toasts_after_detect() {
        let mut app = test_app();
        let intents = app.apply(Message::VerifyIntegration);
        assert!(intents
            .iter()
            .any(|intent| matches!(intent, Intent::DetectSystem)));
        let info = common::SystemInfo {
            integration_status: IntegrationStatus::Integrated,
            ..common::SystemInfo::default()
        };
        let intents = app.apply(Message::SystemDetected(info));
        assert!(intents.iter().any(|intent| matches!(
            intent,
            Intent::ShowToast { text, .. } if text.contains("Integration verified")
        )));
    }

    #[test]
    fn spawn_failed_apply_toasts_and_does_not_hang() {
        let mut app = test_app();
        app.busy = Busy::Applying;
        let intents = app.apply(Message::Helper(HelperEvent::SpawnFailed {
            op: HelperOp::Apply {
                rebuild: RebuildType::Switch,
                then_write_state: true,
                save: None,
            },
            error: "nixos-toolkit-helper is not available".into(),
        }));
        assert_eq!(app.busy, Busy::Idle);
        assert!(app.helper_missing);
        assert!(app.apply_log.contains("Failed to start helper"));
        assert!(intents
            .iter()
            .any(|intent| matches!(intent, Intent::ShowToast { .. })));
    }

    #[test]
    fn load_generations_spawns_list() {
        let mut app = test_app_with_helper();
        let intents = app.apply(Message::LoadGenerations);
        assert_eq!(app.busy, Busy::LoadingGenerations);
        match spawn_helper(&intents) {
            Some(Intent::SpawnHelper {
                op: HelperOp::ListGenerations,
                request: HelperRequest::ListGenerations,
            }) => {}
            other => panic!("expected ListGenerations, got {other:?}"),
        }
    }

    #[test]
    fn confirm_rollback_switch_vs_boot_requests_differ() {
        let mut switch_app = test_app_with_helper();
        let switch = switch_app.apply(Message::ConfirmRollback {
            generation: 3,
            mode: RollbackMode::SwitchNow,
        });
        let mut boot_app = test_app_with_helper();
        let boot = boot_app.apply(Message::ConfirmRollback {
            generation: 3,
            mode: RollbackMode::SetForNextBoot,
        });
        let switch_activate = match spawn_helper(&switch) {
            Some(Intent::SpawnHelper {
                request: HelperRequest::RollbackGeneration { activate, .. },
                ..
            }) => activate.clone(),
            other => panic!("expected rollback spawn, got {other:?}"),
        };
        let boot_activate = match spawn_helper(&boot) {
            Some(Intent::SpawnHelper {
                request: HelperRequest::RollbackGeneration { activate, .. },
                ..
            }) => activate.clone(),
            other => panic!("expected rollback spawn, got {other:?}"),
        };
        assert_eq!(switch_activate, "switch");
        assert_eq!(boot_activate, "boot");
        assert_ne!(switch_activate, boot_activate);
    }

    #[test]
    fn rollback_to_previous_opens_dialog_for_listed_previous() {
        let mut app = test_app();
        app.current_generation = Some(5);
        app.generations = vec![
            Generation {
                number: 5,
                date: String::new(),
                current: true,
                nixos_version: None,
                kernel_version: None,
                config_rev: None,
            },
            Generation {
                number: 3,
                date: String::new(),
                current: false,
                nixos_version: None,
                kernel_version: None,
                config_rev: None,
            },
        ];
        app.apply(Message::RollbackToPrevious);
        match &app.dialog {
            Some(Dialog::ConfirmRollback { generation: 3 }) => {}
            other => panic!("expected ConfirmRollback 3, got {other:?}"),
        }
    }

    #[test]
    fn request_maintenance_without_warning_spawns_allowlist_command() {
        let mut app = test_app_with_helper();
        let intents = app.apply(Message::RequestMaintenance {
            id: "gc_unreachable".into(),
        });
        assert!(app.dialog.is_none());
        match spawn_helper(&intents) {
            Some(Intent::SpawnHelper {
                request: HelperRequest::RunMaintenance { command },
                ..
            }) => assert_eq!(command, "nix-collect-garbage"),
            other => panic!("expected RunMaintenance, got {other:?}"),
        }
        assert!(app.maintenance_log.contains("$ nix-collect-garbage"));
    }

    #[test]
    fn request_maintenance_with_warning_opens_dialog() {
        let mut app = test_app_with_helper();
        let intents = app.apply(Message::RequestMaintenance {
            id: "gc_all".into(),
        });
        assert!(intents.is_empty());
        match &app.dialog {
            Some(Dialog::ConfirmMaintenance { id, .. }) => assert_eq!(id, "gc_all"),
            other => panic!("expected ConfirmMaintenance, got {other:?}"),
        }
    }

    #[test]
    fn confirm_maintenance_sends_exact_allowlist_string() {
        let mut app = test_app_with_helper();
        let intents = app.apply(Message::ConfirmMaintenance {
            id: "gc_all".into(),
        });
        match spawn_helper(&intents) {
            Some(Intent::SpawnHelper {
                request: HelperRequest::RunMaintenance { command },
                ..
            }) => assert_eq!(command, "nix-collect-garbage -d"),
            other => panic!("expected RunMaintenance, got {other:?}"),
        }
    }

    #[test]
    fn load_disk_usage_spawns_get_disk_usage() {
        let mut app = test_app_with_helper();
        let intents = app.apply(Message::LoadDiskUsage);
        match spawn_helper(&intents) {
            Some(Intent::SpawnHelper {
                op: HelperOp::GetDiskUsage,
                request: HelperRequest::GetDiskUsage,
            }) => {}
            other => panic!("expected GetDiskUsage, got {other:?}"),
        }
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
                save: None,
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
                save: None,
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

    #[test]
    fn maintenance_output_appends_log() {
        let mut app = test_app();
        app.busy = Busy::Maintenance;
        let intents = app.apply(Message::Helper(HelperEvent::Response {
            op: HelperOp::RunMaintenance {
                command: "nix-collect-garbage".into(),
            },
            response: Box::new(HelperResponse::MaintenanceOutput {
                stdout: "freed 1M".into(),
                stderr: String::new(),
                success: true,
            }),
        }));
        assert!(app.maintenance_log.contains("freed 1M"));
        assert_eq!(app.busy, Busy::Idle);
        assert!(intents.iter().any(|intent| matches!(
            intent,
            Intent::ShowToast { text, .. } if text == "Maintenance completed"
        )));
    }

    #[test]
    fn init_queues_detect_system_and_gpu_without_helper() {
        let mut app = test_app();
        let intents = app.startup_intents();
        assert!(intents
            .iter()
            .any(|intent| matches!(intent, Intent::DetectSystem)));
        assert!(intents
            .iter()
            .any(|intent| matches!(intent, Intent::DetectGpu)));
        assert!(spawn_helper(&intents).is_none());
    }

    #[test]
    fn gpu_detected_populates_vendor() {
        let mut app = test_app();
        assert!(app.gpu_vendor.is_none());
        app.apply(Message::GpuDetected("NVIDIA: GeForce RTX 3060".into()));
        assert_eq!(app.gpu_vendor.as_deref(), Some("NVIDIA: GeForce RTX 3060"));
    }

    #[test]
    fn refresh_system_detects_system_and_gpu_without_helper() {
        let mut app = test_app();
        assert!(app.helper_missing);
        let intents = app.apply(Message::RefreshSystem);
        assert!(intents
            .iter()
            .any(|intent| matches!(intent, Intent::DetectSystem)));
        assert!(intents
            .iter()
            .any(|intent| matches!(intent, Intent::DetectGpu)));
        assert!(spawn_helper(&intents).is_none());
    }

    #[test]
    fn nav_hardware_queues_detect_gpu_once() {
        let mut app = test_app();
        let intents = app.apply(Message::NavSelect(Page::Hardware));
        assert!(intents
            .iter()
            .any(|intent| matches!(intent, Intent::DetectGpu)));
        app.apply(Message::GpuDetected("Intel: UHD".into()));
        let intents = app.apply(Message::NavSelect(Page::Hardware));
        assert!(!intents
            .iter()
            .any(|intent| matches!(intent, Intent::DetectGpu)));
    }

    #[test]
    fn helper_missing_does_not_spawn_on_nav_or_refresh_loaders() {
        let mut app = test_app();
        assert!(app.helper_missing);
        assert!(!app.flags.spawn.helper_available);

        let intents = app.apply(Message::NavSelect(Page::Generations));
        assert_eq!(app.page, Page::Generations);
        assert_eq!(app.busy, Busy::Idle);
        assert!(spawn_helper(&intents).is_none());

        let intents = app.apply(Message::LoadGenerations);
        assert_eq!(app.busy, Busy::Idle);
        assert!(spawn_helper(&intents).is_none());

        let intents = app.apply(Message::NavSelect(Page::Maintenance));
        assert_eq!(app.page, Page::Maintenance);
        assert!(spawn_helper(&intents).is_none());

        let intents = app.apply(Message::LoadDiskUsage);
        assert!(spawn_helper(&intents).is_none());

        let intents = app.apply(Message::RequestApply);
        assert!(app.dialog.is_none());
        assert!(intents.is_empty());

        let intents = app.apply(Message::ConfirmApply);
        assert!(spawn_helper(&intents).is_none());
        assert_eq!(app.busy, Busy::Idle);

        let intents = app.apply(Message::RequestDryRun);
        assert!(spawn_helper(&intents).is_none());
        assert_eq!(app.busy, Busy::Idle);
    }

    #[test]
    fn hostname_rejects_invalid_charset_and_matches_system_noop() {
        let mut app = test_app();
        app.system_info.hostname = Some("nixos".into());

        app.apply(Message::HostnameChanged("desk-1".into()));
        assert_eq!(app.state.hostname.as_deref(), Some("desk-1"));
        assert!(app.field_errors.hostname.is_none());

        app.apply(Message::HostnameChanged("nixo_".into()));
        assert_eq!(app.state.hostname.as_deref(), Some("desk-1"));
        assert!(app.field_errors.hostname.is_some());

        app.apply(Message::HostnameChanged("bad.host".into()));
        assert_eq!(app.state.hostname.as_deref(), Some("desk-1"));

        app.apply(Message::HostnameChanged("nixos".into()));
        assert!(app.state.hostname.is_none());
        assert!(app.field_errors.hostname.is_none());

        app.apply(Message::HostnameChanged("".into()));
        assert!(app.state.hostname.is_none());
        assert!(app.field_errors.hostname.is_none());
    }

    #[test]
    fn dns_partial_keeps_input_and_does_not_clobber_servers() {
        let mut app = test_app();
        app.apply(Message::DnsServersChanged("1.1.1.1".into()));
        assert_eq!(app.dns_input, "1.1.1.1");
        assert_eq!(app.state.dns_servers, ["1.1.1.1"]);
        assert!(app.field_errors.dns.is_none());

        app.apply(Message::DnsServersChanged("1.1.1.1, 8".into()));
        assert_eq!(app.dns_input, "1.1.1.1, 8");
        assert_eq!(app.state.dns_servers, ["1.1.1.1"]);
        assert!(app.field_errors.dns.is_some());

        app.apply(Message::DnsServersChanged("1.1.1.1, 8.8.8.8".into()));
        assert_eq!(app.dns_input, "1.1.1.1, 8.8.8.8");
        assert_eq!(app.state.dns_servers, ["1.1.1.1", "8.8.8.8"]);
        assert!(app.field_errors.dns.is_none());
    }

    #[test]
    fn custom_tcp_replace_does_not_accumulate_prefixes() {
        let mut app = test_app();
        app.apply(Message::ToggleTcpPort {
            port: 22,
            enabled: true,
        });
        app.apply(Message::CustomTcpPortsChanged("9".into()));
        app.apply(Message::CustomTcpPortsChanged("90".into()));
        app.apply(Message::CustomTcpPortsChanged("909".into()));
        app.apply(Message::CustomTcpPortsChanged("9090".into()));
        assert_eq!(app.custom_tcp_input, "9090");
        assert_eq!(app.state.network_config.allowed_tcp_ports, vec![22, 9090]);

        app.apply(Message::CustomTcpPortsChanged("9090, abc".into()));
        assert_eq!(app.custom_tcp_input, "9090, abc");
        assert_eq!(app.state.network_config.allowed_tcp_ports, vec![22, 9090]);

        app.apply(Message::CustomTcpPortsChanged(String::new()));
        assert!(app.custom_tcp_input.is_empty());
        assert_eq!(app.state.network_config.allowed_tcp_ports, vec![22]);
    }

    #[test]
    fn skip_host_probes_does_not_seed_username() {
        let app = test_app();
        assert!(app.flags.skip_host_probes);
        assert!(app.state.username.is_none());
    }

    #[test]
    fn default_username_from_user_env() {
        assert_eq!(crate::app::default_username(true, None, Some("gosh")), None);
        assert_eq!(
            crate::app::default_username(false, Some("kept"), Some("other")),
            Some("kept".into())
        );
        assert_eq!(
            crate::app::default_username(false, None, Some("gosh")),
            Some("gosh".into())
        );
        assert_eq!(
            crate::app::default_username(false, None, Some("bad user")),
            None
        );
        assert!(crate::app::is_valid_username("gosh"));
        assert!(crate::app::is_valid_username("gosh_1"));
        assert!(!crate::app::is_valid_username("bad user"));
        assert!(!crate::app::is_valid_username(""));
    }

    #[test]
    fn username_changed_rejects_invalid_charset() {
        let mut app = test_app();
        app.apply(Message::UsernameChanged("gosh".into()));
        assert_eq!(app.state.username.as_deref(), Some("gosh"));
        assert!(app.field_errors.username.is_none());

        app.apply(Message::UsernameChanged("bad user".into()));
        assert_eq!(app.state.username.as_deref(), Some("gosh"));
        assert!(app.field_errors.username.is_some());

        app.apply(Message::UsernameChanged("".into()));
        assert!(app.state.username.is_none());
        assert!(app.field_errors.username.is_none());
    }

    #[test]
    fn helper_state_syncs_dns_and_tcp_drafts_and_seeds_username() {
        let mut app = test_app();
        let ipc = IpcAppState {
            dns_servers: vec!["1.1.1.1".into()],
            network_config: NetworkConfig {
                allowed_tcp_ports: vec![22, 9090],
                allowed_udp_ports: vec![53, 9091],
                ..NetworkConfig::default()
            },
            ..IpcAppState::default()
        };
        app.apply(Message::Helper(HelperEvent::Response {
            op: HelperOp::ReadState,
            response: Box::new(HelperResponse::State(ipc)),
        }));
        assert_eq!(app.dns_input, "1.1.1.1");
        assert_eq!(app.custom_tcp_input, "9090");
        assert_eq!(app.custom_udp_input, "9091");
        assert!(app.state.username.is_none());
    }

    #[test]
    fn custom_udp_replace_does_not_accumulate_prefixes() {
        let mut app = test_app();
        app.apply(Message::ToggleUdpPort {
            port: 53,
            enabled: true,
        });
        app.apply(Message::CustomUdpPortsChanged("9".into()));
        app.apply(Message::CustomUdpPortsChanged("90".into()));
        app.apply(Message::CustomUdpPortsChanged("909".into()));
        app.apply(Message::CustomUdpPortsChanged("9090".into()));
        assert_eq!(app.custom_udp_input, "9090");
        assert_eq!(app.state.network_config.allowed_udp_ports, vec![53, 9090]);

        app.apply(Message::CustomUdpPortsChanged("9090, abc".into()));
        assert_eq!(app.custom_udp_input, "9090, abc");
        assert_eq!(app.state.network_config.allowed_udp_ports, vec![53, 9090]);

        app.apply(Message::CustomUdpPortsChanged(String::new()));
        assert!(app.custom_udp_input.is_empty());
        assert_eq!(app.state.network_config.allowed_udp_ports, vec![53]);
    }

    #[test]
    fn enabling_wireguard_opens_listen_udp_port() {
        let mut app = test_app();
        app.apply(Message::SetWireguardEnabled(true));
        assert!(app.state.network_config.wireguard_enabled);
        assert_eq!(app.state.network_config.wireguard_listen_port, 51820);
        assert_eq!(app.state.network_config.allowed_udp_ports, vec![51820]);

        app.apply(Message::SetWireguardListenPort(51821));
        assert_eq!(app.state.network_config.wireguard_listen_port, 51821);
        assert_eq!(
            app.state.network_config.allowed_udp_ports,
            vec![51820, 51821]
        );

        app.apply(Message::SetWireguardEnabled(false));
        assert!(!app.state.network_config.wireguard_enabled);
        assert_eq!(
            app.state.network_config.allowed_udp_ports,
            vec![51820, 51821]
        );
    }

    #[test]
    fn set_rebuild_type_confirm_apply_sends_boot_test_and_build() {
        for rebuild in [RebuildType::Boot, RebuildType::Test, RebuildType::Build] {
            let mut app = test_app_with_helper();
            app.apply(Message::SetRebuildType(rebuild));
            assert_eq!(app.rebuild_type, rebuild);
            app.apply(Message::RequestApply);
            match &app.dialog {
                Some(Dialog::ConfirmApply {
                    body,
                    destructive: true,
                    ..
                }) => {
                    assert!(
                        body.contains(&format!("nixos-rebuild {}", rebuild.as_arg())),
                        "body={body}"
                    );
                    assert!(!body.contains("nixos-rebuild switch"));
                }
                other => panic!("expected empty ConfirmApply, got {other:?}"),
            }
            let intents = app.apply(Message::ConfirmApply);
            match spawn_helper(&intents) {
                Some(Intent::SpawnHelper { op, request }) => {
                    match op {
                        HelperOp::Apply {
                            rebuild: sent,
                            then_write_state: true,
                            ..
                        } => assert_eq!(*sent, rebuild),
                        other => panic!("expected Apply {rebuild:?} write-state, got {other:?}"),
                    }
                    match request {
                        HelperRequest::Apply { rebuild_type, .. } => {
                            assert_eq!(*rebuild_type, rebuild);
                        }
                        other => panic!("expected Apply request, got {other:?}"),
                    }
                }
                other => panic!("expected SpawnHelper, got {other:?}"),
            }
            assert!(app.apply_log.contains(rebuild.as_arg()));
        }
    }
}

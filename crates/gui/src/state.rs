//! Application selection state. The only source of truth for what will be applied.

use common::actions::{BundleDef, ProfileDef};
use common::ipc::{
    AppState as IpcAppState, HardwareConfig, HelperRequest, NetworkConfig, RebuildType,
    ServicesConfig,
};
use common::nix::NixGenOptions;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

/// TCP ports owned by the network chips, not the custom-ports field.
pub const PRESET_TCP_PORTS: [u16; 4] = [22, 80, 443, 8080];

/// Current application state (GUI model).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AppState {
    pub selected_profile: Option<String>,
    pub enabled_bundles: HashSet<String>,
    pub bundle_packages: HashMap<String, HashSet<String>>,
    /// Bundle expander open/closed ids (not persisted).
    #[serde(skip)]
    pub expanded_bundles: HashSet<String>,
    pub hostname: Option<String>,
    pub dns_servers: Vec<String>,
    pub user_groups: HashSet<String>,
    pub username: Option<String>,
    /// Kept in sync with `hardware_config.bluetooth_enabled`.
    pub bluetooth_enabled: bool,
    pub custom_packages: HashSet<String>,
    pub network_config: NetworkConfig,
    pub services_config: ServicesConfig,
    pub hardware_config: HardwareConfig,
    pub has_changes: bool,
}

impl AppState {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn select_profile(&mut self, profile_id: impl Into<String>) {
        self.selected_profile = Some(profile_id.into());
        self.has_changes = true;
    }

    pub fn clear_profile(&mut self) {
        self.selected_profile = None;
        self.has_changes = true;
    }

    pub fn toggle_bundle(&mut self, bundle_id: impl Into<String>) {
        let id = bundle_id.into();
        if self.enabled_bundles.contains(&id) {
            self.enabled_bundles.remove(&id);
        } else {
            self.enabled_bundles.insert(id);
        }
        self.has_changes = true;
    }

    pub fn enable_bundle(&mut self, bundle_id: impl Into<String>) {
        self.enabled_bundles.insert(bundle_id.into());
        self.has_changes = true;
    }

    pub fn disable_bundle(&mut self, bundle_id: &str) {
        self.enabled_bundles.remove(bundle_id);
        self.bundle_packages.remove(bundle_id);
        self.has_changes = true;
    }

    #[must_use]
    pub fn is_bundle_enabled(&self, bundle_id: &str) -> bool {
        self.enabled_bundles.contains(bundle_id)
    }

    pub fn set_bundle_packages(&mut self, bundle_id: impl Into<String>, packages: HashSet<String>) {
        self.bundle_packages.insert(bundle_id.into(), packages);
        self.has_changes = true;
    }

    #[must_use]
    pub fn get_bundle_packages(&self, bundle_id: &str) -> Option<&HashSet<String>> {
        self.bundle_packages.get(bundle_id)
    }

    pub fn toggle_bundle_package(
        &mut self,
        bundle_id: &str,
        package: impl Into<String>,
        enabled: bool,
    ) {
        let pkg = package.into();
        if let Some(packages) = self.bundle_packages.get_mut(bundle_id) {
            if enabled {
                packages.insert(pkg);
            } else {
                packages.remove(&pkg);
            }
            self.has_changes = true;
        }
    }

    pub fn set_bundle_expanded(&mut self, bundle_id: impl Into<String>, expanded: bool) {
        let id = bundle_id.into();
        if expanded {
            self.expanded_bundles.insert(id);
        } else {
            self.expanded_bundles.remove(&id);
        }
    }

    pub fn set_hostname(&mut self, hostname: impl Into<String>) {
        let h = hostname.into();
        self.hostname = if h.is_empty() { None } else { Some(h) };
        self.has_changes = true;
    }

    pub fn set_dns_servers(&mut self, servers: Vec<String>) {
        self.dns_servers = servers.into_iter().filter(|s| !s.is_empty()).collect();
        self.has_changes = true;
    }

    pub fn add_dns_server(&mut self, server: impl Into<String>) {
        let s = server.into();
        if !s.is_empty() && !self.dns_servers.contains(&s) {
            self.dns_servers.push(s);
            self.has_changes = true;
        }
    }

    pub fn clear_dns_servers(&mut self) {
        if !self.dns_servers.is_empty() {
            self.dns_servers.clear();
            self.has_changes = true;
        }
    }

    /// Parse comma-separated IPv4 addresses and store them.
    pub fn parse_and_set_dns(&mut self, raw: &str) -> Result<(), String> {
        let mut servers = Vec::new();
        for part in raw.split([',', '\n']) {
            let s = part.trim();
            if s.is_empty() {
                continue;
            }
            if !is_ipv4(s) {
                return Err(format!("invalid IPv4 address: {s}"));
            }
            if !servers.contains(&s.to_string()) {
                servers.push(s.to_string());
            }
        }
        self.set_dns_servers(servers);
        Ok(())
    }

    pub fn toggle_user_group(&mut self, group: impl Into<String>) {
        let g = group.into();
        if self.user_groups.contains(&g) {
            self.user_groups.remove(&g);
        } else {
            self.user_groups.insert(g);
        }
        self.has_changes = true;
    }

    pub fn set_user_group(&mut self, group: impl Into<String>, enabled: bool) {
        let g = group.into();
        if enabled {
            self.user_groups.insert(g);
        } else {
            self.user_groups.remove(&g);
        }
        self.has_changes = true;
    }

    pub fn add_user_group(&mut self, group: impl Into<String>) {
        self.user_groups.insert(group.into());
        self.has_changes = true;
    }

    pub fn remove_user_group(&mut self, group: &str) {
        self.user_groups.remove(group);
        self.has_changes = true;
    }

    #[must_use]
    pub fn is_in_group(&self, group: &str) -> bool {
        self.user_groups.contains(group)
    }

    pub fn set_username(&mut self, username: impl Into<String>) {
        let u = username.into();
        self.username = if u.is_empty() { None } else { Some(u) };
        self.has_changes = true;
    }

    /// Writes both the sibling bool and `hardware_config.bluetooth_enabled`.
    pub fn set_bluetooth_enabled(&mut self, enabled: bool) {
        self.bluetooth_enabled = enabled;
        self.hardware_config.bluetooth_enabled = enabled;
        self.has_changes = true;
    }

    /// Copy `hardware_config.bluetooth_enabled` onto the sibling field.
    pub fn sync_bluetooth_from_hardware(&mut self) {
        self.bluetooth_enabled = self.hardware_config.bluetooth_enabled;
    }

    pub fn add_custom_package(&mut self, package: impl Into<String>) {
        let pkg = package.into();
        if !pkg.is_empty() {
            self.custom_packages.insert(pkg);
            self.has_changes = true;
        }
    }

    pub fn remove_custom_package(&mut self, package: &str) {
        self.custom_packages.remove(package);
        self.has_changes = true;
    }

    #[must_use]
    pub fn has_custom_package(&self, package: &str) -> bool {
        self.custom_packages.contains(package)
    }

    pub fn mark_applied(&mut self) {
        self.has_changes = false;
    }

    pub fn set_network_config(&mut self, config: NetworkConfig) {
        self.network_config = config;
        self.has_changes = true;
    }

    /// Parse comma-separated TCP ports and merge them into `allowed_tcp_ports`.
    pub fn parse_and_add_tcp_ports(&mut self, raw: &str) -> Result<(), String> {
        let mut ports = self.network_config.allowed_tcp_ports.clone();
        for part in raw.split([',', ' ', '\n']) {
            let s = part.trim();
            if s.is_empty() {
                continue;
            }
            let port: u16 = s.parse().map_err(|_| format!("invalid TCP port: {s}"))?;
            if port == 0 {
                return Err(format!("invalid TCP port: {s}"));
            }
            if !ports.contains(&port) {
                ports.push(port);
            }
        }
        self.network_config.allowed_tcp_ports = ports;
        self.has_changes = true;
        Ok(())
    }

    /// Replace custom TCP extras. Preset chips stay; parsed extras replace the rest.
    pub fn parse_and_set_custom_tcp_ports(&mut self, raw: &str) -> Result<(), String> {
        let mut extras = Vec::new();
        for part in raw.split([',', ' ', '\n']) {
            let s = part.trim();
            if s.is_empty() {
                continue;
            }
            let port: u16 = s.parse().map_err(|_| format!("invalid TCP port: {s}"))?;
            if port == 0 {
                return Err(format!("invalid TCP port: {s}"));
            }
            if PRESET_TCP_PORTS.contains(&port) {
                continue;
            }
            if !extras.contains(&port) {
                extras.push(port);
            }
        }
        let mut ports: Vec<u16> = self
            .network_config
            .allowed_tcp_ports
            .iter()
            .copied()
            .filter(|port| PRESET_TCP_PORTS.contains(port))
            .collect();
        for port in extras {
            if !ports.contains(&port) {
                ports.push(port);
            }
        }
        self.network_config.allowed_tcp_ports = ports;
        self.has_changes = true;
        Ok(())
    }

    pub fn set_tcp_port(&mut self, port: u16, enabled: bool) {
        let ports = &mut self.network_config.allowed_tcp_ports;
        if enabled {
            if !ports.contains(&port) {
                ports.push(port);
            }
        } else {
            ports.retain(|p| *p != port);
        }
        self.has_changes = true;
    }

    pub fn set_services_config(&mut self, config: ServicesConfig) {
        self.services_config = config;
        self.has_changes = true;
    }

    pub fn set_service(&mut self, id: &str, enabled: bool) {
        match id {
            "printing" => self.services_config.printing = enabled,
            "avahi" => self.services_config.avahi = enabled,
            "fwupd" => self.services_config.fwupd = enabled,
            "upower" => self.services_config.upower = enabled,
            "networkmanager" => self.services_config.networkmanager = enabled,
            "resolved" => self.services_config.resolved = enabled,
            "rustdesk" => self.services_config.rustdesk = enabled,
            "syncthing" => self.services_config.syncthing = enabled,
            "locate" => self.services_config.locate = enabled,
            "flatpak" => self.services_config.flatpak = enabled,
            "gnome_keyring" => self.services_config.gnome_keyring = enabled,
            "gnome_tweaks" => self.services_config.gnome_tweaks = enabled,
            "dconf" => self.services_config.dconf = enabled,
            "docker" => self.services_config.docker = enabled,
            "libvirtd" => self.services_config.libvirtd = enabled,
            "postgresql" => self.services_config.postgresql = enabled,
            "redis" => self.services_config.redis = enabled,
            "earlyoom" => self.services_config.earlyoom = enabled,
            "auto_upgrade" => self.services_config.auto_upgrade = enabled,
            "auto_gc" => self.services_config.auto_gc = enabled,
            "store_optimize" => self.services_config.store_optimize = enabled,
            _ => tracing::debug!(id, "unknown service id"),
        }
        self.has_changes = true;
    }

    pub fn set_hardware_config(&mut self, config: HardwareConfig) {
        self.hardware_config = config;
        self.sync_bluetooth_from_hardware();
        self.has_changes = true;
    }

    #[must_use]
    pub fn to_ipc_state(&self) -> IpcAppState {
        IpcAppState {
            selected_profile: self.selected_profile.clone(),
            enabled_bundles: self.enabled_bundles.iter().cloned().collect(),
            bundle_packages: self
                .bundle_packages
                .iter()
                .map(|(k, v)| (k.clone(), v.iter().cloned().collect()))
                .collect(),
            hostname: self.hostname.clone(),
            dns_servers: self.dns_servers.clone(),
            user_groups: self.user_groups.iter().cloned().collect(),
            username: self.username.clone(),
            bluetooth_enabled: self.bluetooth_enabled,
            last_applied: None,
            custom_packages: self.custom_packages.iter().cloned().collect(),
            network_config: self.network_config.clone(),
            services_config: self.services_config.clone(),
            hardware_config: self.hardware_config.clone(),
        }
    }

    /// Create from IPC state. Prefers `hardware_config.bluetooth_enabled` if the sibling diverges.
    #[must_use]
    pub fn from_ipc_state(ipc: IpcAppState) -> Self {
        let bluetooth_enabled = if ipc.hardware_config.bluetooth_enabled != ipc.bluetooth_enabled {
            ipc.hardware_config.bluetooth_enabled
        } else {
            ipc.bluetooth_enabled || ipc.hardware_config.bluetooth_enabled
        };
        let mut hardware_config = ipc.hardware_config;
        hardware_config.bluetooth_enabled = bluetooth_enabled;
        Self {
            selected_profile: ipc.selected_profile,
            enabled_bundles: ipc.enabled_bundles.into_iter().collect(),
            bundle_packages: ipc
                .bundle_packages
                .into_iter()
                .map(|(k, v)| (k, v.into_iter().collect()))
                .collect(),
            expanded_bundles: HashSet::new(),
            hostname: ipc.hostname,
            dns_servers: ipc.dns_servers,
            user_groups: ipc.user_groups.into_iter().collect(),
            username: ipc.username,
            bluetooth_enabled,
            custom_packages: ipc.custom_packages.into_iter().collect(),
            network_config: ipc.network_config,
            services_config: ipc.services_config,
            hardware_config,
            has_changes: false,
        }
    }

    /// Hardware config with the sibling bluetooth flag OR-ed in.
    #[must_use]
    pub fn effective_hardware(&self) -> HardwareConfig {
        self.hardware_config
            .clone()
            .with_bluetooth_or(self.bluetooth_enabled)
    }

    /// GTK empty-apply check (profile + bundles + packages) plus non-default hardware (C3/N5).
    #[must_use]
    pub fn apply_is_empty(&self) -> bool {
        self.selected_profile.is_none()
            && self.enabled_bundles.is_empty()
            && self.custom_packages.is_empty()
            && !self.effective_hardware().has_settings()
    }

    fn bundle_packages_for_ipc(&self) -> HashMap<String, Vec<String>> {
        self.bundle_packages
            .iter()
            .map(|(k, v)| (k.clone(), v.iter().cloned().collect()))
            .collect()
    }

    /// Preview / generate payload. Includes `hardware_config`.
    #[must_use]
    pub fn to_nix_gen_options<'a>(
        &'a self,
        profiles: &'a [ProfileDef],
        bundles: &'a [BundleDef],
    ) -> NixGenOptions<'a> {
        let profile = self
            .selected_profile
            .as_ref()
            .and_then(|id| profiles.iter().find(|p| &p.id == id));
        let enabled: Vec<_> = bundles
            .iter()
            .filter(|b| self.enabled_bundles.contains(&b.id))
            .collect();
        let hardware_config = self.effective_hardware();
        NixGenOptions {
            profile,
            bundles: enabled,
            bundle_packages: self.bundle_packages_for_ipc(),
            hostname: self.hostname.as_deref(),
            dns_servers: self.dns_servers.clone(),
            user_groups: self.user_groups.iter().cloned().collect(),
            username: self.username.as_deref(),
            bluetooth_enabled: hardware_config.bluetooth_enabled,
            custom_packages: self.custom_packages.iter().cloned().collect(),
            network_config: self.network_config.clone(),
            services_config: self.services_config.clone(),
            hardware_config,
        }
    }

    /// Apply IPC payload. Includes `hardware_config` and mirrors bluetooth.
    #[must_use]
    pub fn to_apply_request(&self, rebuild: RebuildType) -> HelperRequest {
        let hardware_config = self.effective_hardware();
        HelperRequest::Apply {
            selected_profile: self.selected_profile.clone(),
            enabled_bundles: self.enabled_bundles.iter().cloned().collect(),
            bundle_packages: self.bundle_packages_for_ipc(),
            hostname: self.hostname.clone(),
            dns_servers: self.dns_servers.clone(),
            user_groups: self.user_groups.iter().cloned().collect(),
            username: self.username.clone(),
            bluetooth_enabled: hardware_config.bluetooth_enabled,
            custom_packages: self.custom_packages.iter().cloned().collect(),
            network_config: self.network_config.clone(),
            services_config: self.services_config.clone(),
            hardware_config,
            rebuild_type: rebuild,
        }
    }

    /// Generate / dry-run IPC payload. Includes `hardware_config`.
    #[must_use]
    pub fn to_generate_request(&self, dry_run: bool) -> HelperRequest {
        let hardware_config = self.effective_hardware();
        HelperRequest::Generate {
            selected_profile: self.selected_profile.clone(),
            enabled_bundles: self.enabled_bundles.iter().cloned().collect(),
            bundle_packages: self.bundle_packages_for_ipc(),
            hostname: self.hostname.clone(),
            dns_servers: self.dns_servers.clone(),
            user_groups: self.user_groups.iter().cloned().collect(),
            username: self.username.clone(),
            bluetooth_enabled: hardware_config.bluetooth_enabled,
            custom_packages: self.custom_packages.iter().cloned().collect(),
            network_config: self.network_config.clone(),
            services_config: self.services_config.clone(),
            hardware_config,
            dry_run,
        }
    }

    #[must_use]
    pub fn summary(&self) -> String {
        let mut parts = Vec::new();

        if let Some(ref profile) = self.selected_profile {
            parts.push(format!("Profile: {profile}"));
        }

        if !self.enabled_bundles.is_empty() {
            let bundles: Vec<_> = self.enabled_bundles.iter().cloned().collect();
            parts.push(format!("Bundles: {}", bundles.join(", ")));
        }

        if let Some(ref hostname) = self.hostname {
            parts.push(format!("Hostname: {hostname}"));
        }

        if !self.dns_servers.is_empty() {
            parts.push(format!("DNS: {}", self.dns_servers.join(", ")));
        }

        if !self.user_groups.is_empty() {
            let groups: Vec<_> = self.user_groups.iter().cloned().collect();
            if let Some(ref user) = self.username {
                parts.push(format!("User {user} in groups: {}", groups.join(", ")));
            } else {
                parts.push(format!("User groups: {}", groups.join(", ")));
            }
        }

        if self.bluetooth_enabled {
            parts.push("Bluetooth: Enabled".to_string());
        }

        if !self.custom_packages.is_empty() {
            let packages: Vec<_> = self.custom_packages.iter().cloned().collect();
            parts.push(format!("Custom packages: {}", packages.join(", ")));
        }

        if self.network_config.has_settings() {
            let mut net_parts = Vec::new();
            if self.network_config.ssh_enabled {
                net_parts.push(format!("SSH on port {}", self.network_config.ssh_port));
            }
            if self.network_config.tailscale_enabled {
                net_parts.push("Tailscale".to_string());
            }
            if !self.network_config.allowed_tcp_ports.is_empty() {
                net_parts.push(format!(
                    "TCP ports: {:?}",
                    self.network_config.allowed_tcp_ports
                ));
            }
            if !net_parts.is_empty() {
                parts.push(format!("Network: {}", net_parts.join(", ")));
            }
        }

        if self.services_config.has_settings() {
            let services = self.services_config.enabled_services();
            parts.push(format!("Services: {}", services.join(", ")));
        }

        if parts.is_empty() {
            "No selections made".to_string()
        } else {
            parts.join("\n")
        }
    }
}

/// Comma-separated extra TCP ports (excludes the preset chips).
#[must_use]
pub fn custom_tcp_input_from_ports(ports: &[u16]) -> String {
    ports
        .iter()
        .filter(|port| !PRESET_TCP_PORTS.contains(port))
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join(", ")
}

fn is_ipv4(s: &str) -> bool {
    let mut parts = s.split('.');
    let mut count = 0;
    for part in parts.by_ref() {
        count += 1;
        if count > 4 {
            return false;
        }
        if part.is_empty() || part.len() > 3 {
            return false;
        }
        if !part.bytes().all(|b| b.is_ascii_digit()) {
            return false;
        }
        if part.len() > 1 && part.starts_with('0') {
            return false;
        }
        match part.parse::<u8>() {
            Ok(_) => {}
            Err(_) => return false,
        }
    }
    count == 4
}

#[cfg(test)]
mod tests {
    use super::*;
    use common::actions::{default_bundles, default_profiles};
    use common::ipc::HardwareConfig;
    use common::nix::generate_preview_full;

    #[test]
    fn bluetooth_mirror_prefers_hardware_on_diverge() {
        let ipc = IpcAppState {
            bluetooth_enabled: false,
            hardware_config: HardwareConfig {
                bluetooth_enabled: true,
                ..HardwareConfig::default()
            },
            ..IpcAppState::default()
        };
        let state = AppState::from_ipc_state(ipc);
        assert!(state.bluetooth_enabled);
        assert!(state.hardware_config.bluetooth_enabled);
    }

    #[test]
    fn set_bluetooth_writes_both() {
        let mut state = AppState::new();
        state.set_bluetooth_enabled(true);
        assert!(state.bluetooth_enabled);
        assert!(state.hardware_config.bluetooth_enabled);
    }

    #[test]
    fn apply_is_empty_counts_non_default_hardware() {
        let mut state = AppState::new();
        assert!(state.apply_is_empty());
        state.network_config.ssh_enabled = true;
        assert!(state.apply_is_empty());
        state.set_bluetooth_enabled(true);
        assert!(!state.apply_is_empty());
    }

    #[test]
    fn to_apply_request_includes_hardware_config() {
        let mut state = AppState::new();
        state.hardware_config.nvidia_driver = Some(0);
        state.hardware_config.nvidia_modesetting = true;
        state.set_bluetooth_enabled(true);
        match state.to_apply_request(RebuildType::Switch) {
            HelperRequest::Apply {
                hardware_config,
                bluetooth_enabled,
                rebuild_type,
                ..
            } => {
                assert_eq!(hardware_config.nvidia_driver, Some(0));
                assert!(hardware_config.nvidia_modesetting);
                assert!(hardware_config.bluetooth_enabled);
                assert!(bluetooth_enabled);
                assert_eq!(rebuild_type, RebuildType::Switch);
            }
            other => panic!("expected Apply, got {other:?}"),
        }
    }

    #[test]
    fn to_generate_request_includes_hardware_config() {
        let mut state = AppState::new();
        state.hardware_config.audio_server = 1;
        match state.to_generate_request(true) {
            HelperRequest::Generate {
                hardware_config,
                dry_run,
                ..
            } => {
                assert_eq!(hardware_config.audio_server, 1);
                assert!(dry_run);
            }
            other => panic!("expected Generate, got {other:?}"),
        }
    }

    #[test]
    fn preview_payload_includes_hardware_nix() {
        let mut state = AppState::new();
        state.hardware_config.nvidia_driver = Some(2);
        let profiles = default_profiles();
        let bundles = default_bundles();
        let options = state.to_nix_gen_options(&profiles, &bundles);
        assert_eq!(options.hardware_config.nvidia_driver, Some(2));
        let preview = generate_preview_full(&options);
        assert!(preview.contains("hardware.nix"));
        assert!(preview.contains("nvidia-open"));
    }
}

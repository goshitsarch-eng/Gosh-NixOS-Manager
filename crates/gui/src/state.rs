//! Application state management

use common::ipc::{AppState as IpcAppState, NetworkConfig, ServicesConfig};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// Current application state
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AppState {
    /// Currently selected profile ID
    pub selected_profile: Option<String>,
    /// Set of enabled bundle IDs
    pub enabled_bundles: HashSet<String>,
    /// Per-bundle package selections: bundle_id -> enabled package names
    pub bundle_packages: std::collections::HashMap<String, HashSet<String>>,
    /// Custom hostname (if changed from current)
    pub hostname: Option<String>,
    /// Custom DNS servers (e.g., ["1.1.1.1", "8.8.8.8"])
    pub dns_servers: Vec<String>,
    /// User groups to add the user to (e.g., ["libvirtd", "docker"])
    pub user_groups: HashSet<String>,
    /// Username for group membership
    pub username: Option<String>,
    /// Bluetooth enabled
    pub bluetooth_enabled: bool,
    /// Custom packages manually added by user (e.g., ["zed-editor", "htop"])
    pub custom_packages: HashSet<String>,
    /// Network configuration (firewall, SSH, VPN)
    pub network_config: NetworkConfig,
    /// Services configuration
    pub services_config: ServicesConfig,
    /// Whether there are unsaved changes
    pub has_changes: bool,
}

impl AppState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Select a profile (replaces any existing selection)
    pub fn select_profile(&mut self, profile_id: impl Into<String>) {
        self.selected_profile = Some(profile_id.into());
        self.has_changes = true;
    }

    /// Clear profile selection
    pub fn clear_profile(&mut self) {
        self.selected_profile = None;
        self.has_changes = true;
    }

    /// Toggle a bundle on/off
    pub fn toggle_bundle(&mut self, bundle_id: impl Into<String>) {
        let id = bundle_id.into();
        if self.enabled_bundles.contains(&id) {
            self.enabled_bundles.remove(&id);
        } else {
            self.enabled_bundles.insert(id);
        }
        self.has_changes = true;
    }

    /// Enable a bundle
    pub fn enable_bundle(&mut self, bundle_id: impl Into<String>) {
        self.enabled_bundles.insert(bundle_id.into());
        self.has_changes = true;
    }

    /// Disable a bundle
    pub fn disable_bundle(&mut self, bundle_id: &str) {
        self.enabled_bundles.remove(bundle_id);
        self.bundle_packages.remove(bundle_id);
        self.has_changes = true;
    }

    /// Check if a bundle is enabled
    pub fn is_bundle_enabled(&self, bundle_id: &str) -> bool {
        self.enabled_bundles.contains(bundle_id)
    }

    /// Set the enabled packages for a bundle
    pub fn set_bundle_packages(&mut self, bundle_id: impl Into<String>, packages: HashSet<String>) {
        self.bundle_packages.insert(bundle_id.into(), packages);
        self.has_changes = true;
    }

    /// Get the enabled packages for a bundle
    pub fn get_bundle_packages(&self, bundle_id: &str) -> Option<&HashSet<String>> {
        self.bundle_packages.get(bundle_id)
    }

    /// Toggle a package within a bundle on/off
    pub fn toggle_bundle_package(&mut self, bundle_id: &str, package: impl Into<String>, enabled: bool) {
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

    /// Set custom hostname
    pub fn set_hostname(&mut self, hostname: impl Into<String>) {
        let h = hostname.into();
        self.hostname = if h.is_empty() { None } else { Some(h) };
        self.has_changes = true;
    }

    /// Set DNS servers
    pub fn set_dns_servers(&mut self, servers: Vec<String>) {
        self.dns_servers = servers.into_iter().filter(|s| !s.is_empty()).collect();
        self.has_changes = true;
    }

    /// Add a DNS server
    pub fn add_dns_server(&mut self, server: impl Into<String>) {
        let s = server.into();
        if !s.is_empty() && !self.dns_servers.contains(&s) {
            self.dns_servers.push(s);
            self.has_changes = true;
        }
    }

    /// Clear DNS servers
    pub fn clear_dns_servers(&mut self) {
        if !self.dns_servers.is_empty() {
            self.dns_servers.clear();
            self.has_changes = true;
        }
    }

    /// Toggle a user group on/off
    pub fn toggle_user_group(&mut self, group: impl Into<String>) {
        let g = group.into();
        if self.user_groups.contains(&g) {
            self.user_groups.remove(&g);
        } else {
            self.user_groups.insert(g);
        }
        self.has_changes = true;
    }

    /// Add user to a group
    pub fn add_user_group(&mut self, group: impl Into<String>) {
        self.user_groups.insert(group.into());
        self.has_changes = true;
    }

    /// Remove user from a group
    pub fn remove_user_group(&mut self, group: &str) {
        self.user_groups.remove(group);
        self.has_changes = true;
    }

    /// Check if user is in a group
    pub fn is_in_group(&self, group: &str) -> bool {
        self.user_groups.contains(group)
    }

    /// Set the username for group membership
    pub fn set_username(&mut self, username: impl Into<String>) {
        let u = username.into();
        self.username = if u.is_empty() { None } else { Some(u) };
        self.has_changes = true;
    }

    /// Set Bluetooth enabled state
    pub fn set_bluetooth_enabled(&mut self, enabled: bool) {
        self.bluetooth_enabled = enabled;
        self.has_changes = true;
    }

    /// Add a custom package
    pub fn add_custom_package(&mut self, package: impl Into<String>) {
        let pkg = package.into();
        if !pkg.is_empty() {
            self.custom_packages.insert(pkg);
            self.has_changes = true;
        }
    }

    /// Remove a custom package
    pub fn remove_custom_package(&mut self, package: &str) {
        self.custom_packages.remove(package);
        self.has_changes = true;
    }

    /// Check if a custom package is installed
    pub fn has_custom_package(&self, package: &str) -> bool {
        self.custom_packages.contains(package)
    }

    /// Mark changes as applied
    pub fn mark_applied(&mut self) {
        self.has_changes = false;
    }

    /// Set network configuration
    pub fn set_network_config(&mut self, config: NetworkConfig) {
        self.network_config = config;
        self.has_changes = true;
    }

    /// Set services configuration
    pub fn set_services_config(&mut self, config: ServicesConfig) {
        self.services_config = config;
        self.has_changes = true;
    }

    /// Convert to IPC state format
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
        }
    }

    /// Create from IPC state format
    pub fn from_ipc_state(ipc: IpcAppState) -> Self {
        Self {
            selected_profile: ipc.selected_profile,
            enabled_bundles: ipc.enabled_bundles.into_iter().collect(),
            bundle_packages: ipc
                .bundle_packages
                .into_iter()
                .map(|(k, v)| (k, v.into_iter().collect()))
                .collect(),
            hostname: ipc.hostname,
            dns_servers: ipc.dns_servers,
            user_groups: ipc.user_groups.into_iter().collect(),
            username: ipc.username,
            bluetooth_enabled: ipc.bluetooth_enabled,
            custom_packages: ipc.custom_packages.into_iter().collect(),
            network_config: ipc.network_config,
            services_config: ipc.services_config,
            has_changes: false,
        }
    }

    /// Get a summary of current selections
    pub fn summary(&self) -> String {
        let mut parts = Vec::new();

        if let Some(ref profile) = self.selected_profile {
            parts.push(format!("Profile: {}", profile));
        }

        if !self.enabled_bundles.is_empty() {
            let bundles: Vec<_> = self.enabled_bundles.iter().cloned().collect();
            parts.push(format!("Bundles: {}", bundles.join(", ")));
        }

        if let Some(ref hostname) = self.hostname {
            parts.push(format!("Hostname: {}", hostname));
        }

        if !self.dns_servers.is_empty() {
            parts.push(format!("DNS: {}", self.dns_servers.join(", ")));
        }

        if !self.user_groups.is_empty() {
            let groups: Vec<_> = self.user_groups.iter().cloned().collect();
            if let Some(ref user) = self.username {
                parts.push(format!("User {} in groups: {}", user, groups.join(", ")));
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
                net_parts.push(format!("TCP ports: {:?}", self.network_config.allowed_tcp_ports));
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

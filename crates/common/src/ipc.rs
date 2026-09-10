//! IPC message types for communication between GUI and helper

use crate::config::SystemInfo;
use serde::{Deserialize, Serialize};

/// Request from GUI to helper
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload")]
pub enum HelperRequest {
    /// Check if helper has required permissions
    CheckPermissions,

    /// Get system information
    GetSystemInfo,

    /// Validate a configuration before applying
    Validate {
        selected_profile: Option<String>,
        enabled_bundles: Vec<String>,
        hostname: Option<String>,
    },

    /// Generate Nix configuration files (dry-run mode)
    Generate {
        selected_profile: Option<String>,
        enabled_bundles: Vec<String>,
        bundle_packages: std::collections::HashMap<String, Vec<String>>,
        hostname: Option<String>,
        dns_servers: Vec<String>,
        user_groups: Vec<String>,
        username: Option<String>,
        bluetooth_enabled: bool,
        custom_packages: Vec<String>,
        network_config: NetworkConfig,
        services_config: ServicesConfig,
        #[serde(default)]
        hardware_config: HardwareConfig,
        dry_run: bool,
    },

    /// Apply configuration with nixos-rebuild
    Apply {
        selected_profile: Option<String>,
        enabled_bundles: Vec<String>,
        bundle_packages: std::collections::HashMap<String, Vec<String>>,
        hostname: Option<String>,
        dns_servers: Vec<String>,
        user_groups: Vec<String>,
        username: Option<String>,
        bluetooth_enabled: bool,
        custom_packages: Vec<String>,
        network_config: NetworkConfig,
        services_config: ServicesConfig,
        #[serde(default)]
        hardware_config: HardwareConfig,
        rebuild_type: RebuildType,
    },

    /// Ensure directories exist
    EnsureDirectories,

    /// Read current state from state.json
    ReadState,

    /// Write state to state.json
    WriteState { state: AppState },

    /// List all NixOS generations
    ListGenerations,

    /// Rollback to a specific generation
    RollbackGeneration {
        generation: u32,
        /// `"switch"` (activate now) or `"boot"` (next boot). Old JSON omits this.
        #[serde(default = "default_switch")]
        activate: String,
    },

    /// Delete specific generations
    DeleteGenerations { generations: Vec<u32> },

    /// Run a maintenance command
    RunMaintenance { command: String },

    /// Get disk usage information
    GetDiskUsage,
}

/// Type of nixos-rebuild to run
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RebuildType {
    /// nixos-rebuild switch
    Switch,
    /// nixos-rebuild boot
    Boot,
    /// nixos-rebuild test
    Test,
    /// nixos-rebuild build (no activation)
    Build,
    /// nixos-rebuild dry-build
    DryBuild,
}

impl RebuildType {
    pub fn as_arg(&self) -> &'static str {
        match self {
            Self::Switch => "switch",
            Self::Boot => "boot",
            Self::Test => "test",
            Self::Build => "build",
            Self::DryBuild => "dry-build",
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Switch => "Switch (activate now)",
            Self::Boot => "Boot (activate on reboot)",
            Self::Test => "Test (temporary activation)",
            Self::Build => "Build only",
            Self::DryBuild => "Dry build (show what would build)",
        }
    }
}

/// Response from helper to GUI
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload")]
#[allow(clippy::large_enum_variant)]
pub enum HelperResponse {
    /// Simple success
    Ok,

    /// Operation failed
    Error {
        message: String,
        details: Option<String>,
    },

    /// Permissions check result
    Permissions {
        can_read_config: bool,
        can_write_managed: bool,
        can_run_rebuild: bool,
    },

    /// System information
    SystemInfo(SystemInfo),

    /// Validation result
    ValidationResult {
        valid: bool,
        errors: Vec<String>,
        warnings: Vec<String>,
    },

    /// Generated files preview
    GenerationResult {
        files: Vec<GeneratedFile>,
        preview: String,
    },

    /// Log output during apply
    Log { level: LogLevel, message: String },

    /// Apply completed
    ApplyComplete { success: bool, message: String },

    /// Current application state
    State(AppState),

    /// List of NixOS generations
    Generations(Vec<Generation>),

    /// Maintenance command output
    MaintenanceOutput {
        stdout: String,
        stderr: String,
        success: bool,
    },

    /// Disk usage information
    DiskUsage(DiskUsageInfo),
}

/// Information about a NixOS generation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Generation {
    /// Generation number
    pub number: u32,
    /// Date and time the generation was created
    pub date: String,
    /// Whether this is the current generation
    pub current: bool,
    /// NixOS version (if detectable)
    pub nixos_version: Option<String>,
    /// Kernel version (if detectable)
    pub kernel_version: Option<String>,
    /// Configuration revision (if using flakes)
    pub config_rev: Option<String>,
}

/// Disk usage information for Nix store
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiskUsageInfo {
    /// Size of /nix/store as a human-readable string (e.g., "45G")
    pub store_size: String,
    /// Number of system generations
    pub generation_count: u32,
    /// Any error message if partial data was retrieved
    pub error: Option<String>,
}

/// A file that will be/was generated
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneratedFile {
    pub path: String,
    pub content: String,
}

/// Log level for streaming logs
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LogLevel {
    Debug,
    Info,
    Warning,
    Error,
}

impl LogLevel {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Debug => "DEBUG",
            Self::Info => "INFO",
            Self::Warning => "WARN",
            Self::Error => "ERROR",
        }
    }
}

/// Network configuration for firewall, SSH, and VPN
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct NetworkConfig {
    /// Enable firewall
    #[serde(default = "default_true")]
    pub firewall_enabled: bool,
    /// Allowed TCP ports
    #[serde(default)]
    pub allowed_tcp_ports: Vec<u16>,
    /// Allowed UDP ports
    #[serde(default)]
    pub allowed_udp_ports: Vec<u16>,
    /// Enable SSH server
    #[serde(default)]
    pub ssh_enabled: bool,
    /// SSH port (default 22)
    #[serde(default = "default_ssh_port")]
    pub ssh_port: u16,
    /// Allow password authentication
    #[serde(default)]
    pub ssh_password_auth: bool,
    /// Root login policy: "no", "prohibit-password", or "yes"
    #[serde(default = "default_root_login")]
    pub ssh_root_login: String,
    /// Enable fail2ban for SSH protection
    #[serde(default)]
    pub fail2ban_enabled: bool,
    /// Enable Tailscale VPN
    #[serde(default)]
    pub tailscale_enabled: bool,
}

fn default_true() -> bool {
    true
}

fn default_ssh_port() -> u16 {
    22
}

fn default_root_login() -> String {
    "no".to_string()
}

fn default_switch() -> String {
    "switch".to_string()
}

impl NetworkConfig {
    /// Check if any network settings are configured (non-default)
    pub fn has_settings(&self) -> bool {
        !self.allowed_tcp_ports.is_empty()
            || !self.allowed_udp_ports.is_empty()
            || self.ssh_enabled
            || self.tailscale_enabled
            || !self.firewall_enabled // Disabled is non-default
    }
}

/// Services configuration for system services
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServicesConfig {
    #[serde(default)]
    pub printing: bool,
    #[serde(default)]
    pub avahi: bool,
    #[serde(default)]
    pub fwupd: bool,
    #[serde(default)]
    pub upower: bool,
    #[serde(default)]
    pub networkmanager: bool,
    #[serde(default)]
    pub resolved: bool,
    #[serde(default)]
    pub rustdesk: bool,
    #[serde(default)]
    pub syncthing: bool,
    #[serde(default)]
    pub locate: bool,
    #[serde(default)]
    pub flatpak: bool,
    #[serde(default)]
    pub gnome_keyring: bool,
    #[serde(default)]
    pub gnome_tweaks: bool,
    #[serde(default)]
    pub dconf: bool,
    #[serde(default)]
    pub docker: bool,
    #[serde(default)]
    pub libvirtd: bool,
    #[serde(default)]
    pub postgresql: bool,
    #[serde(default)]
    pub redis: bool,
    #[serde(default)]
    pub earlyoom: bool,
    #[serde(default)]
    pub auto_upgrade: bool,
    #[serde(default)]
    pub auto_gc: bool,
    #[serde(default)]
    pub store_optimize: bool,
}

impl ServicesConfig {
    /// Get list of enabled service IDs
    pub fn enabled_services(&self) -> Vec<&'static str> {
        let mut services = Vec::new();
        if self.printing {
            services.push("printing");
        }
        if self.avahi {
            services.push("avahi");
        }
        if self.fwupd {
            services.push("fwupd");
        }
        if self.upower {
            services.push("upower");
        }
        if self.networkmanager {
            services.push("networkmanager");
        }
        if self.resolved {
            services.push("resolved");
        }
        if self.rustdesk {
            services.push("rustdesk");
        }
        if self.syncthing {
            services.push("syncthing");
        }
        if self.locate {
            services.push("locate");
        }
        if self.flatpak {
            services.push("flatpak");
        }
        if self.gnome_keyring {
            services.push("gnome_keyring");
        }
        if self.gnome_tweaks {
            services.push("gnome_tweaks");
        }
        if self.dconf {
            services.push("dconf");
        }
        if self.docker {
            services.push("docker");
        }
        if self.libvirtd {
            services.push("libvirtd");
        }
        if self.postgresql {
            services.push("postgresql");
        }
        if self.redis {
            services.push("redis");
        }
        if self.earlyoom {
            services.push("earlyoom");
        }
        if self.auto_upgrade {
            services.push("auto_upgrade");
        }
        if self.auto_gc {
            services.push("auto_gc");
        }
        if self.store_optimize {
            services.push("store_optimize");
        }
        services
    }

    /// Check if any services are enabled
    pub fn has_settings(&self) -> bool {
        !self.enabled_services().is_empty()
    }
}

/// Hardware configuration for GPU, audio, bluetooth, and power management
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HardwareConfig {
    /// NVIDIA driver selection: 0=Stable, 1=Beta, 2=Open, 3=Nouveau, None=not NVIDIA
    #[serde(default)]
    pub nvidia_driver: Option<u8>,
    /// Enable kernel modesetting for NVIDIA (GTK SwitchRow default was on)
    #[serde(default = "default_true")]
    pub nvidia_modesetting: bool,
    /// Enable NVIDIA power management
    #[serde(default)]
    pub nvidia_powermanagement: bool,
    /// Use open-source NVIDIA kernel modules
    #[serde(default)]
    pub nvidia_open: bool,
    /// Audio server: 0=PipeWire, 1=PulseAudio, 2=None
    #[serde(default)]
    pub audio_server: u8,
    /// Enable low-latency audio settings
    #[serde(default)]
    pub audio_lowlatency: bool,
    /// Enable Bluetooth hardware and services
    #[serde(default)]
    pub bluetooth_enabled: bool,
    /// Automatically power on Bluetooth at boot
    #[serde(default)]
    pub bluetooth_autopower: bool,
    /// Power profile: 0=Balanced, 1=Performance, 2=Power Saver
    #[serde(default)]
    pub power_profile: u8,
    /// Enable TLP power management
    #[serde(default)]
    pub tlp_enabled: bool,
    /// Enable Intel Thermald
    #[serde(default)]
    pub thermald_enabled: bool,
}

impl Default for HardwareConfig {
    fn default() -> Self {
        Self {
            nvidia_driver: None,
            nvidia_modesetting: default_true(),
            nvidia_powermanagement: false,
            nvidia_open: false,
            audio_server: 0,
            audio_lowlatency: false,
            bluetooth_enabled: false,
            bluetooth_autopower: false,
            power_profile: 0,
            tlp_enabled: false,
            thermald_enabled: false,
        }
    }
}

impl HardwareConfig {
    /// True when any field differs from the struct default.
    #[must_use]
    pub fn has_settings(&self) -> bool {
        self != &Self::default()
    }

    /// OR the sibling `bluetooth_enabled` flag onto this config.
    #[must_use]
    pub fn with_bluetooth_or(mut self, bluetooth_enabled: bool) -> Self {
        self.bluetooth_enabled = self.bluetooth_enabled || bluetooth_enabled;
        self
    }
}

/// Persisted application state
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppState {
    /// Selected profile ID (e.g., "gnome", "kde")
    pub selected_profile: Option<String>,
    /// Enabled bundle IDs
    pub enabled_bundles: Vec<String>,
    /// Per-bundle package selections: bundle_id -> list of enabled package names
    /// When a bundle is in enabled_bundles but not in bundle_packages, all packages are installed
    #[serde(default)]
    pub bundle_packages: std::collections::HashMap<String, Vec<String>>,
    /// Custom hostname (if changed)
    pub hostname: Option<String>,
    /// Custom DNS servers (e.g., ["1.1.1.1", "8.8.8.8"])
    pub dns_servers: Vec<String>,
    /// User groups to add the user to (e.g., ["libvirtd", "docker"])
    pub user_groups: Vec<String>,
    /// Username for group membership
    pub username: Option<String>,
    /// Bluetooth enabled
    #[serde(default)]
    pub bluetooth_enabled: bool,
    /// Last successful apply timestamp
    pub last_applied: Option<String>,
    /// Custom packages manually added by user (e.g., ["zed-editor", "htop"])
    pub custom_packages: Vec<String>,
    /// Network configuration (firewall, SSH, VPN)
    #[serde(default)]
    pub network_config: NetworkConfig,
    /// Services configuration
    #[serde(default)]
    pub services_config: ServicesConfig,
    /// Hardware configuration (GPU, audio, bluetooth, power)
    #[serde(default)]
    pub hardware_config: HardwareConfig,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// GTK-era Apply JSON has no `hardware_config` field.
    const OLD_APPLY_JSON: &str = r#"{
        "type": "Apply",
        "payload": {
            "selected_profile": "gnome",
            "enabled_bundles": [],
            "bundle_packages": {},
            "hostname": null,
            "dns_servers": [],
            "user_groups": [],
            "username": null,
            "bluetooth_enabled": true,
            "custom_packages": [],
            "network_config": {
                "firewall_enabled": true,
                "allowed_tcp_ports": [],
                "allowed_udp_ports": [],
                "ssh_enabled": false,
                "ssh_port": 22,
                "ssh_password_auth": false,
                "ssh_root_login": "no",
                "fail2ban_enabled": false,
                "tailscale_enabled": false
            },
            "services_config": {},
            "rebuild_type": "Switch"
        }
    }"#;

    const OLD_GENERATE_JSON: &str = r#"{
        "type": "Generate",
        "payload": {
            "selected_profile": null,
            "enabled_bundles": [],
            "bundle_packages": {},
            "hostname": null,
            "dns_servers": [],
            "user_groups": [],
            "username": null,
            "bluetooth_enabled": false,
            "custom_packages": [],
            "network_config": {},
            "services_config": {},
            "dry_run": true
        }
    }"#;

    #[test]
    fn old_apply_json_without_hardware_config_deserializes() {
        let request: HelperRequest =
            serde_json::from_str(OLD_APPLY_JSON).expect("old Apply JSON should deserialize");
        match request {
            HelperRequest::Apply {
                bluetooth_enabled,
                hardware_config,
                selected_profile,
                rebuild_type,
                ..
            } => {
                assert!(bluetooth_enabled);
                assert_eq!(hardware_config, HardwareConfig::default());
                assert!(!hardware_config.has_settings());
                assert_eq!(selected_profile.as_deref(), Some("gnome"));
                assert_eq!(rebuild_type, RebuildType::Switch);
            }
            other => panic!("expected Apply, got {other:?}"),
        }
    }

    #[test]
    fn old_generate_json_without_hardware_config_deserializes() {
        let request: HelperRequest =
            serde_json::from_str(OLD_GENERATE_JSON).expect("old Generate JSON should deserialize");
        match request {
            HelperRequest::Generate {
                hardware_config,
                dry_run,
                ..
            } => {
                assert_eq!(hardware_config, HardwareConfig::default());
                assert!(dry_run);
            }
            other => panic!("expected Generate, got {other:?}"),
        }
    }

    #[test]
    fn hardware_has_settings_tracks_non_default_fields() {
        assert!(!HardwareConfig::default().has_settings());
        assert!(HardwareConfig::default().nvidia_modesetting);
        assert!(HardwareConfig {
            bluetooth_enabled: true,
            ..HardwareConfig::default()
        }
        .has_settings());
        assert!(HardwareConfig {
            nvidia_driver: Some(0),
            ..HardwareConfig::default()
        }
        .has_settings());
        assert!(HardwareConfig {
            audio_server: 1,
            ..HardwareConfig::default()
        }
        .has_settings());
    }

    #[test]
    fn nvidia_modesetting_defaults_true_and_explicit_false_round_trips() {
        let missing: HardwareConfig =
            serde_json::from_str("{}").expect("empty HardwareConfig JSON");
        assert!(missing.nvidia_modesetting);
        assert!(!missing.has_settings());

        let explicit_false: HardwareConfig =
            serde_json::from_str(r#"{"nvidia_modesetting":false}"#)
                .expect("explicit false should deserialize");
        assert!(!explicit_false.nvidia_modesetting);
        assert!(explicit_false.has_settings());

        let explicit_true: HardwareConfig = serde_json::from_str(r#"{"nvidia_modesetting":true}"#)
            .expect("explicit true should deserialize");
        assert!(explicit_true.nvidia_modesetting);
        assert!(!explicit_true.has_settings());
    }

    #[test]
    fn with_bluetooth_or_merges_sibling() {
        let hw = HardwareConfig::default().with_bluetooth_or(true);
        assert!(hw.bluetooth_enabled);
        let already = HardwareConfig {
            bluetooth_enabled: true,
            ..HardwareConfig::default()
        };
        assert!(already.with_bluetooth_or(false).bluetooth_enabled);
    }

    #[test]
    fn old_rollback_generation_json_without_activate_defaults_to_switch() {
        let request: HelperRequest =
            serde_json::from_str(r#"{"type":"RollbackGeneration","payload":{"generation":42}}"#)
                .expect("old RollbackGeneration JSON should deserialize");
        match request {
            HelperRequest::RollbackGeneration {
                generation,
                activate,
            } => {
                assert_eq!(generation, 42);
                assert_eq!(activate, "switch");
            }
            other => panic!("expected RollbackGeneration, got {other:?}"),
        }
    }

    #[test]
    fn rollback_generation_boot_round_trips() {
        let request = HelperRequest::RollbackGeneration {
            generation: 7,
            activate: "boot".into(),
        };
        let json = serde_json::to_string(&request).expect("serialize");
        let back: HelperRequest = serde_json::from_str(&json).expect("deserialize");
        match back {
            HelperRequest::RollbackGeneration {
                generation,
                activate,
            } => {
                assert_eq!(generation, 7);
                assert_eq!(activate, "boot");
            }
            other => panic!("expected RollbackGeneration, got {other:?}"),
        }
    }
}

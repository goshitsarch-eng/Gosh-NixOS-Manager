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
    RollbackGeneration { generation: u32 },

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
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
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
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
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
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct HardwareConfig {
    /// NVIDIA driver selection: 0=Stable, 1=Beta, 2=Open, 3=Nouveau, None=not NVIDIA
    #[serde(default)]
    pub nvidia_driver: Option<u8>,
    /// Enable kernel modesetting for NVIDIA
    #[serde(default)]
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

/// Persisted application state
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
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

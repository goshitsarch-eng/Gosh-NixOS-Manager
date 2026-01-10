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
        custom_packages: Vec<String>,
        dry_run: bool,
    },

    /// Apply configuration with nixos-rebuild
    Apply {
        selected_profile: Option<String>,
        enabled_bundles: Vec<String>,
        bundle_packages: std::collections::HashMap<String, Vec<String>>,
        hostname: Option<String>,
        custom_packages: Vec<String>,
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
pub enum HelperResponse {
    /// Simple success
    Ok,

    /// Operation failed
    Error { message: String, details: Option<String> },

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
    MaintenanceOutput { stdout: String, stderr: String, success: bool },
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
    /// Last successful apply timestamp
    pub last_applied: Option<String>,
    /// Custom packages manually added by user (e.g., ["zed-editor", "htop"])
    pub custom_packages: Vec<String>,
}

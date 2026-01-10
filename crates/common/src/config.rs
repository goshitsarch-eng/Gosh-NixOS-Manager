//! System configuration types and paths

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// NixOS configuration mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConfigMode {
    /// Classic configuration.nix style
    Classic,
    /// Flake-based configuration
    Flake,
    /// Unknown or could not be detected
    Unknown,
}

impl ConfigMode {
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Classic => "Classic (configuration.nix)",
            Self::Flake => "Flake-based",
            Self::Unknown => "Unknown",
        }
    }
}

/// Integration status with the toolkit
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum IntegrationStatus {
    /// User has added the import
    Integrated,
    /// Import not detected
    NotIntegrated,
    /// Could not determine (manual check needed)
    Unknown,
}

/// System information detected at runtime
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemInfo {
    /// Whether the system is NixOS
    pub is_nixos: bool,
    /// NixOS version (e.g., "24.05")
    pub nixos_version: Option<String>,
    /// Configuration mode (classic vs flake)
    pub config_mode: ConfigMode,
    /// Path to the main configuration file
    pub config_path: Option<PathBuf>,
    /// Whether our module is imported
    pub integration_status: IntegrationStatus,
    /// Current hostname
    pub hostname: Option<String>,
    /// Current desktop environment
    pub current_desktop: Option<String>,
}

impl Default for SystemInfo {
    fn default() -> Self {
        Self {
            is_nixos: false,
            nixos_version: None,
            config_mode: ConfigMode::Unknown,
            config_path: None,
            integration_status: IntegrationStatus::Unknown,
            hostname: None,
            current_desktop: None,
        }
    }
}

/// Paths used by the toolkit
pub mod paths {
    use std::path::PathBuf;

    /// Base directory for managed NixOS configuration
    pub const MANAGED_DIR: &str = "/etc/nixos/nixos-toolkit";

    /// State directory within managed dir
    pub const STATE_DIR: &str = "/etc/nixos/nixos-toolkit/state";

    /// Profiles directory within managed dir
    pub const PROFILES_DIR: &str = "/etc/nixos/nixos-toolkit/profiles";

    /// Bundles directory within managed dir
    pub const BUNDLES_DIR: &str = "/etc/nixos/nixos-toolkit/bundles";

    /// Main selected.nix file that user imports
    pub const SELECTED_NIX: &str = "/etc/nixos/nixos-toolkit/state/selected.nix";

    /// Optional JSON state file for UI state
    pub const STATE_JSON: &str = "/etc/nixos/nixos-toolkit/state/state.json";

    /// Hostname snippet file
    pub const HOSTNAME_NIX: &str = "/etc/nixos/nixos-toolkit/state/hostname.nix";

    /// DNS configuration snippet file
    pub const DNS_NIX: &str = "/etc/nixos/nixos-toolkit/state/dns.nix";

    /// User groups snippet file
    pub const USERS_NIX: &str = "/etc/nixos/nixos-toolkit/state/users.nix";

    /// Custom packages snippet file
    pub const CUSTOM_PACKAGES_NIX: &str = "/etc/nixos/nixos-toolkit/state/custom-packages.nix";

    /// Get the managed directory path
    pub fn managed_dir() -> PathBuf {
        PathBuf::from(MANAGED_DIR)
    }

    /// Get the state directory path
    pub fn state_dir() -> PathBuf {
        PathBuf::from(STATE_DIR)
    }

    /// Get the selected.nix path
    pub fn selected_nix() -> PathBuf {
        PathBuf::from(SELECTED_NIX)
    }

    /// Get templates directory from environment or default
    pub fn templates_dir() -> PathBuf {
        std::env::var("NIXOS_TOOLKIT_TEMPLATES_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("./nix/templates"))
    }
}

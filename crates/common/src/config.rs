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

/// Path fragments that mean `selected.nix` is imported.
///
/// Matching is substring-based on uncommented source. A `#` line comment or
/// `/* */` block that contains one of these strings is **not** integration.
/// A bare `nixos-toolkit` mention is not enough.
const SELECTED_NIX_IMPORT_MARKERS: &[&str] = &[
    "./nixos-toolkit/state/selected.nix",
    "/etc/nixos/nixos-toolkit/state/selected.nix",
    "nixos-toolkit/state/selected.nix",
];

/// Whether `configuration.nix` and/or `flake.nix` import the managed module.
///
/// Either file is sufficient. Both are considered; callers must not skip
/// `flake.nix` when `selected.nix` already exists.
#[must_use]
pub fn detect_integration_status(
    configuration_nix: Option<&str>,
    flake_nix: Option<&str>,
) -> IntegrationStatus {
    if file_imports_selected_nix(configuration_nix) || file_imports_selected_nix(flake_nix) {
        IntegrationStatus::Integrated
    } else {
        IntegrationStatus::NotIntegrated
    }
}

fn file_imports_selected_nix(content: Option<&str>) -> bool {
    content.is_some_and(|c| {
        let code = strip_nix_comments(c);
        SELECTED_NIX_IMPORT_MARKERS
            .iter()
            .any(|m| code.contains(m))
    })
}

/// Drop `#` line comments and `/* */` blocks. Strings (`"..."` / `''...''`) are kept.
fn strip_nix_comments(src: &str) -> String {
    let mut out = String::with_capacity(src.len());
    let bytes = src.as_bytes();
    let mut i = 0;
    let mut in_dquote = false;
    let mut in_indented = false;
    while i < bytes.len() {
        let c = bytes[i] as char;
        if in_dquote {
            out.push(c);
            if c == '\\' && i + 1 < bytes.len() {
                out.push(bytes[i + 1] as char);
                i += 2;
                continue;
            }
            if c == '"' {
                in_dquote = false;
            }
            i += 1;
            continue;
        }
        if in_indented {
            out.push(c);
            if c == '\'' && i + 1 < bytes.len() && bytes[i + 1] == b'\'' {
                out.push('\'');
                in_indented = false;
                i += 2;
                continue;
            }
            i += 1;
            continue;
        }
        if c == '"' {
            in_dquote = true;
            out.push(c);
            i += 1;
            continue;
        }
        if c == '\'' && i + 1 < bytes.len() && bytes[i + 1] == b'\'' {
            in_indented = true;
            out.push_str("''");
            i += 2;
            continue;
        }
        if c == '#' {
            while i < bytes.len() && bytes[i] != b'\n' {
                i += 1;
            }
            continue;
        }
        if c == '/' && i + 1 < bytes.len() && bytes[i + 1] == b'*' {
            i += 2;
            while i + 1 < bytes.len() && !(bytes[i] == b'*' && bytes[i + 1] == b'/') {
                i += 1;
            }
            i = (i + 2).min(bytes.len());
            continue;
        }
        out.push(c);
        i += 1;
    }
    out
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
    use std::path::{Component, Path, PathBuf};

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

    /// Hardware configuration snippet file (Bluetooth, GPU, etc.)
    pub const HARDWARE_NIX: &str = "/etc/nixos/nixos-toolkit/state/hardware.nix";

    /// Network configuration snippet file (firewall, SSH, VPN)
    pub const NETWORK_NIX: &str = "/etc/nixos/nixos-toolkit/state/network.nix";

    /// Services configuration snippet file
    pub const SERVICES_NIX: &str = "/etc/nixos/nixos-toolkit/state/services.nix";

    /// Allow-unfree snippet imported when generated config needs non-free packages
    pub const UNFREE_NIX: &str = "/etc/nixos/nixos-toolkit/state/unfree.nix";

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

    /// True when `path` resolves under [`MANAGED_DIR`] (write jail).
    #[must_use]
    pub fn is_allowed_managed_path(path: &Path) -> bool {
        if !path.is_absolute() {
            return false;
        }
        if path.components().any(|c| matches!(c, Component::ParentDir)) {
            return false;
        }

        let managed = Path::new(MANAGED_DIR);
        if !path.starts_with(managed) {
            return false;
        }

        let resolved = resolve_existing_prefix(path);
        let managed_resolved = resolve_existing_prefix(managed);
        resolved.starts_with(&managed_resolved)
    }

    fn resolve_existing_prefix(path: &Path) -> PathBuf {
        let mut suffix: Vec<std::ffi::OsString> = Vec::new();
        let mut current = path.to_path_buf();
        loop {
            if current.exists() {
                if let Ok(canon) = current.canonicalize() {
                    let mut out = canon;
                    for part in suffix.iter().rev() {
                        out.push(part);
                    }
                    return out;
                }
                break;
            }
            match current.file_name() {
                Some(name) => {
                    suffix.push(name.to_os_string());
                    if !current.pop() {
                        break;
                    }
                }
                None => break,
            }
        }
        path.to_path_buf()
    }
}

#[cfg(test)]
mod tests {
    use super::{detect_integration_status, IntegrationStatus};

    #[test]
    fn flake_only_import_is_integrated() {
        let flake = r#"
            {
              nixosConfigurations.box = {
                modules = [
                  ./configuration.nix
                  ./nixos-toolkit/state/selected.nix
                ];
              };
            }
        "#;
        assert_eq!(
            detect_integration_status(None, Some(flake)),
            IntegrationStatus::Integrated
        );
        assert_eq!(
            detect_integration_status(Some("# no import here\n"), Some(flake)),
            IntegrationStatus::Integrated
        );
    }

    #[test]
    fn classic_import_is_integrated() {
        let configuration = r#"
            { config, pkgs, ... }:
            {
              imports = [
                ./hardware-configuration.nix
                ./nixos-toolkit/state/selected.nix
              ];
            }
        "#;
        assert_eq!(
            detect_integration_status(Some(configuration), None),
            IntegrationStatus::Integrated
        );
        assert_eq!(
            detect_integration_status(Some(configuration), Some("{ }")),
            IntegrationStatus::Integrated
        );
    }

    #[test]
    fn absolute_selected_nix_path_is_integrated() {
        let configuration = r#"imports = [ /etc/nixos/nixos-toolkit/state/selected.nix ];"#;
        assert_eq!(
            detect_integration_status(Some(configuration), None),
            IntegrationStatus::Integrated
        );
    }

    #[test]
    fn unprefixed_selected_nix_path_is_integrated() {
        let flake = r#"modules = [ nixos-toolkit/state/selected.nix ];"#;
        assert_eq!(
            detect_integration_status(None, Some(flake)),
            IntegrationStatus::Integrated
        );
    }

    #[test]
    fn bare_nixos_toolkit_word_is_not_integrated() {
        let configuration = r#"
            # I should look at nixos-toolkit sometime
            { config, pkgs, ... }: { networking.hostName = "box"; }
        "#;
        let flake = r#"
            # nixos-toolkit notes
            { inputs.nixpkgs.url = "github:NixOS/nixpkgs"; }
        "#;
        assert_eq!(
            detect_integration_status(Some(configuration), Some(flake)),
            IntegrationStatus::NotIntegrated
        );
        assert_eq!(
            detect_integration_status(Some(configuration), None),
            IntegrationStatus::NotIntegrated
        );
    }

    #[test]
    fn neither_file_is_not_integrated() {
        assert_eq!(
            detect_integration_status(None, None),
            IntegrationStatus::NotIntegrated
        );
    }

    #[test]
    fn comment_with_exact_import_path_is_not_integrated() {
        let line = r#"# ./nixos-toolkit/state/selected.nix"#;
        let block = r#"
            { config, pkgs, ... }: {
              /* /etc/nixos/nixos-toolkit/state/selected.nix */
              networking.hostName = "box";
            }
        "#;
        assert_eq!(
            detect_integration_status(Some(line), None),
            IntegrationStatus::NotIntegrated
        );
        assert_eq!(
            detect_integration_status(Some(block), None),
            IntegrationStatus::NotIntegrated
        );
    }

    #[test]
    fn import_with_trailing_comment_is_integrated() {
        let configuration =
            r#"imports = [ ./nixos-toolkit/state/selected.nix ]; # toolkit"#;
        assert_eq!(
            detect_integration_status(Some(configuration), None),
            IntegrationStatus::Integrated
        );
    }

    #[test]
    fn is_allowed_managed_path_rejects_escape() {
        use super::paths;
        use std::path::Path;
        assert!(!paths::is_allowed_managed_path(Path::new(
            "../etc/nixos/nixos-toolkit/state/hostname.nix"
        )));
        assert!(!paths::is_allowed_managed_path(Path::new("/tmp/evil")));
        assert!(!paths::is_allowed_managed_path(Path::new(
            "/etc/nixos/nixos-toolkit/../../tmp/evil"
        )));
        assert!(!paths::is_allowed_managed_path(Path::new(
            "/etc/nixos/configuration.nix"
        )));
        assert!(paths::is_allowed_managed_path(Path::new(
            "/etc/nixos/nixos-toolkit/state/hostname.nix"
        )));
        assert!(paths::is_allowed_managed_path(Path::new(
            "/etc/nixos/nixos-toolkit/state/state.json.tmp"
        )));
    }
}

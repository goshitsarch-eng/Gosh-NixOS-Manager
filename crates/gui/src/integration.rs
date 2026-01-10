//! System detection and integration checking

use common::{ConfigMode, IntegrationStatus, SystemInfo};
use std::fs;
use std::path::Path;
use std::process::Command;

/// Detect system configuration and integration status
pub fn detect_system() -> SystemInfo {
    SystemInfo {
        is_nixos: is_nixos(),
        nixos_version: get_nixos_version(),
        config_mode: detect_config_mode(),
        config_path: find_config_path(),
        integration_status: detect_integration(),
        hostname: get_hostname(),
        current_desktop: get_current_desktop(),
    }
}

/// Check if this is a NixOS system
fn is_nixos() -> bool {
    Path::new("/etc/NIXOS").exists()
}

/// Get NixOS version from /etc/os-release
fn get_nixos_version() -> Option<String> {
    fs::read_to_string("/etc/os-release")
        .ok()
        .and_then(|content| {
            content
                .lines()
                .find(|l| l.starts_with("VERSION_ID="))
                .map(|l| l.trim_start_matches("VERSION_ID=").trim_matches('"').to_string())
        })
}

/// Detect whether using classic or flake configuration
fn detect_config_mode() -> ConfigMode {
    if Path::new("/etc/nixos/flake.nix").exists() {
        ConfigMode::Flake
    } else if Path::new("/etc/nixos/configuration.nix").exists() {
        ConfigMode::Classic
    } else {
        ConfigMode::Unknown
    }
}

/// Find the main configuration file path
fn find_config_path() -> Option<std::path::PathBuf> {
    let candidates = [
        "/etc/nixos/flake.nix",
        "/etc/nixos/configuration.nix",
    ];

    for path in candidates {
        let p = Path::new(path);
        if p.exists() {
            return Some(p.to_path_buf());
        }
    }

    None
}

/// Check if the nixos-toolkit module is imported
fn detect_integration() -> IntegrationStatus {
    // Check classic configuration.nix
    if let Ok(content) = fs::read_to_string("/etc/nixos/configuration.nix") {
        if content.contains("nixos-toolkit") || content.contains("./nixos-toolkit") {
            return IntegrationStatus::Integrated;
        }
    }

    // Check if the managed directory exists with selected.nix
    if Path::new("/etc/nixos/nixos-toolkit/state/selected.nix").exists() {
        // Directory exists, but we need to verify the import
        if let Ok(content) = fs::read_to_string("/etc/nixos/configuration.nix") {
            if content.contains("nixos-toolkit") {
                return IntegrationStatus::Integrated;
            }
        }
        // Directory exists but import not found
        return IntegrationStatus::NotIntegrated;
    }

    // Check for flake-based integration
    if let Ok(content) = fs::read_to_string("/etc/nixos/flake.nix") {
        if content.contains("nixos-toolkit") {
            return IntegrationStatus::Integrated;
        }
    }

    // Check if our managed directory exists at all
    if Path::new("/etc/nixos/nixos-toolkit").exists() {
        return IntegrationStatus::NotIntegrated;
    }

    IntegrationStatus::NotIntegrated
}

/// Get the current hostname
fn get_hostname() -> Option<String> {
    // Try /etc/hostname first
    if let Ok(hostname) = fs::read_to_string("/etc/hostname") {
        let hostname = hostname.trim().to_string();
        if !hostname.is_empty() {
            return Some(hostname);
        }
    }

    // Fall back to hostname command
    Command::new("hostname")
        .output()
        .ok()
        .and_then(|output| {
            if output.status.success() {
                String::from_utf8(output.stdout)
                    .ok()
                    .map(|s| s.trim().to_string())
            } else {
                None
            }
        })
}

/// Get the current desktop environment
fn get_current_desktop() -> Option<String> {
    std::env::var("XDG_CURRENT_DESKTOP").ok()
}

/// Generate integration snippet for classic configuration
pub fn classic_integration_snippet() -> String {
    r#"# Step 1: Create the toolkit directory (run once):
sudo mkdir -p /etc/nixos/nixos-toolkit/state

# Step 2: Create a placeholder file (run once):
sudo tee /etc/nixos/nixos-toolkit/state/selected.nix > /dev/null << 'EOF'
{ config, lib, pkgs, ... }: { imports = []; }
EOF

# Step 3: Add this import to /etc/nixos/configuration.nix:
  imports =
    [
      ./hardware-configuration.nix
      ./nixos-toolkit/state/selected.nix  # <-- Add this line
    ];

# Step 4: Rebuild your system:
sudo nixos-rebuild switch"#
    .to_string()
}

/// Generate integration snippet for flake configuration
pub fn flake_integration_snippet() -> String {
    r#"# Step 1: Create the toolkit directory (run once):
sudo mkdir -p /etc/nixos/nixos-toolkit/state

# Step 2: Create a placeholder file (run once):
sudo tee /etc/nixos/nixos-toolkit/state/selected.nix > /dev/null << 'EOF'
{ config, lib, pkgs, ... }: { imports = []; }
EOF

# Step 3: Add this import to your modules list in /etc/nixos/flake.nix:
  modules = [
    ./configuration.nix
    ./nixos-toolkit/state/selected.nix  # <-- Add this line
  ];

# Step 4: Rebuild your system:
sudo nixos-rebuild switch --flake .#"#
    .to_string()
}

/// Check if we can write to the managed directory
pub fn can_write_managed_dir() -> bool {
    let dir = Path::new("/etc/nixos/nixos-toolkit");
    if dir.exists() {
        // Check if writable
        fs::metadata(dir)
            .map(|m| !m.permissions().readonly())
            .unwrap_or(false)
    } else {
        // Check if parent is writable
        fs::metadata("/etc/nixos")
            .map(|m| !m.permissions().readonly())
            .unwrap_or(false)
    }
}

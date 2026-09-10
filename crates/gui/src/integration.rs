//! System detection and integration checking (unprivileged, no GTK).

use common::{ConfigMode, IntegrationStatus, SystemInfo};
use std::fs;
use std::path::Path;
use std::process::Command;

/// Detect system configuration and integration status.
#[must_use]
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

fn is_nixos() -> bool {
    Path::new("/etc/NIXOS").exists()
}

fn get_nixos_version() -> Option<String> {
    fs::read_to_string("/etc/os-release")
        .ok()
        .and_then(|content| {
            content
                .lines()
                .find(|l| l.starts_with("VERSION_ID="))
                .map(|l| {
                    l.trim_start_matches("VERSION_ID=")
                        .trim_matches('"')
                        .to_string()
                })
        })
}

fn detect_config_mode() -> ConfigMode {
    if Path::new("/etc/nixos/flake.nix").exists() {
        ConfigMode::Flake
    } else if Path::new("/etc/nixos/configuration.nix").exists() {
        ConfigMode::Classic
    } else {
        ConfigMode::Unknown
    }
}

fn find_config_path() -> Option<std::path::PathBuf> {
    let candidates = ["/etc/nixos/flake.nix", "/etc/nixos/configuration.nix"];
    for path in candidates {
        let p = Path::new(path);
        if p.exists() {
            return Some(p.to_path_buf());
        }
    }
    None
}

fn detect_integration() -> IntegrationStatus {
    if let Ok(content) = fs::read_to_string("/etc/nixos/configuration.nix") {
        if content.contains("nixos-toolkit") || content.contains("./nixos-toolkit") {
            return IntegrationStatus::Integrated;
        }
    }

    if Path::new("/etc/nixos/nixos-toolkit/state/selected.nix").exists() {
        if let Ok(content) = fs::read_to_string("/etc/nixos/configuration.nix") {
            if content.contains("nixos-toolkit") {
                return IntegrationStatus::Integrated;
            }
        }
        return IntegrationStatus::NotIntegrated;
    }

    if let Ok(content) = fs::read_to_string("/etc/nixos/flake.nix") {
        if content.contains("nixos-toolkit") {
            return IntegrationStatus::Integrated;
        }
    }

    if Path::new("/etc/nixos/nixos-toolkit").exists() {
        return IntegrationStatus::NotIntegrated;
    }

    IntegrationStatus::NotIntegrated
}

/// Current hostname from `/etc/hostname` or the `hostname` command.
#[must_use]
pub fn get_hostname() -> Option<String> {
    if let Ok(hostname) = fs::read_to_string("/etc/hostname") {
        let hostname = hostname.trim().to_string();
        if !hostname.is_empty() {
            return Some(hostname);
        }
    }

    Command::new("hostname").output().ok().and_then(|output| {
        if output.status.success() {
            String::from_utf8(output.stdout)
                .ok()
                .map(|s| s.trim().to_string())
        } else {
            None
        }
    })
}

fn get_current_desktop() -> Option<String> {
    std::env::var("XDG_CURRENT_DESKTOP").ok()
}

/// Generate integration snippet for classic configuration.
#[must_use]
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

/// Generate integration snippet for flake configuration.
#[must_use]
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

/// Check if we can write to the managed directory.
#[must_use]
pub fn can_write_managed_dir() -> bool {
    let dir = Path::new("/etc/nixos/nixos-toolkit");
    if dir.exists() {
        fs::metadata(dir)
            .map(|m| !m.permissions().readonly())
            .unwrap_or(false)
    } else {
        fs::metadata("/etc/nixos")
            .map(|m| !m.permissions().readonly())
            .unwrap_or(false)
    }
}

/// Detect GPU via `lspci`. Falls back to a placeholder when unavailable (Flatpak parity).
#[must_use]
pub fn detect_gpu() -> String {
    let output = Command::new("lspci").output().ok();
    if let Some(output) = output {
        let stdout = String::from_utf8_lossy(&output.stdout);
        for line in stdout.lines() {
            let lower = line.to_lowercase();
            if lower.contains("vga") || lower.contains("3d") || lower.contains("display") {
                if lower.contains("nvidia") {
                    return format!(
                        "NVIDIA: {}",
                        line.split(':').next_back().unwrap_or("Unknown").trim()
                    );
                } else if lower.contains("amd") || lower.contains("radeon") {
                    return format!(
                        "AMD: {}",
                        line.split(':').next_back().unwrap_or("Unknown").trim()
                    );
                } else if lower.contains("intel") {
                    return format!(
                        "Intel: {}",
                        line.split(':').next_back().unwrap_or("Unknown").trim()
                    );
                }
            }
        }
    }
    "Unknown GPU (lspci not available)".to_string()
}

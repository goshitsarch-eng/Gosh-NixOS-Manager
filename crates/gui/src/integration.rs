//! System detection and integration checking (unprivileged, no GTK).

use crate::helper::spawn::in_flatpak;
use common::config::detect_integration_status;
use common::{ConfigMode, IntegrationStatus, SystemInfo};
use std::fs;
use std::path::Path;
use std::process::{Command, Output};

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
    host_path_exists("/etc/NIXOS")
}

fn get_nixos_version() -> Option<String> {
    // Read the host os-release via spawn inside Flatpak; do not bind-mount it.
    host_read_to_string("/etc/os-release").and_then(|content| {
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
    if host_path_exists("/etc/nixos/flake.nix") {
        ConfigMode::Flake
    } else if host_path_exists("/etc/nixos/configuration.nix") {
        ConfigMode::Classic
    } else {
        ConfigMode::Unknown
    }
}

fn find_config_path() -> Option<std::path::PathBuf> {
    let candidates = ["/etc/nixos/flake.nix", "/etc/nixos/configuration.nix"];
    for path in candidates {
        if host_path_exists(path) {
            return Some(Path::new(path).to_path_buf());
        }
    }
    None
}

fn detect_integration() -> IntegrationStatus {
    let configuration_nix = host_read_to_string("/etc/nixos/configuration.nix");
    let flake_nix = host_read_to_string("/etc/nixos/flake.nix");
    detect_integration_status(configuration_nix.as_deref(), flake_nix.as_deref())
}

/// Current hostname from `/etc/hostname` or the `hostname` command.
#[must_use]
pub fn get_hostname() -> Option<String> {
    if let Some(hostname) = host_read_to_string("/etc/hostname") {
        let hostname = hostname.trim().to_string();
        if !hostname.is_empty() {
            return Some(hostname);
        }
    }

    host_command_output(&["hostname"]).and_then(|output| {
        if output.status.success() {
            String::from_utf8(output.stdout)
                .ok()
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
        } else {
            None
        }
    })
}

fn get_current_desktop() -> Option<String> {
    std::env::var("XDG_CURRENT_DESKTOP").ok()
}

/// True when `path` exists on the host (via `flatpak-spawn --host` in a sandbox).
#[must_use]
pub(crate) fn host_path_exists(path: &str) -> bool {
    if in_flatpak() {
        Command::new("flatpak-spawn")
            .args(["--host", "--", "test", "-e", path])
            .status()
            .map(|status| status.success())
            .unwrap_or(false)
    } else {
        Path::new(path).exists()
    }
}

/// Read a host file as UTF-8 (via `flatpak-spawn --host -- cat` in a sandbox).
#[must_use]
pub(crate) fn host_read_to_string(path: &str) -> Option<String> {
    if in_flatpak() {
        let output = Command::new("flatpak-spawn")
            .args(["--host", "--", "cat", path])
            .output()
            .ok()?;
        if !output.status.success() {
            return None;
        }
        String::from_utf8(output.stdout).ok()
    } else {
        fs::read_to_string(path).ok()
    }
}

fn host_command_output(args: &[&str]) -> Option<Output> {
    if in_flatpak() {
        Command::new("flatpak-spawn")
            .arg("--host")
            .arg("--")
            .args(args)
            .output()
            .ok()
    } else {
        let (program, rest) = args.split_first()?;
        Command::new(program).args(rest).output().ok()
    }
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

fn lspci_command() -> Command {
    if in_flatpak() {
        let mut cmd = Command::new("flatpak-spawn");
        cmd.args(["--host", "--", "lspci"]);
        cmd
    } else {
        Command::new("lspci")
    }
}

/// Parse `lspci` stdout. NVIDIA wins over other VGA/3D/display devices.
#[must_use]
pub(crate) fn gpu_from_lspci(stdout: &str) -> String {
    let mut fallback = None;
    for line in stdout.lines() {
        let lower = line.to_lowercase();
        if !(lower.contains("vga") || lower.contains("3d") || lower.contains("display")) {
            continue;
        }
        let name = line.split(':').next_back().unwrap_or("Unknown").trim();
        if lower.contains("nvidia") {
            return format!("NVIDIA: {name}");
        }
        if fallback.is_some() {
            continue;
        }
        if lower.contains("amd") || lower.contains("radeon") {
            fallback = Some(format!("AMD: {name}"));
        } else if lower.contains("intel") {
            fallback = Some(format!("Intel: {name}"));
        }
    }
    fallback.unwrap_or_else(|| "Unknown GPU (lspci not available)".to_string())
}

/// Detect GPU via `lspci`. Host-spawns inside Flatpak.
#[must_use]
pub fn detect_gpu() -> String {
    match lspci_command().output() {
        Ok(output) if output.status.success() => {
            gpu_from_lspci(&String::from_utf8_lossy(&output.stdout))
        }
        _ => "Unknown GPU (lspci not available)".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn host_path_helpers_use_local_fs_outside_flatpak() {
        if in_flatpak() {
            return;
        }
        let path = std::env::temp_dir().join(format!(
            "nixos-toolkit-host-read-{}.txt",
            std::process::id()
        ));
        std::fs::write(&path, "toolkit-host-probe\n").expect("write temp host probe file");
        let path_str = path.to_str().expect("utf-8 temp path");
        assert!(host_path_exists(path_str));
        assert_eq!(
            host_read_to_string(path_str).as_deref(),
            Some("toolkit-host-probe\n")
        );
        assert!(!host_path_exists("/no/such/nixos-toolkit-host-path"));
        assert!(host_read_to_string("/no/such/nixos-toolkit-host-path").is_none());
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn gpu_from_lspci_prefers_nvidia_on_hybrid() {
        let sample = "\
00:02.0 VGA compatible controller: Intel Corporation Arrow Lake-H [Intel Graphics]
01:00.0 VGA compatible controller: NVIDIA Corporation GB205M [GeForce RTX 5070 Ti Mobile] (rev a1)
";
        let gpu = gpu_from_lspci(sample);
        assert!(gpu.to_ascii_lowercase().contains("nvidia"));
        assert!(gpu.contains("GeForce RTX 5070"));
    }

    #[test]
    fn gpu_from_lspci_unknown_without_vga() {
        assert_eq!(
            gpu_from_lspci("00:1f.3 Audio device: Intel Corporation\n"),
            "Unknown GPU (lspci not available)"
        );
    }
}

//! Command handlers for helper requests

use crate::nix_gen;
use crate::rebuild;
use common::actions::{default_bundles, default_profiles};
use common::config::{paths, ConfigMode, IntegrationStatus, SystemInfo};
use common::ipc::{
    AppState, Generation, HardwareConfig, HelperResponse, LogLevel, NetworkConfig, RebuildType,
    ServicesConfig,
};
use std::fs;
use std::io::Write;
use std::path::Path;

/// Send a log message to the GUI
fn send_log(level: LogLevel, message: String) {
    let log = HelperResponse::Log { level, message };
    if let Ok(json) = serde_json::to_string(&log) {
        let mut stdout = std::io::stdout();
        let _ = writeln!(stdout, "{}", json);
        let _ = stdout.flush();
    }
}

/// Check if we have the required permissions
pub fn check_permissions() -> HelperResponse {
    let can_read_config = Path::new("/etc/nixos").exists() && fs::read_dir("/etc/nixos").is_ok();

    let can_write_managed =
        check_write_permission(paths::MANAGED_DIR) || check_write_permission("/etc/nixos");

    let can_run_rebuild = which("nixos-rebuild").is_some();

    HelperResponse::Permissions {
        can_read_config,
        can_write_managed,
        can_run_rebuild,
    }
}

fn check_write_permission(path: &str) -> bool {
    let p = Path::new(path);
    if p.exists() {
        // Try to open for writing
        fs::OpenOptions::new()
            .append(true)
            .open(p.join(".write_test"))
            .map(|_| {
                let _ = fs::remove_file(p.join(".write_test"));
                true
            })
            .unwrap_or(false)
    } else {
        // Check parent
        if let Some(parent) = p.parent() {
            check_write_permission(parent.to_str().unwrap_or(""))
        } else {
            false
        }
    }
}

fn which(cmd: &str) -> Option<std::path::PathBuf> {
    std::env::var_os("PATH").and_then(|paths| {
        std::env::split_paths(&paths)
            .filter_map(|dir| {
                let full_path = dir.join(cmd);
                if full_path.is_file() {
                    Some(full_path)
                } else {
                    None
                }
            })
            .next()
    })
}

/// Get system information
pub fn get_system_info() -> HelperResponse {
    let is_nixos = Path::new("/etc/NIXOS").exists();

    let nixos_version = fs::read_to_string("/etc/os-release")
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
        });

    let config_mode = if Path::new("/etc/nixos/flake.nix").exists() {
        ConfigMode::Flake
    } else if Path::new("/etc/nixos/configuration.nix").exists() {
        ConfigMode::Classic
    } else {
        ConfigMode::Unknown
    };

    let config_path = if Path::new("/etc/nixos/flake.nix").exists() {
        Some("/etc/nixos/flake.nix".into())
    } else if Path::new("/etc/nixos/configuration.nix").exists() {
        Some("/etc/nixos/configuration.nix".into())
    } else {
        None
    };

    let integration_status = detect_integration();

    let hostname = fs::read_to_string("/etc/hostname")
        .map(|s| s.trim().to_string())
        .ok();

    let current_desktop = std::env::var("XDG_CURRENT_DESKTOP").ok();

    HelperResponse::SystemInfo(SystemInfo {
        is_nixos,
        nixos_version,
        config_mode,
        config_path,
        integration_status,
        hostname,
        current_desktop,
    })
}

fn detect_integration() -> IntegrationStatus {
    // Check if selected.nix exists
    if !Path::new(paths::SELECTED_NIX).exists() {
        return IntegrationStatus::NotIntegrated;
    }

    // Check if it's imported in configuration.nix with correct path
    if let Ok(content) = fs::read_to_string("/etc/nixos/configuration.nix") {
        // Check for exact import paths that would work
        if content.contains("./nixos-toolkit/state/selected.nix")
            || content.contains("/etc/nixos/nixos-toolkit/state/selected.nix")
            || content.contains("nixos-toolkit/state/selected.nix")
        {
            return IntegrationStatus::Integrated;
        }
    }

    // Check flake.nix with correct path
    if let Ok(content) = fs::read_to_string("/etc/nixos/flake.nix") {
        if content.contains("nixos-toolkit/state/selected.nix") {
            return IntegrationStatus::Integrated;
        }
    }

    IntegrationStatus::NotIntegrated
}

/// Validate a configuration
pub fn validate(
    selected_profile: Option<String>,
    enabled_bundles: Vec<String>,
    hostname: Option<String>,
) -> HelperResponse {
    let mut errors = Vec::new();
    let mut warnings = Vec::new();

    // Validate profile using dynamic profile list
    if let Some(ref profile) = selected_profile {
        let valid_profiles: Vec<String> = default_profiles().into_iter().map(|p| p.id).collect();
        if !valid_profiles.contains(profile) {
            errors.push(format!("Unknown profile: {}", profile));
        }
    }

    // Validate bundles using dynamic bundle list
    let valid_bundles: Vec<String> = default_bundles().into_iter().map(|b| b.id).collect();
    for bundle in &enabled_bundles {
        if !valid_bundles.contains(bundle) {
            warnings.push(format!("Unknown bundle: {}", bundle));
        }
    }

    // Validate hostname
    if let Some(ref h) = hostname {
        if h.is_empty() {
            errors.push("Hostname cannot be empty".into());
        } else if !h.chars().all(|c| c.is_alphanumeric() || c == '-') {
            errors.push("Hostname contains invalid characters".into());
        } else if h.len() > 63 {
            errors.push("Hostname too long (max 63 characters)".into());
        }
    }

    // Check integration
    if !Path::new(paths::MANAGED_DIR).exists() {
        warnings.push("Managed directory does not exist yet".into());
    }

    HelperResponse::ValidationResult {
        valid: errors.is_empty(),
        errors,
        warnings,
    }
}

/// Generate configuration files
#[allow(clippy::too_many_arguments)]
pub fn generate(
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
    hardware_config: HardwareConfig,
    dry_run: bool,
) -> HelperResponse {
    // First ensure directories exist
    if !dry_run {
        if let HelperResponse::Error { message, details } = ensure_directories() {
            return HelperResponse::Error { message, details };
        }
    }

    // Generate files
    match nix_gen::generate_all_files(
        &selected_profile,
        &enabled_bundles,
        &bundle_packages,
        hostname.as_deref(),
        &dns_servers,
        &user_groups,
        username.as_deref(),
        bluetooth_enabled,
        &custom_packages,
        &network_config,
        &services_config,
        &hardware_config,
        dry_run,
    ) {
        Ok(files) => {
            // Generate preview
            let preview = files
                .iter()
                .map(|f| format!("=== {} ===\n{}\n", f.path, f.content))
                .collect::<Vec<_>>()
                .join("\n");

            HelperResponse::GenerationResult { files, preview }
        }
        Err(e) => HelperResponse::Error {
            message: "Failed to generate configuration".into(),
            details: Some(e.to_string()),
        },
    }
}

/// Apply configuration
#[allow(clippy::too_many_arguments)]
pub fn apply(
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
    hardware_config: HardwareConfig,
    rebuild_type: RebuildType,
) -> HelperResponse {
    // Ensure directories exist before generating files
    if let HelperResponse::Error { message, details } = ensure_directories() {
        return HelperResponse::Error { message, details };
    }

    // Generate files (not dry run)
    match nix_gen::generate_all_files(
        &selected_profile,
        &enabled_bundles,
        &bundle_packages,
        hostname.as_deref(),
        &dns_servers,
        &user_groups,
        username.as_deref(),
        bluetooth_enabled,
        &custom_packages,
        &network_config,
        &services_config,
        &hardware_config,
        false,
    ) {
        Ok(files) => {
            // Log each file that was written
            for f in &files {
                send_log(LogLevel::Info, format!("Wrote: {}", f.path));
            }
            send_log(
                LogLevel::Info,
                format!("Generated {} configuration files", files.len()),
            );
        }
        Err(e) => {
            return HelperResponse::Error {
                message: "Failed to generate configuration".into(),
                details: Some(e.to_string()),
            };
        }
    }

    // Detect config mode for rebuild
    let config_mode = if Path::new("/etc/nixos/flake.nix").exists() {
        ConfigMode::Flake
    } else {
        ConfigMode::Classic
    };

    // Run nixos-rebuild
    rebuild::run_rebuild(rebuild_type, config_mode)
}

/// Ensure required directories exist and create placeholder files
pub fn ensure_directories() -> HelperResponse {
    let dirs = [
        paths::MANAGED_DIR,
        paths::STATE_DIR,
        paths::PROFILES_DIR,
        paths::BUNDLES_DIR,
    ];

    for dir in dirs {
        if let Err(e) = fs::create_dir_all(dir) {
            return HelperResponse::Error {
                message: format!("Failed to create directory: {}", dir),
                details: Some(e.to_string()),
            };
        }
    }

    // Create placeholder selected.nix if it doesn't exist
    // This allows users to add the import to configuration.nix before making selections
    if !Path::new(paths::SELECTED_NIX).exists() {
        let placeholder = r#"# NixOS Toolkit - Managed Configuration
# This file is managed by nixos-toolkit.
#
# No profiles or bundles have been selected yet.
# Use the NixOS Toolkit app to select a desktop profile and bundles,
# then click "Apply" to generate your configuration.

{ config, lib, pkgs, ... }:

{
  imports = [
    # Profiles and bundles will be added here when you apply changes
  ];
}
"#;
        if let Err(e) = fs::write(paths::SELECTED_NIX, placeholder) {
            return HelperResponse::Error {
                message: "Failed to create placeholder selected.nix".into(),
                details: Some(e.to_string()),
            };
        }
        tracing::info!("Created placeholder {}", paths::SELECTED_NIX);
    }

    HelperResponse::Ok
}

/// Read application state from state.json, falling back to parsing existing Nix files
pub fn read_state() -> HelperResponse {
    match fs::read_to_string(paths::STATE_JSON) {
        Ok(content) => match serde_json::from_str::<AppState>(&content) {
            Ok(state) => HelperResponse::State(state),
            Err(e) => HelperResponse::Error {
                message: "Failed to parse state".into(),
                details: Some(e.to_string()),
            },
        },
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            // state.json doesn't exist - try to reconstruct from existing Nix files
            tracing::info!("state.json not found, attempting to reconstruct from Nix files");
            let state = reconstruct_state_from_nix();
            HelperResponse::State(state)
        }
        Err(e) => HelperResponse::Error {
            message: "Failed to read state".into(),
            details: Some(e.to_string()),
        },
    }
}

/// Reconstruct application state by parsing existing Nix configuration files
fn reconstruct_state_from_nix() -> AppState {
    let mut state = AppState::default();

    // Parse selected.nix to find enabled bundles and profile
    if let Ok(content) = fs::read_to_string(paths::SELECTED_NIX) {
        // Extract profile from imports like "../profiles/gnome.nix"
        for line in content.lines() {
            let line = line.trim();
            if line.contains("../profiles/") && line.ends_with(".nix") {
                if let Some(profile_id) = line
                    .strip_prefix("../profiles/")
                    .or_else(|| line.split("../profiles/").nth(1))
                {
                    let profile_id = profile_id.trim_end_matches(".nix").trim_matches('"').trim();
                    if !profile_id.is_empty() {
                        state.selected_profile = Some(profile_id.to_string());
                        tracing::info!("Reconstructed profile: {}", profile_id);
                    }
                }
            }

            // Extract bundles from imports like "../bundles/gaming.nix"
            if line.contains("../bundles/") && line.ends_with(".nix") {
                if let Some(bundle_part) = line.split("../bundles/").nth(1) {
                    let bundle_id = bundle_part
                        .trim_end_matches(".nix")
                        .trim_matches('"')
                        .trim();
                    if !bundle_id.is_empty() {
                        state.enabled_bundles.push(bundle_id.to_string());
                        tracing::info!("Reconstructed bundle: {}", bundle_id);
                    }
                }
            }
        }
    }

    // Parse custom-packages.nix to find custom packages
    if let Ok(content) = fs::read_to_string(paths::CUSTOM_PACKAGES_NIX) {
        let mut in_packages_block = false;
        for line in content.lines() {
            let line = line.trim();

            // Detect start of package list
            if line.contains("environment.systemPackages") {
                in_packages_block = true;
                continue;
            }

            // Detect end of package list
            if in_packages_block && line.starts_with("];") {
                break;
            }

            // Extract package names (skip comments and empty lines)
            if in_packages_block
                && !line.is_empty()
                && !line.starts_with('#')
                && !line.starts_with('[')
            {
                let pkg = line.trim_end_matches(';').trim();
                if !pkg.is_empty() && pkg != "with pkgs;" {
                    state.custom_packages.push(pkg.to_string());
                    tracing::info!("Reconstructed custom package: {}", pkg);
                }
            }
        }
    }

    // Parse hostname.nix if it exists
    if let Ok(content) = fs::read_to_string(paths::HOSTNAME_NIX) {
        for line in content.lines() {
            if line.contains("networking.hostName") {
                if let Some(hostname) = line.split('"').nth(1) {
                    state.hostname = Some(hostname.to_string());
                    tracing::info!("Reconstructed hostname: {}", hostname);
                }
            }
        }
    }

    if state.selected_profile.is_some()
        || !state.enabled_bundles.is_empty()
        || !state.custom_packages.is_empty()
    {
        tracing::info!("Successfully reconstructed state from existing Nix files");
    }

    state
}

/// Write application state to state.json
pub fn write_state(state: AppState) -> HelperResponse {
    // Ensure directory exists
    if let Err(e) = fs::create_dir_all(paths::STATE_DIR) {
        return HelperResponse::Error {
            message: "Failed to create state directory".into(),
            details: Some(e.to_string()),
        };
    }

    match serde_json::to_string_pretty(&state) {
        Ok(content) => {
            // Atomic write
            let temp_path = format!("{}.tmp", paths::STATE_JSON);
            if let Err(e) = fs::write(&temp_path, &content) {
                return HelperResponse::Error {
                    message: "Failed to write state".into(),
                    details: Some(e.to_string()),
                };
            }
            if let Err(e) = fs::rename(&temp_path, paths::STATE_JSON) {
                return HelperResponse::Error {
                    message: "Failed to save state".into(),
                    details: Some(e.to_string()),
                };
            }
            HelperResponse::Ok
        }
        Err(e) => HelperResponse::Error {
            message: "Failed to serialize state".into(),
            details: Some(e.to_string()),
        },
    }
}

/// List all NixOS system generations
pub fn list_generations() -> HelperResponse {
    use std::process::Command;

    let output = Command::new("nix-env")
        .args(["--list-generations", "-p", "/nix/var/nix/profiles/system"])
        .output();

    match output {
        Ok(output) => {
            if !output.status.success() {
                return HelperResponse::Error {
                    message: "Failed to list generations".into(),
                    details: Some(String::from_utf8_lossy(&output.stderr).to_string()),
                };
            }

            let stdout = String::from_utf8_lossy(&output.stdout);
            let mut generations = Vec::new();

            for line in stdout.lines() {
                if let Some(mut gen) = parse_generation_line(line) {
                    let (nixos_version, kernel_version) = generation_metadata(gen.number);
                    gen.nixos_version = nixos_version;
                    gen.kernel_version = kernel_version;
                    generations.push(gen);
                }
            }

            // Sort by generation number descending
            generations.sort_by_key(|a| std::cmp::Reverse(a.number));

            HelperResponse::Generations(generations)
        }
        Err(e) => HelperResponse::Error {
            message: "Failed to execute nix-env".into(),
            details: Some(e.to_string()),
        },
    }
}

fn parse_generation_line(line: &str) -> Option<Generation> {
    let line = line.trim();
    if line.is_empty() {
        return None;
    }

    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.is_empty() {
        return None;
    }

    let number: u32 = parts[0].parse().ok()?;
    let current = line.contains("(current)");

    // Try to extract date
    let date = if parts.len() >= 3 {
        format!("{} {}", parts[1], parts.get(2).unwrap_or(&""))
    } else {
        "Unknown".to_string()
    };

    Some(Generation {
        number,
        date,
        current,
        nixos_version: None,
        kernel_version: None,
        config_rev: None,
    })
}

/// NixOS and kernel versions from a generation path. Two filesystem reads; missing files stay None.
fn generation_metadata(number: u32) -> (Option<String>, Option<String>) {
    let gen_path = format!("/nix/var/nix/profiles/system-{number}-link");
    let nixos_version = fs::read_to_string(format!("{gen_path}/nixos-version"))
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());
    let kernel_version = fs::read_link(format!("{gen_path}/kernel"))
        .ok()
        .as_deref()
        .and_then(kernel_version_from_link);
    (nixos_version, kernel_version)
}

fn kernel_version_from_link(path: &Path) -> Option<String> {
    let path_str = path.to_string_lossy();
    path_str
        .split('/')
        .find(|part| part.contains("-linux-"))
        .and_then(|part| {
            part.find("-linux-")
                .map(|idx| part[idx + 7..].to_string())
                .filter(|s| !s.is_empty())
        })
}

/// Rollback to a specific generation
pub fn rollback_generation(generation: u32, activate: String) -> HelperResponse {
    use std::process::Command;

    if activate != "switch" && activate != "boot" {
        return HelperResponse::Error {
            message: "Invalid activate mode".into(),
            details: Some(format!(
                "activate must be \"switch\" or \"boot\", got {activate}"
            )),
        };
    }

    // Switch to the specified generation
    let switch_output = Command::new("nix-env")
        .args([
            "-p",
            "/nix/var/nix/profiles/system",
            "--switch-generation",
            &generation.to_string(),
        ])
        .output();

    match switch_output {
        Ok(output) if output.status.success() => {
            // Activate the generation (`switch` now or `boot` for next reboot).
            let activate_output =
                Command::new("/nix/var/nix/profiles/system/bin/switch-to-configuration")
                    .arg(&activate)
                    .output();

            match activate_output {
                Ok(output) if output.status.success() => HelperResponse::Ok,
                Ok(output) => HelperResponse::Error {
                    message: "Failed to activate generation".into(),
                    details: Some(String::from_utf8_lossy(&output.stderr).to_string()),
                },
                Err(e) => HelperResponse::Error {
                    message: "Failed to run switch-to-configuration".into(),
                    details: Some(e.to_string()),
                },
            }
        }
        Ok(output) => HelperResponse::Error {
            message: "Failed to switch generation".into(),
            details: Some(String::from_utf8_lossy(&output.stderr).to_string()),
        },
        Err(e) => HelperResponse::Error {
            message: "Failed to execute nix-env".into(),
            details: Some(e.to_string()),
        },
    }
}

/// Delete specific generations
pub fn delete_generations(generations: Vec<u32>) -> HelperResponse {
    use std::process::Command;

    let gen_args: Vec<String> = generations.iter().map(|g| g.to_string()).collect();

    let output = Command::new("nix-env")
        .args(["-p", "/nix/var/nix/profiles/system", "--delete-generations"])
        .args(&gen_args)
        .output();

    match output {
        Ok(output) if output.status.success() => HelperResponse::Ok,
        Ok(output) => HelperResponse::Error {
            message: "Failed to delete generations".into(),
            details: Some(String::from_utf8_lossy(&output.stderr).to_string()),
        },
        Err(e) => HelperResponse::Error {
            message: "Failed to execute nix-env".into(),
            details: Some(e.to_string()),
        },
    }
}

/// Run a maintenance command
pub fn run_maintenance(command: String) -> HelperResponse {
    use std::process::Command;

    // Only allow specific maintenance commands for security
    let allowed_commands = [
        "nix-collect-garbage",
        "nix-collect-garbage -d",
        "nix-store --optimise",
        "nix-store --verify --check-contents",
        "nix-channel --update",
    ];

    if !allowed_commands.contains(&command.as_str()) {
        return HelperResponse::Error {
            message: "Command not allowed".into(),
            details: Some(format!(
                "Only these commands are allowed: {:?}",
                allowed_commands
            )),
        };
    }

    // Parse and execute command
    let parts: Vec<&str> = command.split_whitespace().collect();
    if parts.is_empty() {
        return HelperResponse::Error {
            message: "Empty command".into(),
            details: None,
        };
    }

    let output = Command::new(parts[0]).args(&parts[1..]).output();

    match output {
        Ok(output) => HelperResponse::MaintenanceOutput {
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            success: output.status.success(),
        },
        Err(e) => HelperResponse::Error {
            message: "Failed to execute command".into(),
            details: Some(e.to_string()),
        },
    }
}

/// Get disk usage information for Nix store and generations
pub fn get_disk_usage() -> HelperResponse {
    use std::process::Command;

    let mut store_size = "Unknown".to_string();
    let mut generation_count = 0u32;
    let mut error = None;

    // Get Nix store size using du
    match Command::new("du").args(["-sh", "/nix/store"]).output() {
        Ok(output) if output.status.success() => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            // du output format: "45G\t/nix/store"
            if let Some(size) = stdout.split_whitespace().next() {
                store_size = size.to_string();
            }
        }
        Ok(output) => {
            let stderr = String::from_utf8_lossy(&output.stderr);
            error = Some(format!("du failed: {}", stderr.trim()));
        }
        Err(e) => {
            error = Some(format!("Failed to run du: {}", e));
        }
    }

    // Count generations using nix-env
    match Command::new("nix-env")
        .args(["--list-generations", "-p", "/nix/var/nix/profiles/system"])
        .output()
    {
        Ok(output) if output.status.success() => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            generation_count = stdout.lines().filter(|l| !l.trim().is_empty()).count() as u32;
        }
        Ok(output) => {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let err_msg = format!("nix-env failed: {}", stderr.trim());
            error = error
                .map(|e| format!("{}; {}", e, err_msg))
                .or(Some(err_msg));
        }
        Err(e) => {
            let err_msg = format!("Failed to run nix-env: {}", e);
            error = error
                .map(|e| format!("{}; {}", e, err_msg))
                .or(Some(err_msg));
        }
    }

    HelperResponse::DiskUsage(common::ipc::DiskUsageInfo {
        store_size,
        generation_count,
        error,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kernel_version_from_store_path() {
        let path = Path::new("/nix/store/xxx-linux-6.1.0/bzImage");
        assert_eq!(kernel_version_from_link(path).as_deref(), Some("6.1.0"));
    }

    #[test]
    fn rollback_rejects_unknown_activate() {
        match rollback_generation(1, "explode".into()) {
            HelperResponse::Error { message, details } => {
                assert!(message.contains("Invalid"));
                assert!(details.is_some_and(|d| d.contains("explode")));
            }
            other => panic!("expected Error, got {other:?}"),
        }
    }
}

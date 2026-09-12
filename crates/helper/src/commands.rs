//! Command handlers for helper requests

use crate::nix_gen;
use crate::rebuild;
use common::actions::{default_bundles, default_profiles};
use common::config::{detect_integration_status, paths, ConfigMode, IntegrationStatus, SystemInfo};
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
    if p.is_dir() {
        // Ephemeral probe inside `path`; deleted on drop. Never uses a stable `.write_test` name.
        tempfile::Builder::new()
            .prefix(".write_test-")
            .tempfile_in(p)
            .is_ok()
    } else if let Some(parent) = p.parent() {
        parent.to_str().is_some_and(check_write_permission)
    } else {
        false
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
    let selected_exists = Path::new(paths::SELECTED_NIX).exists();
    if selected_exists {
        tracing::info!("selected.nix exists at {}", paths::SELECTED_NIX);
    } else {
        tracing::info!(
            "selected.nix is not present yet; integration still counts if configuration.nix or flake.nix imports it"
        );
    }

    let configuration_nix = fs::read_to_string("/etc/nixos/configuration.nix").ok();
    let flake_nix = fs::read_to_string("/etc/nixos/flake.nix").ok();
    detect_integration_status(configuration_nix.as_deref(), flake_nix.as_deref())
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
    let dry = matches!(rebuild_type, RebuildType::DryBuild);
    // Snapshot before EnsureDirectories so a first-time dry-build can restore
    // "managed dir did not exist" instead of leaving the placeholder tree.
    let snapshot = if dry {
        match snapshot_managed_dir() {
            Ok(snap) => Some(snap),
            Err(e) => {
                return HelperResponse::Error {
                    message: "Failed to snapshot managed configuration for dry-run".into(),
                    details: Some(e.to_string()),
                };
            }
        }
    } else {
        None
    };

    if let HelperResponse::Error { message, details } = ensure_directories() {
        let ensure_err = HelperResponse::Error { message, details };
        if let Some(snapshot) = snapshot {
            return dry_build_response(ensure_err, restore_managed_snapshot(snapshot));
        }
        return ensure_err;
    }

    // Dry-build still writes so nixos-rebuild can evaluate the proposed tree;
    // the snapshot is restored afterwards so nothing is left on disk.
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
            for f in &files {
                let prefix = if dry { "Would write" } else { "Wrote" };
                send_log(LogLevel::Info, format!("{prefix}: {}", f.path));
            }
            send_log(
                LogLevel::Info,
                format!("Generated {} configuration files", files.len()),
            );
        }
        Err(e) => {
            let generate_err = HelperResponse::Error {
                message: "Failed to generate configuration".into(),
                details: Some(e.to_string()),
            };
            if let Some(snapshot) = snapshot {
                return dry_build_response(generate_err, restore_managed_snapshot(snapshot));
            }
            return generate_err;
        }
    }

    // Detect config mode for rebuild
    let config_mode = if Path::new("/etc/nixos/flake.nix").exists() {
        ConfigMode::Flake
    } else {
        ConfigMode::Classic
    };

    let response = rebuild::run_rebuild(rebuild_type, config_mode);
    if let Some(snapshot) = snapshot {
        return dry_build_response(response, restore_managed_snapshot(snapshot));
    }
    response
}

/// Restore a dry-build snapshot. Returns the restore error, if any.
fn restore_managed_snapshot(snapshot: ManagedDirSnapshot) -> Option<String> {
    match snapshot.restore() {
        Ok(()) => {
            send_log(
                LogLevel::Info,
                "Previous managed configuration restored.".into(),
            );
            None
        }
        Err(e) => {
            send_log(
                LogLevel::Error,
                format!("Restoring managed files failed: {e}"),
            );
            Some(e.to_string())
        }
    }
}

/// Combine a dry-build rebuild result with snapshot restore outcome.
///
/// Restore failure always becomes [`HelperResponse::Error`]. If the rebuild
/// also failed, the message mentions both.
fn dry_build_response(rebuild: HelperResponse, restore_err: Option<String>) -> HelperResponse {
    let Some(restore_err) = restore_err else {
        return rebuild;
    };

    match rebuild {
        HelperResponse::ApplyComplete {
            success: false,
            message,
        } => HelperResponse::Error {
            message: format!(
                "Dry-build failed ({message}) and the managed tree could not be restored"
            ),
            details: Some(restore_err),
        },
        HelperResponse::Error { message, details } => {
            let rebuild_details = details.unwrap_or_default();
            HelperResponse::Error {
                message: format!(
                    "Dry-build failed ({message}) and the managed tree could not be restored"
                ),
                details: Some(if rebuild_details.is_empty() {
                    restore_err
                } else {
                    format!("rebuild: {rebuild_details}; restore: {restore_err}")
                }),
            }
        }
        _ => HelperResponse::Error {
            message: "Dry-build finished but the managed tree could not be restored".into(),
            details: Some(restore_err),
        },
    }
}

struct ManagedDirSnapshot {
    backup: Option<tempfile::TempDir>,
    existed: bool,
}

fn snapshot_managed_dir() -> anyhow::Result<ManagedDirSnapshot> {
    let src = Path::new(paths::MANAGED_DIR);
    if !src.exists() {
        return Ok(ManagedDirSnapshot {
            backup: None,
            existed: false,
        });
    }
    let backup = tempfile::Builder::new()
        .prefix("nixos-toolkit-dryrun-")
        .tempdir()?;
    copy_dir_all(src, backup.path())?;
    Ok(ManagedDirSnapshot {
        backup: Some(backup),
        existed: true,
    })
}

impl ManagedDirSnapshot {
    fn restore(self) -> anyhow::Result<()> {
        let dest = Path::new(paths::MANAGED_DIR);
        if !self.existed {
            if dest.exists() {
                fs::remove_dir_all(dest)?;
            }
            return Ok(());
        }
        let backup = self
            .backup
            .as_ref()
            .expect("existed snapshot always has a backup dir");
        if dest.exists() {
            fs::remove_dir_all(dest)?;
        }
        copy_dir_all(backup.path(), dest)?;
        Ok(())
    }
}

fn copy_dir_all(src: &Path, dst: &Path) -> anyhow::Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let dest = dst.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_dir_all(&entry.path(), &dest)?;
        } else {
            fs::copy(entry.path(), dest)?;
        }
    }
    Ok(())
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
        if let Err(e) = crate::nix_gen::atomic_write(paths::SELECTED_NIX, placeholder) {
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
            Err(e) => {
                tracing::warn!(
                    "Failed to parse state.json ({e}); reconstructing from managed Nix files"
                );
                send_log(
                    LogLevel::Warning,
                    format!("state.json was unreadable ({e}); reconstructing from Nix files"),
                );
                HelperResponse::State(reconstruct_state_from_nix())
            }
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

    if let Ok(content) = fs::read_to_string(paths::SELECTED_NIX) {
        let (profile, bundles) = parse_selected_nix(&content);
        if let Some(ref profile_id) = profile {
            tracing::info!("Reconstructed profile: {profile_id}");
        }
        for bundle_id in &bundles {
            tracing::info!("Reconstructed bundle: {bundle_id}");
        }
        state.selected_profile = profile;
        for bundle_id in &bundles {
            if let Some(packages) = reconstruct_bundle_packages(bundle_id) {
                tracing::info!(
                    "Reconstructed {} package selection(s) for bundle {bundle_id}",
                    packages.len()
                );
                state.bundle_packages.insert(bundle_id.clone(), packages);
            }
        }
        state.enabled_bundles = bundles;
    }

    if let Ok(content) = fs::read_to_string(paths::CUSTOM_PACKAGES_NIX) {
        state.custom_packages = parse_custom_packages_snippet(&content);
        for pkg in &state.custom_packages {
            tracing::info!("Reconstructed custom package: {pkg}");
        }
    }

    if let Ok(content) = fs::read_to_string(paths::HOSTNAME_NIX) {
        if let Some(hostname) = parse_hostname_snippet(&content) {
            tracing::info!("Reconstructed hostname: {hostname}");
            state.hostname = Some(hostname);
        }
    }

    if let Ok(content) = fs::read_to_string(paths::DNS_NIX) {
        state.dns_servers = parse_dns_snippet(&content);
        if !state.dns_servers.is_empty() {
            tracing::info!(
                "Reconstructed DNS servers: {}",
                state.dns_servers.join(", ")
            );
        }
    }

    if let Ok(content) = fs::read_to_string(paths::USERS_NIX) {
        let (username, groups) = parse_users_snippet(&content);
        if let Some(ref user) = username {
            tracing::info!("Reconstructed username: {user}");
        }
        if !groups.is_empty() {
            tracing::info!("Reconstructed user groups: {}", groups.join(", "));
        }
        state.username = username;
        state.user_groups = groups;
    }

    if let Ok(content) = fs::read_to_string(paths::HARDWARE_NIX) {
        state.hardware_config = parse_hardware_snippet(&content);
        state.bluetooth_enabled = state.hardware_config.bluetooth_enabled;
        tracing::info!(
            "Reconstructed hardware (bluetooth={}, nvidia={:?}, audio={})",
            state.hardware_config.bluetooth_enabled,
            state.hardware_config.nvidia_driver,
            state.hardware_config.audio_server
        );
    }

    if let Ok(content) = fs::read_to_string(paths::NETWORK_NIX) {
        state.network_config = parse_network_snippet(&content);
        tracing::info!(
            "Reconstructed network (firewall={}, ssh={}, tailscale={}, wireguard={})",
            state.network_config.firewall_enabled,
            state.network_config.ssh_enabled,
            state.network_config.tailscale_enabled,
            state.network_config.wireguard_enabled
        );
    }

    if let Ok(content) = fs::read_to_string(paths::SERVICES_NIX) {
        state.services_config = parse_services_snippet(&content);
        tracing::info!(
            "Reconstructed services: {}",
            state.services_config.enabled_services().join(", ")
        );
    }

    if state.selected_profile.is_some()
        || !state.enabled_bundles.is_empty()
        || !state.custom_packages.is_empty()
        || state.hostname.is_some()
        || !state.dns_servers.is_empty()
        || state.username.is_some()
        || state.hardware_config.has_settings()
        || state.network_config.has_settings()
        || state.services_config.has_settings()
    {
        tracing::info!("Successfully reconstructed state from existing Nix files");
    }

    state
}

fn skip_nix_comment(line: &str) -> Option<&str> {
    let line = line.trim();
    if line.is_empty() || line.starts_with('#') {
        None
    } else {
        Some(line)
    }
}

/// `lhs = true` / `lhs = false` on a non-comment line, ignoring a trailing `;`.
fn nix_assignment_bool(line: &str) -> Option<(&str, bool)> {
    let line = skip_nix_comment(line)?;
    let line = line.trim_end_matches(';').trim();
    let (lhs, rhs) = line.split_once('=')?;
    let lhs = lhs.trim();
    let val = match rhs.trim() {
        "true" => true,
        "false" => false,
        _ => return None,
    };
    Some((lhs, val))
}

fn parse_quoted_strings(s: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c != '"' {
            continue;
        }
        let mut cur = String::new();
        while let Some(ch) = chars.next() {
            match ch {
                '\\' => {
                    if let Some(n) = chars.next() {
                        cur.push(n);
                    }
                }
                '"' => break,
                _ => cur.push(ch),
            }
        }
        if !cur.is_empty() {
            out.push(cur);
        }
    }
    out
}

fn parse_u16_list(line: &str) -> Vec<u16> {
    let Some(start) = line.find('[') else {
        return Vec::new();
    };
    let rest = &line[start + 1..];
    let Some(end) = rest.find(']') else {
        return Vec::new();
    };
    rest[..end]
        .split_whitespace()
        .filter_map(|tok| tok.trim_matches(',').parse().ok())
        .collect()
}

fn update_nix_context(context: &mut String, line: &str) {
    let Some(trimmed) = skip_nix_comment(line) else {
        return;
    };
    if trimmed == "};" || trimmed == "}" {
        context.clear();
        return;
    }
    if let Some(eq) = trimmed.find('=') {
        if trimmed.contains('{') {
            *context = trimmed[..eq].trim().to_string();
        }
    }
}

fn parse_selected_nix(content: &str) -> (Option<String>, Vec<String>) {
    let mut profile = None;
    let mut bundles = Vec::new();
    for line in content.lines() {
        let Some(line) = skip_nix_comment(line) else {
            continue;
        };
        if let Some(rest) = line.split("../profiles/").nth(1) {
            let id = rest
                .split(".nix")
                .next()
                .unwrap_or("")
                .trim_matches(|c: char| c == '"' || c.is_whitespace());
            if !id.is_empty() {
                profile = Some(id.to_string());
            }
        }
        if let Some(rest) = line.split("../bundles/").nth(1) {
            let id = rest
                .split(".nix")
                .next()
                .unwrap_or("")
                .trim_matches(|c: char| c == '"' || c.is_whitespace());
            if !id.is_empty() {
                bundles.push(id.to_string());
            }
        }
    }
    (profile, bundles)
}

/// Reconstruct catalog package ids from a generated fallback bundle file.
///
/// Full templates (`# Managed by NixOS Toolkit`) are left alone so the next
/// apply still copies the template instead of switching to a packages fallback.
fn reconstruct_bundle_packages(bundle_id: &str) -> Option<Vec<String>> {
    let path = format!("{}/{}.nix", paths::BUNDLES_DIR, bundle_id);
    let content = fs::read_to_string(path).ok()?;
    if !content.contains("Generated by NixOS Toolkit") {
        return None;
    }
    let attrs = parse_bundle_package_attrs(&content);
    if attrs.is_empty() {
        return Some(Vec::new());
    }
    Some(
        attrs
            .into_iter()
            .map(|attr| catalog_id_for_nix_attr(&attr))
            .collect(),
    )
}

fn parse_bundle_package_attrs(content: &str) -> Vec<String> {
    let mut packages = Vec::new();
    let mut in_packages_block = false;
    for line in content.lines() {
        let line = line.trim();
        if line.contains("environment.systemPackages") || line.contains("fonts.packages") {
            in_packages_block = true;
            continue;
        }
        if in_packages_block && (line.starts_with("];") || line == "]") {
            break;
        }
        if in_packages_block {
            let Some(line) = skip_nix_comment(line) else {
                continue;
            };
            if line.starts_with('[') {
                continue;
            }
            let pkg = line.trim_end_matches(';').trim();
            if !pkg.is_empty() && pkg != "with pkgs;" {
                packages.push(pkg.to_string());
            }
        }
    }
    packages
}

fn catalog_id_for_nix_attr(attr: &str) -> String {
    for bundle in default_bundles() {
        for package in &bundle.packages {
            if package.resolved_nix_attr() == attr || package.id == attr {
                return package.id.clone();
            }
        }
    }
    attr.to_string()
}

fn parse_custom_packages_snippet(content: &str) -> Vec<String> {
    let mut packages = Vec::new();
    let mut in_packages_block = false;
    for line in content.lines() {
        let line = line.trim();
        if line.contains("environment.systemPackages") {
            in_packages_block = true;
            continue;
        }
        if in_packages_block && line.starts_with("];") {
            break;
        }
        if in_packages_block {
            let Some(line) = skip_nix_comment(line) else {
                continue;
            };
            if line.starts_with('[') {
                continue;
            }
            let pkg = line.trim_end_matches(';').trim();
            if !pkg.is_empty() && pkg != "with pkgs;" {
                packages.push(pkg.to_string());
            }
        }
    }
    packages
}

fn parse_hostname_snippet(content: &str) -> Option<String> {
    for line in content.lines() {
        let Some(line) = skip_nix_comment(line) else {
            continue;
        };
        if line.contains("networking.hostName") {
            if let Some(hostname) = line.split('"').nth(1) {
                if !hostname.is_empty() {
                    return Some(hostname.to_string());
                }
            }
        }
    }
    None
}

fn parse_dns_snippet(content: &str) -> Vec<String> {
    let mut servers = Vec::new();
    let mut in_list = false;
    for line in content.lines() {
        let Some(trimmed) = skip_nix_comment(line) else {
            continue;
        };
        if trimmed.contains("networking.nameservers") {
            in_list = true;
        }
        if in_list {
            servers.extend(parse_quoted_strings(trimmed));
            if trimmed.contains(']') {
                break;
            }
        }
    }
    servers
}

fn parse_users_snippet(content: &str) -> (Option<String>, Vec<String>) {
    let mut username = None;
    let mut groups = Vec::new();
    for line in content.lines() {
        let Some(line) = skip_nix_comment(line) else {
            continue;
        };
        if let Some(rest) = line.strip_prefix("users.users.") {
            let rest = rest.trim();
            if let Some(name) = rest.strip_prefix('"') {
                if let Some(end) = name.find('"') {
                    let name = &name[..end];
                    if !name.is_empty() {
                        username = Some(name.to_string());
                    }
                }
            } else {
                let name = rest
                    .split_whitespace()
                    .next()
                    .unwrap_or("")
                    .trim_end_matches('=')
                    .trim();
                if !name.is_empty() {
                    username = Some(name.to_string());
                }
            }
        }
        if line.contains("extraGroups") {
            groups = parse_quoted_strings(line);
        }
    }
    (username, groups)
}

fn parse_hardware_snippet(content: &str) -> HardwareConfig {
    let mut hw = HardwareConfig::default();
    let mut context = String::new();
    let mut pulse: Option<bool> = None;
    let mut pipewire: Option<bool> = None;

    for line in content.lines() {
        update_nix_context(&mut context, line);
        let Some(line) = skip_nix_comment(line) else {
            continue;
        };

        if line.contains("videoDrivers") {
            if line.contains("nouveau") {
                hw.nvidia_driver = Some(3);
            } else if line.contains("nvidia") && hw.nvidia_driver.is_none() {
                hw.nvidia_driver = Some(0);
            }
        }
        if line.contains("nvidiaPackages.beta") {
            hw.nvidia_driver = Some(1);
        } else if line.contains("nvidiaPackages.latest") {
            hw.nvidia_driver = Some(2);
        } else if line.contains("nvidiaPackages.stable") {
            hw.nvidia_driver = Some(0);
        }

        if line.contains("92-low-latency") {
            hw.audio_lowlatency = true;
        }
        if line.contains("powerprofilesctl set performance") {
            hw.power_profile = 1;
        } else if line.contains("powerprofilesctl set power-saver") {
            hw.power_profile = 2;
        } else if line.contains("powerprofilesctl set balanced") {
            hw.power_profile = 0;
        }

        if let Some((lhs, val)) = nix_assignment_bool(line) {
            if lhs.ends_with("modesetting.enable") {
                hw.nvidia_modesetting = val;
            }
            if lhs.ends_with("powerManagement.enable") {
                hw.nvidia_powermanagement = val;
            }
            if lhs == "open" && hw.nvidia_driver.is_some() {
                hw.nvidia_open = val;
            }
            if lhs.ends_with("services.pulseaudio.enable")
                || (context == "services.pulseaudio" && lhs == "enable")
            {
                pulse = Some(val);
            }
            if lhs.ends_with("services.pipewire.enable")
                || (context == "services.pipewire" && lhs == "enable")
            {
                pipewire = Some(val);
            }
            if (lhs.ends_with("hardware.bluetooth.enable")
                || (context == "hardware.bluetooth" && lhs == "enable"))
                && val
            {
                hw.bluetooth_enabled = true;
            }
            if (lhs == "powerOnBoot" || lhs.ends_with("hardware.bluetooth.powerOnBoot")) && val {
                hw.bluetooth_autopower = true;
            }
            if (lhs.ends_with("services.tlp.enable")
                || (context == "services.tlp" && lhs == "enable"))
                && val
            {
                hw.tlp_enabled = true;
            }
            if lhs.ends_with("services.thermald.enable") && val {
                hw.thermald_enabled = true;
            }
        }
    }

    if hw.audio_lowlatency {
        hw.audio_server = 0;
    } else if pulse == Some(true) {
        hw.audio_server = 1;
    } else if pipewire == Some(false) && pulse == Some(false) {
        hw.audio_server = 2;
    } else if pipewire == Some(true) {
        hw.audio_server = 0;
    }

    hw
}

fn parse_wireguard_listen_port_marker(line: &str) -> Option<u16> {
    let rest = line
        .trim()
        .strip_prefix("# nixos-toolkit.wireguardListenPort =")?;
    let port: u16 = rest.trim().trim_end_matches(';').trim().parse().ok()?;
    (port != 0).then_some(port)
}

fn parse_network_snippet(content: &str) -> NetworkConfig {
    let mut cfg = NetworkConfig::default();
    let mut context = String::new();

    for line in content.lines() {
        if let Some(port) = parse_wireguard_listen_port_marker(line) {
            cfg.wireguard_listen_port = port;
            continue;
        }
        update_nix_context(&mut context, line);
        let Some(line) = skip_nix_comment(line) else {
            continue;
        };

        if line.contains("allowedTCPPorts") {
            cfg.allowed_tcp_ports = parse_u16_list(line);
        }
        if line.contains("allowedUDPPorts") {
            cfg.allowed_udp_ports = parse_u16_list(line);
        }
        if context == "services.openssh" && line.contains("ports") {
            if let Some(port) = parse_u16_list(line).first().copied() {
                cfg.ssh_port = port;
            }
        }
        if line.contains("PermitRootLogin") {
            if let Some(value) = parse_quoted_strings(line).into_iter().next() {
                cfg.ssh_root_login = value;
            }
        }

        if let Some((lhs, val)) = nix_assignment_bool(line) {
            if lhs.ends_with("networking.firewall.enable")
                || (context == "networking.firewall" && lhs == "enable")
            {
                cfg.firewall_enabled = val;
            }
            if (lhs.ends_with("services.openssh.enable")
                || (context == "services.openssh" && lhs == "enable"))
                && val
            {
                cfg.ssh_enabled = true;
                if cfg.ssh_port == 0 {
                    cfg.ssh_port = 22;
                }
                if cfg.ssh_root_login.is_empty() {
                    cfg.ssh_root_login = "no".into();
                }
            }
            if lhs == "PasswordAuthentication" {
                cfg.ssh_password_auth = val;
            }
            if (lhs.ends_with("services.fail2ban.enable")
                || (context == "services.fail2ban" && lhs == "enable"))
                && val
            {
                cfg.fail2ban_enabled = true;
            }
            if lhs.ends_with("services.tailscale.enable") && val {
                cfg.tailscale_enabled = true;
            }
            if (lhs.ends_with("networking.wireguard.enable")
                || (context == "networking.wireguard" && lhs == "enable"))
                && val
            {
                cfg.wireguard_enabled = true;
            }
        }
    }

    cfg
}

fn line_has_pkg(line: &str, pkg: &str) -> bool {
    let Some(line) = skip_nix_comment(line) else {
        return false;
    };
    line.split(|c: char| !(c.is_ascii_alphanumeric() || c == '-' || c == '_'))
        .any(|tok| tok == pkg)
}

fn parse_services_snippet(content: &str) -> ServicesConfig {
    let mut cfg = ServicesConfig::default();
    let mut context = String::new();

    for line in content.lines() {
        update_nix_context(&mut context, line);
        let Some(line) = skip_nix_comment(line) else {
            continue;
        };

        if line_has_pkg(line, "rustdesk") {
            cfg.rustdesk = true;
        }
        if line_has_pkg(line, "gnome-tweaks") {
            cfg.gnome_tweaks = true;
        }

        if let Some((lhs, true)) = nix_assignment_bool(line) {
            if lhs.ends_with("services.printing.enable") {
                cfg.printing = true;
            }
            if lhs.ends_with("services.avahi.enable")
                || (context == "services.avahi" && lhs == "enable")
            {
                cfg.avahi = true;
            }
            if lhs.ends_with("services.fwupd.enable") {
                cfg.fwupd = true;
            }
            if lhs.ends_with("services.upower.enable") {
                cfg.upower = true;
            }
            if lhs.ends_with("networking.networkmanager.enable") {
                cfg.networkmanager = true;
            }
            if lhs.ends_with("services.resolved.enable") {
                cfg.resolved = true;
            }
            if lhs.ends_with("services.syncthing.enable") {
                cfg.syncthing = true;
            }
            if lhs.ends_with("services.locate.enable")
                || (context == "services.locate" && lhs == "enable")
            {
                cfg.locate = true;
            }
            if lhs.ends_with("services.flatpak.enable") {
                cfg.flatpak = true;
            }
            if lhs.ends_with("services.gnome.gnome-keyring.enable") {
                cfg.gnome_keyring = true;
            }
            if lhs.ends_with("programs.dconf.enable") {
                cfg.dconf = true;
            }
            if lhs.ends_with("virtualisation.docker.enable") {
                cfg.docker = true;
            }
            if lhs.ends_with("virtualisation.libvirtd.enable") {
                cfg.libvirtd = true;
            }
            if lhs.ends_with("services.postgresql.enable") {
                cfg.postgresql = true;
            }
            if lhs.contains("services.redis") && lhs.ends_with("enable") {
                cfg.redis = true;
            }
            if lhs.ends_with("services.earlyoom.enable") {
                cfg.earlyoom = true;
            }
            if lhs.ends_with("system.autoUpgrade.enable") {
                cfg.auto_upgrade = true;
            }
            if (context == "nix.gc" && lhs == "automatic") || lhs.ends_with("nix.gc.automatic") {
                cfg.auto_gc = true;
            }
            if lhs.ends_with("nix.settings.auto-optimise-store") {
                cfg.store_optimize = true;
            }
        }
    }

    cfg
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
        Ok(content) => match crate::nix_gen::atomic_write(paths::STATE_JSON, &content) {
            Ok(()) => HelperResponse::Ok,
            Err(e) => HelperResponse::Error {
                message: "Failed to write state".into(),
                details: Some(e.to_string()),
            },
        },
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

const NIX_CHANNEL_UPDATE: &str = "nix-channel --update";
const NIX_FLAKE_UPDATE: &str = "nix flake update --flake /etc/nixos";

const ALLOWED_MAINTENANCE_COMMANDS: &[&str] = &[
    "nix-collect-garbage",
    "nix-collect-garbage -d",
    "nix-store --optimise",
    "nix-store --verify --check-contents",
    NIX_CHANNEL_UPDATE,
    NIX_FLAKE_UPDATE,
];

/// Map a requested maintenance command to the exact string that will be run.
///
/// On flake hosts, `nix-channel --update` is rewritten to [`NIX_FLAKE_UPDATE`].
fn maintenance_command_to_run(requested: &str, flake_nix_exists: bool) -> Option<String> {
    if !ALLOWED_MAINTENANCE_COMMANDS.contains(&requested) {
        return None;
    }
    if requested == NIX_CHANNEL_UPDATE && flake_nix_exists {
        Some(NIX_FLAKE_UPDATE.to_string())
    } else {
        Some(requested.to_string())
    }
}

/// Run a maintenance command
pub fn run_maintenance(command: String) -> HelperResponse {
    use std::process::Command;

    let flake_nix_exists = Path::new("/etc/nixos/flake.nix").exists();
    let requested = command;
    let Some(command) = maintenance_command_to_run(&requested, flake_nix_exists) else {
        return HelperResponse::Error {
            message: "Command not allowed".into(),
            details: Some(format!(
                "Only these commands are allowed: {:?}",
                ALLOWED_MAINTENANCE_COMMANDS
            )),
        };
    };

    if requested != command {
        send_log(
            LogLevel::Info,
            "Detected flake at /etc/nixos/flake.nix; running nix flake update --flake /etc/nixos"
                .into(),
        );
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
    use common::nix::{
        generate_custom_packages_nix, generate_dns_nix, generate_fallback_bundle,
        generate_hardware_nix, generate_hostname_nix, generate_network_nix,
        generate_services_nix, generate_user_groups_nix,
    };

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

    #[test]
    fn dry_build_restore_success_keeps_rebuild_response() {
        let rebuild = HelperResponse::ApplyComplete {
            success: true,
            message: "ok".into(),
        };
        match dry_build_response(rebuild, None) {
            HelperResponse::ApplyComplete { success: true, .. } => {}
            other => panic!("expected ApplyComplete success, got {other:?}"),
        }
    }

    #[test]
    fn dry_build_restore_failure_after_success_is_error() {
        match dry_build_response(
            HelperResponse::ApplyComplete {
                success: true,
                message: "nixos-rebuild dry-build completed successfully".into(),
            },
            Some("permission denied".into()),
        ) {
            HelperResponse::Error { message, details } => {
                assert!(message.contains("managed tree could not be restored"));
                assert!(details.is_some_and(|d| d.contains("permission denied")));
            }
            other => panic!("expected Error, got {other:?}"),
        }
    }

    #[test]
    fn dry_build_restore_and_rebuild_failure_mentions_both() {
        match dry_build_response(
            HelperResponse::ApplyComplete {
                success: false,
                message: "nixos-rebuild dry-build failed with exit code 1".into(),
            },
            Some("restore io".into()),
        ) {
            HelperResponse::Error { message, details } => {
                assert!(message.contains("Dry-build failed"));
                assert!(message.contains("managed tree could not be restored"));
                assert!(details.is_some_and(|d| d.contains("restore io")));
            }
            other => panic!("expected Error, got {other:?}"),
        }
    }

    #[test]
    fn dry_build_restore_failure_with_rebuild_error_mentions_both() {
        match dry_build_response(
            HelperResponse::Error {
                message: "Failed to spawn nixos-rebuild".into(),
                details: Some("enoent".into()),
            },
            Some("restore io".into()),
        ) {
            HelperResponse::Error { message, details } => {
                assert!(message.contains("Failed to spawn nixos-rebuild"));
                assert!(message.contains("managed tree could not be restored"));
                assert!(details.is_some_and(|d| d.contains("enoent") && d.contains("restore io")));
            }
            other => panic!("expected Error, got {other:?}"),
        }
    }

    #[test]
    fn check_write_permission_uses_ephemeral_file() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().to_str().expect("utf-8 temp path");
        assert!(check_write_permission(path));
        let leftovers: Vec<_> = fs::read_dir(dir.path())
            .expect("read tempdir")
            .filter_map(Result::ok)
            .filter(|e| e.file_name().to_string_lossy().starts_with(".write_test"))
            .collect();
        assert!(
            leftovers.is_empty(),
            "expected no leftover .write_test files, found {leftovers:?}"
        );
    }

    #[test]
    fn check_write_permission_falls_back_to_parent() {
        let dir = tempfile::tempdir().expect("tempdir");
        let missing = dir.path().join("does-not-exist");
        assert!(check_write_permission(
            missing.to_str().expect("utf-8 temp path")
        ));
    }

    #[test]
    fn parse_hostname_snippet_from_generated() {
        let nix = generate_hostname_nix("desk-1");
        assert_eq!(parse_hostname_snippet(&nix).as_deref(), Some("desk-1"));
    }

    #[test]
    fn parse_dns_snippet_from_generated() {
        let nix = generate_dns_nix(&["1.1.1.1".into(), "8.8.8.8".into()]);
        assert_eq!(
            parse_dns_snippet(&nix),
            vec!["1.1.1.1".to_string(), "8.8.8.8".to_string()]
        );
    }

    #[test]
    fn parse_users_snippet_from_generated() {
        let nix = generate_user_groups_nix("gosh-1", &["libvirtd".into(), "docker".into()]);
        let (user, groups) = parse_users_snippet(&nix);
        assert_eq!(user.as_deref(), Some("gosh-1"));
        assert_eq!(groups, vec!["libvirtd".to_string(), "docker".to_string()]);
    }

    #[test]
    fn parse_custom_packages_snippet_from_generated() {
        let nix = generate_custom_packages_nix(&["htop".into(), "ripgrep".into()]);
        assert_eq!(
            parse_custom_packages_snippet(&nix),
            vec!["htop".to_string(), "ripgrep".to_string()]
        );
    }

    #[test]
    fn generated_fallback_bundle_maps_catalog_ids() {
        let nix = generate_fallback_bundle("security", &["bitwarden".into(), "keepassxc".into()]);
        assert!(nix.contains("Generated by NixOS Toolkit"));
        let attrs = parse_bundle_package_attrs(&nix);
        assert!(attrs.contains(&"bitwarden-desktop".to_string()));
        assert!(attrs.contains(&"keepassxc".to_string()));
        assert_eq!(catalog_id_for_nix_attr("bitwarden-desktop"), "bitwarden");
        assert_eq!(catalog_id_for_nix_attr("keepassxc"), "keepassxc");
    }

    #[test]
    fn managed_template_header_is_not_generated_fallback() {
        let nix = "# Fonts Collection Bundle\n# Managed by NixOS Toolkit\n";
        assert!(!nix.contains("Generated by NixOS Toolkit"));
    }

    #[test]
    fn parse_selected_nix_profile_and_bundles() {
        let nix = r#"
{ config, lib, pkgs, ... }:
{
  imports = [
    ../profiles/gnome.nix
    ../bundles/gaming.nix
    ../bundles/dev-tools.nix
    ./hostname.nix
  ];
}
"#;
        let (profile, bundles) = parse_selected_nix(nix);
        assert_eq!(profile.as_deref(), Some("gnome"));
        assert_eq!(bundles, vec!["gaming".to_string(), "dev-tools".to_string()]);
    }

    #[test]
    fn parse_hardware_bluetooth_tlp_thermald() {
        let hw = HardwareConfig {
            bluetooth_enabled: true,
            bluetooth_autopower: true,
            tlp_enabled: true,
            thermald_enabled: true,
            ..HardwareConfig::default()
        };
        let parsed = parse_hardware_snippet(&generate_hardware_nix(&hw));
        assert!(parsed.bluetooth_enabled);
        assert!(parsed.bluetooth_autopower);
        assert!(parsed.tlp_enabled);
        assert!(parsed.thermald_enabled);
    }

    #[test]
    fn parse_hardware_nvidia_beta_pulse_and_modesetting() {
        let hw = HardwareConfig {
            nvidia_driver: Some(1),
            nvidia_modesetting: false,
            nvidia_powermanagement: true,
            nvidia_open: true,
            audio_server: 1,
            ..HardwareConfig::default()
        };
        let parsed = parse_hardware_snippet(&generate_hardware_nix(&hw));
        assert_eq!(parsed.nvidia_driver, Some(1));
        assert!(!parsed.nvidia_modesetting);
        assert!(parsed.nvidia_powermanagement);
        assert!(parsed.nvidia_open);
        assert_eq!(parsed.audio_server, 1);
    }

    #[test]
    fn parse_hardware_nouveau_audio_none_and_power_profile() {
        let hw = HardwareConfig {
            nvidia_driver: Some(3),
            audio_server: 2,
            power_profile: 1,
            ..HardwareConfig::default()
        };
        let parsed = parse_hardware_snippet(&generate_hardware_nix(&hw));
        assert_eq!(parsed.nvidia_driver, Some(3));
        assert_eq!(parsed.audio_server, 2);
        assert_eq!(parsed.power_profile, 1);
    }

    #[test]
    fn parse_hardware_pipewire_lowlatency() {
        let hw = HardwareConfig {
            audio_server: 0,
            audio_lowlatency: true,
            ..HardwareConfig::default()
        };
        let parsed = parse_hardware_snippet(&generate_hardware_nix(&hw));
        assert_eq!(parsed.audio_server, 0);
        assert!(parsed.audio_lowlatency);
    }

    #[test]
    fn parse_network_snippet_firewall_ssh_vpn_and_wireguard() {
        let nix = r#"
{ config, lib, pkgs, ... }:
{
  networking.firewall = {
    enable = true;
    allowedTCPPorts = [ 80 443 ];
    allowedUDPPorts = [ 53 ];
  };
  services.openssh = {
    enable = true;
    ports = [ 2222 ];
    settings = {
      PasswordAuthentication = true;
      PermitRootLogin = "prohibit-password";
    };
  };
  services.fail2ban = {
    enable = true;
    jails.sshd = {
      enabled = true;
    };
  };
  services.tailscale.enable = true;
  networking.wireguard.enable = true;
}
"#;
        let parsed = parse_network_snippet(nix);
        assert!(parsed.firewall_enabled);
        assert_eq!(parsed.allowed_tcp_ports, vec![80, 443]);
        assert_eq!(parsed.allowed_udp_ports, vec![53]);
        assert!(parsed.ssh_enabled);
        assert_eq!(parsed.ssh_port, 2222);
        assert!(parsed.ssh_password_auth);
        assert_eq!(parsed.ssh_root_login, "prohibit-password");
        assert!(parsed.fail2ban_enabled);
        assert!(parsed.tailscale_enabled);
        assert!(parsed.wireguard_enabled);
        assert_eq!(parsed.wireguard_listen_port, 51820);
    }

    #[test]
    fn parse_network_snippet_round_trips_generated_wireguard_listen_port() {
        let cfg = NetworkConfig {
            wireguard_enabled: true,
            wireguard_listen_port: 51821,
            allowed_udp_ports: vec![53],
            ..NetworkConfig::default()
        };
        let parsed = parse_network_snippet(&generate_network_nix(&cfg));
        assert!(parsed.wireguard_enabled);
        assert_eq!(parsed.wireguard_listen_port, 51821);
        assert_eq!(parsed.allowed_udp_ports, vec![53, 51821]);
    }

    #[test]
    fn parse_network_snippet_firewall_disabled() {
        let nix = r#"
{
  networking.firewall = {
    enable = false;
    allowedTCPPorts = [ ];
    allowedUDPPorts = [ ];
  };
}
"#;
        let parsed = parse_network_snippet(nix);
        assert!(!parsed.firewall_enabled);
        assert!(!parsed.ssh_enabled);
    }

    #[test]
    fn parse_services_snippet_from_generated() {
        let nix = generate_services_nix(&[
            "printing",
            "fwupd",
            "docker",
            "libvirtd",
            "dconf",
            "postgresql",
            "redis",
            "earlyoom",
            "auto_upgrade",
            "auto_gc",
            "store_optimize",
            "flatpak",
            "syncthing",
            "locate",
            "networkmanager",
            "resolved",
            "upower",
            "gnome_keyring",
            "rustdesk",
            "gnome_tweaks",
        ]);
        let parsed = parse_services_snippet(&nix);
        assert!(parsed.printing);
        assert!(parsed.avahi); // printing also enables avahi
        assert!(parsed.fwupd);
        assert!(parsed.docker);
        assert!(parsed.libvirtd);
        assert!(parsed.dconf);
        assert!(parsed.postgresql);
        assert!(parsed.redis);
        assert!(parsed.earlyoom);
        assert!(parsed.auto_upgrade);
        assert!(parsed.auto_gc);
        assert!(parsed.store_optimize);
        assert!(parsed.flatpak);
        assert!(parsed.syncthing);
        assert!(parsed.locate);
        assert!(parsed.networkmanager);
        assert!(parsed.resolved);
        assert!(parsed.upower);
        assert!(parsed.gnome_keyring);
        assert!(parsed.rustdesk);
        assert!(parsed.gnome_tweaks);
    }

    #[test]
    fn parse_services_ignores_false_enable() {
        let nix = r#"
{
  services.printing.enable = false;
  virtualisation.docker.enable = true;
}
"#;
        let parsed = parse_services_snippet(nix);
        assert!(!parsed.printing);
        assert!(parsed.docker);
    }

    #[test]
    fn maintenance_rewrites_channel_update_on_flake_hosts() {
        assert_eq!(
            maintenance_command_to_run(NIX_CHANNEL_UPDATE, true).as_deref(),
            Some(NIX_FLAKE_UPDATE)
        );
        assert_eq!(
            maintenance_command_to_run(NIX_CHANNEL_UPDATE, false).as_deref(),
            Some(NIX_CHANNEL_UPDATE)
        );
        assert_eq!(
            maintenance_command_to_run(NIX_FLAKE_UPDATE, true).as_deref(),
            Some(NIX_FLAKE_UPDATE)
        );
        assert_eq!(
            maintenance_command_to_run("nix-collect-garbage", true).as_deref(),
            Some("nix-collect-garbage")
        );
        assert!(maintenance_command_to_run("rm -rf /", false).is_none());
    }
}

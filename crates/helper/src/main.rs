//! NixOS Toolkit Helper - Privileged Operations
//!
//! This binary handles privileged operations for the NixOS Toolkit:
//! - Writing configuration files to /etc/nixos/nixos-toolkit/
//! - Running nixos-rebuild commands
//! - Reading system configuration
//!
//! Communication is via JSON over stdin/stdout.

mod commands;
mod nix_gen;
mod rebuild;

use anyhow::Result;
use common::ipc::{HelperRequest, HelperResponse};
use std::io::{self, BufRead, Write};
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

fn main() -> Result<()> {
    // Initialize logging to stderr (stdout is for IPC)
    tracing_subscriber::registry()
        .with(fmt::layer().with_writer(io::stderr))
        .with(EnvFilter::from_default_env().add_directive("helper=info".parse().unwrap()))
        .init();

    tracing::info!("NixOS Toolkit Helper started");

    let stdin = io::stdin();
    let mut stdout = io::stdout();

    for line in stdin.lock().lines() {
        match line {
            Ok(line) => {
                if line.is_empty() {
                    continue;
                }

                let response = match serde_json::from_str::<HelperRequest>(&line) {
                    Ok(request) => handle_request(request),
                    Err(e) => {
                        tracing::error!("Failed to parse request: {}", e);
                        HelperResponse::Error {
                            message: "Invalid request format".into(),
                            details: Some(e.to_string()),
                        }
                    }
                };

                // Send response
                match serde_json::to_string(&response) {
                    Ok(json) => {
                        if writeln!(stdout, "{}", json).is_err() {
                            break;
                        }
                        if stdout.flush().is_err() {
                            break;
                        }
                    }
                    Err(e) => {
                        tracing::error!("Failed to serialize response: {}", e);
                    }
                }
            }
            Err(e) => {
                tracing::error!("Error reading from stdin: {}", e);
                break;
            }
        }
    }

    tracing::info!("NixOS Toolkit Helper shutting down");
    Ok(())
}

fn handle_request(request: HelperRequest) -> HelperResponse {
    tracing::info!("Handling request: {:?}", std::mem::discriminant(&request));

    match request {
        HelperRequest::CheckPermissions => commands::check_permissions(),
        HelperRequest::GetSystemInfo => commands::get_system_info(),
        HelperRequest::Validate {
            selected_profile,
            enabled_bundles,
            hostname,
        } => commands::validate(selected_profile, enabled_bundles, hostname),
        HelperRequest::Generate {
            selected_profile,
            enabled_bundles,
            bundle_packages,
            hostname,
            dns_servers,
            user_groups,
            username,
            bluetooth_enabled,
            custom_packages,
            network_config,
            services_config,
            dry_run,
        } => commands::generate(selected_profile, enabled_bundles, bundle_packages, hostname, dns_servers, user_groups, username, bluetooth_enabled, custom_packages, network_config, services_config, dry_run),
        HelperRequest::Apply {
            selected_profile,
            enabled_bundles,
            bundle_packages,
            hostname,
            dns_servers,
            user_groups,
            username,
            bluetooth_enabled,
            custom_packages,
            network_config,
            services_config,
            rebuild_type,
        } => commands::apply(selected_profile, enabled_bundles, bundle_packages, hostname, dns_servers, user_groups, username, bluetooth_enabled, custom_packages, network_config, services_config, rebuild_type),
        HelperRequest::EnsureDirectories => commands::ensure_directories(),
        HelperRequest::ReadState => commands::read_state(),
        HelperRequest::WriteState { state } => commands::write_state(state),
        HelperRequest::ListGenerations => commands::list_generations(),
        HelperRequest::RollbackGeneration { generation } => commands::rollback_generation(generation),
        HelperRequest::DeleteGenerations { generations } => commands::delete_generations(generations),
        HelperRequest::RunMaintenance { command } => commands::run_maintenance(command),
        HelperRequest::GetDiskUsage => commands::get_disk_usage(),
    }
}

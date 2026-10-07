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
use common::validate::MAX_HELPER_JSON_LINE;
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
    let mut input = stdin.lock();

    loop {
        match read_json_line(&mut input, MAX_HELPER_JSON_LINE) {
            Ok(None) => break,
            Ok(Some(Err(()))) => {
                let response = HelperResponse::Error {
                    message: "Request too large".into(),
                    details: Some(format!("JSON line exceeded {MAX_HELPER_JSON_LINE} bytes")),
                };
                if let Ok(json) = serde_json::to_string(&response) {
                    if writeln!(stdout, "{json}").is_err() || stdout.flush().is_err() {
                        break;
                    }
                }
                continue;
            }
            Ok(Some(Ok(line))) => {
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

/// Read one stdin line without growing past `max` bytes.
fn read_json_line(reader: &mut impl BufRead, max: usize) -> io::Result<Option<Result<String, ()>>> {
    let mut buf = Vec::new();
    loop {
        let mut byte = [0u8; 1];
        let n = reader.read(&mut byte)?;
        if n == 0 {
            if buf.is_empty() {
                return Ok(None);
            }
            break;
        }
        if byte[0] == b'\n' {
            break;
        }
        if buf.len() >= max {
            // Drain through the delimiter without allocating an unbounded buffer.
            loop {
                let available = reader.fill_buf()?;
                if available.is_empty() {
                    break;
                }
                if let Some(end) = available.iter().position(|b| *b == b'\n') {
                    reader.consume(end + 1);
                    break;
                }
                let count = available.len();
                reader.consume(count);
            }
            return Ok(Some(Err(())));
        }
        buf.push(byte[0]);
    }
    Ok(Some(Ok(String::from_utf8_lossy(&buf).into_owned())))
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
            hardware_config,
            dry_run,
        } => commands::generate(
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
            hardware_config,
            dry_run,
        ),
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
            hardware_config,
            rebuild_type,
        } => commands::apply(
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
            hardware_config,
            rebuild_type,
        ),
        HelperRequest::EnsureDirectories => commands::ensure_directories(),
        HelperRequest::ReadState => commands::read_state(),
        HelperRequest::WriteState { state } => commands::write_state(state),
        HelperRequest::ListGenerations => commands::list_generations(),
        HelperRequest::RollbackGeneration {
            generation,
            activate,
        } => commands::rollback_generation(generation, activate),
        HelperRequest::DeleteGenerations { generations } => {
            commands::delete_generations(generations)
        }
        HelperRequest::RunMaintenance { command } => commands::run_maintenance(command),
        HelperRequest::GetDiskUsage => commands::get_disk_usage(),
    }
}

#[cfg(test)]
mod protocol_tests {
    use super::*;
    #[test]
    fn oversized_record_is_drained_and_next_request_remains_readable() {
        let mut input = io::Cursor::new(b"oversized\n{}\n");
        assert!(matches!(
            read_json_line(&mut input, 3).unwrap(),
            Some(Err(()))
        ));
        assert_eq!(
            read_json_line(&mut input, 3).unwrap(),
            Some(Ok("{}".into()))
        );
    }
    #[test]
    fn exact_limit_and_final_record_without_newline_are_accepted() {
        let mut input = io::Cursor::new(b"{}\n{}");
        assert_eq!(
            read_json_line(&mut input, 2).unwrap(),
            Some(Ok("{}".into()))
        );
        assert_eq!(
            read_json_line(&mut input, 2).unwrap(),
            Some(Ok("{}".into()))
        );
        assert_eq!(read_json_line(&mut input, 2).unwrap(), None);
    }
}

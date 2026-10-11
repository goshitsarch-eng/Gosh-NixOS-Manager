//! nixos-rebuild execution

use common::config::ConfigMode;
use common::ipc::{HelperResponse, LogLevel, RebuildType};
use std::io::Write;
use std::process::Command;

/// Send a log message to the GUI
fn send_log(level: LogLevel, message: String) {
    let log = HelperResponse::Log { level, message };
    if let Ok(json) = serde_json::to_string(&log) {
        let mut stdout = std::io::stdout();
        let _ = writeln!(stdout, "{}", json);
        let _ = stdout.flush();
    }
}

/// Run nixos-rebuild with the specified type
pub fn run_rebuild(rebuild_type: RebuildType, config_mode: ConfigMode) -> HelperResponse {
    let rebuild_arg = rebuild_type.as_arg();

    // Build command based on config mode
    let mut cmd = Command::new("nixos-rebuild");
    cmd.arg(rebuild_arg);

    match config_mode {
        ConfigMode::Flake => {
            cmd.arg("--flake").arg("/etc/nixos");
        }
        ConfigMode::Classic | ConfigMode::Unknown => {
            // Classic mode uses default paths
        }
    }

    send_log(
        LogLevel::Info,
        format!("Running: nixos-rebuild {rebuild_arg}"),
    );
    let mut insecure_package = false;
    let result = crate::operation::run(&mut cmd, |is_error, message| {
        insecure_package |= message.contains("marked as insecure");
        let level = if is_error && (message.contains("error:") || message.contains("Error:")) {
            LogLevel::Error
        } else if message.contains("warning:") || message.contains("Warning:") {
            LogLevel::Warning
        } else {
            LogLevel::Info
        };
        send_log(level, message.to_string());
    });
    match result {
        Ok(output) => HelperResponse::ApplyComplete {
            success: output.status.success(),
            message: if output.status.success() {
                format!("nixos-rebuild {rebuild_arg} completed successfully")
            } else {
                let message = format!(
                    "nixos-rebuild {rebuild_arg} failed with exit code {}",
                    output.status.code().unwrap_or(-1)
                );
                if insecure_package {
                    format!("{message}. Nix blocked an insecure package. Deselect the affected package or use a host channel with a supported version. The log names the dependency; security checks remain enabled.")
                } else {
                    message
                }
            },
        },
        Err(error) => HelperResponse::Error {
            message: if error.kind() == std::io::ErrorKind::Interrupted {
                "Rebuild interrupted; verify the running system before retrying".into()
            } else {
                "Failed to run nixos-rebuild".into()
            },
            details: Some(error.to_string()),
        },
    }
}

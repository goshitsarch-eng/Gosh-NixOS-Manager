//! nixos-rebuild execution

use common::config::ConfigMode;
use common::ipc::{HelperResponse, LogLevel, RebuildType};
use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::thread;

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
            // Get hostname for flake reference
            let hostname = std::fs::read_to_string("/etc/hostname")
                .map(|s| s.trim().to_string())
                .unwrap_or_else(|_| "nixos".to_string());

            cmd.arg("--flake")
                .arg(format!("/etc/nixos#{}", hostname));
        }
        ConfigMode::Classic | ConfigMode::Unknown => {
            // Classic mode uses default paths
        }
    }

    // Capture output - use pseudo-tty behavior with stderr merged for better streaming
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());

    send_log(LogLevel::Info, format!("Running: nixos-rebuild {}", rebuild_arg));
    send_log(LogLevel::Info, "Building system configuration...".to_string());

    // Spawn the process
    let mut child = match cmd.spawn() {
        Ok(c) => c,
        Err(e) => {
            return HelperResponse::Error {
                message: "Failed to spawn nixos-rebuild".into(),
                details: Some(e.to_string()),
            };
        }
    };

    // Take ownership of stdout and stderr
    let stdout = child.stdout.take();
    let stderr = child.stderr.take();

    // Create channel for collecting output from both streams
    let (tx, rx) = mpsc::channel();

    // Spawn thread to read stdout
    let tx_stdout = tx.clone();
    let stdout_thread = thread::spawn(move || {
        if let Some(stdout) = stdout {
            let reader = BufReader::new(stdout);
            for line in reader.lines().flatten() {
                let _ = tx_stdout.send((LogLevel::Info, line));
            }
        }
    });

    // Spawn thread to read stderr (where most nixos-rebuild output goes)
    let tx_stderr = tx;
    let stderr_thread = thread::spawn(move || {
        if let Some(stderr) = stderr {
            let reader = BufReader::new(stderr);
            for line in reader.lines().flatten() {
                // Determine log level based on content
                let level = if line.contains("error:") || line.contains("Error:") {
                    LogLevel::Error
                } else if line.contains("warning:") || line.contains("Warning:") {
                    LogLevel::Warning
                } else {
                    LogLevel::Info
                };
                let _ = tx_stderr.send((level, line));
            }
        }
    });

    // Receive and send log messages as they come in
    // This runs until both sender threads are done
    while let Ok((level, message)) = rx.recv() {
        send_log(level, message);
    }

    // Wait for reader threads to finish
    let _ = stdout_thread.join();
    let _ = stderr_thread.join();

    // Wait for the process to complete
    match child.wait() {
        Ok(status) => {
            if status.success() {
                send_log(LogLevel::Info, "Build completed successfully!".to_string());
                HelperResponse::ApplyComplete {
                    success: true,
                    message: format!("nixos-rebuild {} completed successfully", rebuild_arg),
                }
            } else {
                let code = status.code().unwrap_or(-1);
                send_log(LogLevel::Error, format!("Build failed with exit code {}", code));
                HelperResponse::ApplyComplete {
                    success: false,
                    message: format!("nixos-rebuild {} failed with exit code {}", rebuild_arg, code),
                }
            }
        }
        Err(e) => {
            send_log(LogLevel::Error, format!("Process error: {}", e));
            HelperResponse::Error {
                message: "Failed to wait for nixos-rebuild".into(),
                details: Some(e.to_string()),
            }
        }
    }
}

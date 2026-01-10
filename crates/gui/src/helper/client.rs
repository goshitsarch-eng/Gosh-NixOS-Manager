//! Client for communicating with the privileged helper process

use anyhow::{Context, Result};
use common::ipc::{HelperRequest, HelperResponse};
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;

/// Client for the nixos-toolkit-helper process
pub struct HelperClient {
    child: Child,
    response_rx: Receiver<HelperResponse>,
    #[allow(dead_code)]
    reader_thread: thread::JoinHandle<()>,
}

impl HelperClient {
    /// Get the path to the helper binary
    fn helper_path() -> String {
        // 1. Check environment variable first (set by nix wrapper)
        if let Ok(path) = std::env::var("NIXOS_TOOLKIT_HELPER") {
            tracing::debug!("Using helper from NIXOS_TOOLKIT_HELPER: {}", path);
            return path;
        }

        // 2. Check relative to the current executable (development mode)
        if let Ok(exe) = std::env::current_exe() {
            if let Some(dir) = exe.parent() {
                let relative_path = dir.join("nixos-toolkit-helper");
                if relative_path.exists() {
                    let path = relative_path.to_string_lossy().to_string();
                    tracing::debug!("Using helper relative to exe: {}", path);
                    return path;
                }
            }
        }

        // 3. Check common installation paths
        for path in [
            "/run/current-system/sw/bin/nixos-toolkit-helper",
            "/usr/local/bin/nixos-toolkit-helper",
            "/usr/bin/nixos-toolkit-helper",
        ] {
            if std::path::Path::new(path).exists() {
                tracing::debug!("Using helper from system path: {}", path);
                return path.to_string();
            }
        }

        // 4. Fall back to PATH lookup
        tracing::debug!("Falling back to PATH lookup for nixos-toolkit-helper");
        "nixos-toolkit-helper".to_string()
    }

    /// Spawn the helper with pkexec for privilege escalation
    pub fn spawn_privileged() -> Result<Self> {
        let helper = Self::helper_path();
        tracing::info!("Spawning helper with pkexec: {}", helper);
        Self::spawn_with_command("pkexec", &[&helper])
    }

    /// Spawn the helper directly (for development/dry-run)
    pub fn spawn() -> Result<Self> {
        let helper = Self::helper_path();
        tracing::info!("Spawning helper directly: {}", helper);
        Self::spawn_with_command(&helper, &[])
    }

    /// Spawn helper with custom command
    fn spawn_with_command(cmd: &str, args: &[&str]) -> Result<Self> {
        let mut child = Command::new(cmd)
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            // Remove SHELL env var to avoid pkexec rejecting nix-develop shells
            // that aren't in /etc/shells
            .env_remove("SHELL")
            .spawn()
            .context("Failed to spawn helper process")?;

        let stdout = child
            .stdout
            .take()
            .context("Failed to get helper stdout")?;

        let (tx, rx): (Sender<HelperResponse>, Receiver<HelperResponse>) = mpsc::channel();

        // Spawn thread to read responses
        let reader_thread = thread::spawn(move || {
            let reader = BufReader::new(stdout);
            for line in reader.lines() {
                match line {
                    Ok(line) => {
                        if line.is_empty() {
                            continue;
                        }
                        match serde_json::from_str::<HelperResponse>(&line) {
                            Ok(response) => {
                                if tx.send(response).is_err() {
                                    break;
                                }
                            }
                            Err(e) => {
                                tracing::error!("Failed to parse helper response: {}", e);
                            }
                        }
                    }
                    Err(e) => {
                        tracing::error!("Error reading from helper: {}", e);
                        break;
                    }
                }
            }
        });

        Ok(Self {
            child,
            response_rx: rx,
            reader_thread,
        })
    }

    /// Send a request to the helper
    pub fn send(&mut self, request: &HelperRequest) -> Result<()> {
        let stdin = self
            .child
            .stdin
            .as_mut()
            .context("Helper stdin not available")?;

        let json = serde_json::to_string(request)?;
        writeln!(stdin, "{}", json)?;
        stdin.flush()?;

        Ok(())
    }

    /// Receive the next response (blocking)
    pub fn recv(&self) -> Option<HelperResponse> {
        self.response_rx.recv().ok()
    }

    /// Try to receive a response (non-blocking)
    pub fn try_recv(&self) -> Option<HelperResponse> {
        self.response_rx.try_recv().ok()
    }

    /// Receive with timeout
    pub fn recv_timeout(&self, timeout: std::time::Duration) -> Option<HelperResponse> {
        self.response_rx.recv_timeout(timeout).ok()
    }

    /// Check if the helper process is still running
    pub fn is_running(&mut self) -> bool {
        match self.child.try_wait() {
            Ok(None) => true,
            Ok(Some(_)) => false,
            Err(_) => false,
        }
    }

    /// Kill the helper process
    pub fn kill(&mut self) -> Result<()> {
        self.child.kill().context("Failed to kill helper")
    }
}

impl Drop for HelperClient {
    fn drop(&mut self) {
        // Try to gracefully terminate the helper
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// Helper bridge for async communication with GTK main loop
pub struct HelperBridge {
    client: Option<HelperClient>,
}

impl HelperBridge {
    pub fn new() -> Self {
        Self { client: None }
    }

    /// Start the helper (privileged)
    pub fn start_privileged(&mut self) -> Result<()> {
        self.client = Some(HelperClient::spawn_privileged()?);
        Ok(())
    }

    /// Start the helper (unprivileged)
    pub fn start(&mut self) -> Result<()> {
        self.client = Some(HelperClient::spawn()?);
        Ok(())
    }

    /// Send a request
    pub fn send(&mut self, request: &HelperRequest) -> Result<()> {
        if let Some(ref mut client) = self.client {
            client.send(request)
        } else {
            anyhow::bail!("Helper not started")
        }
    }

    /// Try to receive a response
    pub fn try_recv(&self) -> Option<HelperResponse> {
        self.client.as_ref().and_then(|c| c.try_recv())
    }

    /// Stop the helper
    pub fn stop(&mut self) {
        if let Some(mut client) = self.client.take() {
            let _ = client.kill();
        }
    }

    /// Check if helper is available
    pub fn is_available(&mut self) -> bool {
        self.client
            .as_mut()
            .map(|c| c.is_running())
            .unwrap_or(false)
    }
}

impl Default for HelperBridge {
    fn default() -> Self {
        Self::new()
    }
}

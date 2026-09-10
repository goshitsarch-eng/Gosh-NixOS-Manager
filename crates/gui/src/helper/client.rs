//! Line-delimited JSON client for the privileged helper process.

use crate::helper::spawn::SpawnSpec;
use anyhow::{Context, Result};
use common::ipc::{HelperRequest, HelperResponse};
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;
use std::time::Duration;

/// Client for the nixos-toolkit-helper process.
pub struct HelperClient {
    child: Child,
    response_rx: Receiver<HelperResponse>,
    #[allow(dead_code)]
    reader_thread: thread::JoinHandle<()>,
}

impl HelperClient {
    /// Spawn the helper using an injectable [`SpawnSpec`].
    ///
    /// Does not spawn when `spec.helper_available` is false.
    pub fn spawn(spec: &SpawnSpec) -> Result<Self> {
        if !spec.helper_available || spec.program.as_os_str().is_empty() {
            anyhow::bail!("nixos-toolkit-helper is not available");
        }

        tracing::info!(
            program = %spec.program.display(),
            args = ?spec.args,
            "spawning helper"
        );

        let mut cmd = Command::new(&spec.program);
        cmd.args(&spec.args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit());

        for key in &spec.remove_env {
            cmd.env_remove(key);
        }
        for (key, value) in &spec.extra_env {
            cmd.env(key, value);
        }

        let mut child = cmd.spawn().context("Failed to spawn helper process")?;

        let stdout = child.stdout.take().context("Failed to get helper stdout")?;

        let (tx, rx): (Sender<HelperResponse>, Receiver<HelperResponse>) = mpsc::channel();

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
                                tracing::error!(
                                    "Failed to parse helper response: {e}; line={line}"
                                );
                            }
                        }
                    }
                    Err(e) => {
                        tracing::error!("Error reading from helper: {e}");
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

    pub fn send(&mut self, request: &HelperRequest) -> Result<()> {
        let stdin = self
            .child
            .stdin
            .as_mut()
            .context("Helper stdin not available")?;

        let json = serde_json::to_string(request)?;
        writeln!(stdin, "{json}")?;
        stdin.flush()?;
        Ok(())
    }

    #[must_use]
    pub fn recv(&self) -> Option<HelperResponse> {
        self.response_rx.recv().ok()
    }

    #[must_use]
    pub fn try_recv(&self) -> Option<HelperResponse> {
        self.response_rx.try_recv().ok()
    }

    #[must_use]
    pub fn recv_timeout(&self, timeout: Duration) -> Option<HelperResponse> {
        self.response_rx.recv_timeout(timeout).ok()
    }

    #[must_use]
    pub fn is_running(&mut self) -> bool {
        match self.child.try_wait() {
            Ok(None) => true,
            Ok(Some(_)) | Err(_) => false,
        }
    }

    pub fn kill(&mut self) -> Result<()> {
        self.child.kill().context("Failed to kill helper")
    }
}

impl Drop for HelperClient {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

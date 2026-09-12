//! Helper session stream. One process per op (or Apply chain). Missing helper → SpawnFailed.

use crate::helper::client::HelperClient;
use crate::helper::spawn::SpawnSpec;
use crate::message::{HelperEvent, HelperOp};
use common::ipc::{HelperRequest, HelperResponse, LogLevel, RebuildType};
use futures::stream::Stream;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

/// Stream helper events as they arrive (dedicated thread, not a buffered `async` block).
pub fn stream(
    spec: SpawnSpec,
    op: HelperOp,
    request: HelperRequest,
    cancel: Arc<AtomicBool>,
) -> impl Stream<Item = HelperEvent> + Send {
    let (tx, rx) = futures::channel::mpsc::unbounded();
    let _ = std::thread::Builder::new()
        .name("nixos-toolkit-helper".into())
        .spawn(move || {
            run_emitting(spec, op, request, &cancel, |event| {
                let _ = tx.unbounded_send(event);
            });
        });
    rx
}

/// Run one helper process to completion.
/// Switch/Boot/Test/Build: EnsureDirectories → Apply → optional WriteState.
/// DryBuild: Apply only (the helper snapshots before creating directories).
#[cfg(test)]
pub(crate) fn run_blocking(
    spec: SpawnSpec,
    op: HelperOp,
    request: HelperRequest,
) -> Vec<HelperEvent> {
    let mut events = Vec::new();
    run_emitting(spec, op, request, &AtomicBool::new(false), |event| {
        events.push(event)
    });
    events
}

fn run_emitting(
    spec: SpawnSpec,
    op: HelperOp,
    request: HelperRequest,
    cancel: &AtomicBool,
    mut emit: impl FnMut(HelperEvent),
) {
    if !spec.helper_available {
        emit(HelperEvent::SpawnFailed {
            op,
            error: helper_missing_message().to_string(),
        });
        return;
    }

    match HelperClient::spawn(&spec) {
        Ok(mut client) => {
            emit(HelperEvent::Spawned { op: event_op(&op) });
            match &op {
                HelperOp::Apply {
                    then_write_state,
                    save,
                    ..
                } => run_apply_chain(
                    &mut client,
                    &op,
                    &request,
                    *then_write_state,
                    save.as_deref(),
                    cancel,
                    &mut emit,
                ),
                _ => run_single(&mut client, &op, &request, cancel, &mut emit),
            }
        }
        Err(err) => emit(HelperEvent::SpawnFailed {
            op,
            error: err.to_string(),
        }),
    }
}

/// Events clone the op without the ipc snapshot so Log lines stay small.
fn event_op(op: &HelperOp) -> HelperOp {
    match op {
        HelperOp::Apply {
            rebuild,
            then_write_state,
            ..
        } => HelperOp::Apply {
            rebuild: *rebuild,
            then_write_state: *then_write_state,
            save: None,
        },
        other => other.clone(),
    }
}

fn push_log(emit: &mut impl FnMut(HelperEvent), op: &HelperOp, message: impl Into<String>) {
    emit(HelperEvent::Response {
        op: event_op(op),
        response: Box::new(HelperResponse::Log {
            level: LogLevel::Info,
            message: message.into(),
        }),
    });
}

fn run_single(
    client: &mut HelperClient,
    op: &HelperOp,
    request: &HelperRequest,
    cancel: &AtomicBool,
    emit: &mut impl FnMut(HelperEvent),
) {
    if let Err(err) = client.send(request) {
        emit(HelperEvent::SpawnFailed {
            op: event_op(op),
            error: err.to_string(),
        });
        return;
    }
    let _ = recv_until_terminal(client, op, timeout_for(op), cancel, emit, true, true);
}

fn run_apply_chain(
    client: &mut HelperClient,
    op: &HelperOp,
    apply_request: &HelperRequest,
    then_write_state: bool,
    save: Option<&common::ipc::AppState>,
    cancel: &AtomicBool,
    emit: &mut impl FnMut(HelperEvent),
) {
    let dry_build = matches!(
        op,
        HelperOp::Apply {
            rebuild: RebuildType::DryBuild,
            ..
        }
    );
    // Dry-build snapshots inside Apply before EnsureDirectories. Sending
    // EnsureDirectories first would create a placeholder tree and make restore
    // leave that placeholder instead of the pre-dry-run state.
    if !dry_build {
        push_log(emit, op, "Ensuring directories exist...");
        if let Err(err) = client.send(&HelperRequest::EnsureDirectories) {
            emit(HelperEvent::SpawnFailed {
                op: event_op(op),
                error: err.to_string(),
            });
            return;
        }

        match recv_until_terminal(
            client,
            op,
            timeout_for(&HelperOp::EnsureDirectories),
            cancel,
            emit,
            true,
            true,
        ) {
            Some(HelperResponse::Ok) => {
                push_log(emit, op, "Directories ready.");
            }
            Some(other) => {
                emit(HelperEvent::Response {
                    op: event_op(op),
                    response: Box::new(other),
                });
                return;
            }
            None => return,
        }
    }

    if let Err(err) = client.send(apply_request) {
        emit(HelperEvent::SpawnFailed {
            op: event_op(op),
            error: err.to_string(),
        });
        return;
    }

    match recv_until_terminal(client, op, timeout_for(op), cancel, emit, false, true) {
        Some(HelperResponse::ApplyComplete {
            success,
            mut message,
        }) => {
            if success && then_write_state {
                if let Some(state) = save {
                    if !write_state_after_apply(client, op, state, cancel, emit)
                        && !message.to_ascii_lowercase().contains("state save failed")
                    {
                        message.push_str("; State save failed");
                    }
                }
            } else if success && !then_write_state {
                push_log(emit, op, "Dry run completed - no changes applied.");
            }
            emit(HelperEvent::Response {
                op: event_op(op),
                response: Box::new(HelperResponse::ApplyComplete { success, message }),
            });
        }
        Some(other) => {
            emit(HelperEvent::Response {
                op: event_op(op),
                response: Box::new(other),
            });
        }
        None => {}
    }
}

fn write_state_after_apply(
    client: &mut HelperClient,
    op: &HelperOp,
    state: &common::ipc::AppState,
    cancel: &AtomicBool,
    emit: &mut impl FnMut(HelperEvent),
) -> bool {
    if let Err(err) = client.send(&HelperRequest::WriteState {
        state: state.clone(),
    }) {
        push_log(emit, op, format!("Warning: State save failed: {err}"));
        return false;
    }

    match recv_until_terminal(
        client,
        op,
        timeout_for(&HelperOp::WriteState),
        cancel,
        emit,
        false,
        false,
    ) {
        Some(HelperResponse::Ok) => {
            push_log(emit, op, "State saved for next session.");
            true
        }
        Some(HelperResponse::Error { message, .. }) => {
            push_log(emit, op, format!("Warning: State save failed: {message}"));
            false
        }
        Some(other) => {
            emit(HelperEvent::Response {
                op: event_op(op),
                response: Box::new(other),
            });
            false
        }
        None => {
            push_log(
                emit,
                op,
                "Warning: State save failed: timed out or helper closed; rebuild may have succeeded.",
            );
            false
        }
    }
}

/// Read until a non-Log response. Logs are always forwarded.
/// When `forward_terminal` is true, the terminal response is also pushed.
/// When `emit_eof` is false, timeout/close is returned as `None` without an event
/// (WriteState after Apply: GTK still finishes the rebuild).
fn recv_until_terminal(
    client: &mut HelperClient,
    op: &HelperOp,
    timeout: Option<Duration>,
    cancel: &AtomicBool,
    emit: &mut impl FnMut(HelperEvent),
    forward_terminal: bool,
    emit_eof: bool,
) -> Option<HelperResponse> {
    let started = std::time::Instant::now();
    let slice = Duration::from_millis(250);
    loop {
        if cancel.load(Ordering::Relaxed) {
            let _ = client.kill();
            if emit_eof {
                emit(HelperEvent::Closed {
                    op: event_op(op),
                    error: Some("cancelled".into()),
                });
            }
            return None;
        }

        if let Some(limit) = timeout {
            if started.elapsed() >= limit {
                let _ = client.kill();
                if emit_eof {
                    emit(HelperEvent::Timeout { op: event_op(op) });
                }
                return None;
            }
        }

        match client.recv_timeout(slice) {
            Some(response) => {
                let terminal = is_terminal(&response);
                if terminal {
                    if forward_terminal {
                        emit(HelperEvent::Response {
                            op: event_op(op),
                            response: Box::new(response.clone()),
                        });
                    }
                    return Some(response);
                }
                emit(HelperEvent::Response {
                    op: event_op(op),
                    response: Box::new(response),
                });
            }
            None => {
                if !client.is_running() {
                    if emit_eof {
                        emit(HelperEvent::Closed {
                            op: event_op(op),
                            error: Some("helper stdout closed".into()),
                        });
                    }
                    return None;
                }
            }
        }
    }
}

fn timeout_for(op: &HelperOp) -> Option<Duration> {
    match op {
        HelperOp::ReadState => Some(Duration::from_secs(300)),
        HelperOp::EnsureDirectories => Some(Duration::from_secs(10)),
        HelperOp::WriteState => Some(Duration::from_secs(30)),
        HelperOp::Apply { .. } => None,
        HelperOp::GetDiskUsage | HelperOp::ListGenerations => Some(Duration::from_secs(180)),
        HelperOp::RunMaintenance { .. } => Some(Duration::from_secs(900)),
        _ => Some(Duration::from_secs(30)),
    }
}

fn is_terminal(response: &HelperResponse) -> bool {
    !matches!(response, HelperResponse::Log { .. })
}

/// Banner copy when the host helper cannot be found (packaging §3).
#[must_use]
pub fn helper_missing_message() -> &'static str {
    "Privileged helper not found. You can preview configuration and save local preferences. Apply, generations, and maintenance require nixos-toolkit-helper on the host (NixOS Toolkit system module) and will ask for authentication."
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::helper::spawn::SpawnSpec;
    use common::ipc::{AppState as IpcAppState, RebuildType};
    use std::fs;
    use std::os::unix::fs::PermissionsExt;
    use std::path::{Path, PathBuf};
    use std::time::{SystemTime, UNIX_EPOCH};

    const HELPER_PY: &str = r#"#!/usr/bin/env python3
import json, os, sys

def append(path, text):
    if path:
        with open(path, "a") as f:
            f.write(text)

append(os.environ.get("FAKE_HELPER_SPAWNS"), "spawn\n")
req_log = os.environ.get("FAKE_HELPER_REQUESTS")
apply_ok = os.environ.get("FAKE_HELPER_APPLY_SUCCESS", "1") != "0"

for raw in sys.stdin:
    line = raw.strip()
    if not line:
        continue
    append(req_log, line + "\n")
    try:
        req = json.loads(line)
    except json.JSONDecodeError:
        print(json.dumps({"type": "Error", "payload": {"message": "Invalid request format", "details": None}}), flush=True)
        continue
    typ = req.get("type")
    if typ == "Apply":
        print(json.dumps({
            "type": "ApplyComplete",
            "payload": {"success": apply_ok, "message": "ok" if apply_ok else "failed"},
        }), flush=True)
    else:
        print(json.dumps({"type": "Ok"}), flush=True)
"#;

    fn temp_dir() -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let dir = std::env::temp_dir().join(format!(
            "nixos-toolkit-session-{}-{}",
            std::process::id(),
            nanos
        ));
        fs::create_dir_all(&dir).expect("temp dir");
        dir
    }

    fn scripted_spec(dir: &Path) -> (SpawnSpec, PathBuf, PathBuf) {
        let helper = dir.join("fake-helper");
        fs::write(&helper, HELPER_PY).expect("write helper");
        let mut perms = fs::metadata(&helper).expect("meta").permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&helper, perms).expect("chmod");

        let spawns = dir.join("spawns");
        let requests = dir.join("requests");
        let mut spec = SpawnSpec::for_tests(helper);
        spec.extra_env
            .push(("FAKE_HELPER_SPAWNS".into(), spawns.display().to_string()));
        spec.extra_env.push((
            "FAKE_HELPER_REQUESTS".into(),
            requests.display().to_string(),
        ));
        (spec, spawns, requests)
    }

    fn request_types(path: &Path) -> Vec<String> {
        fs::read_to_string(path)
            .unwrap_or_default()
            .lines()
            .filter_map(|line| {
                let value: serde_json::Value = serde_json::from_str(line).ok()?;
                value.get("type")?.as_str().map(ToOwned::to_owned)
            })
            .collect()
    }

    fn spawn_count(path: &Path) -> usize {
        fs::read_to_string(path)
            .unwrap_or_default()
            .lines()
            .filter(|line| !line.trim().is_empty())
            .count()
    }

    fn apply_request() -> HelperRequest {
        crate::state::AppState::new().to_apply_request(RebuildType::Switch)
    }

    #[test]
    fn apply_plus_save_is_one_spawn_and_writes_state() {
        let dir = temp_dir();
        let (spec, spawns, requests) = scripted_spec(&dir);
        let save = IpcAppState {
            selected_profile: Some("gnome".into()),
            ..IpcAppState::default()
        };
        let events = run_blocking(
            spec,
            HelperOp::Apply {
                rebuild: RebuildType::Switch,
                then_write_state: true,
                save: Some(Box::new(save)),
            },
            apply_request(),
        );
        assert_eq!(spawn_count(&spawns), 1, "Apply+save must use one process");
        assert_eq!(
            request_types(&requests),
            ["EnsureDirectories", "Apply", "WriteState"]
        );
        assert!(events.iter().any(|event| matches!(
            event,
            HelperEvent::Response { response, .. }
                if matches!(**response, HelperResponse::ApplyComplete { success: true, .. })
        )));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn dry_run_does_not_write_state() {
        let dir = temp_dir();
        let (spec, spawns, requests) = scripted_spec(&dir);
        let events = run_blocking(
            spec,
            HelperOp::Apply {
                rebuild: RebuildType::DryBuild,
                then_write_state: false,
                save: Some(Box::new(IpcAppState::default())),
            },
            crate::state::AppState::new().to_apply_request(RebuildType::DryBuild),
        );
        assert_eq!(spawn_count(&spawns), 1);
        assert_eq!(request_types(&requests), ["Apply"]);
        assert!(!request_types(&requests).iter().any(|t| t == "WriteState"));
        assert!(events.iter().any(|event| {
            matches!(
                event,
                HelperEvent::Response { response, .. }
                    if matches!(
                        response.as_ref(),
                        HelperResponse::Log { message, .. } if message.contains("Dry run")
                    )
            )
        }));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn failed_apply_does_not_write_state() {
        let dir = temp_dir();
        let (mut spec, spawns, requests) = scripted_spec(&dir);
        spec.extra_env
            .push(("FAKE_HELPER_APPLY_SUCCESS".into(), "0".into()));
        let events = run_blocking(
            spec,
            HelperOp::Apply {
                rebuild: RebuildType::Switch,
                then_write_state: true,
                save: Some(Box::new(IpcAppState::default())),
            },
            apply_request(),
        );
        assert_eq!(spawn_count(&spawns), 1);
        assert_eq!(request_types(&requests), ["EnsureDirectories", "Apply"]);
        assert!(events.iter().any(|event| matches!(
            event,
            HelperEvent::Response { response, .. }
                if matches!(**response, HelperResponse::ApplyComplete { success: false, .. })
        )));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn streamed_apply_emits_spawned_before_complete() {
        let dir = temp_dir();
        let (spec, _, _) = scripted_spec(&dir);
        let events = run_blocking(
            spec,
            HelperOp::Apply {
                rebuild: RebuildType::Switch,
                then_write_state: false,
                save: None,
            },
            apply_request(),
        );
        let spawned = events.iter().position(|event| {
            matches!(
                event,
                HelperEvent::Spawned {
                    op: HelperOp::Apply { .. }
                }
            )
        });
        let complete = events.iter().position(|event| {
            matches!(
                event,
                HelperEvent::Response { response, .. }
                    if matches!(**response, HelperResponse::ApplyComplete { .. })
            )
        });
        assert!(spawned.is_some());
        assert!(complete.is_some());
        assert!(spawned.unwrap() < complete.unwrap());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn missing_helper_does_not_spawn() {
        let spec = SpawnSpec::unavailable(None, Vec::new(), Vec::new());
        let events = run_blocking(spec, HelperOp::ReadState, HelperRequest::ReadState);
        assert!(matches!(
            events.as_slice(),
            [HelperEvent::SpawnFailed {
                op: HelperOp::ReadState,
                ..
            }]
        ));
    }
}

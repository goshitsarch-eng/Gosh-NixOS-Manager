//! Helper session stream. One process per op (or Apply chain). Missing helper → SpawnFailed.

use crate::helper::client::HelperClient;
use crate::helper::spawn::SpawnSpec;
use crate::message::{HelperEvent, HelperOp};
use common::ipc::{HelperRequest, HelperResponse, LogLevel, RebuildType};
use futures::stream::{self, Stream, StreamExt};
use std::time::Duration;

/// Stream helper events for one privileged operation.
///
/// When the spec says the helper is missing, yields [`HelperEvent::SpawnFailed`]
/// and does not spawn a privileged helper.
pub fn stream(
    spec: SpawnSpec,
    op: HelperOp,
    request: HelperRequest,
) -> impl Stream<Item = HelperEvent> + Send {
    stream::once(async move { run_blocking(spec, op, request) }).flat_map(stream::iter)
}

/// Run one helper process to completion.
/// Switch/Boot/Test/Build: EnsureDirectories → Apply → optional WriteState.
/// DryBuild: Apply only (the helper snapshots before creating directories).
pub(crate) fn run_blocking(
    spec: SpawnSpec,
    op: HelperOp,
    request: HelperRequest,
) -> Vec<HelperEvent> {
    if !spec.helper_available {
        return vec![HelperEvent::SpawnFailed {
            op,
            error: helper_missing_message().to_string(),
        }];
    }

    match HelperClient::spawn(&spec) {
        Ok(mut client) => {
            let mut events = vec![HelperEvent::Spawned { op: event_op(&op) }];
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
                    &mut events,
                ),
                _ => run_single(&mut client, &op, &request, &mut events),
            }
            events
        }
        Err(err) => vec![HelperEvent::SpawnFailed {
            op,
            error: err.to_string(),
        }],
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

fn push_log(events: &mut Vec<HelperEvent>, op: &HelperOp, message: impl Into<String>) {
    events.push(HelperEvent::Response {
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
    events: &mut Vec<HelperEvent>,
) {
    if let Err(err) = client.send(request) {
        events.push(HelperEvent::SpawnFailed {
            op: event_op(op),
            error: err.to_string(),
        });
        return;
    }
    let _ = recv_until_terminal(client, op, timeout_for(op), events, true, true);
}

fn run_apply_chain(
    client: &mut HelperClient,
    op: &HelperOp,
    apply_request: &HelperRequest,
    then_write_state: bool,
    save: Option<&common::ipc::AppState>,
    events: &mut Vec<HelperEvent>,
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
        push_log(events, op, "Ensuring directories exist...");
        if let Err(err) = client.send(&HelperRequest::EnsureDirectories) {
            events.push(HelperEvent::SpawnFailed {
                op: event_op(op),
                error: err.to_string(),
            });
            return;
        }

        match recv_until_terminal(
            client,
            op,
            timeout_for(&HelperOp::EnsureDirectories),
            events,
            true,
            true,
        ) {
            Some(HelperResponse::Ok) => {
                push_log(events, op, "Directories ready.");
            }
            Some(other) => {
                events.push(HelperEvent::Response {
                    op: event_op(op),
                    response: Box::new(other),
                });
                return;
            }
            None => return,
        }
    }

    if let Err(err) = client.send(apply_request) {
        events.push(HelperEvent::SpawnFailed {
            op: event_op(op),
            error: err.to_string(),
        });
        return;
    }

    match recv_until_terminal(client, op, timeout_for(op), events, false, true) {
        Some(HelperResponse::ApplyComplete { success, message }) => {
            if success && then_write_state {
                if let Some(state) = save {
                    write_state_after_apply(client, op, state, events);
                }
            } else if success && !then_write_state {
                push_log(events, op, "Dry run completed - no changes applied.");
            }
            events.push(HelperEvent::Response {
                op: event_op(op),
                response: Box::new(HelperResponse::ApplyComplete { success, message }),
            });
        }
        Some(other) => {
            events.push(HelperEvent::Response {
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
    events: &mut Vec<HelperEvent>,
) {
    if let Err(err) = client.send(&HelperRequest::WriteState {
        state: state.clone(),
    }) {
        push_log(events, op, format!("Warning: Could not save state: {err}"));
        return;
    }

    match recv_until_terminal(
        client,
        op,
        timeout_for(&HelperOp::WriteState),
        events,
        false,
        false,
    ) {
        Some(HelperResponse::Ok) => {
            push_log(events, op, "State saved for next session.");
        }
        Some(HelperResponse::Error { message, .. }) => {
            push_log(events, op, format!("Warning: State save failed: {message}"));
        }
        Some(other) => {
            events.push(HelperEvent::Response {
                op: event_op(op),
                response: Box::new(other),
            });
        }
        None => {
            push_log(
                events,
                op,
                "Warning: State save timed out or helper closed; rebuild may have succeeded.",
            );
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
    events: &mut Vec<HelperEvent>,
    forward_terminal: bool,
    emit_eof: bool,
) -> Option<HelperResponse> {
    loop {
        let response = if let Some(timeout) = timeout {
            client.recv_timeout(timeout)
        } else {
            client.recv()
        };
        match response {
            Some(response) => {
                let terminal = is_terminal(&response);
                if terminal {
                    if forward_terminal {
                        events.push(HelperEvent::Response {
                            op: event_op(op),
                            response: Box::new(response.clone()),
                        });
                    }
                    return Some(response);
                }
                events.push(HelperEvent::Response {
                    op: event_op(op),
                    response: Box::new(response),
                });
            }
            None => {
                if emit_eof {
                    if timeout.is_some() {
                        events.push(HelperEvent::Timeout { op: event_op(op) });
                    } else {
                        events.push(HelperEvent::Closed {
                            op: event_op(op),
                            error: Some("helper stdout closed".into()),
                        });
                    }
                }
                return None;
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
        HelperOp::ListGenerations | HelperOp::GetDiskUsage | HelperOp::RunMaintenance { .. } => {
            Some(Duration::from_secs(60))
        }
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

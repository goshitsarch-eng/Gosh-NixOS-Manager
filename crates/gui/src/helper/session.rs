//! Helper session stream. Task 1: spawn via [`HelperClient`]; skip/missing → SpawnFailed.

use crate::helper::client::HelperClient;
use crate::helper::spawn::SpawnSpec;
use crate::message::{HelperEvent, HelperOp};
use common::ipc::HelperRequest;
use futures::stream::{self, Stream, StreamExt};
use std::time::Duration;

/// Stream helper events for one privileged operation.
///
/// When the spec says the helper is missing, yields [`HelperEvent::SpawnFailed`]
/// and does not invoke pkexec.
pub fn stream(
    spec: SpawnSpec,
    op: HelperOp,
    request: HelperRequest,
) -> impl Stream<Item = HelperEvent> + Send {
    stream::once(async move { run_blocking(spec, op, request) }).flat_map(stream::iter)
}

fn run_blocking(spec: SpawnSpec, op: HelperOp, request: HelperRequest) -> Vec<HelperEvent> {
    if !spec.helper_available {
        return vec![HelperEvent::SpawnFailed {
            op,
            error: helper_missing_message().to_string(),
        }];
    }

    let mut events = Vec::new();
    match HelperClient::spawn(&spec) {
        Ok(mut client) => {
            events.push(HelperEvent::Spawned { op: op.clone() });
            if let Err(err) = client.send(&request) {
                events.push(HelperEvent::SpawnFailed {
                    op,
                    error: err.to_string(),
                });
                return events;
            }

            let timeout = timeout_for(&op);
            loop {
                let response = if let Some(timeout) = timeout {
                    client.recv_timeout(timeout)
                } else {
                    client.recv()
                };
                match response {
                    Some(response) => {
                        let terminal = is_terminal(&response);
                        events.push(HelperEvent::Response {
                            op: op.clone(),
                            response: Box::new(response),
                        });
                        if terminal {
                            break;
                        }
                    }
                    None => {
                        if timeout.is_some() {
                            events.push(HelperEvent::Timeout { op: op.clone() });
                        } else {
                            events.push(HelperEvent::Closed {
                                op: op.clone(),
                                error: Some("helper stdout closed".into()),
                            });
                        }
                        break;
                    }
                }
            }
        }
        Err(err) => {
            events.push(HelperEvent::SpawnFailed {
                op,
                error: err.to_string(),
            });
        }
    }
    events
}

fn timeout_for(op: &HelperOp) -> Option<Duration> {
    match op {
        HelperOp::ReadState => Some(Duration::from_secs(5)),
        HelperOp::EnsureDirectories => Some(Duration::from_secs(10)),
        HelperOp::WriteState => Some(Duration::from_secs(3)),
        HelperOp::Apply { .. } => None,
        HelperOp::ListGenerations | HelperOp::GetDiskUsage | HelperOp::RunMaintenance { .. } => {
            Some(Duration::from_secs(60))
        }
        _ => Some(Duration::from_secs(30)),
    }
}

fn is_terminal(response: &common::ipc::HelperResponse) -> bool {
    use common::ipc::HelperResponse;
    !matches!(response, HelperResponse::Log { .. })
}

/// Banner copy when the host helper cannot be found (packaging §3).
#[must_use]
pub fn helper_missing_message() -> &'static str {
    "Privileged helper not found. You can preview configuration and save local preferences. Apply, generations, and maintenance require nixos-toolkit-helper on the host (NixOS Toolkit system module) and will ask for authentication."
}

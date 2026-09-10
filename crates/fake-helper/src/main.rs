//! JSON-line helper double. Never touches `/etc/nixos`.
//!
//! Reads one JSON object per stdin line. Default reply is `{"type":"Ok"}`.
//! Set `FAKE_HELPER_SCRIPT` to a JSONL file (one `HelperResponse` per request)
//! to script replies in order. Extra requests after the script ends still get Ok.
//!
//! Script lines may be raw tagged JSON (`{"type":"State","payload":…}`) or the
//! shorthands `Ok`, `State`, `ApplyComplete`, and `Log`.

use common::ipc::{AppState, HelperResponse, LogLevel};
use std::fs;
use std::io::{self, BufRead, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

const DEFAULT_OK: &str = r#"{"type":"Ok"}"#;
const INVALID_JSON: &str =
    r#"{"type":"Error","payload":{"message":"Invalid request format","details":null}}"#;

fn main() -> ExitCode {
    match run(io::stdin().lock(), io::stdout(), script_from_env()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("fake-helper: {err}");
            ExitCode::FAILURE
        }
    }
}

fn script_from_env() -> Option<PathBuf> {
    std::env::var_os("FAKE_HELPER_SCRIPT")
        .map(PathBuf::from)
        .filter(|path| !path.as_os_str().is_empty())
}

fn load_script(path: &Path) -> io::Result<Vec<String>> {
    let text = fs::read_to_string(path)?;
    Ok(text
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(ToOwned::to_owned)
        .collect())
}

fn serialize_response(response: &HelperResponse) -> String {
    serde_json::to_string(response).expect("HelperResponse always serializes")
}

fn canned_state() -> HelperResponse {
    HelperResponse::State(AppState::default())
}

fn canned_apply_complete() -> HelperResponse {
    HelperResponse::ApplyComplete {
        success: true,
        message: "ok".into(),
    }
}

fn canned_log() -> HelperResponse {
    HelperResponse::Log {
        level: LogLevel::Info,
        message: "building".into(),
    }
}

/// Expand a script line: shorthand names or raw tagged JSON.
fn expand_scripted(line: &str) -> String {
    match line.trim() {
        "Ok" => DEFAULT_OK.to_owned(),
        "State" => serialize_response(&canned_state()),
        "ApplyComplete" => serialize_response(&canned_apply_complete()),
        "Log" => serialize_response(&canned_log()),
        other => other.to_owned(),
    }
}

fn reply_for(request_line: &str, scripted: Option<&str>) -> String {
    if let Some(line) = scripted {
        return expand_scripted(line);
    }
    if serde_json::from_str::<serde_json::Value>(request_line).is_err() {
        return INVALID_JSON.to_owned();
    }
    DEFAULT_OK.to_owned()
}

fn run<R, W>(reader: R, mut writer: W, script_path: Option<PathBuf>) -> io::Result<()>
where
    R: BufRead,
    W: Write,
{
    let replies = match script_path {
        Some(path) => load_script(&path)?,
        None => Vec::new(),
    };
    let mut idx = 0usize;
    for line in reader.lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        let scripted = replies.get(idx).map(String::as_str);
        idx += 1;
        writeln!(writer, "{}", reply_for(&line, scripted))?;
        writer.flush()?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use common::ipc::{HelperRequest, HelperResponse};
    use std::io::Cursor;

    fn parse_response(json: &str) -> HelperResponse {
        serde_json::from_str(json).unwrap_or_else(|err| panic!("parse {json}: {err}"))
    }

    fn assert_tagged(json: &str, type_name: &str) {
        let value: serde_json::Value = serde_json::from_str(json).expect("json object");
        assert_eq!(value["type"], type_name, "{json}");
        if type_name == "Ok" {
            assert!(value.get("payload").is_none());
        } else {
            assert!(value.get("payload").is_some(), "{json}");
        }
    }

    #[test]
    fn default_reply_is_ok() {
        assert_eq!(reply_for(r#"{"type":"ReadState"}"#, None), DEFAULT_OK);
        assert_tagged(DEFAULT_OK, "Ok");
        assert!(matches!(parse_response(DEFAULT_OK), HelperResponse::Ok));
    }

    #[test]
    fn invalid_json_is_error() {
        let reply = reply_for("not-json", None);
        assert_tagged(&reply, "Error");
        match parse_response(&reply) {
            HelperResponse::Error { message, details } => {
                assert_eq!(message, "Invalid request format");
                assert!(details.is_none());
            }
            other => panic!("expected Error, got {other:?}"),
        }
    }

    #[test]
    fn scripted_line_wins() {
        let scripted = serialize_response(&canned_state());
        assert_eq!(
            reply_for(r#"{"type":"ReadState"}"#, Some(&scripted)),
            scripted
        );
    }

    #[test]
    fn shorthand_state_apply_complete_and_log_match_tagged_ipc() {
        let state_json = reply_for(r#"{"type":"ReadState"}"#, Some("State"));
        assert_tagged(&state_json, "State");
        assert!(matches!(
            parse_response(&state_json),
            HelperResponse::State(_)
        ));

        let apply_json = reply_for(r#"{"type":"Apply"}"#, Some("ApplyComplete"));
        assert_tagged(&apply_json, "ApplyComplete");
        match parse_response(&apply_json) {
            HelperResponse::ApplyComplete { success, message } => {
                assert!(success);
                assert_eq!(message, "ok");
            }
            other => panic!("expected ApplyComplete, got {other:?}"),
        }

        let log_json = reply_for(r#"{"type":"Apply"}"#, Some("Log"));
        assert_tagged(&log_json, "Log");
        match parse_response(&log_json) {
            HelperResponse::Log { level, message } => {
                assert_eq!(level, LogLevel::Info);
                assert_eq!(message, "building");
            }
            other => panic!("expected Log, got {other:?}"),
        }
    }

    #[test]
    fn raw_tagged_json_round_trips() {
        let samples = [
            canned_state(),
            canned_apply_complete(),
            canned_log(),
            HelperResponse::Ok,
        ];
        for sample in samples {
            let json = serialize_response(&sample);
            let back = parse_response(&json);
            assert_eq!(serialize_response(&back), json);
        }
    }

    #[test]
    fn read_state_request_is_tagged() {
        let req: HelperRequest = serde_json::from_str(r#"{"type":"ReadState"}"#).expect("request");
        assert!(matches!(req, HelperRequest::ReadState));
    }

    #[test]
    fn echoes_ok_for_each_json_line() {
        let input = Cursor::new("{\"type\":\"ReadState\"}\n\n{\"type\":\"GetSystemInfo\"}\n");
        let mut out = Vec::new();
        run(input, &mut out, None).expect("run");
        let text = String::from_utf8(out).expect("utf8");
        let lines: Vec<_> = text.lines().collect();
        assert_eq!(lines, [DEFAULT_OK, DEFAULT_OK]);
    }

    #[test]
    fn script_file_emits_typed_replies() {
        let path = std::env::temp_dir().join(format!(
            "fake-helper-script-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock")
                .as_nanos()
        ));
        std::fs::write(&path, "State\nLog\nApplyComplete\n").expect("write script");
        let input =
            Cursor::new("{\"type\":\"ReadState\"}\n{\"type\":\"Apply\"}\n{\"type\":\"Apply\"}\n");
        let mut out = Vec::new();
        run(input, &mut out, Some(path.clone())).expect("run");
        let _ = std::fs::remove_file(&path);
        let lines: Vec<_> = String::from_utf8(out)
            .expect("utf8")
            .lines()
            .map(str::to_owned)
            .collect();
        assert_eq!(lines.len(), 3);
        assert!(matches!(
            parse_response(&lines[0]),
            HelperResponse::State(_)
        ));
        assert!(matches!(
            parse_response(&lines[1]),
            HelperResponse::Log { .. }
        ));
        assert!(matches!(
            parse_response(&lines[2]),
            HelperResponse::ApplyComplete { .. }
        ));
    }
}

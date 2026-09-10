//! JSON-line helper double. Never touches `/etc/nixos`.
//!
//! Reads one JSON object per stdin line. Default reply is `{"type":"Ok"}`.
//! Set `FAKE_HELPER_SCRIPT` to a JSONL file (one `HelperResponse` per request)
//! to script replies in order. Extra requests after the script ends still get Ok.

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

fn reply_for(request_line: &str, scripted: Option<&str>) -> String {
    if let Some(line) = scripted {
        return line.to_owned();
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
    use std::io::Cursor;

    #[test]
    fn default_reply_is_ok() {
        assert_eq!(reply_for(r#"{"type":"ReadState"}"#, None), DEFAULT_OK);
    }

    #[test]
    fn invalid_json_is_error() {
        let reply = reply_for("not-json", None);
        assert!(reply.contains("Error"));
    }

    #[test]
    fn scripted_line_wins() {
        let scripted = r#"{"type":"State","payload":{}}"#;
        assert_eq!(
            reply_for(r#"{"type":"ReadState"}"#, Some(scripted)),
            scripted
        );
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
}

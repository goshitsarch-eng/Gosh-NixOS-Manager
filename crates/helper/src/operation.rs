//! Cooperative cancellation of privileged subprocess groups.
use std::cell::RefCell;
use std::io::{self, Read};
use std::os::unix::process::CommandExt;
use std::process::{Command, Output, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc};
use std::time::{Duration, Instant};

thread_local! {
    static CANCELLATION: RefCell<Arc<AtomicBool>> = RefCell::new(Arc::new(AtomicBool::new(false)));
}

pub fn with_cancellation<T>(token: Arc<AtomicBool>, action: impl FnOnce() -> T) -> T {
    CANCELLATION.with(|current| {
        let previous = current.replace(token);
        let result = action();
        current.replace(previous);
        result
    })
}

pub fn cancelled() -> bool {
    CANCELLATION.with(|token| token.borrow().load(Ordering::SeqCst))
}

fn interrupted() -> io::Error {
    io::Error::new(
        io::ErrorKind::Interrupted,
        "Operation interrupted; verify the running system before retrying",
    )
}

fn signal_group(id: u32, signal: i32) {
    // SAFETY: The child starts its own process group. A negative pid targets
    // that group, allowing a root helper to stop its own command descendants.
    unsafe {
        libc::kill(-(id as i32), signal);
    }
}

struct CommandGroup(u32, bool);
impl Drop for CommandGroup {
    fn drop(&mut self) {
        if self.1 {
            signal_group(self.0, libc::SIGKILL);
        }
    }
}

/// Drain both output pipes while monitoring cancellation. Capture is bounded;
/// streamed rebuild logs remain available even when capture is truncated.
pub fn run(command: &mut Command, mut log: impl FnMut(bool, &str)) -> io::Result<Output> {
    if cancelled() {
        return Err(interrupted());
    }
    command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .process_group(0);
    let mut child = command.spawn()?;
    let id = child.id();
    let mut group = CommandGroup(id, true);
    let (sender, receiver) = mpsc::sync_channel(64);
    let stdout = child.stdout.take().expect("piped stdout");
    let stderr = child.stderr.take().expect("piped stderr");
    let readers: Vec<_> = [
        (false, Box::new(stdout) as Box<dyn Read + Send>),
        (true, Box::new(stderr) as Box<dyn Read + Send>),
    ]
    .into_iter()
    .map(|(is_error, mut pipe)| {
        let sender = sender.clone();
        std::thread::spawn(move || {
            let mut buffer = [0; 8192];
            loop {
                match pipe.read(&mut buffer) {
                    Ok(0) => break,
                    Ok(length) => {
                        if sender.send((is_error, buffer[..length].to_vec())).is_err() {
                            break;
                        }
                    }
                    Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
                    Err(_) => break,
                }
            }
        })
    })
    .collect();
    drop(sender);
    let mut captured = [Vec::new(), Vec::new()];
    let mut pending = [Vec::new(), Vec::new()];
    let mut stopped = None;
    let mut status = None;
    let mut closed = false;
    const CAPTURE_LIMIT: usize = 256 * 1024;
    loop {
        if cancelled() && stopped.is_none() {
            signal_group(id, libc::SIGTERM);
            stopped = Some(Instant::now());
        }
        if stopped.is_some_and(|at| at.elapsed() >= Duration::from_secs(2)) {
            signal_group(id, libc::SIGKILL);
        }
        match receiver.recv_timeout(Duration::from_millis(25)) {
            Ok((is_error, bytes)) => {
                let index = usize::from(is_error);
                let room = CAPTURE_LIMIT.saturating_sub(captured[index].len());
                captured[index].extend_from_slice(&bytes[..bytes.len().min(room)]);
                pending[index].extend_from_slice(&bytes);
                while let Some(end) = pending[index].iter().position(|byte| *byte == b'\n') {
                    log(is_error, &String::from_utf8_lossy(&pending[index][..end]));
                    pending[index].drain(..=end);
                }
                if pending[index].len() >= 8192 {
                    log(is_error, &String::from_utf8_lossy(&pending[index]));
                    pending[index].clear();
                }
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => closed = true,
            Err(mpsc::RecvTimeoutError::Timeout) => {}
        }
        if status.is_none() {
            status = child.try_wait()?;
        }
        if status.is_some() && closed {
            if stopped.is_some() {
                signal_group(id, libc::SIGKILL);
            }
            break;
        }
        if closed {
            std::thread::sleep(Duration::from_millis(25));
        }
    }
    for reader in readers {
        let _ = reader.join();
    }
    for (index, bytes) in pending.iter().enumerate() {
        if !bytes.is_empty() {
            log(index == 1, &String::from_utf8_lossy(bytes));
        }
    }
    if stopped.is_some() {
        return Err(interrupted());
    }
    for output in &mut captured {
        if output.len() == CAPTURE_LIMIT {
            output.extend_from_slice(b"\n[Captured output truncated; see streamed logs]\n");
        }
    }
    let [stdout, stderr] = captured;
    group.1 = false;
    Ok(Output {
        status: status.expect("waited child"),
        stdout,
        stderr,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cancels_command_group_and_drains_both_pipes() {
        let token = Arc::new(AtomicBool::new(false));
        let trigger = token.clone();
        let cancellation = std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(150));
            trigger.store(true, Ordering::SeqCst);
        });
        let start = Instant::now();
        let result = with_cancellation(token, || {
            run(
                Command::new("sh").args([
                    "-c",
                    "trap '' TERM; echo started; echo error >&2; sleep 30 & wait",
                ]),
                |_, _| {},
            )
        });
        cancellation.join().unwrap();
        assert_eq!(result.unwrap_err().kind(), io::ErrorKind::Interrupted);
        assert!(start.elapsed() < Duration::from_secs(5));
    }

    #[test]
    fn preserves_exit_status_and_bounds_capture() {
        let output = run(
            Command::new("sh").args(["-c", "printf ok; printf error >&2; exit 7"]),
            |_, _| {},
        )
        .unwrap();
        assert_eq!(output.status.code(), Some(7));
        assert_eq!(output.stdout, b"ok");
        assert_eq!(output.stderr, b"error");
        let output = run(
            Command::new("sh").args(["-c", "head -c 1048576 /dev/zero"]),
            |_, _| {},
        )
        .unwrap();
        assert!(output.stdout.len() < 300_000);
    }
}

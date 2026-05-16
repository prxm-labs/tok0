use anyhow::{bail, Context, Result};
use std::io::Read;
use std::process::{Command, Output, Stdio};
use std::time::{Duration, Instant};

/// Env var to override the default per-command timeout, in seconds.
/// Useful for slow builds: `TOK0_COMMAND_TIMEOUT=600 tok0 cargo build`.
/// Precedence: env > caller-supplied default. Invalid values are ignored
/// silently (no warning) so a malformed value can't break the run path.
const TIMEOUT_ENV: &str = "TOK0_COMMAND_TIMEOUT";

/// If `TOK0_COMMAND_TIMEOUT` is set to a positive integer (seconds),
/// returns that as a `Duration`. Otherwise returns the caller default.
/// Pure + testable via env mutation.
fn resolve_timeout(default: Duration) -> Duration {
    match std::env::var(TIMEOUT_ENV) {
        Ok(v) => match v.trim().parse::<u64>() {
            Ok(n) if n > 0 => Duration::from_secs(n),
            _ => default,
        },
        Err(_) => default,
    }
}

/// Execute a command with a timeout. Returns the output if the command
/// completes within the timeout, or an error if it times out.
///
/// The child inherits stdin so pipelines like `cat file | tok0 grep
/// pattern` deliver bytes to the spawned command. stdout/stderr are
/// drained concurrently on background threads so large outputs (cargo
/// build, npm install) don't deadlock when the OS pipe buffer fills.
pub fn execute_with_timeout(cmd: &str, args: &[&str], timeout: Duration) -> Result<Output> {
    let timeout = resolve_timeout(timeout);

    let mut child = Command::new(cmd)
        .args(args)
        .stdin(Stdio::inherit())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .with_context(|| format!("Failed to spawn: {} {}", cmd, args.join(" ")))?;

    // Take ownership of the pipe handles so the child won't deadlock
    // waiting for us to read them (a >64 KB cargo build trivially
    // fills the OS pipe buffer otherwise).
    let mut stdout = child.stdout.take().expect("stdout piped");
    let mut stderr = child.stderr.take().expect("stderr piped");
    let stdout_thread = std::thread::spawn(move || {
        let mut buf = Vec::new();
        let _ = stdout.read_to_end(&mut buf);
        buf
    });
    let stderr_thread = std::thread::spawn(move || {
        let mut buf = Vec::new();
        let _ = stderr.read_to_end(&mut buf);
        buf
    });

    let start = Instant::now();
    let status = loop {
        match child
            .try_wait()
            .context("Failed to check child process status")?
        {
            Some(status) => break status,
            None => {
                if start.elapsed() >= timeout {
                    let _ = child.kill();
                    let _ = child.wait();
                    // Join the reader threads so we don't leak them.
                    let _ = stdout_thread.join();
                    let _ = stderr_thread.join();
                    bail!(
                        "Command timed out after {:.1}s: {} {}",
                        timeout.as_secs_f64(),
                        cmd,
                        args.join(" ")
                    );
                }
                std::thread::sleep(Duration::from_millis(10));
            }
        }
    };

    let stdout = stdout_thread.join().unwrap_or_default();
    let stderr = stderr_thread.join().unwrap_or_default();
    Ok(Output {
        status,
        stdout,
        stderr,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    /// Env-mutation tests must serialize — std::env is process-wide.
    static ENV_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn test_command_completes_within_timeout() {
        let result = execute_with_timeout("echo", &["hello"], Duration::from_secs(5));
        assert!(result.is_ok(), "echo should complete within timeout");
        let output = result.expect("should succeed");
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("hello"));
    }

    #[test]
    fn test_command_timeout() {
        let result = execute_with_timeout("sleep", &["10"], Duration::from_millis(100));
        assert!(result.is_err(), "sleep 10 should timeout at 100ms");
        let err = result.unwrap_err();
        let msg = format!("{:#}", err);
        assert!(
            msg.contains("timed out"),
            "Error should mention timeout: {}",
            msg
        );
    }

    #[test]
    fn test_command_not_found() {
        let result = execute_with_timeout("nonexistent_cmd_xyz", &[], Duration::from_secs(1));
        assert!(result.is_err(), "nonexistent command should fail");
    }

    #[test]
    fn test_command_with_args() {
        let result = execute_with_timeout("echo", &["one", "two", "three"], Duration::from_secs(5));
        assert!(result.is_ok());
        let output = result.expect("should succeed");
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("one two three"));
    }

    #[test]
    fn test_command_exit_code_propagation() {
        let result = execute_with_timeout("false", &[], Duration::from_secs(5));
        assert!(
            result.is_ok(),
            "false should complete (just with non-zero exit)"
        );
        let output = result.expect("should get output");
        assert!(
            !output.status.success(),
            "false should have non-zero exit code"
        );
    }

    #[test]
    fn test_short_timeout_fast_command() {
        let result = execute_with_timeout("true", &[], Duration::from_millis(500));
        assert!(result.is_ok(), "true should complete within 500ms");
    }

    /// Large outputs (>64 KB pipe buffer) must drain concurrently —
    /// otherwise the child blocks writing and we never see the exit.
    ///
    /// Cross-platform: `bash` on Unix (always present, brace expansion
    /// avoids the `seq` coreutil), Windows PowerShell on Windows (which
    /// is guaranteed on windows-latest and dodges the `System32\bash.exe`
    /// WSL shim that fails silently when WSL is disabled).
    #[test]
    fn test_large_output_no_deadlock() {
        let line = "x".repeat(200);

        #[cfg(unix)]
        let result = execute_with_timeout(
            "bash",
            &[
                "-c",
                &format!("for i in {{1..1000}}; do echo {}; done", line),
            ],
            Duration::from_secs(5),
        );

        #[cfg(windows)]
        let result = execute_with_timeout(
            "powershell",
            &[
                "-NoProfile",
                "-Command",
                &format!("1..1000 | ForEach-Object {{ Write-Output '{}' }}", line),
            ],
            Duration::from_secs(5),
        );

        let output = result.expect("should not deadlock");
        assert!(
            output.status.success(),
            "child exited non-zero (status={:?}); stderr={}",
            output.status.code(),
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(
            output.stdout.len() > 150_000,
            "got {} bytes — drain might be deadlocked",
            output.stdout.len()
        );
    }

    #[test]
    fn resolve_timeout_default_when_unset() {
        let _g = ENV_LOCK.lock().expect("lock");
        std::env::remove_var(TIMEOUT_ENV);
        assert_eq!(
            resolve_timeout(Duration::from_secs(30)),
            Duration::from_secs(30)
        );
    }

    #[test]
    fn resolve_timeout_env_override() {
        let _g = ENV_LOCK.lock().expect("lock");
        std::env::set_var(TIMEOUT_ENV, "120");
        assert_eq!(
            resolve_timeout(Duration::from_secs(30)),
            Duration::from_secs(120)
        );
        std::env::remove_var(TIMEOUT_ENV);
    }

    #[test]
    fn resolve_timeout_ignores_malformed_env() {
        let _g = ENV_LOCK.lock().expect("lock");
        for bad in ["", "abc", "-1", "0", "1.5"] {
            std::env::set_var(TIMEOUT_ENV, bad);
            assert_eq!(
                resolve_timeout(Duration::from_secs(30)),
                Duration::from_secs(30),
                "malformed env {bad:?} must fall back to default"
            );
        }
        std::env::remove_var(TIMEOUT_ENV);
    }
}

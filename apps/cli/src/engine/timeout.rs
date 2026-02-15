use anyhow::{bail, Context, Result};
use std::process::{Command, Output, Stdio};
use std::time::{Duration, Instant};

/// Execute a command with a timeout. Returns the output if the command
/// completes within the timeout, or an error if it times out.
pub fn execute_with_timeout(cmd: &str, args: &[&str], timeout: Duration) -> Result<Output> {
    let mut child = Command::new(cmd)
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .with_context(|| format!("Failed to spawn: {} {}", cmd, args.join(" ")))?;

    let start = Instant::now();

    loop {
        match child
            .try_wait()
            .context("Failed to check child process status")?
        {
            Some(_status) => {
                return child
                    .wait_with_output()
                    .with_context(|| format!("Failed to get output: {} {}", cmd, args.join(" ")));
            }
            None => {
                if start.elapsed() >= timeout {
                    let _ = child.kill();
                    let _ = child.wait();
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
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
        // Even with a short timeout, a fast command should succeed
        let result = execute_with_timeout("true", &[], Duration::from_millis(500));
        assert!(result.is_ok(), "true should complete within 500ms");
    }
}

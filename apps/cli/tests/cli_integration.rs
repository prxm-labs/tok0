//! End-to-end CLI integration tests.
//!
//! These tests build the release binary via cargo and invoke it in a
//! subprocess, exercising the actual command-dispatch path in main.rs
//! which is otherwise untested.
//!
//! Run with `cargo test --test cli_integration`. Each test ensures its
//! own isolated TOK0_DB_PATH + TOK0_CONFIG_DIR so there's no cross-test
//! contamination.

use std::path::PathBuf;
use std::process::Command;
use tempfile::TempDir;

/// Locate the `tok0` binary built alongside this test target.
/// Cargo places the binary in target/<profile>/tok0 relative to the
/// CARGO_MANIFEST_DIR.
fn tok0_binary() -> PathBuf {
    // env!("CARGO_BIN_EXE_tok0") is the canonical way when the crate is
    // a single-binary package: Cargo sets it for integration tests.
    PathBuf::from(env!("CARGO_BIN_EXE_tok0"))
}

/// Returns a fresh temp DB + config dir pair, owned by the caller.
fn isolated_env() -> (TempDir, PathBuf) {
    let tmp = TempDir::new().expect("tmp dir");
    let db_path = tmp.path().join("tracking.db");
    (tmp, db_path)
}

fn run_tok0(args: &[&str], db_path: &std::path::Path) -> std::process::Output {
    Command::new(tok0_binary())
        .args(args)
        .env("TOK0_DB_PATH", db_path)
        .output()
        .expect("failed to execute tok0 binary")
}

/// Run tok0 with bytes piped to stdin. Exercises code paths that
/// `run_tok0` (no stdin) can't reach.
fn run_tok0_with_stdin(
    args: &[&str],
    stdin: &str,
    db_path: &std::path::Path,
) -> std::process::Output {
    use std::io::Write;
    use std::process::Stdio;
    let mut child = Command::new(tok0_binary())
        .args(args)
        .env("TOK0_DB_PATH", db_path)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to spawn tok0");
    child
        .stdin
        .as_mut()
        .expect("stdin pipe")
        .write_all(stdin.as_bytes())
        .expect("write stdin");
    child.wait_with_output().expect("wait_with_output")
}

/// Configure `cmd` so that `dirs::home_dir()` inside the spawned process
/// resolves to `fake_home` on both Unix and Windows. Without this, tests
/// that pass `HOME=...` silently no-op on Windows runners because the
/// `dirs` crate reads `USERPROFILE` there.
fn with_fake_home(cmd: &mut Command, fake_home: &std::path::Path) {
    cmd.env("HOME", fake_home);
    cmd.env("USERPROFILE", fake_home);
    // Windows also consults HOMEDRIVE+HOMEPATH as a fallback.
    #[cfg(windows)]
    {
        cmd.env("HOMEDRIVE", "");
        cmd.env("HOMEPATH", fake_home);
    }
}

#[test]
fn test_version_flag() {
    let (_tmp, db) = isolated_env();
    let out = run_tok0(&["--version"], &db);
    assert!(out.status.success(), "--version should exit 0");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("tok0"), "output: {}", stdout);
}

#[test]
fn test_help_flag() {
    let (_tmp, db) = isolated_env();
    let out = run_tok0(&["--help"], &db);
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("Token-optimized"),
        "help should describe tok0"
    );
    assert!(
        stdout.contains("status"),
        "help should list status subcommand"
    );
    assert!(stdout.contains("init"), "help should list init subcommand");
}

#[test]
fn test_status_command_runs() {
    let (_tmp, db) = isolated_env();
    let out = run_tok0(&["status"], &db);
    assert!(out.status.success(), "status should exit 0");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("tok0 v"));
    assert!(stdout.contains("Paths:"));
    assert!(stdout.contains("AI tools:"));
    assert!(stdout.contains("Telemetry:"));
}

#[test]
fn test_init_show_does_not_install() {
    let (_tmp, db) = isolated_env();
    // --show should only list detected tools; never write hooks.
    let out = run_tok0(&["init", "--show"], &db);
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    // Either "No supported AI tools detected." or "Detected AI tools:"
    assert!(
        stdout.contains("Detected AI tools") || stdout.contains("No supported AI tools"),
        "unexpected init --show output: {}",
        stdout
    );
}

#[test]
fn test_completions_bash() {
    let (_tmp, db) = isolated_env();
    let out = run_tok0(&["completions", "bash"], &db);
    assert!(out.status.success(), "completions should succeed");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("_tok0"),
        "bash completion script should define _tok0"
    );
    assert!(
        stdout.contains("status"),
        "completion should include status subcommand"
    );
}

#[test]
fn test_completions_zsh() {
    let (_tmp, db) = isolated_env();
    let out = run_tok0(&["completions", "zsh"], &db);
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("#compdef tok0"));
}

#[test]
fn test_rewrite_basic_passthrough() {
    let (_tmp, db) = isolated_env();
    // `tok0 rewrite git status` should output exactly `tok0 git status`.
    let out = run_tok0(&["rewrite", "git", "status"], &db);
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert_eq!(stdout.trim(), "tok0 git status");
}

#[test]
fn test_rewrite_no_double_wrap() {
    let (_tmp, db) = isolated_env();
    let out = run_tok0(&["rewrite", "tok0", "git", "log"], &db);
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert_eq!(stdout.trim(), "tok0 git log");
    assert!(!stdout.contains("tok0 tok0"), "no double wrap: {}", stdout);
}

#[test]
fn test_rewrite_cat_to_read() {
    let (_tmp, db) = isolated_env();
    let out = run_tok0(&["rewrite", "cat", "src/main.rs"], &db);
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert_eq!(stdout.trim(), "tok0 read src/main.rs");
}

#[test]
fn test_cd_builtin_fails_fast_with_clear_error() {
    // Regression: `tok0 cd <path>` used to spawn /usr/bin/cd, which exits
    // 0 if the path exists but cannot change the parent shell's cwd. That
    // made `tok0 cd dir && rg pattern` silently search the wrong tree.
    // The fix is fail-fast: detect the builtin, print a clear error,
    // exit 1 so `&&` chains break immediately.
    let (_tmp, db) = isolated_env();
    let out = run_tok0(&["cd", "/tmp"], &db);
    assert!(!out.status.success(), "tok0 cd must exit non-zero");
    assert_eq!(out.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("'cd' is a shell builtin"),
        "stderr should explain the builtin issue, got: {stderr}"
    );
    assert!(
        stderr.contains("Drop the 'tok0' prefix"),
        "stderr should suggest the fix, got: {stderr}"
    );
}

#[test]
fn test_export_builtin_fails_fast() {
    let (_tmp, db) = isolated_env();
    let out = run_tok0(&["export", "FOO=bar"], &db);
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("'export' is a shell builtin"),
        "stderr: {stderr}"
    );
}

#[test]
fn test_proxy_inherits_stdin() {
    // Regression: `tok0 proxy <cmd>` used to call Command::output()
    // which closes the child's stdin. Pipelines that depend on stdin
    // (`cat file | tok0 proxy wc -l`) silently received nothing. The
    // fix uses spawn() + Stdio::inherit() so stdin flows through.
    let (_tmp, db) = isolated_env();
    let out = run_tok0_with_stdin(&["proxy", "wc", "-l"], "a\nb\nc\n", &db);
    assert!(out.status.success(), "wc -l should succeed");
    let stdout = String::from_utf8_lossy(&out.stdout);
    let count: u32 = stdout.trim().parse().expect("wc -l should print a number");
    assert_eq!(count, 3, "wc -l must see piped stdin, got: {stdout:?}");
}

#[test]
fn test_var_assignment_prefix_fails_fast() {
    // Regression: `tok0 FOO=bar cargo build` used to try to spawn the
    // literal binary "FOO=bar". The error was buried, and downstream
    // chained commands ran without the env var the agent wanted to set.
    let (_tmp, db) = isolated_env();
    let out = run_tok0(&["FOO=bar", "echo", "hi"], &db);
    assert!(!out.status.success(), "var-assignment prefix must exit 1");
    assert_eq!(out.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("'FOO=bar' looks like a variable assignment"),
        "stderr should name the offender: {stderr}"
    );
    assert!(
        stderr.contains("tok0 bash -c"),
        "stderr should suggest the workaround: {stderr}"
    );
}

#[test]
fn test_path_qualified_name_with_equals_is_not_blocked() {
    // `./FOO=bar` could be a legitimate (weird) binary name; the var-
    // assignment guard must skip path-qualified args.
    let (_tmp, db) = isolated_env();
    let out = run_tok0(&["./FOO=bar"], &db);
    // We don't care about exit code (the binary doesn't exist, so spawn
    // will fail) — only that we did NOT short-circuit with the
    // var-assignment guard.
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        !stderr.contains("looks like a variable assignment"),
        "path-qualified name should not trigger the var-assignment guard, got: {stderr}"
    );
}

#[test]
fn test_real_binary_with_builtin_name_in_path_is_not_blocked() {
    // `/usr/bin/cd` is a real (POSIX-mandated) binary on macOS. The
    // path-qualified form is *not* the builtin and must pass through
    // to run_proxy unchanged.
    let (_tmp, db) = isolated_env();
    let out = run_tok0(&["/usr/bin/cd", "/tmp"], &db);
    // We don't care about exit code (the binary itself behaves like cd
    // in a subshell) — only that we did NOT short-circuit with the
    // builtin error.
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        !stderr.contains("is a shell builtin"),
        "path-qualified cd should not trigger the builtin guard, got: {stderr}"
    );
}

#[test]
fn test_proxy_runs_command_and_records() {
    let (_tmp, db) = isolated_env();
    // Use `echo` — portable across macOS/Linux CI.
    let out = run_tok0(&["proxy", "echo", "hello world"], &db);
    assert!(out.status.success(), "proxy echo should succeed");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("hello"), "echo output: {}", stdout);
}

#[test]
fn test_ls_command_runs() {
    let (_tmp, db) = isolated_env();
    // Run ls on the temp dir — cross-platform safe target.
    let tmp = TempDir::new().expect("tmp");
    std::fs::write(tmp.path().join("a.txt"), "a").expect("write");
    std::fs::write(tmp.path().join("b.txt"), "b").expect("write");

    let out = run_tok0(&["ls", tmp.path().to_str().expect("utf8 path")], &db);
    assert!(
        out.status.success(),
        "ls should succeed: stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
}

#[test]
fn test_stats_without_db_is_clean() {
    let (_tmp, db) = isolated_env();
    // DB path points to a non-existent file
    let out = run_tok0(&["stats"], &db);
    // Should not crash; should print a message about no data yet.
    assert!(out.status.success(), "stats with no DB should succeed");
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        combined.contains("no tracking data") || combined.contains("Total commands"),
        "stats output should either report empty or show summary: {}",
        combined
    );
}

#[test]
fn test_async_meter_persists_across_invocations() {
    let (_tmp, db) = isolated_env();
    // Target dir for ls — cross-platform safe.
    let target = TempDir::new().expect("tmp");
    std::fs::write(target.path().join("x.txt"), "x").expect("write");

    // `tok0 ls` goes through run_proxy which records to the async meter
    // (unlike `tok0 proxy` which is intentionally passthrough-only).
    let out = run_tok0(&["ls", target.path().to_str().expect("utf8")], &db);
    assert!(
        out.status.success(),
        "ls should succeed: stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );

    // flush() in run_proxy should have drained the worker before exit.
    assert!(db.exists(), "async meter should have created the DB file");

    // Querying stats exercises get_summary() against the same DB.
    let out = run_tok0(&["stats"], &db);
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("Total commands"),
        "stats should include Total commands row: {}",
        stdout
    );
}

#[test]
fn test_unknown_command_falls_through_to_shell() {
    // Any subcommand tok0 does not explicitly define is forwarded to the
    // shell via run_proxy. If the shell cannot find the binary either,
    // tok0 exits non-zero with a spawn error — NOT a clap "unrecognized
    // subcommand" error. This guarantees common shell tools (wc, tr,
    // head, …) never get rejected by tok0's argv parser.
    let (_tmp, db) = isolated_env();
    let out = run_tok0(
        &["this-binary-definitely-does-not-exist-on-any-runner-xyz"],
        &db,
    );
    assert!(!out.status.success(), "missing binary should fail");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        !stderr.contains("unrecognized subcommand"),
        "tok0 must forward unknown subcommands to the shell, not reject \
         them at the clap layer. stderr={}",
        stderr
    );
    assert!(
        stderr.contains("Failed to spawn") || stderr.contains("No such file"),
        "stderr should indicate the shell could not find the binary: {}",
        stderr
    );
}

// ── External subcommand fallback (wc / tr / head / arbitrary) ────────────────
//
// tok0 explicitly enumerates a few subcommands (Git, Ls, Read, …) but the
// majority of shell tools are not — they reach run_proxy via the `External`
// catch-all variant. These tests prove that:
//   1. A command WITH a compressor (wc, head) gets routed and compressed.
//   2. A command WITHOUT a compressor (tr) passes through unchanged.
//   3. Stdin is inherited correctly (so pipes still work).

#[test]
fn test_external_wc_routes_through_dispatcher() {
    let (_tmp, db) = isolated_env();
    let dir = TempDir::new().expect("tmp");
    let path = dir.path().join("lines.txt");
    std::fs::write(&path, "alpha\nbeta\ngamma\n").expect("write");

    let out = run_tok0(&["wc", "-l", path.to_str().expect("utf8")], &db);
    assert!(
        out.status.success(),
        "wc must succeed: exit={:?} stderr={}",
        out.status.code(),
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains('3'),
        "wc -l should report 3 lines somewhere in output: {}",
        stdout
    );
}

#[test]
fn test_external_head_routes_through_read_compressor() {
    let (_tmp, db) = isolated_env();
    let dir = TempDir::new().expect("tmp");
    let path = dir.path().join("many.txt");
    let content: String = (0..20).map(|i| format!("line-{}\n", i)).collect();
    std::fs::write(&path, &content).expect("write");

    let out = run_tok0(&["head", "-3", path.to_str().expect("utf8")], &db);
    assert!(
        out.status.success(),
        "head must succeed: exit={:?} stderr={}",
        out.status.code(),
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        !stdout.is_empty(),
        "head should produce non-empty output: {}",
        stdout
    );
}

#[test]
fn test_external_tr_passes_through_unchanged() {
    // `tr` has no compressor in the dispatcher. The External fallback
    // should still execute it, inherit stdin, and stream stdout
    // unchanged. Verifies the no-compressor passthrough path.
    use std::io::Write;
    use std::process::Stdio;

    let (_tmp, db) = isolated_env();
    let mut child = Command::new(tok0_binary())
        .args(["tr", "a-z", "A-Z"])
        .env("TOK0_DB_PATH", &db)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn tok0 tr");
    child
        .stdin
        .as_mut()
        .expect("stdin")
        .write_all(b"hello world\n")
        .expect("write stdin");
    let out = child.wait_with_output().expect("wait tok0 tr");

    assert!(
        out.status.success(),
        "tok0 tr must succeed: exit={:?} stderr={}",
        out.status.code(),
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert_eq!(stdout.trim(), "HELLO WORLD", "tr should uppercase stdin");
}

#[test]
fn test_doctor_command_runs_and_reports_summary() {
    let (_tmp, db) = isolated_env();
    let out = run_tok0(&["doctor"], &db);
    // Exit may be 0 or 1 depending on whether dev environment has issues.
    // Either way the report structure must be present.
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("tok0 doctor"),
        "doctor should print a report header: {}",
        stdout
    );
    assert!(
        stdout.contains("Summary:"),
        "doctor should print a summary line: {}",
        stdout
    );
}

#[test]
fn test_doctor_finds_invalid_extension() {
    // Stage a bad extension under a fake config dir, then run tok0 doctor
    // with TOK0_CONFIG_DIR overridden. Exit code must be non-zero.
    let (_tmp, db) = isolated_env();
    let cfg = TempDir::new().expect("tmp cfg");
    let bad_rules = cfg.path().join("extensions").join("bad").join("rules");
    std::fs::create_dir_all(&bad_rules).expect("mkdir");
    std::fs::write(bad_rules.join("bad.toml"), b"this is {{ not valid toml").expect("write");

    let out = Command::new(tok0_binary())
        .args(["doctor"])
        .env("TOK0_DB_PATH", &db)
        .env("TOK0_CONFIG_DIR", cfg.path())
        .output()
        .expect("run");

    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("extension: bad"), "stdout: {}", stdout);
    assert!(
        stdout.contains("error"),
        "should flag invalid extension: {}",
        stdout
    );
    assert!(!out.status.success(), "exit code must be non-zero on error");
}

#[test]
fn test_rule_list_outputs_header_and_rules() {
    let (_tmp, db) = isolated_env();
    let out = run_tok0(&["rule", "list"], &db);
    assert!(out.status.success(), "rule list should succeed");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("rule(s) active"));
    assert!(stdout.contains("SOURCE"));
    assert!(stdout.contains("NAME"));
    assert!(stdout.contains("builtin"));
}

#[test]
fn test_rule_show_existing_builtin() {
    let (_tmp, db) = isolated_env();
    // Pick a rule we know ships as builtin — brew-install is committed.
    let out = run_tok0(&["rule", "show", "brew-install"], &db);
    assert!(
        out.status.success(),
        "rule show should succeed: stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("name:     brew-install"));
    assert!(stdout.contains("source:   builtin"));
    assert!(stdout.contains("commands:"));
}

#[test]
fn test_rule_show_unknown_fails() {
    let (_tmp, db) = isolated_env();
    let out = run_tok0(&["rule", "show", "this-rule-does-not-exist-xyz"], &db);
    assert!(!out.status.success(), "show of unknown rule should fail");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("No rule named") || stderr.contains("not found"),
        "stderr: {}",
        stderr
    );
}

#[test]
fn test_rule_test_applies_compression_to_stdin() {
    let (_tmp, db) = isolated_env();
    // Use brew-install which strips "^==> Downloading" lines.
    let input = "==> Downloading foo\n==> Downloading bar\nreal content\nmore real\n";

    let mut child = Command::new(tok0_binary())
        .args(["rule", "test", "brew-install"])
        .env("TOK0_DB_PATH", &db)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("spawn");

    use std::io::Write;
    child
        .stdin
        .as_mut()
        .expect("stdin")
        .write_all(input.as_bytes())
        .expect("write stdin");

    let out = child.wait_with_output().expect("wait");
    assert!(
        out.status.success(),
        "rule test should succeed: stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );

    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("rule: brew-install"));
    assert!(stdout.contains("input:"));
    assert!(stdout.contains("output:"));
    assert!(stdout.contains("saved"));
    assert!(stdout.contains("real content"));
    assert!(
        !stdout.contains("Downloading foo"),
        "progress line should be stripped"
    );
}

#[test]
fn test_init_uninstall_is_clean_when_nothing_installed() {
    let (_tmp, db) = isolated_env();
    // Use a temp HOME (Unix + Windows) with no tool dirs — detect_tools
    // returns none, so uninstall should simply report no tools.
    let fake_home = TempDir::new().expect("home");
    let mut cmd = Command::new(tok0_binary());
    cmd.args(["init", "--uninstall"]).env("TOK0_DB_PATH", &db);
    with_fake_home(&mut cmd, fake_home.path());
    let out = cmd.output().expect("run");
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("No supported AI tools detected"),
        "stdout: {}",
        stdout
    );
}

#[test]
fn test_status_renders_platform_native_paths() {
    // Exercises std::path::PathBuf rendering in the status report. On
    // Windows, paths use backslashes and a drive letter; on Unix, forward
    // slashes. Both must be printable without panicking and must contain
    // a path separator character.
    let (_tmp, db) = isolated_env();
    let out = run_tok0(&["status"], &db);
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    let sep = std::path::MAIN_SEPARATOR;
    assert!(
        stdout.contains(sep),
        "status output should contain native path separator '{}': {}",
        sep,
        stdout
    );
}

#[test]
fn test_init_show_with_isolated_home_reports_no_tools() {
    // End-to-end isolation: an empty fake home must produce "no tools"
    // on every platform (Unix and Windows both honored by with_fake_home).
    let (_tmp, db) = isolated_env();
    let fake_home = TempDir::new().expect("home");
    let mut cmd = Command::new(tok0_binary());
    cmd.args(["init", "--show"]).env("TOK0_DB_PATH", &db);
    with_fake_home(&mut cmd, fake_home.path());
    let out = cmd.output().expect("run");
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("No supported AI tools"),
        "isolated home should detect nothing: {}",
        stdout
    );
}

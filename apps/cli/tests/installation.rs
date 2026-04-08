//! End-to-end CLI install + verify integration tests.
//!
//! Complements `cli_integration.rs` by exercising the user-facing install
//! flow against a fully-isolated fake `$HOME`. Earlier tests only covered
//! `tok0 init --show` and `--uninstall` on a clean dir; this file fills
//! the gap by:
//!   - actually installing per `ToolTarget` and asserting the resulting
//!     files at the canonical 2026 paths;
//!   - round-tripping `init` → `init --uninstall` per tool;
//!   - confirming idempotency at the CLI level (run twice → "already installed");
//!   - distinguishing `--global` vs project-local install for ClaudeCode;
//!   - covering `tok0 verify` (ok / tampered / missing) which had zero CLI
//!     coverage before this file;
//!   - chaining `init` → SHA-256 → `verify` to prove installed hooks are
//!     verifiable end to end.

use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use std::process::Command;
use tempfile::TempDir;

// ─── Helpers ─────────────────────────────────────────────────────────────────

fn tok0_binary() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_tok0"))
}

fn isolated_db() -> (TempDir, PathBuf) {
    let tmp = TempDir::new().expect("tmp dir");
    let db = tmp.path().join("tracking.db");
    (tmp, db)
}

/// Run `tok0` with isolated `$HOME` (`USERPROFILE` on Windows) and DB.
/// Mirrors the helper in `cli_integration.rs::with_fake_home` so the same
/// invariants hold across both files.
fn run_tok0(args: &[&str], home: &Path, db: &Path) -> std::process::Output {
    let mut cmd = Command::new(tok0_binary());
    cmd.args(args);
    cmd.env("TOK0_DB_PATH", db);
    cmd.env("HOME", home);
    cmd.env("USERPROFILE", home);
    #[cfg(windows)]
    {
        cmd.env("HOMEDRIVE", "");
        cmd.env("HOMEPATH", home);
    }
    cmd.output().expect("failed to execute tok0 binary")
}

fn run_tok0_in(args: &[&str], home: &Path, cwd: &Path, db: &Path) -> std::process::Output {
    let mut cmd = Command::new(tok0_binary());
    cmd.args(args);
    cmd.current_dir(cwd);
    cmd.env("TOK0_DB_PATH", db);
    cmd.env("HOME", home);
    cmd.env("USERPROFILE", home);
    #[cfg(windows)]
    {
        cmd.env("HOMEDRIVE", "");
        cmd.env("HOMEPATH", home);
    }
    cmd.output().expect("failed to execute tok0 binary")
}

fn sha256_hex(path: &Path) -> String {
    let bytes = std::fs::read(path).expect("read file for hashing");
    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    format!("{:x}", hasher.finalize())
}

// ─── Per-tool fixture table ──────────────────────────────────────────────────

/// One row per detect-able instructions-file tool. Each row carries the
/// directory tok0 watches for detection (`signal_dir`) and the canonical
/// path (relative to `$HOME`) where `tok0 init` is expected to write the
/// rule file. Order roughly mirrors `ToolTarget` in `bridge/setup.rs`.
struct ToolFixture {
    label: &'static str,
    signal_dir: &'static str,
    expected_rel: &'static str,
}

const TOOLS: &[ToolFixture] = &[
    ToolFixture {
        label: "Cursor",
        signal_dir: ".cursor",
        expected_rel: ".cursor/tok0.md",
    },
    ToolFixture {
        label: "GeminiCli",
        signal_dir: ".gemini",
        expected_rel: ".gemini/GEMINI.md",
    },
    ToolFixture {
        label: "Windsurf",
        signal_dir: ".codeium/windsurf",
        expected_rel: ".codeium/windsurf/memories/global_rules.md",
    },
    ToolFixture {
        label: "Cline",
        signal_dir: "Documents/Cline",
        expected_rel: "Documents/Cline/Rules/tok0.md",
    },
    ToolFixture {
        label: "Amp",
        signal_dir: ".config/amp",
        expected_rel: ".config/amp/AGENTS.md",
    },
    ToolFixture {
        label: "OpenCode",
        signal_dir: ".config/opencode",
        expected_rel: ".config/opencode/AGENTS.md",
    },
    ToolFixture {
        label: "KimiCode",
        signal_dir: ".kimi",
        expected_rel: ".kimi/tok0.md",
    },
];

// ─── Per-tool: install creates file at canonical path ────────────────────────

#[test]
fn test_init_installs_each_tool_at_canonical_path() {
    for tool in TOOLS {
        let home = TempDir::new().expect("home");
        let (_db_tmp, db) = isolated_db();
        std::fs::create_dir_all(home.path().join(tool.signal_dir)).expect("seed signal dir");

        let out = run_tok0(&["init"], home.path(), &db);
        assert!(
            out.status.success(),
            "{}: tok0 init should succeed. stderr={}",
            tool.label,
            String::from_utf8_lossy(&out.stderr)
        );

        let installed = home.path().join(tool.expected_rel);
        assert!(
            installed.exists(),
            "{}: expected installed file at {}; stdout={}",
            tool.label,
            installed.display(),
            String::from_utf8_lossy(&out.stdout)
        );

        let body = std::fs::read_to_string(&installed)
            .unwrap_or_else(|_| panic!("{}: read installed file", tool.label));
        assert!(
            body.contains("tok0"),
            "{}: installed file should contain tok0 marker. body={}",
            tool.label,
            body
        );

        let stdout = String::from_utf8_lossy(&out.stdout);
        assert!(
            stdout.contains(tool.label),
            "{}: stdout should mention the tool. stdout={}",
            tool.label,
            stdout
        );
    }
}

// ─── Per-tool: second `tok0 init` is a no-op (idempotent at CLI level) ───────

#[test]
fn test_init_is_idempotent_at_cli_level() {
    for tool in TOOLS {
        let home = TempDir::new().expect("home");
        let (_db_tmp, db) = isolated_db();
        std::fs::create_dir_all(home.path().join(tool.signal_dir)).expect("seed signal dir");

        // First run: actual install.
        let first = run_tok0(&["init"], home.path(), &db);
        assert!(first.status.success(), "{}: first init failed", tool.label);

        let installed = home.path().join(tool.expected_rel);
        let body_first = std::fs::read_to_string(&installed).expect("read after first install");

        // Second run: must not duplicate, must report already installed.
        let second = run_tok0(&["init"], home.path(), &db);
        assert!(
            second.status.success(),
            "{}: second init failed",
            tool.label
        );
        let stdout_second = String::from_utf8_lossy(&second.stdout);
        assert!(
            stdout_second.contains("already installed"),
            "{}: idempotent second run should say 'already installed'. stdout={}",
            tool.label,
            stdout_second
        );

        let body_second = std::fs::read_to_string(&installed).expect("read after second install");
        assert_eq!(
            body_first, body_second,
            "{}: file content must not change on second init",
            tool.label
        );
    }
}

// ─── Per-tool: install → uninstall round-trip removes the file ───────────────

#[test]
fn test_init_uninstall_round_trip_per_tool() {
    for tool in TOOLS {
        let home = TempDir::new().expect("home");
        let (_db_tmp, db) = isolated_db();
        std::fs::create_dir_all(home.path().join(tool.signal_dir)).expect("seed signal dir");

        let installed = home.path().join(tool.expected_rel);

        let install = run_tok0(&["init"], home.path(), &db);
        assert!(install.status.success(), "{}: install failed", tool.label);
        assert!(
            installed.exists(),
            "{}: file should exist after install",
            tool.label
        );

        let uninstall = run_tok0(&["init", "--uninstall"], home.path(), &db);
        assert!(
            uninstall.status.success(),
            "{}: uninstall failed: stderr={}",
            tool.label,
            String::from_utf8_lossy(&uninstall.stderr)
        );

        // For tools whose instructions file was tok0-only, uninstall removes
        // the file outright. For tools sharing `AGENTS.md` with user content
        // we'd see the file remain (covered by the "preserves" tests below).
        // In this test there was no other content, so the file must be gone.
        assert!(
            !installed.exists(),
            "{}: uninstall should remove tok0-only file at {}",
            tool.label,
            installed.display()
        );
    }
}

// ─── Pre-existing user content is preserved on install + uninstall ───────────

#[test]
fn test_init_preserves_pre_existing_user_rules_in_agents_md() {
    // GeminiCli, Amp, and OpenCode all use a shared instruction file
    // (`GEMINI.md` / `AGENTS.md`). If the user already has rules there,
    // tok0 must append rather than clobber, and uninstall must remove
    // only the tok0 section.
    let cases = &[
        ToolFixture {
            label: "GeminiCli",
            signal_dir: ".gemini",
            expected_rel: ".gemini/GEMINI.md",
        },
        ToolFixture {
            label: "Amp",
            signal_dir: ".config/amp",
            expected_rel: ".config/amp/AGENTS.md",
        },
        ToolFixture {
            label: "OpenCode",
            signal_dir: ".config/opencode",
            expected_rel: ".config/opencode/AGENTS.md",
        },
    ];

    let user_rules = "# My personal rules\n\n- Always be concise.\n- Cite sources.\n";

    for tool in cases {
        let home = TempDir::new().expect("home");
        let (_db_tmp, db) = isolated_db();
        let abs = home.path().join(tool.expected_rel);
        std::fs::create_dir_all(abs.parent().expect("parent")).expect("mk parent");
        std::fs::write(&abs, user_rules).expect("seed user rules");

        let out = run_tok0(&["init"], home.path(), &db);
        assert!(out.status.success(), "{}: init failed", tool.label);

        let merged = std::fs::read_to_string(&abs).expect("read merged");
        assert!(
            merged.contains("My personal rules"),
            "{}: tok0 init must preserve pre-existing user content. body={}",
            tool.label,
            merged
        );
        assert!(
            merged.contains("tok0"),
            "{}: tok0 init must append tok0 marker. body={}",
            tool.label,
            merged
        );

        let uninstall = run_tok0(&["init", "--uninstall"], home.path(), &db);
        assert!(
            uninstall.status.success(),
            "{}: uninstall failed",
            tool.label
        );

        // After uninstall the file should still exist with user content.
        assert!(
            abs.exists(),
            "{}: shared instruction file with user content must survive uninstall",
            tool.label
        );
        let after = std::fs::read_to_string(&abs).expect("read after uninstall");
        assert!(
            after.contains("My personal rules"),
            "{}: user content must remain after uninstall. body={}",
            tool.label,
            after
        );
    }
}

// ─── ClaudeCode: --global writes to $HOME, default writes to project dir ─────

#[test]
fn test_init_global_writes_settings_to_home() {
    let home = TempDir::new().expect("home");
    let cwd = TempDir::new().expect("cwd");
    let (_db_tmp, db) = isolated_db();
    std::fs::create_dir_all(home.path().join(".claude")).expect("seed .claude");

    let out = run_tok0_in(&["init", "--global"], home.path(), cwd.path(), &db);
    assert!(
        out.status.success(),
        "init --global failed: stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );

    let global = home.path().join(".claude").join("settings.json");
    assert!(
        global.exists(),
        "--global must write to $HOME/.claude/settings.json"
    );
    let body = std::fs::read_to_string(&global).expect("read settings");
    assert!(body.contains("tok0"), "settings.json should contain tok0");
    assert!(
        body.contains("PreToolUse"),
        "settings.json should contain PreToolUse hook"
    );

    // The project-local path must NOT have been touched.
    assert!(
        !cwd.path().join(".claude/settings.json").exists(),
        "--global must not write to cwd/.claude"
    );
}

#[test]
fn test_init_default_writes_settings_to_cwd_for_claude_code() {
    let home = TempDir::new().expect("home");
    let cwd = TempDir::new().expect("cwd");
    let (_db_tmp, db) = isolated_db();
    // Signal: $HOME/.claude exists so Claude Code is detected, but without
    // --global we expect installation under cwd/.claude.
    std::fs::create_dir_all(home.path().join(".claude")).expect("seed .claude");

    let out = run_tok0_in(&["init"], home.path(), cwd.path(), &db);
    assert!(
        out.status.success(),
        "init (no --global) failed: stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );

    let project_settings = cwd.path().join(".claude").join("settings.json");
    assert!(
        project_settings.exists(),
        "default install must land in cwd/.claude/settings.json"
    );
    let body = std::fs::read_to_string(&project_settings).expect("read project settings");
    assert!(body.contains("tok0"));
    assert!(body.contains("PreToolUse"));
}

#[test]
fn test_init_preserves_pre_existing_claude_settings() {
    let home = TempDir::new().expect("home");
    let (_db_tmp, db) = isolated_db();
    let claude = home.path().join(".claude");
    std::fs::create_dir_all(&claude).expect("mk .claude");

    // Seed an unrelated user setting that must survive installation.
    let pre = r#"{"editor": {"theme": "solarized"}}"#;
    std::fs::write(claude.join("settings.json"), pre).expect("seed settings");

    let out = run_tok0(&["init", "--global"], home.path(), &db);
    assert!(out.status.success(), "init failed");

    let body = std::fs::read_to_string(claude.join("settings.json")).expect("read merged settings");
    assert!(
        body.contains("solarized"),
        "tok0 init must preserve unrelated keys: {}",
        body
    );
    assert!(
        body.contains("tok0"),
        "tok0 init must inject tok0 entry: {}",
        body
    );
}

// ─── tok0 init --show with a real signal lists detected tools ───────────────

#[test]
fn test_init_show_reports_detected_tool() {
    let home = TempDir::new().expect("home");
    let (_db_tmp, db) = isolated_db();
    std::fs::create_dir_all(home.path().join(".cursor")).expect("seed cursor");

    let out = run_tok0(&["init", "--show"], home.path(), &db);
    assert!(out.status.success(), "init --show failed");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("Detected AI tools"),
        "should report detection: {}",
        stdout
    );
    assert!(
        stdout.contains("Cursor"),
        "Cursor should be listed: {}",
        stdout
    );

    // --show must never write anything.
    assert!(
        !home.path().join(".cursor/tok0.md").exists(),
        "--show must not install"
    );
}

// ─── tok0 verify: ok / tampered / missing ────────────────────────────────────

#[test]
fn test_verify_ok_for_correct_hash() {
    let tmp = TempDir::new().expect("tmp");
    let (_db_tmp, db) = isolated_db();
    let path = tmp.path().join("hook.sh");
    std::fs::write(&path, b"#!/bin/sh\nexec tok0 rewrite \"$@\"\n").expect("write");

    let hash = sha256_hex(&path);
    let path_str = path.to_str().expect("utf8");

    let out = run_tok0(&["verify", path_str, &hash], tmp.path(), &db);
    assert!(
        out.status.success(),
        "verify with correct hash should exit 0. stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.to_lowercase().contains("ok"),
        "stdout should report ok: {}",
        stdout
    );
}

#[test]
fn test_verify_tampered_for_wrong_hash() {
    let tmp = TempDir::new().expect("tmp");
    let (_db_tmp, db) = isolated_db();
    let path = tmp.path().join("hook.sh");
    std::fs::write(&path, b"#!/bin/sh\nexec tok0 rewrite \"$@\"\n").expect("write");

    let wrong = "0".repeat(64);
    let path_str = path.to_str().expect("utf8");

    let out = run_tok0(&["verify", path_str, &wrong], tmp.path(), &db);
    assert!(
        !out.status.success(),
        "verify with wrong hash must fail (non-zero exit)"
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("TAMPERED"),
        "stderr should report tampering: {}",
        stderr
    );
}

#[test]
fn test_verify_missing_when_path_absent() {
    let tmp = TempDir::new().expect("tmp");
    let (_db_tmp, db) = isolated_db();
    let absent = tmp.path().join("does-not-exist.sh");
    let any_hash = "a".repeat(64);
    let path_str = absent.to_str().expect("utf8");

    let out = run_tok0(&["verify", path_str, &any_hash], tmp.path(), &db);
    assert!(
        !out.status.success(),
        "verify on missing path must fail (non-zero exit)"
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("MISSING"),
        "stderr should report missing: {}",
        stderr
    );
}

// ─── End-to-end: init → SHA-256 → verify reports ok ──────────────────────────

#[test]
fn test_init_then_verify_settles_ok() {
    // Pick GeminiCli — its install writes a single file at a known path,
    // and we can hash it directly without parsing JSON.
    let home = TempDir::new().expect("home");
    let (_db_tmp, db) = isolated_db();
    std::fs::create_dir_all(home.path().join(".gemini")).expect("seed gemini");

    let install = run_tok0(&["init"], home.path(), &db);
    assert!(install.status.success(), "init failed");

    let installed = home.path().join(".gemini").join("GEMINI.md");
    assert!(installed.exists(), "expected GEMINI.md to be installed");

    let hash = sha256_hex(&installed);
    let path_str = installed.to_str().expect("utf8");

    let verify = run_tok0(&["verify", path_str, &hash], home.path(), &db);
    assert!(
        verify.status.success(),
        "verify after init must succeed: stderr={}",
        String::from_utf8_lossy(&verify.stderr)
    );
}

// ─── Init reports nothing-installed cleanly when no tools detected ───────────

#[test]
fn test_init_with_empty_home_reports_no_tools() {
    let home = TempDir::new().expect("home");
    let (_db_tmp, db) = isolated_db();

    let out = run_tok0(&["init"], home.path(), &db);
    assert!(
        out.status.success(),
        "init with empty home should still exit 0"
    );
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        combined.contains("No supported AI tools"),
        "should report no tools detected: {}",
        combined
    );
}

// ─── Uninstall report is clear when signal is present but file isn't ─────────

#[test]
fn test_uninstall_when_signal_present_but_never_installed() {
    let home = TempDir::new().expect("home");
    let (_db_tmp, db) = isolated_db();
    std::fs::create_dir_all(home.path().join(".cursor")).expect("seed cursor");

    let out = run_tok0(&["init", "--uninstall"], home.path(), &db);
    assert!(out.status.success(), "uninstall should be no-op-friendly");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("no hook found") || stdout.contains("already clean"),
        "should report nothing to remove: {}",
        stdout
    );
}

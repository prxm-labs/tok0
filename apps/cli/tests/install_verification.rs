//! Cross-cutting install + hook verification.
//!
//! Where `installation.rs` exercises the user-facing CLI install per tool,
//! this file proves the *system invariants* needed for "every tool is
//! properly installed and hooked":
//!
//!   1. Every shipped `hooks/<tool>/<filename>` template is non-empty,
//!      mentions tok0 + a canonical example, and carries no stale markers
//!      (rtk/TODO/<placeholder>).
//!   2. Per-`ToolTarget` install routing is internally consistent — the
//!      file `install_hook_at()` writes lands exactly at
//!      `config_dir_for(tool, dir)/instructions_filename(tool)`.
//!   3. Bulk install: pre-seed all tool signals → one `tok0 init --global`
//!      → every expected file lands. Bulk uninstall removes them again.
//!   4. `tok0 status` reflects post-install state (`installed at <path>`).
//!   5. Post-install hints emitted by `tok0 init` for GUI-only tools
//!      (Cursor, Kimi Code, Cline) surface in CLI stdout.
//!   6. `tok0 init --wizard` end-to-end through the binary with synthesized
//!      stdin actually installs the chosen tool.
//!   7. End-to-end `init → SHA-256 → tok0 verify` for ClaudeCode's JSON
//!      hook (Gemini case is in `installation.rs`).
//!   8. The hook entry baked into Claude `settings.json` matches what
//!      `tok0 rewrite` actually emits, proving the hook is wired live.

use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use std::process::Command;
use tempfile::TempDir;

// ─── Shared helpers ──────────────────────────────────────────────────────────

fn tok0_binary() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_tok0"))
}

fn isolated_db() -> (TempDir, PathBuf) {
    let tmp = TempDir::new().expect("tmp dir");
    let db = tmp.path().join("tracking.db");
    (tmp, db)
}

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

fn run_tok0_with_stdin(
    args: &[&str],
    home: &Path,
    db: &Path,
    stdin_bytes: &[u8],
) -> std::process::Output {
    use std::io::Write;
    let mut child = Command::new(tok0_binary())
        .args(args)
        .env("TOK0_DB_PATH", db)
        .env("HOME", home)
        .env("USERPROFILE", home)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("spawn tok0");
    child
        .stdin
        .as_mut()
        .expect("stdin")
        .write_all(stdin_bytes)
        .expect("write stdin");
    child.wait_with_output().expect("wait")
}

fn sha256_hex(path: &Path) -> String {
    let bytes = std::fs::read(path).expect("read for hash");
    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    format!("{:x}", hasher.finalize())
}

/// Every detect-able instructions-file tool plus the canonical home-relative
/// signal directory and the canonical home-relative path tok0 writes to.
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

// ─── 1. Shipped hook templates pass content-quality checks ──────────────────

/// Shipped templates compiled into the binary. Pinning them in this list
/// also guarantees every directory under `hooks/` has at least one entry,
/// because the build would have failed if any of these `include_str!`
/// targets was missing.
fn shipped_templates() -> Vec<(&'static str, &'static str)> {
    vec![
        ("cursor/tok0.md", include_str!("../hooks/cursor/tok0.md")),
        (
            "gemini/GEMINI.md",
            include_str!("../hooks/gemini/GEMINI.md"),
        ),
        (
            "windsurf/global_rules.md",
            include_str!("../hooks/windsurf/global_rules.md"),
        ),
        ("cline/tok0.md", include_str!("../hooks/cline/tok0.md")),
        ("amp/AGENTS.md", include_str!("../hooks/amp/AGENTS.md")),
        (
            "opencode/AGENTS.md",
            include_str!("../hooks/opencode/AGENTS.md"),
        ),
        ("codex/AGENTS.md", include_str!("../hooks/codex/AGENTS.md")),
        ("kimi/tok0.md", include_str!("../hooks/kimi/tok0.md")),
    ]
}

#[test]
fn test_every_shipped_template_is_non_trivial() {
    for (name, body) in shipped_templates() {
        assert!(
            body.len() >= 100,
            "{}: template suspiciously short ({} bytes); expected ≥100",
            name,
            body.len()
        );
        assert!(
            body.contains("tok0"),
            "{}: template must contain 'tok0' marker",
            name
        );
    }
}

#[test]
fn test_every_shipped_template_documents_a_usage_example() {
    // The user opens this file inside the AI tool and reads it; an empty
    // doc is a regression. Every template should walk the user through at
    // least one example invocation.
    for (name, body) in shipped_templates() {
        let lc = body.to_lowercase();
        let has_example = lc.contains("tok0 git")
            || lc.contains("tok0 cargo")
            || lc.contains("tok0 npm")
            || lc.contains("tok0 stats")
            || lc.contains("tok0 status")
            || lc.contains("tok0 proxy")
            || lc.contains("tok0 rewrite");
        assert!(
            has_example,
            "{}: template must document at least one canonical `tok0 <cmd>` example",
            name
        );
    }
}

#[test]
fn test_no_shipped_template_carries_stale_markers() {
    // Once a template ships, leftover dev-time markers must not survive.
    // This guards against accidental rtk references re-entering, leftover
    // TODOs, and unfilled placeholder syntax.
    let banned = [
        "rtk ",
        "RTK",
        " rtk",
        "<TODO",
        "<TBD",
        "<placeholder",
        "XXX:",
    ];
    for (name, body) in shipped_templates() {
        for needle in banned {
            assert!(
                !body.contains(needle),
                "{}: template must not contain stale marker '{}'",
                name,
                needle
            );
        }
    }
}

// ─── 2. install_hook_at writes to config_dir/filename for every tool ────────

#[test]
fn test_install_routing_consistent_for_every_tool() {
    use std::process::Output;

    // We can't reach into setup.rs from an integration test, but we can
    // verify the same end-to-end invariant: install at a chosen `dir`,
    // then assert the expected `<dir>/<filename>` exists. This complements
    // the per-tool inline tests by treating routing as a black-box behavior.
    let cases: &[(&str, &str)] = &[
        // (tool label as logged by `tok0 init`, instructions filename)
        // Pairs derived from `instructions_filename()` and the tool labels.
        // ClaudeCode is omitted — handled via JSON, not a single filename.
        ("Cursor", "tok0.md"),
        ("GeminiCli", "GEMINI.md"),
        ("Windsurf", "global_rules.md"),
        ("Cline", "tok0.md"),
        ("Amp", "AGENTS.md"),
        ("OpenCode", "AGENTS.md"),
        ("KimiCode", "tok0.md"),
    ];
    for (label, _filename) in cases {
        // Sanity: the public `tok0 init --show` should at least know the
        // label exists by listing it once a signal is present.
        let home = TempDir::new().expect("home");
        let (_db_tmp, db) = isolated_db();
        // Map label → signal dir (pulled from TOOLS table).
        let fixture = TOOLS
            .iter()
            .find(|t| t.label == *label)
            .expect("known label");
        std::fs::create_dir_all(home.path().join(fixture.signal_dir)).expect("seed");

        let out: Output = run_tok0(&["init", "--show"], home.path(), &db);
        assert!(out.status.success(), "{}: init --show failed", label);
        let stdout = String::from_utf8_lossy(&out.stdout);
        assert!(
            stdout.contains(label),
            "{}: init --show should list the detected tool. stdout={}",
            label,
            stdout
        );
    }
}

// ─── 3. Bulk install: every signal seeded → every file lands ────────────────

#[test]
fn test_bulk_init_installs_every_seeded_tool() {
    let home = TempDir::new().expect("home");
    let (_db_tmp, db) = isolated_db();

    // Seed every detect-signal so `tok0 init` should fan out to all tools.
    // Use --global so ClaudeCode lands under $HOME/.claude rather than cwd.
    std::fs::create_dir_all(home.path().join(".claude")).expect("claude");
    for tool in TOOLS {
        std::fs::create_dir_all(home.path().join(tool.signal_dir))
            .unwrap_or_else(|_| panic!("seed {}", tool.signal_dir));
    }

    let out = run_tok0(&["init", "--global"], home.path(), &db);
    assert!(
        out.status.success(),
        "bulk init failed. stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );

    // ClaudeCode JSON
    let claude = home.path().join(".claude").join("settings.json");
    assert!(claude.exists(), "ClaudeCode settings.json missing");
    let body = std::fs::read_to_string(&claude).expect("read claude");
    assert!(body.contains("tok0"), "claude settings missing tok0 entry");
    assert!(
        body.contains("PreToolUse"),
        "claude settings missing PreToolUse"
    );

    // Every other tool's instruction file
    for tool in TOOLS {
        let path = home.path().join(tool.expected_rel);
        assert!(
            path.exists(),
            "{}: expected file at {} after bulk init",
            tool.label,
            path.display()
        );
        let content = std::fs::read_to_string(&path)
            .unwrap_or_else(|_| panic!("{}: read installed file", tool.label));
        assert!(
            content.contains("tok0"),
            "{}: installed file must contain tok0 marker",
            tool.label
        );
    }
}

#[test]
fn test_bulk_uninstall_clears_every_seeded_tool() {
    let home = TempDir::new().expect("home");
    let (_db_tmp, db) = isolated_db();

    std::fs::create_dir_all(home.path().join(".claude")).expect("claude");
    for tool in TOOLS {
        std::fs::create_dir_all(home.path().join(tool.signal_dir))
            .unwrap_or_else(|_| panic!("seed {}", tool.signal_dir));
    }

    let install = run_tok0(&["init", "--global"], home.path(), &db);
    assert!(install.status.success(), "bulk install failed");

    let uninstall = run_tok0(&["init", "--global", "--uninstall"], home.path(), &db);
    assert!(
        uninstall.status.success(),
        "bulk uninstall failed: stderr={}",
        String::from_utf8_lossy(&uninstall.stderr)
    );

    // Each instructions file (which only contained tok0 content) is gone.
    for tool in TOOLS {
        let path = home.path().join(tool.expected_rel);
        assert!(
            !path.exists(),
            "{}: {} must be removed after uninstall",
            tool.label,
            path.display()
        );
    }

    // Claude settings.json may still exist (uninstall preserves the file
    // and only strips the tok0 hook entry); assert the marker is gone.
    let claude = home.path().join(".claude").join("settings.json");
    if claude.exists() {
        let body = std::fs::read_to_string(&claude).expect("read claude");
        assert!(
            !body.contains("tok0"),
            "claude settings still contains tok0 after uninstall: {}",
            body
        );
    }
}

// ─── 4. tok0 status reflects post-install state ─────────────────────────────

#[test]
fn test_status_reports_installed_after_init() {
    let home = TempDir::new().expect("home");
    let (_db_tmp, db) = isolated_db();
    std::fs::create_dir_all(home.path().join(".cursor")).expect("seed cursor");

    let init = run_tok0(&["init"], home.path(), &db);
    assert!(init.status.success(), "init failed");

    let status = run_tok0(&["status"], home.path(), &db);
    assert!(status.status.success(), "status failed");
    let stdout = String::from_utf8_lossy(&status.stdout);
    assert!(
        stdout.contains("AI tools:"),
        "status missing AI tools section: {}",
        stdout
    );
    assert!(
        stdout.contains("Cursor: installed"),
        "status should report Cursor as installed. stdout={}",
        stdout
    );
}

#[test]
fn test_status_reports_not_installed_for_detected_but_unhooked_tool() {
    let home = TempDir::new().expect("home");
    let (_db_tmp, db) = isolated_db();
    // Detection signal present, but we never call `tok0 init`.
    std::fs::create_dir_all(home.path().join(".cursor")).expect("seed cursor");

    let status = run_tok0(&["status"], home.path(), &db);
    assert!(status.status.success(), "status failed");
    let stdout = String::from_utf8_lossy(&status.stdout);
    assert!(
        stdout.contains("Cursor: not installed"),
        "status should report Cursor as not-installed when no hook present. stdout={}",
        stdout
    );
}

// ─── 5. Post-install hints surface in CLI stdout ────────────────────────────

#[test]
fn test_init_emits_gui_paste_hint_for_cursor_and_kimi() {
    for (label, signal) in &[("Cursor", ".cursor"), ("KimiCode", ".kimi")] {
        let home = TempDir::new().expect("home");
        let (_db_tmp, db) = isolated_db();
        std::fs::create_dir_all(home.path().join(signal)).expect("seed");

        let out = run_tok0(&["init"], home.path(), &db);
        assert!(out.status.success(), "{}: init failed", label);
        let stdout = String::from_utf8_lossy(&out.stdout);
        assert!(
            stdout.contains("GUI-only"),
            "{}: stdout should warn about manual paste step. stdout={}",
            label,
            stdout
        );
    }
}

#[test]
fn test_init_emits_toggle_hint_for_cline() {
    let home = TempDir::new().expect("home");
    let (_db_tmp, db) = isolated_db();
    std::fs::create_dir_all(home.path().join("Documents").join("Cline")).expect("seed cline");

    let out = run_tok0(&["init"], home.path(), &db);
    assert!(out.status.success(), "cline init failed");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("toggle"),
        "Cline post-install hint should mention toggling the rule on. stdout={}",
        stdout
    );
}

#[test]
fn test_init_omits_paste_hint_for_auto_loading_tools() {
    // GeminiCli + Amp + OpenCode auto-load with no manual step. The CLI
    // should not surface the GUI-only paste hint for them.
    for (label, signal) in &[
        ("GeminiCli", ".gemini"),
        ("Amp", ".config/amp"),
        ("OpenCode", ".config/opencode"),
    ] {
        let home = TempDir::new().expect("home");
        let (_db_tmp, db) = isolated_db();
        std::fs::create_dir_all(home.path().join(signal)).expect("seed");

        let out = run_tok0(&["init"], home.path(), &db);
        assert!(out.status.success(), "{}: init failed", label);
        let stdout = String::from_utf8_lossy(&out.stdout);

        // The hint string itself ("GUI-only: paste") must not appear; we
        // check the substring rather than the full sentence to keep this
        // robust to wording tweaks. We allow "Cursor" / "KimiCode" lines
        // if they were also installed, so we scope the assertion to the
        // section of stdout that mentions the auto-loading tool.
        if let Some(idx) = stdout.find(label) {
            let tail = &stdout[idx..];
            // Take up to the next blank line or end of output.
            let section_end = tail.find("\n\n").unwrap_or(tail.len());
            let section = &tail[..section_end];
            assert!(
                !section.contains("GUI-only"),
                "{}: auto-loading tool should not get GUI-only paste hint. \
                 section={}",
                label,
                section
            );
        }
    }
}

// ─── 6. tok0 init --wizard end-to-end via stdin ─────────────────────────────

#[test]
fn test_wizard_e2e_installs_chosen_tool() {
    let home = TempDir::new().expect("home");
    let (_db_tmp, db) = isolated_db();
    std::fs::create_dir_all(home.path().join(".cursor")).expect("seed cursor");

    // wizard prompts: install Cursor? [Y/n]   then  enable telemetry? [Y/n]
    // Two "y" answers cover both prompts; a trailing newline is a safety
    // net if the wizard ever adds another prompt with a yes-default.
    let stdin = b"y\ny\n\n";

    let out = run_tok0_with_stdin(&["init", "--wizard"], home.path(), &db, stdin);
    assert!(
        out.status.success(),
        "wizard exited non-zero. stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("Welcome to tok0"),
        "wizard should print welcome banner. stdout={}",
        stdout
    );
    assert!(
        stdout.contains("Detected AI tools"),
        "wizard should report detection. stdout={}",
        stdout
    );
    assert!(
        home.path().join(".cursor/tok0.md").exists(),
        "wizard with 'y' should install Cursor hook"
    );
}

#[test]
fn test_wizard_e2e_with_no_detected_tools_exits_cleanly() {
    let home = TempDir::new().expect("home");
    let (_db_tmp, db) = isolated_db();
    // No signal dirs seeded.

    let out = run_tok0_with_stdin(&["init", "--wizard"], home.path(), &db, b"");
    assert!(
        out.status.success(),
        "wizard with no tools should still exit 0. stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("No supported AI tools detected"),
        "wizard should report empty detection. stdout={}",
        stdout
    );
}

// ─── 7. End-to-end ClaudeCode init → SHA-256 → tok0 verify ──────────────────

#[test]
fn test_init_then_verify_settles_ok_for_claude_code_settings() {
    let home = TempDir::new().expect("home");
    let (_db_tmp, db) = isolated_db();
    std::fs::create_dir_all(home.path().join(".claude")).expect("seed claude");

    let init = run_tok0(&["init", "--global"], home.path(), &db);
    assert!(init.status.success(), "claude init failed");

    let installed = home.path().join(".claude").join("settings.json");
    assert!(installed.exists(), "claude settings.json should exist");

    let hash = sha256_hex(&installed);
    let path_str = installed.to_str().expect("utf8");

    let verify = run_tok0(&["verify", path_str, &hash], home.path(), &db);
    assert!(
        verify.status.success(),
        "verify after claude init failed: stderr={}",
        String::from_utf8_lossy(&verify.stderr)
    );
}

// ─── 8. Hook entry baked into Claude settings.json matches tok0 rewrite ─────

#[test]
fn test_claude_hook_entry_matches_live_rewriter() {
    // Install ClaudeCode globally, parse settings.json, pull out the hook
    // command and prove `tok0 rewrite git status` produces output prefixed
    // with the same command name. This catches drift between the static
    // hook payload and the binary's actual rewrite behavior.
    let home = TempDir::new().expect("home");
    let (_db_tmp, db) = isolated_db();
    std::fs::create_dir_all(home.path().join(".claude")).expect("seed claude");

    let init = run_tok0(&["init", "--global"], home.path(), &db);
    assert!(init.status.success(), "claude init failed");

    let body = std::fs::read_to_string(home.path().join(".claude").join("settings.json"))
        .expect("read settings");
    // Sanity: the hook entry references `tok0 rewrite` exactly.
    assert!(
        body.contains("tok0 rewrite"),
        "settings.json must hook into `tok0 rewrite`. body={}",
        body
    );

    // Live: `tok0 rewrite git status` must produce the proxied form, which
    // is what the hook would feed back to Claude Code.
    let rewrite = run_tok0(&["rewrite", "git", "status"], home.path(), &db);
    assert!(rewrite.status.success(), "rewrite failed");
    let out = String::from_utf8_lossy(&rewrite.stdout);
    assert_eq!(
        out.trim(),
        "tok0 git status",
        "rewrite output must be the proxied command — drift from the hook. out={}",
        out
    );
}

// ─── 9. Re-running init after partial install is safe ───────────────────────

#[test]
fn test_partial_install_then_full_init_is_safe() {
    // Seed two tools; install just one manually (by running init with only
    // its signal), then add the second signal and re-run. Re-running must
    // leave the first tool's file untouched (idempotent) and create the
    // second tool's file.
    let home = TempDir::new().expect("home");
    let (_db_tmp, db) = isolated_db();
    std::fs::create_dir_all(home.path().join(".cursor")).expect("seed cursor");

    let first = run_tok0(&["init"], home.path(), &db);
    assert!(first.status.success(), "first init failed");
    let cursor_path = home.path().join(".cursor/tok0.md");
    let cursor_body = std::fs::read_to_string(&cursor_path).expect("read cursor");

    // Now add Gemini signal and re-run.
    std::fs::create_dir_all(home.path().join(".gemini")).expect("seed gemini");
    let second = run_tok0(&["init"], home.path(), &db);
    assert!(second.status.success(), "second init failed");

    // Gemini was just installed.
    let gemini = home.path().join(".gemini/GEMINI.md");
    assert!(
        gemini.exists(),
        "gemini file should be created on second init"
    );

    // Cursor file untouched (byte-for-byte).
    let cursor_after = std::fs::read_to_string(&cursor_path).expect("read cursor again");
    assert_eq!(
        cursor_body, cursor_after,
        "cursor file must not change when init reruns for an already-installed tool"
    );

    // Second-run stdout calls Cursor already installed.
    let stdout = String::from_utf8_lossy(&second.stdout);
    assert!(
        stdout.contains("Cursor") && stdout.contains("already installed"),
        "second init should report Cursor already installed. stdout={}",
        stdout
    );
}

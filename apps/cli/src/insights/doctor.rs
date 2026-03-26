//! `tok0 doctor` — diagnostic health checks.
//!
//! Runs a sequence of checks against the user's environment and prints a
//! structured report. Each check returns a `Diagnostic` with a status,
//! detail, and optional fix hint. The checker pipeline is testable without
//! touching the real filesystem: every check function accepts the paths
//! it reads so tests can point it at a temp dir.

use anyhow::Result;
use std::path::{Path, PathBuf};

use crate::bridge::setup::{config_dir_for, detect_tools_in, instructions_filename, HOOK_MARKER};
use crate::bridge::trust;

/// Outcome of a single diagnostic check.
#[derive(Debug, PartialEq)]
pub enum CheckStatus {
    Ok,
    Warn,
    Error,
}

impl CheckStatus {
    fn label(&self) -> &'static str {
        match self {
            CheckStatus::Ok => "ok",
            CheckStatus::Warn => "warn",
            CheckStatus::Error => "error",
        }
    }
}

/// Structured diagnostic produced by a single check.
#[derive(Debug)]
pub struct Diagnostic {
    pub name: String,
    pub status: CheckStatus,
    pub detail: String,
    pub fix_hint: Option<String>,
}

impl Diagnostic {
    fn ok(name: impl Into<String>, detail: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            status: CheckStatus::Ok,
            detail: detail.into(),
            fix_hint: None,
        }
    }

    fn warn(name: impl Into<String>, detail: impl Into<String>, fix: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            status: CheckStatus::Warn,
            detail: detail.into(),
            fix_hint: Some(fix.into()),
        }
    }

    fn error(name: impl Into<String>, detail: impl Into<String>, fix: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            status: CheckStatus::Error,
            detail: detail.into(),
            fix_hint: Some(fix.into()),
        }
    }
}

// ─── Individual checks (pure, take their dependencies) ───────────────────

/// Check that the config directory exists and is writable.
pub fn check_config_dir(config_path: &Path) -> Diagnostic {
    let parent = match config_path.parent() {
        Some(p) => p,
        None => {
            return Diagnostic::error(
                "config dir",
                "config path has no parent directory",
                "Unusual config_path; investigate TOK0_CONFIG_DIR",
            );
        }
    };

    if !parent.exists() {
        return Diagnostic::warn(
            "config dir",
            format!("{} does not exist", parent.display()),
            "Will be created on first `tok0 telemetry on` or similar",
        );
    }

    // Try to create a temp file to verify writability.
    let probe = parent.join(".tok0-doctor-probe");
    match std::fs::write(&probe, b"") {
        Ok(()) => {
            let _ = std::fs::remove_file(&probe);
            Diagnostic::ok("config dir", format!("{} writable", parent.display()))
        }
        Err(e) => Diagnostic::error(
            "config dir",
            format!("{} not writable: {}", parent.display(), e),
            "Check directory permissions",
        ),
    }
}

/// Check that the tracking DB path is writable (or can be created).
pub fn check_db_writable(db_path: &Path) -> Diagnostic {
    let parent = match db_path.parent() {
        Some(p) => p,
        None => return Diagnostic::ok("db path", format!("{}", db_path.display())),
    };

    if !parent.exists() {
        // Try to create the directory structure — this mirrors what the
        // metering worker does on first record.
        if let Err(e) = std::fs::create_dir_all(parent) {
            return Diagnostic::error(
                "db path",
                format!("{} cannot be created: {}", parent.display(), e),
                "Check permissions on parent directory",
            );
        }
    }

    let probe = parent.join(".tok0-db-probe");
    match std::fs::write(&probe, b"") {
        Ok(()) => {
            let _ = std::fs::remove_file(&probe);
            Diagnostic::ok("db path", format!("{} writable", db_path.display()))
        }
        Err(e) => Diagnostic::error(
            "db path",
            format!("{} parent not writable: {}", db_path.display(), e),
            "Check directory permissions or set TOK0_DB_PATH",
        ),
    }
}

/// Check that installed hooks still contain the tok0 marker. Stale hooks
/// are the most common source of "tok0 stopped working" reports.
pub fn check_hooks_not_stale(home: &Path) -> Vec<Diagnostic> {
    let mut diags = Vec::new();
    for tool in detect_tools_in(Some(home)) {
        let dir = config_dir_for(&tool, home);
        let path = match instructions_filename(&tool) {
            Some(name) => dir.join(name),
            // Claude Code: check settings.json for the hook command
            None => dir.join("settings.json"),
        };

        if !path.exists() {
            diags.push(Diagnostic::warn(
                format!("{:?} hook", tool),
                format!("{} not found", path.display()),
                format!("Run `tok0 init` to reinstall the {:?} hook", tool),
            ));
            continue;
        }

        let contents = match std::fs::read_to_string(&path) {
            Ok(c) => c,
            Err(e) => {
                diags.push(Diagnostic::error(
                    format!("{:?} hook", tool),
                    format!("cannot read {}: {}", path.display(), e),
                    "Check file permissions",
                ));
                continue;
            }
        };

        if !contents.contains(HOOK_MARKER) {
            diags.push(Diagnostic::warn(
                format!("{:?} hook", tool),
                format!("{} exists but is missing the `tok0` marker", path.display()),
                format!("Run `tok0 init` to reinstall the {:?} hook", tool),
            ));
        } else {
            diags.push(Diagnostic::ok(
                format!("{:?} hook", tool),
                format!("{} contains tok0 marker", path.display()),
            ));
        }
    }
    diags
}

/// Check that every extension's `rules/` dir contains valid TOML.
pub fn check_extensions_loadable(extensions_dir: &Path) -> Vec<Diagnostic> {
    let mut diags = Vec::new();
    if !extensions_dir.exists() {
        return diags; // no extensions = no problem
    }

    let entries = match std::fs::read_dir(extensions_dir) {
        Ok(e) => e,
        Err(e) => {
            diags.push(Diagnostic::error(
                "extensions",
                format!("cannot read {}: {}", extensions_dir.display(), e),
                "Check permissions",
            ));
            return diags;
        }
    };

    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        let rules_dir = entry.path().join("rules");
        if !rules_dir.exists() {
            continue; // extension without rules is allowed
        }
        match crate::engine::rules::load_rules_from_dir(&rules_dir, None) {
            Ok(rules) => {
                diags.push(Diagnostic::ok(
                    format!("extension: {}", name),
                    format!("{} rule(s) loaded", rules.len()),
                ));
            }
            Err(e) => {
                diags.push(Diagnostic::error(
                    format!("extension: {}", name),
                    format!("failed to load rules: {}", e),
                    format!("Fix or remove extension `{}`", name),
                ));
            }
        }
    }
    diags
}

/// Check that if `.tok0/filters/*.toml` exists in the current project, the
/// project has been explicitly trusted. Otherwise those rules are being
/// silently skipped at runtime.
pub fn check_project_trust(project_dir: &Path) -> Diagnostic {
    let filters = project_dir.join(".tok0").join("filters");
    if !filters.is_dir() {
        return Diagnostic::ok("project trust", "no project-local rules");
    }

    // Count rule files — if 0, silently OK
    let count = std::fs::read_dir(&filters)
        .map(|entries| {
            entries
                .filter_map(|e| e.ok())
                .filter(|e| e.path().extension().and_then(|x| x.to_str()) == Some("toml"))
                .count()
        })
        .unwrap_or(0);

    if count == 0 {
        return Diagnostic::ok("project trust", "no .toml files in .tok0/filters/");
    }

    match trust::is_trusted(project_dir).unwrap_or(false) {
        true => Diagnostic::ok(
            "project trust",
            format!("{} project rule(s) loaded (trusted)", count),
        ),
        false => Diagnostic::warn(
            "project trust",
            format!(
                "{} project rule(s) present but project untrusted — rules are not loaded",
                count
            ),
            "Run `tok0 trust` in this directory to enable the rules",
        ),
    }
}

// ─── Report rendering ────────────────────────────────────────────────────

pub fn format_report(diagnostics: &[Diagnostic]) -> String {
    let mut out = String::from("tok0 doctor — health report\n\n");
    let mut ok = 0;
    let mut warn = 0;
    let mut err = 0;
    for d in diagnostics {
        match d.status {
            CheckStatus::Ok => ok += 1,
            CheckStatus::Warn => warn += 1,
            CheckStatus::Error => err += 1,
        }
        out.push_str(&format!(
            "  [{:>5}] {}: {}\n",
            d.status.label(),
            d.name,
            d.detail
        ));
        if let Some(fix) = &d.fix_hint {
            out.push_str(&format!("          \u{21aa} {}\n", fix));
        }
    }
    out.push('\n');
    out.push_str(&format!(
        "Summary: {} ok, {} warn, {} error\n",
        ok, warn, err
    ));
    out
}

// ─── CLI entry point ────────────────────────────────────────────────────

/// Run every diagnostic and return `(report, exit_code)`. Exit code is 0
/// if no errors, 1 if any check returned `Error`. Warn does not fail.
pub fn run_all(
    home: &Path,
    config_path: &Path,
    db_path: &Path,
    extensions_dir: &Path,
    project_dir: &Path,
) -> (String, i32) {
    let mut diagnostics = Vec::new();
    diagnostics.push(check_config_dir(config_path));
    diagnostics.push(check_db_writable(db_path));
    diagnostics.extend(check_hooks_not_stale(home));
    diagnostics.extend(check_extensions_loadable(extensions_dir));
    diagnostics.push(check_project_trust(project_dir));

    let exit = if diagnostics.iter().any(|d| d.status == CheckStatus::Error) {
        1
    } else {
        0
    };
    (format_report(&diagnostics), exit)
}

/// CLI: `tok0 doctor`. Returns the process exit code.
pub fn run() -> Result<()> {
    let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
    let config_path = crate::engine::config::config_path();
    let db_path = crate::engine::config::db_path();
    let extensions_dir = crate::extensions::catalog::extensions_dir();
    let project_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));

    let (report, exit_code) = run_all(&home, &config_path, &db_path, &extensions_dir, &project_dir);
    print!("{}", report);
    if exit_code != 0 {
        std::process::exit(exit_code);
    }
    Ok(())
}

// ─── Tests ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_check_config_dir_ok_when_writable() {
        let tmp = TempDir::new().expect("tmp");
        let config_path = tmp.path().join("config.toml");
        let d = check_config_dir(&config_path);
        assert_eq!(d.status, CheckStatus::Ok);
    }

    #[test]
    fn test_check_config_dir_warns_when_missing_parent() {
        let tmp = TempDir::new().expect("tmp");
        let config_path = tmp.path().join("deeply").join("nested").join("config.toml");
        let d = check_config_dir(&config_path);
        assert_eq!(d.status, CheckStatus::Warn);
    }

    #[test]
    fn test_check_db_writable_ok() {
        let tmp = TempDir::new().expect("tmp");
        let db = tmp.path().join("tracking.db");
        let d = check_db_writable(&db);
        assert_eq!(d.status, CheckStatus::Ok);
    }

    #[test]
    fn test_check_db_creates_missing_parent() {
        let tmp = TempDir::new().expect("tmp");
        let db = tmp.path().join("newdir").join("tracking.db");
        let d = check_db_writable(&db);
        assert_eq!(d.status, CheckStatus::Ok);
        assert!(db.parent().unwrap().exists(), "parent should be created");
    }

    #[test]
    fn test_check_hooks_not_stale_no_tools() {
        let tmp = TempDir::new().expect("tmp");
        // Empty home — no tools detected, no diagnostics produced
        let diags = check_hooks_not_stale(tmp.path());
        assert!(diags.is_empty());
    }

    #[test]
    fn test_check_extensions_loadable_empty_dir() {
        let tmp = TempDir::new().expect("tmp");
        let ext_dir = tmp.path().join("nope");
        let diags = check_extensions_loadable(&ext_dir);
        assert!(diags.is_empty(), "missing dir is silently OK");
    }

    #[test]
    fn test_check_extensions_loadable_valid_rule() {
        let tmp = TempDir::new().expect("tmp");
        let ext_dir = tmp.path();
        let rules = ext_dir.join("myext").join("rules");
        std::fs::create_dir_all(&rules).expect("mkdir");
        std::fs::write(
            rules.join("r.toml"),
            "[filter]\nname = \"test\"\ncommands = [\"foo\"]\n",
        )
        .expect("write");

        let diags = check_extensions_loadable(ext_dir);
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].status, CheckStatus::Ok);
        assert!(diags[0].name.contains("myext"));
    }

    #[test]
    fn test_check_extensions_loadable_invalid_toml() {
        let tmp = TempDir::new().expect("tmp");
        let ext_dir = tmp.path();
        let rules = ext_dir.join("bad").join("rules");
        std::fs::create_dir_all(&rules).expect("mkdir");
        std::fs::write(rules.join("bad.toml"), b"this is not {{ valid toml").expect("write");

        let diags = check_extensions_loadable(ext_dir);
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].status, CheckStatus::Error);
        assert!(diags[0].fix_hint.is_some());
    }

    #[test]
    fn test_check_project_trust_no_filters() {
        let tmp = TempDir::new().expect("tmp");
        let d = check_project_trust(tmp.path());
        assert_eq!(d.status, CheckStatus::Ok);
        assert!(d.detail.contains("no project-local rules"));
    }

    #[test]
    fn test_check_project_trust_untrusted_warns() {
        let tmp = TempDir::new().expect("tmp");
        let filters = tmp.path().join(".tok0").join("filters");
        std::fs::create_dir_all(&filters).expect("mkdir");
        std::fs::write(
            filters.join("r.toml"),
            "[filter]\nname = \"x\"\ncommands = [\"y\"]\n",
        )
        .expect("write");

        let d = check_project_trust(tmp.path());
        assert_eq!(d.status, CheckStatus::Warn);
        assert!(d.fix_hint.as_deref().unwrap_or("").contains("tok0 trust"));
    }

    #[test]
    fn test_check_project_trust_trusted_ok() {
        let tmp = TempDir::new().expect("tmp");
        let filters = tmp.path().join(".tok0").join("filters");
        std::fs::create_dir_all(&filters).expect("mkdir");
        std::fs::write(
            filters.join("r.toml"),
            "[filter]\nname = \"x\"\ncommands = [\"y\"]\n",
        )
        .expect("write");
        trust::trust_project(tmp.path()).expect("trust");

        let d = check_project_trust(tmp.path());
        assert_eq!(d.status, CheckStatus::Ok);
        assert!(d.detail.contains("trusted"));
    }

    #[test]
    fn test_format_report_contains_counts_and_hints() {
        let diags = vec![
            Diagnostic::ok("a", "a is fine"),
            Diagnostic::warn("b", "b is iffy", "fix b"),
            Diagnostic::error("c", "c is bad", "fix c"),
        ];
        let out = format_report(&diags);
        assert!(out.contains("1 ok, 1 warn, 1 error"));
        assert!(out.contains("a is fine"));
        assert!(out.contains("fix b"));
        assert!(out.contains("fix c"));
        assert!(out.contains("[   ok]"));
        assert!(out.contains("[ warn]"));
        assert!(out.contains("[error]"));
    }

    #[test]
    fn test_run_all_exits_zero_when_clean() {
        let tmp = TempDir::new().expect("tmp");
        let home = tmp.path().join("home");
        std::fs::create_dir_all(&home).expect("mkdir");
        let config_path = tmp.path().join("cfg").join("config.toml");
        let db_path = tmp.path().join("db").join("t.db");
        let ext_dir = tmp.path().join("ext");
        let project_dir = tmp.path().join("proj");
        std::fs::create_dir_all(&project_dir).expect("mkdir");

        let (_report, code) = run_all(&home, &config_path, &db_path, &ext_dir, &project_dir);
        assert_eq!(code, 0);
    }

    #[test]
    fn test_run_all_exits_one_on_invalid_extension() {
        let tmp = TempDir::new().expect("tmp");
        let home = tmp.path().join("home");
        std::fs::create_dir_all(&home).expect("mkdir");
        let config_path = tmp.path().join("cfg").join("config.toml");
        let db_path = tmp.path().join("db").join("t.db");
        let ext_dir = tmp.path().join("ext");
        let rules = ext_dir.join("bad").join("rules");
        std::fs::create_dir_all(&rules).expect("mkdir");
        std::fs::write(rules.join("bad.toml"), b"not {{valid}}").expect("write");
        let project_dir = tmp.path().join("proj");
        std::fs::create_dir_all(&project_dir).expect("mkdir");

        let (report, code) = run_all(&home, &config_path, &db_path, &ext_dir, &project_dir);
        assert_eq!(code, 1, "report should fail exit code:\n{}", report);
    }
}

use anyhow::Result;
use std::path::PathBuf;

use crate::bridge::setup::{
    config_dir_for, detect_tools, home_dir_or_default, instructions_filename, ToolTarget,
    HOOK_MARKER,
};
use crate::bridge::trust;
use crate::engine::{config, meter::Tracker};

/// Hook installation status for a given AI tool.
#[derive(Debug, PartialEq)]
pub enum HookStatus {
    Installed,
    NotInstalled,
    Unsupported,
}

/// Snapshot of tok0's current environment.
#[derive(Debug)]
pub struct StatusReport {
    pub version: String,
    pub config_path: PathBuf,
    pub config_exists: bool,
    pub db_path: PathBuf,
    pub db_exists: bool,
    pub total_commands: u64,
    pub total_saved_tokens: u64,
    pub tools: Vec<(String, HookStatus, Option<PathBuf>)>,
    pub project_dir: PathBuf,
    pub project_trusted: bool,
    pub extensions: Vec<String>,
    pub telemetry_enabled: bool,
}

/// Gather the current status by reading config, DB, and filesystem state.
pub fn gather() -> Result<StatusReport> {
    let version = env!("CARGO_PKG_VERSION").to_string();
    let config_path = config::config_path();
    let config_exists = config_path.exists();
    let loaded_config = config::load_config().unwrap_or_default();

    let db_path = config::db_path();
    let db_exists = db_path.exists();
    let (total_commands, total_saved_tokens) = if db_exists {
        Tracker::new(&db_path)
            .and_then(|t| t.get_summary())
            .map(|s| (s.total_commands, s.total_saved))
            .unwrap_or((0, 0))
    } else {
        (0, 0)
    };

    let project_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let project_trusted = trust::is_trusted(&project_dir).unwrap_or(false);

    let home = home_dir_or_default();
    let detected = detect_tools();
    let tools = detected
        .into_iter()
        .map(|tool| {
            let (status, path) = hook_status_for(&tool, home.as_deref());
            (format!("{:?}", tool), status, path)
        })
        .collect();

    let extensions = crate::extensions::catalog::list_installed().unwrap_or_default();

    Ok(StatusReport {
        version,
        config_path,
        config_exists,
        db_path,
        db_exists,
        total_commands,
        total_saved_tokens,
        tools,
        project_dir,
        project_trusted,
        extensions,
        telemetry_enabled: loaded_config.telemetry.enabled,
    })
}

/// Check whether a tool's hook is installed by inspecting its config file.
fn hook_status_for(
    tool: &ToolTarget,
    home: Option<&std::path::Path>,
) -> (HookStatus, Option<PathBuf>) {
    let Some(home) = home else {
        return (HookStatus::NotInstalled, None);
    };

    let config_dir = config_dir_for(tool, home);
    let path = match tool {
        ToolTarget::ClaudeCode => config_dir.join("settings.json"),
        other => {
            let Some(fname) = instructions_filename(other) else {
                return (HookStatus::Unsupported, None);
            };
            config_dir.join(fname)
        }
    };

    let installed = std::fs::read_to_string(&path)
        .map(|c| c.contains(HOOK_MARKER))
        .unwrap_or(false);
    (
        if installed {
            HookStatus::Installed
        } else {
            HookStatus::NotInstalled
        },
        Some(path),
    )
}

/// Render the status report as a human-readable string.
pub fn format_status(report: &StatusReport) -> String {
    let mut out = String::new();
    out.push_str(&format!("tok0 v{}\n", report.version));
    out.push('\n');

    out.push_str("Paths:\n");
    out.push_str(&format!(
        "  config:  {} {}\n",
        report.config_path.display(),
        if report.config_exists {
            ""
        } else {
            "(missing)"
        }
    ));
    out.push_str(&format!(
        "  db:      {} {}\n",
        report.db_path.display(),
        if report.db_exists { "" } else { "(missing)" }
    ));
    out.push('\n');

    out.push_str("Savings:\n");
    if report.db_exists {
        out.push_str(&format!("  commands:     {}\n", report.total_commands));
        out.push_str(&format!("  tokens saved: {}\n", report.total_saved_tokens));
    } else {
        out.push_str("  no tracking data yet (run some commands first)\n");
    }
    out.push('\n');

    out.push_str("AI tools:\n");
    if report.tools.is_empty() {
        out.push_str("  none detected\n");
    } else {
        for (name, status, path) in &report.tools {
            let label = match status {
                HookStatus::Installed => "installed",
                HookStatus::NotInstalled => "not installed",
                HookStatus::Unsupported => "detected (hook not yet supported)",
            };
            let path_str = path
                .as_ref()
                .map(|p| format!(" at {}", p.display()))
                .unwrap_or_default();
            out.push_str(&format!("  {}: {}{}\n", name, label, path_str));
        }
    }
    out.push('\n');

    out.push_str("Project:\n");
    out.push_str(&format!("  dir:     {}\n", report.project_dir.display()));
    out.push_str(&format!(
        "  trust:   {}\n",
        if report.project_trusted {
            "trusted"
        } else {
            "untrusted (run `tok0 trust` to enable .tok0/filters/*.toml)"
        }
    ));
    out.push('\n');

    out.push_str("Extensions:\n");
    if report.extensions.is_empty() {
        out.push_str("  none installed\n");
    } else {
        for name in &report.extensions {
            out.push_str(&format!("  {}\n", name));
        }
    }
    out.push('\n');

    out.push_str(&format!(
        "Telemetry: {}\n",
        if report.telemetry_enabled {
            "on"
        } else {
            "off"
        }
    ));

    out
}

/// CLI entry point for `tok0 status`.
pub fn run() -> Result<()> {
    let report = gather()?;
    print!("{}", format_status(&report));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_report() -> StatusReport {
        StatusReport {
            version: "9.9.9".to_string(),
            config_path: PathBuf::from("/tmp/config.toml"),
            config_exists: true,
            db_path: PathBuf::from("/tmp/tok0.db"),
            db_exists: true,
            total_commands: 42,
            total_saved_tokens: 12345,
            tools: vec![
                (
                    "ClaudeCode".to_string(),
                    HookStatus::Installed,
                    Some(PathBuf::from("/tmp/settings.json")),
                ),
                ("Cursor".to_string(), HookStatus::Unsupported, None),
            ],
            project_dir: PathBuf::from("/tmp/project"),
            project_trusted: true,
            extensions: vec!["my-ext".to_string()],
            telemetry_enabled: true,
        }
    }

    #[test]
    fn test_format_includes_version() {
        let out = format_status(&make_report());
        assert!(out.contains("tok0 v9.9.9"));
    }

    #[test]
    fn test_format_includes_paths() {
        let out = format_status(&make_report());
        assert!(out.contains("/tmp/config.toml"));
        assert!(out.contains("/tmp/tok0.db"));
    }

    #[test]
    fn test_format_shows_missing_db_hint() {
        let mut r = make_report();
        r.db_exists = false;
        let out = format_status(&r);
        assert!(out.contains("no tracking data yet"));
        assert!(!out.contains("tokens saved: 0"));
    }

    #[test]
    fn test_format_shows_savings() {
        let out = format_status(&make_report());
        assert!(out.contains("commands:     42"));
        assert!(out.contains("tokens saved: 12345"));
    }

    #[test]
    fn test_format_shows_hook_installed() {
        let out = format_status(&make_report());
        assert!(out.contains("ClaudeCode: installed"));
        assert!(out.contains("/tmp/settings.json"));
    }

    #[test]
    fn test_format_shows_hook_unsupported() {
        let out = format_status(&make_report());
        assert!(out.contains("Cursor: detected (hook not yet supported)"));
    }

    #[test]
    fn test_format_shows_no_tools() {
        let mut r = make_report();
        r.tools.clear();
        let out = format_status(&r);
        assert!(out.contains("none detected"));
    }

    #[test]
    fn test_format_shows_trust_trusted() {
        let out = format_status(&make_report());
        assert!(out.contains("trust:   trusted"));
    }

    #[test]
    fn test_format_shows_trust_untrusted_hint() {
        let mut r = make_report();
        r.project_trusted = false;
        let out = format_status(&r);
        assert!(out.contains("untrusted"));
        assert!(out.contains("tok0 trust"));
    }

    #[test]
    fn test_format_shows_extensions() {
        let out = format_status(&make_report());
        assert!(out.contains("my-ext"));
    }

    #[test]
    fn test_format_shows_no_extensions() {
        let mut r = make_report();
        r.extensions.clear();
        let out = format_status(&r);
        assert!(out.contains("none installed"));
    }

    #[test]
    fn test_format_shows_telemetry_on() {
        let out = format_status(&make_report());
        assert!(out.contains("Telemetry: on"));
    }

    #[test]
    fn test_format_shows_telemetry_off() {
        let mut r = make_report();
        r.telemetry_enabled = false;
        let out = format_status(&r);
        assert!(out.contains("Telemetry: off"));
    }

    #[test]
    fn test_gather_runs_without_panic() {
        // Just verify gather() succeeds on a real environment.
        // It's fine if values are mostly defaults — we just want no panic.
        let report = gather().expect("gather should succeed");
        assert!(!report.version.is_empty());
    }
}

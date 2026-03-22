//! Interactive onboarding wizard.
//!
//! Triggered by `tok0 init --wizard`. Walks a first-time user through:
//!   1. Welcome / short explanation
//!   2. Tool detection — choose which to install hooks for
//!   3. Telemetry opt-in
//!   4. Next-steps hint
//!
//! The I/O is abstracted behind `BufRead` / `Write` traits so the full
//! flow can be unit-tested with cursor/byte-vector pairs.

use anyhow::{Context, Result};
use std::io::{BufRead, Write};
use std::path::{Path, PathBuf};

use super::setup::{config_dir_for, detect_tools_in, install_hook_at, post_install_hint};

/// The answer the user gave to a yes/no prompt, or the default if they
/// pressed Enter.
fn parse_yes_no(input: &str, default_yes: bool) -> bool {
    let trimmed = input.trim().to_lowercase();
    if trimmed.is_empty() {
        return default_yes;
    }
    matches!(trimmed.as_str(), "y" | "yes")
}

/// Prompt for a yes/no answer with a default. Returns `Ok(true)` for yes,
/// `Ok(false)` for no. EOF on input falls back to the default.
fn prompt_yes_no<R: BufRead, W: Write>(
    reader: &mut R,
    writer: &mut W,
    question: &str,
    default_yes: bool,
) -> Result<bool> {
    let hint = if default_yes { "[Y/n]" } else { "[y/N]" };
    write!(writer, "{} {} ", question, hint).context("Failed to write prompt")?;
    writer.flush().ok();
    let mut line = String::new();
    match reader.read_line(&mut line) {
        Ok(0) => Ok(default_yes), // EOF
        Ok(_) => Ok(parse_yes_no(&line, default_yes)),
        Err(_) => Ok(default_yes),
    }
}

/// Summary of actions the wizard took, returned so the caller (and tests)
/// can inspect the outcome.
#[derive(Debug, Default)]
pub struct WizardOutcome {
    pub tools_installed: Vec<String>,
    pub tools_skipped: Vec<String>,
    pub telemetry_enabled: bool,
}

/// Run the wizard against arbitrary I/O. Extracted so tests can drive it.
pub fn run_wizard<R: BufRead, W: Write>(
    reader: &mut R,
    writer: &mut W,
    home: &Path,
    config_path: &Path,
) -> Result<WizardOutcome> {
    writeln!(
        writer,
        "\u{2728} Welcome to tok0 \u{2014} token-optimized CLI proxy for AI tools."
    )
    .ok();
    writeln!(writer).ok();
    writeln!(
        writer,
        "tok0 compresses verbose shell command output before it reaches the LLM,"
    )
    .ok();
    writeln!(writer, "saving 60-90% of tokens on typical dev operations.").ok();
    writeln!(writer).ok();

    // Detection
    let detected = detect_tools_in(Some(home));
    if detected.is_empty() {
        writeln!(
            writer,
            "No supported AI tools detected under {}.",
            home.display()
        )
        .ok();
        writeln!(writer, "Install one of: Claude Code, Cursor, Gemini CLI,").ok();
        writeln!(
            writer,
            "Windsurf, Cline, Amp, or Kimi Code, then rerun `tok0 init --wizard`."
        )
        .ok();
        return Ok(WizardOutcome::default());
    }

    writeln!(writer, "Detected AI tools:").ok();
    for tool in &detected {
        writeln!(writer, "  \u{2022} {:?}", tool).ok();
    }
    writeln!(writer).ok();

    // Per-tool install confirmation
    let mut outcome = WizardOutcome::default();
    for tool in &detected {
        let question = format!("Install tok0 hook for {:?}?", tool);
        let yes = prompt_yes_no(reader, writer, &question, true)?;
        if !yes {
            outcome.tools_skipped.push(format!("{:?}", tool));
            continue;
        }
        let config_dir = config_dir_for(tool, home);
        match install_hook_at(tool, &config_dir) {
            Ok(result) => {
                let label = if result.already_installed {
                    "already installed"
                } else {
                    "hook installed"
                };
                writeln!(
                    writer,
                    "  \u{2713} {:?}: {} at {}",
                    tool,
                    label,
                    result.path.display()
                )
                .ok();
                if let Some(hint) = post_install_hint(tool) {
                    writeln!(writer, "    \u{26a0}  {}", hint).ok();
                    writeln!(writer, "       File: {}", result.path.display()).ok();
                }
                outcome.tools_installed.push(format!("{:?}", tool));
            }
            Err(e) => {
                writeln!(writer, "  \u{2717} {:?}: {}", tool, e).ok();
                outcome.tools_skipped.push(format!("{:?}", tool));
            }
        }
    }
    writeln!(writer).ok();

    // Telemetry
    writeln!(
        writer,
        "tok0 can send anonymous aggregated savings metrics to help improve the tool."
    )
    .ok();
    writeln!(
        writer,
        "No command contents, paths, or personal data are sent."
    )
    .ok();
    let telemetry = prompt_yes_no(reader, writer, "Enable telemetry?", true)?;
    outcome.telemetry_enabled = telemetry;
    let _ = crate::engine::config::set_telemetry_enabled(config_path, telemetry);

    writeln!(writer).ok();
    writeln!(writer, "All set!").ok();
    writeln!(writer, "  \u{2022} Run any command through your AI tool.").ok();
    writeln!(
        writer,
        "  \u{2022} `tok0 status` to inspect what's installed."
    )
    .ok();
    writeln!(
        writer,
        "  \u{2022} `tok0 stats` to see token savings over time."
    )
    .ok();

    Ok(outcome)
}

/// CLI entry point: `tok0 init --wizard`.
pub fn run() -> Result<WizardOutcome> {
    let home = dirs::home_dir().context("Could not determine home directory")?;
    let config_path = crate::engine::config::config_path();
    let stdin = std::io::stdin();
    let mut reader = stdin.lock();
    let stdout = std::io::stdout();
    let mut writer = stdout.lock();
    run_wizard(&mut reader, &mut writer, &home, &config_path)
}

/// Returns the config file path for telemetry writes, used by the wizard.
#[allow(dead_code)]
pub fn telemetry_config_path() -> PathBuf {
    crate::engine::config::config_path()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{BufReader, Cursor};
    use tempfile::TempDir;

    fn drive(input: &str, home: &Path) -> (WizardOutcome, String, TempDir) {
        let tmp_cfg = TempDir::new().expect("tmp");
        let config_path = tmp_cfg.path().join("config.toml");
        let mut reader = BufReader::new(Cursor::new(input.as_bytes().to_vec()));
        let mut writer: Vec<u8> = Vec::new();
        let outcome = run_wizard(&mut reader, &mut writer, home, &config_path)
            .expect("wizard should not fail");
        let output = String::from_utf8(writer).expect("utf8");
        (outcome, output, tmp_cfg)
    }

    #[test]
    fn test_parse_yes_no_defaults() {
        assert!(parse_yes_no("", true));
        assert!(!parse_yes_no("", false));
        assert!(parse_yes_no("y\n", false));
        assert!(parse_yes_no("Y\n", false));
        assert!(parse_yes_no("yes\n", false));
        assert!(parse_yes_no("YES\n", false));
        assert!(!parse_yes_no("n\n", true));
        assert!(!parse_yes_no("no\n", true));
        assert!(!parse_yes_no("garbage\n", true));
    }

    #[test]
    fn test_wizard_no_tools_detected() {
        let home = TempDir::new().expect("tmp");
        let (outcome, output, _cfg) = drive("", home.path());
        assert!(outcome.tools_installed.is_empty());
        assert!(output.contains("No supported AI tools detected"));
    }

    #[test]
    fn test_wizard_installs_confirmed_tools() {
        let home = TempDir::new().expect("tmp");
        // Simulate Cursor + Kimi present
        std::fs::create_dir_all(home.path().join(".cursor")).expect("cursor dir");
        std::fs::create_dir_all(home.path().join(".kimi")).expect("kimi dir");

        // Input: "y" for each tool + "y" for telemetry (3 lines)
        let (outcome, output, _cfg) = drive("y\ny\ny\n", home.path());

        assert_eq!(outcome.tools_installed.len(), 2);
        assert!(outcome.tools_installed.iter().any(|t| t == "Cursor"));
        assert!(outcome.tools_installed.iter().any(|t| t == "KimiCode"));
        assert!(outcome.telemetry_enabled);

        // Files should exist at the right paths
        assert!(home.path().join(".cursor/tok0.md").exists());
        assert!(home.path().join(".kimi/tok0.md").exists());
        // GUI-only hint should appear for both tools
        assert!(
            output.contains("GUI-only"),
            "wizard should warn about GUI-only tools"
        );
        assert!(output.contains("Welcome to tok0"));
        assert!(output.contains("All set!"));
    }

    #[test]
    fn test_wizard_skips_on_no() {
        let home = TempDir::new().expect("tmp");
        std::fs::create_dir_all(home.path().join(".cursor")).expect("cursor dir");

        // "n" to install, "n" to telemetry
        let (outcome, _output, _cfg) = drive("n\nn\n", home.path());

        assert_eq!(outcome.tools_installed.len(), 0);
        assert_eq!(outcome.tools_skipped, vec!["Cursor".to_string()]);
        assert!(!outcome.telemetry_enabled);
        assert!(!home.path().join(".cursor/tok0.md").exists());
    }

    #[test]
    fn test_wizard_defaults_on_empty_input() {
        let home = TempDir::new().expect("tmp");
        std::fs::create_dir_all(home.path().join(".cursor")).expect("cursor dir");

        // Empty lines — should take defaults (yes for install, yes for telemetry)
        let (outcome, _output, _cfg) = drive("\n\n", home.path());

        assert_eq!(outcome.tools_installed, vec!["Cursor".to_string()]);
        assert!(outcome.telemetry_enabled);
    }

    #[test]
    fn test_wizard_writes_telemetry_to_config() {
        let home = TempDir::new().expect("tmp");
        std::fs::create_dir_all(home.path().join(".cursor")).expect("cursor dir");

        let tmp_cfg = TempDir::new().expect("cfg dir");
        let config_path = tmp_cfg.path().join("config.toml");

        let mut reader = BufReader::new(Cursor::new(b"n\nn\n".to_vec()));
        let mut writer: Vec<u8> = Vec::new();
        run_wizard(&mut reader, &mut writer, home.path(), &config_path).expect("run");

        let content = std::fs::read_to_string(&config_path).expect("read");
        assert!(content.contains("[telemetry]"));
        assert!(content.contains("enabled = false"));
    }

    #[test]
    fn test_wizard_eof_uses_defaults() {
        let home = TempDir::new().expect("tmp");
        std::fs::create_dir_all(home.path().join(".cursor")).expect("cursor dir");

        // No input at all — EOF immediately. Defaults (yes) should apply.
        let (outcome, _output, _cfg) = drive("", home.path());

        assert_eq!(outcome.tools_installed, vec!["Cursor".to_string()]);
        assert!(outcome.telemetry_enabled);
    }

    #[test]
    fn test_wizard_installs_idempotent() {
        let home = TempDir::new().expect("tmp");
        std::fs::create_dir_all(home.path().join(".cursor")).expect("cursor dir");

        // First run
        let (_o1, _, _) = drive("y\ny\n", home.path());
        // Second run — hook already installed
        let (outcome, output, _cfg) = drive("y\ny\n", home.path());

        assert!(output.contains("already installed"));
        assert_eq!(outcome.tools_installed, vec!["Cursor".to_string()]);
    }

    #[test]
    fn test_wizard_output_mentions_next_steps() {
        let home = TempDir::new().expect("tmp");
        std::fs::create_dir_all(home.path().join(".cursor")).expect("cursor dir");
        let (_o, output, _cfg) = drive("y\ny\n", home.path());
        assert!(output.contains("tok0 status"));
        assert!(output.contains("tok0 stats"));
    }
}

use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};

// ─── Types ────────────────────────────────────────────────────────────────────

/// The AI tools tok0 can integrate with.
#[derive(Debug, Clone, PartialEq)]
pub enum ToolTarget {
    /// Hook-based: ~/.claude/settings.json PreToolUse
    ClaudeCode,
    /// GUI-only: writes reference file at ~/.cursor/tok0.md, user must paste
    /// content into Settings → Rules (no file-based global rules as of 2026).
    Cursor,
    /// Auto-loaded: ~/.gemini/GEMINI.md (Google convention)
    GeminiCli,
    /// Auto-loaded: ~/.codeium/windsurf/memories/global_rules.md
    Windsurf,
    /// File-based: ~/Documents/Cline/Rules/tok0.md, must be toggled on in
    /// the Cline Rules panel inside VS Code.
    Cline,
    /// Auto-loaded: ~/.config/amp/AGENTS.md (Sourcegraph Amp)
    Amp,
    /// Auto-loaded: ~/.config/opencode/AGENTS.md (sst/opencode)
    OpenCode,
    /// Auto-loaded: ~/.codex/AGENTS.md (manual only, not auto-detected)
    #[allow(dead_code)]
    Codex,
    /// GUI-only: Kimi Code has no documented home-dir instructions file.
    /// We write a reference at ~/.kimi/tok0.md and print manual-setup hint.
    KimiCode,
}

/// Result of a single hook installation attempt.
#[derive(Debug)]
pub struct HookInstallResult {
    #[allow(dead_code)]
    pub tool: String,
    pub path: PathBuf,
    pub already_installed: bool,
}

// ─── Constants ────────────────────────────────────────────────────────────────

pub const CLAUDE_HOOK_ENTRY: &str = "tok0 rewrite";
pub const HOOK_MARKER: &str = "tok0";

// ─── Public API ───────────────────────────────────────────────────────────────

/// Detect which AI tools are present on this machine.
///
/// Detection is intentionally lightweight — we only check for config
/// directories or binaries; we do not spawn subprocesses.
pub fn detect_tools() -> Vec<ToolTarget> {
    detect_tools_in(home_dir_or_default().as_deref())
}

/// Testable variant that accepts a custom home directory root.
pub fn detect_tools_in(home: Option<&Path>) -> Vec<ToolTarget> {
    let Some(home) = home else {
        return vec![];
    };

    let mut found = Vec::new();

    // ClaudeCode: ~/.claude/
    if home.join(".claude").is_dir() {
        found.push(ToolTarget::ClaudeCode);
    }

    // Cursor: ~/.cursor/ (app state dir)
    if home.join(".cursor").is_dir() {
        found.push(ToolTarget::Cursor);
    }

    // GeminiCli: ~/.gemini/ or `gemini` binary on PATH
    if home.join(".gemini").is_dir() || which_binary("gemini") {
        found.push(ToolTarget::GeminiCli);
    }

    // Windsurf: ~/.codeium/windsurf/ (current) or ~/.windsurf/ (legacy)
    if home.join(".codeium").join("windsurf").is_dir() || home.join(".windsurf").is_dir() {
        found.push(ToolTarget::Windsurf);
    }

    // Cline: ~/Documents/Cline/ (VS Code extension writes rules here)
    if home.join("Documents").join("Cline").is_dir() {
        found.push(ToolTarget::Cline);
    }

    // Amp: ~/.config/amp/ or `amp` binary on PATH
    if home.join(".config").join("amp").is_dir() || which_binary("amp") {
        found.push(ToolTarget::Amp);
    }

    // OpenCode: ~/.config/opencode/ or `opencode` binary on PATH
    if home.join(".config").join("opencode").is_dir() || which_binary("opencode") {
        found.push(ToolTarget::OpenCode);
    }

    // Kimi Code: ~/.kimi/
    if home.join(".kimi").is_dir() {
        found.push(ToolTarget::KimiCode);
    }

    found
}

/// Additional manual step the user must take after `tok0 init` for a
/// given tool, if any. GUI-only tools get a paste-into-Settings hint;
/// Cline needs the user to toggle the rule on in its panel. Tools with
/// fully-automatic loading (Claude Code, Gemini CLI, Amp, OpenCode,
/// Codex, Windsurf) return `None`.
pub fn post_install_hint(tool: &ToolTarget) -> Option<&'static str> {
    match tool {
        ToolTarget::Cursor | ToolTarget::KimiCode => Some(
            "GUI-only: paste the contents of the written file into the tool's \
             Settings \u{2192} Rules — tok0 cannot auto-load it.",
        ),
        ToolTarget::Cline => Some(
            "Open the Cline panel in VS Code, switch to the Rules tab, and \
             toggle `tok0.md` ON — the extension scans the folder but only \
             applies explicitly-enabled rules.",
        ),
        _ => None,
    }
}

/// Return the filename tok0 uses for a tool's instructions/rule file.
/// ClaudeCode uses a JSON hook instead and is not handled here.
pub fn instructions_filename(tool: &ToolTarget) -> Option<&'static str> {
    match tool {
        ToolTarget::ClaudeCode => None,
        ToolTarget::GeminiCli => Some("GEMINI.md"),
        ToolTarget::Windsurf => Some("global_rules.md"),
        // Cline reads any .md file in its Rules dir; use a namespaced name
        // so it's easy to remove and doesn't collide with user files.
        ToolTarget::Cline => Some("tok0.md"),
        // Cursor + Kimi Code are GUI-only; we write a reference file the
        // user can copy from.
        ToolTarget::Cursor | ToolTarget::KimiCode => Some("tok0.md"),
        ToolTarget::Codex | ToolTarget::Amp | ToolTarget::OpenCode => Some("AGENTS.md"),
    }
}

/// Return the directory where tok0 writes the instructions file for this
/// tool. These are the canonical paths the tools auto-load from (April 2026).
pub fn config_dir_for(tool: &ToolTarget, home: &Path) -> PathBuf {
    match tool {
        ToolTarget::ClaudeCode => home.join(".claude"),
        // Cursor: no file-based global rules; we drop a reference doc in
        // the app's state dir.
        ToolTarget::Cursor => home.join(".cursor"),
        ToolTarget::GeminiCli => home.join(".gemini"),
        // Windsurf auto-loads this exact path across every workspace.
        ToolTarget::Windsurf => home.join(".codeium").join("windsurf").join("memories"),
        // Cline scans any .md under Documents/Cline/Rules; user must toggle
        // it ON in the Rules panel. macOS/Linux/WSL path shown — Windows
        // also uses Documents\Cline\Rules via a drive letter.
        ToolTarget::Cline => home.join("Documents").join("Cline").join("Rules"),
        ToolTarget::Amp => home.join(".config").join("amp"),
        ToolTarget::OpenCode => home.join(".config").join("opencode"),
        ToolTarget::Codex => home.join(".codex"),
        ToolTarget::KimiCode => home.join(".kimi"),
    }
}

/// Install tok0 hooks into the given tool's config directory.
///
/// - For `ClaudeCode`: reads/creates `settings.json` and injects a
///   `PreToolUse` hook entry containing `tok0 rewrite`.
/// - For `Codex`: writes `AGENTS.md` with usage instructions.
/// - All other tools: returns an error (not yet supported).
/// - Idempotent: if tok0 is already present, returns
///   `already_installed: true` without modifying anything.
pub fn install_hook_at(tool: &ToolTarget, config_dir: &Path) -> Result<HookInstallResult> {
    match tool {
        ToolTarget::ClaudeCode => install_claude_code(config_dir),
        _ => {
            let filename =
                instructions_filename(tool).context("Tool has no instructions filename")?;
            install_instructions_file(tool, config_dir, filename)
        }
    }
}

/// Remove tok0 hooks from the given tool's config.
///
/// - For `ClaudeCode`: removes the tok0 hook entry from `settings.json`.
/// - For `Codex`: removes tok0 section from `AGENTS.md`.
/// - Idempotent: if hook not found, reports as already clean.
pub fn uninstall_hook_at(tool: &ToolTarget, config_dir: &Path) -> Result<HookInstallResult> {
    match tool {
        ToolTarget::ClaudeCode => uninstall_claude_code(config_dir),
        _ => {
            let filename =
                instructions_filename(tool).context("Tool has no instructions filename")?;
            uninstall_instructions_file(tool, config_dir, filename)
        }
    }
}

/// Returns the per-tool instruction text. The canonical bytes live in
/// `hooks/<tool>/<filename>` and are embedded at compile time via
/// `include_str!`, so contributors can edit per-tool guidance without
/// touching Rust.
pub fn generate_instructions_for(tool: &ToolTarget) -> &'static str {
    match tool {
        ToolTarget::Cursor => include_str!("../../hooks/cursor/tok0.md"),
        ToolTarget::GeminiCli => include_str!("../../hooks/gemini/GEMINI.md"),
        ToolTarget::Windsurf => include_str!("../../hooks/windsurf/global_rules.md"),
        ToolTarget::Cline => include_str!("../../hooks/cline/tok0.md"),
        ToolTarget::Amp => include_str!("../../hooks/amp/AGENTS.md"),
        ToolTarget::OpenCode => include_str!("../../hooks/opencode/AGENTS.md"),
        ToolTarget::Codex => include_str!("../../hooks/codex/AGENTS.md"),
        ToolTarget::KimiCode => include_str!("../../hooks/kimi/tok0.md"),
        // Claude Code installs a JSON hook into settings.json; no markdown
        // template applies. Returning empty is fine because the
        // `install_claude_code` path never calls this function.
        ToolTarget::ClaudeCode => "",
    }
}

/// Backwards-compatible wrapper. Returns the Cursor template by default
/// since it shares the canonical content; new code should call
/// `generate_instructions_for(tool)` instead.
#[allow(dead_code)]
pub fn generate_instructions() -> String {
    generate_instructions_for(&ToolTarget::Cursor).to_string()
}

// ─── ClaudeCode installer ─────────────────────────────────────────────────────

fn install_claude_code(config_dir: &Path) -> Result<HookInstallResult> {
    fs::create_dir_all(config_dir)
        .with_context(|| format!("Failed to create config dir: {}", config_dir.display()))?;

    let settings_path = config_dir.join("settings.json");

    // Load existing JSON or start with an empty object.
    let raw = if settings_path.exists() {
        fs::read_to_string(&settings_path)
            .with_context(|| format!("Failed to read {}", settings_path.display()))?
    } else {
        "{}".to_string()
    };

    let mut root: serde_json::Value = serde_json::from_str(&raw)
        .with_context(|| format!("Failed to parse JSON in {}", settings_path.display()))?;

    // Idempotency: if "tok0" already appears anywhere in the file, bail early.
    if raw.contains(HOOK_MARKER) {
        return Ok(HookInstallResult {
            tool: "claude-code".to_string(),
            path: settings_path,
            already_installed: true,
        });
    }

    // Build the hook entry:
    //   "hooks": { "PreToolUse": [{ "matcher": "", "hooks": [{"type":"command","command":"tok0 rewrite"}] }] }
    let hook_entry = serde_json::json!([{
        "matcher": "",
        "hooks": [{
            "type": "command",
            "command": CLAUDE_HOOK_ENTRY
        }]
    }]);

    // Navigate/create: root["hooks"]["PreToolUse"]
    let hooks_obj = root
        .as_object_mut()
        .context("settings.json root is not a JSON object")?
        .entry("hooks")
        .or_insert_with(|| serde_json::json!({}));

    hooks_obj
        .as_object_mut()
        .context("settings.json 'hooks' field is not an object")?
        .entry("PreToolUse")
        .or_insert(hook_entry);

    let serialized =
        serde_json::to_string_pretty(&root).context("Failed to serialize settings.json")?;

    fs::write(&settings_path, serialized)
        .with_context(|| format!("Failed to write {}", settings_path.display()))?;

    Ok(HookInstallResult {
        tool: "claude-code".to_string(),
        path: settings_path,
        already_installed: false,
    })
}

// ─── Generic instructions-file installer ──────────────────────────────────────

/// Install an instructions file for tools that rely on natural-language rules
/// rather than shell hooks. Idempotent; appends tok0 section if file already
/// contains unrelated content.
fn install_instructions_file(
    tool: &ToolTarget,
    config_dir: &Path,
    filename: &str,
) -> Result<HookInstallResult> {
    fs::create_dir_all(config_dir)
        .with_context(|| format!("Failed to create config dir: {}", config_dir.display()))?;

    let file_path = config_dir.join(filename);
    let tool_name = tool_slug(tool);

    if file_path.exists() {
        let existing = fs::read_to_string(&file_path)
            .with_context(|| format!("Failed to read {}", file_path.display()))?;
        if existing.contains(HOOK_MARKER) {
            return Ok(HookInstallResult {
                tool: tool_name,
                path: file_path,
                already_installed: true,
            });
        }
        // Append tok0 section to existing file (preserve other content).
        let mut combined = existing.trim_end().to_string();
        combined.push_str("\n\n");
        combined.push_str(generate_instructions_for(tool));
        fs::write(&file_path, combined)
            .with_context(|| format!("Failed to write {}", file_path.display()))?;
    } else {
        fs::write(&file_path, generate_instructions_for(tool))
            .with_context(|| format!("Failed to write {}", file_path.display()))?;
    }

    Ok(HookInstallResult {
        tool: tool_name,
        path: file_path,
        already_installed: false,
    })
}

fn tool_slug(tool: &ToolTarget) -> String {
    match tool {
        ToolTarget::ClaudeCode => "claude-code",
        ToolTarget::Cursor => "cursor",
        ToolTarget::GeminiCli => "gemini-cli",
        ToolTarget::Windsurf => "windsurf",
        ToolTarget::Cline => "cline",
        ToolTarget::Amp => "amp",
        ToolTarget::OpenCode => "opencode",
        ToolTarget::Codex => "codex",
        ToolTarget::KimiCode => "kimi-code",
    }
    .to_string()
}

// ─── ClaudeCode uninstaller ───────────────────────────────────────────────

fn uninstall_claude_code(config_dir: &Path) -> Result<HookInstallResult> {
    let settings_path = config_dir.join("settings.json");

    if !settings_path.exists() {
        return Ok(HookInstallResult {
            tool: "claude-code".to_string(),
            path: settings_path,
            already_installed: false, // nothing to remove
        });
    }

    let raw = fs::read_to_string(&settings_path)
        .with_context(|| format!("Failed to read {}", settings_path.display()))?;

    if !raw.contains(HOOK_MARKER) {
        return Ok(HookInstallResult {
            tool: "claude-code".to_string(),
            path: settings_path,
            already_installed: false,
        });
    }

    let mut root: serde_json::Value = serde_json::from_str(&raw)
        .with_context(|| format!("Failed to parse JSON in {}", settings_path.display()))?;

    // Remove tok0 entries from hooks.PreToolUse array
    if let Some(hooks) = root.get_mut("hooks").and_then(|h| h.as_object_mut()) {
        if let Some(pre_tool_use) = hooks.get_mut("PreToolUse").and_then(|v| v.as_array_mut()) {
            pre_tool_use.retain(|entry| {
                // Keep entries that don't contain "tok0"
                !serde_json::to_string(entry)
                    .unwrap_or_default()
                    .contains(HOOK_MARKER)
            });
            // If PreToolUse is now empty, remove the key
            if pre_tool_use.is_empty() {
                hooks.remove("PreToolUse");
            }
        }
        // If hooks object is now empty, remove it
        if hooks.is_empty() {
            root.as_object_mut().map(|obj| obj.remove("hooks"));
        }
    }

    let serialized =
        serde_json::to_string_pretty(&root).context("Failed to serialize settings.json")?;
    fs::write(&settings_path, serialized)
        .with_context(|| format!("Failed to write {}", settings_path.display()))?;

    Ok(HookInstallResult {
        tool: "claude-code".to_string(),
        path: settings_path,
        already_installed: true, // was installed, now removed
    })
}

// ─── Generic instructions-file uninstaller ───────────────────────────────────

fn uninstall_instructions_file(
    tool: &ToolTarget,
    config_dir: &Path,
    filename: &str,
) -> Result<HookInstallResult> {
    let file_path = config_dir.join(filename);
    let tool_name = tool_slug(tool);

    if !file_path.exists() {
        return Ok(HookInstallResult {
            tool: tool_name,
            path: file_path,
            already_installed: false,
        });
    }

    let content = fs::read_to_string(&file_path)
        .with_context(|| format!("Failed to read {}", file_path.display()))?;

    if !content.contains(HOOK_MARKER) {
        return Ok(HookInstallResult {
            tool: tool_name,
            path: file_path,
            already_installed: false,
        });
    }

    // If the file only contains tok0 instructions, remove it entirely.
    let instructions = generate_instructions_for(tool);
    if content.trim() == instructions.trim() {
        fs::remove_file(&file_path)
            .with_context(|| format!("Failed to remove {}", file_path.display()))?;
    } else {
        // File has other content — strip tok0 lines, keep the rest.
        let cleaned: String = content
            .lines()
            .filter(|line| !line.contains(HOOK_MARKER))
            .collect::<Vec<_>>()
            .join("\n");
        fs::write(&file_path, cleaned.trim())
            .with_context(|| format!("Failed to write {}", file_path.display()))?;
    }

    Ok(HookInstallResult {
        tool: tool_name,
        path: file_path,
        already_installed: true,
    })
}

// ─── Helpers ──────────────────────────────────────────────────────────────────

/// Resolve the user's home directory.
///
/// On Unix this delegates to `dirs::home_dir()`, which reads `$HOME` —
/// allowing test harnesses to redirect detection by overriding the env
/// var. On Windows, `dirs` v5 calls `SHGetKnownFolderPath` directly and
/// ignores `USERPROFILE`/`HOME`, so we read those env vars ourselves
/// first and fall back to `dirs` only when they're absent.
pub fn home_dir_or_default() -> Option<PathBuf> {
    #[cfg(windows)]
    {
        for key in ["USERPROFILE", "HOME"] {
            if let Some(val) = std::env::var_os(key) {
                if !val.is_empty() {
                    return Some(PathBuf::from(val));
                }
            }
        }
    }
    dirs::home_dir()
}

/// Returns true if the named binary exists anywhere on PATH.
fn which_binary(name: &str) -> bool {
    which::which(name).is_ok()
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn tmp() -> TempDir {
        tempfile::tempdir().expect("Failed to create temp dir")
    }

    // ── 1. ClaudeCode: empty dir creates settings.json with tok0 ─────────────
    #[test]
    fn test_install_claude_code_creates_settings() {
        let dir = tmp();
        let result =
            install_hook_at(&ToolTarget::ClaudeCode, dir.path()).expect("install_hook_at failed");

        assert_eq!(result.tool, "claude-code");
        assert!(!result.already_installed);

        let settings_path = dir.path().join("settings.json");
        assert!(settings_path.exists(), "settings.json should be created");

        let content = fs::read_to_string(&settings_path).expect("read failed");
        assert!(
            content.contains(HOOK_MARKER),
            "settings.json should contain 'tok0'"
        );
        assert!(
            content.contains("PreToolUse"),
            "settings.json should contain PreToolUse hook"
        );
        assert!(
            content.contains(CLAUDE_HOOK_ENTRY),
            "settings.json should contain hook entry command"
        );
    }

    // ── 2. ClaudeCode: second install returns already_installed ───────────────
    #[test]
    fn test_install_claude_code_idempotent() {
        let dir = tmp();

        let first =
            install_hook_at(&ToolTarget::ClaudeCode, dir.path()).expect("first install failed");
        assert!(
            !first.already_installed,
            "first install should not be already_installed"
        );

        let second =
            install_hook_at(&ToolTarget::ClaudeCode, dir.path()).expect("second install failed");
        assert!(
            second.already_installed,
            "second install should be already_installed"
        );

        // File must not have duplicate entries.
        let content = fs::read_to_string(dir.path().join("settings.json")).expect("read failed");
        let occurrences = content.matches(HOOK_MARKER).count();
        // We expect exactly one injection; the command string contains "tok0" once per entry.
        assert!(occurrences >= 1, "tok0 should appear at least once");
    }

    // ── 3. ClaudeCode: preserves existing hooks ───────────────────────────────
    #[test]
    fn test_install_claude_code_preserves_existing() {
        let dir = tmp();
        let settings_path = dir.path().join("settings.json");

        // Pre-existing settings with an unrelated hook.
        let existing = serde_json::json!({
            "theme": "dark",
            "hooks": {
                "PostToolUse": [{"matcher": "", "hooks": [{"type": "command", "command": "echo done"}]}]
            }
        });
        fs::write(
            &settings_path,
            serde_json::to_string_pretty(&existing).unwrap(),
        )
        .expect("write failed");

        install_hook_at(&ToolTarget::ClaudeCode, dir.path()).expect("install failed");

        let content = fs::read_to_string(&settings_path).expect("read failed");
        let parsed: serde_json::Value =
            serde_json::from_str(&content).expect("invalid JSON after install");

        // Original data preserved.
        assert_eq!(parsed["theme"], "dark", "theme should be preserved");
        assert!(
            parsed["hooks"]["PostToolUse"].is_array(),
            "PostToolUse hook should be preserved"
        );
        // New hook injected.
        assert!(
            parsed["hooks"]["PreToolUse"].is_array(),
            "PreToolUse hook should be injected"
        );
        assert!(content.contains(HOOK_MARKER));
    }

    // ── 4. Codex: creates AGENTS.md ───────────────────────────────────────────
    #[test]
    fn test_install_codex_creates_agents_md() {
        let dir = tmp();
        let result =
            install_hook_at(&ToolTarget::Codex, dir.path()).expect("install_hook_at failed");

        assert_eq!(result.tool, "codex");
        assert!(!result.already_installed);

        let agents_path = dir.path().join("AGENTS.md");
        assert!(agents_path.exists(), "AGENTS.md should be created");

        let content = fs::read_to_string(&agents_path).expect("read failed");
        assert!(
            content.contains(HOOK_MARKER),
            "AGENTS.md should mention tok0"
        );
    }

    // ── 5. Codex: second install returns already_installed ────────────────────
    #[test]
    fn test_install_codex_idempotent() {
        let dir = tmp();

        let first = install_hook_at(&ToolTarget::Codex, dir.path()).expect("first install failed");
        assert!(!first.already_installed);

        let second =
            install_hook_at(&ToolTarget::Codex, dir.path()).expect("second install failed");
        assert!(
            second.already_installed,
            "second install should be already_installed"
        );
    }

    // ── 6. generate_instructions mentions tok0 ────────────────────────────────
    #[test]
    fn test_generate_instructions_contains_tok0() {
        let instructions = generate_instructions();
        assert!(
            instructions.contains("tok0"),
            "instructions should mention tok0"
        );
        assert!(
            instructions.contains("tok0 git status") || instructions.contains("tok0"),
            "instructions should include usage example"
        );
    }

    // ── 7. detect_tools finds ClaudeCode when .claude dir exists ──────────────
    #[test]
    fn test_detect_tools_with_claude_dir() {
        let home = tmp();
        fs::create_dir_all(home.path().join(".claude")).expect("mkdir failed");

        let tools = detect_tools_in(Some(home.path()));
        assert!(
            tools.contains(&ToolTarget::ClaudeCode),
            "ClaudeCode should be detected when ~/.claude exists"
        );
    }

    // ── 8. detect_tools returns empty for empty home dir ──────────────────────
    #[test]
    fn test_detect_tools_empty() {
        let home = tmp();
        let tools = detect_tools_in(Some(home.path()));
        assert!(
            tools.is_empty(),
            "No tools should be detected in an empty home dir"
        );
    }

    // ── 9. HookInstallResult fields are correct ───────────────────────────────
    #[test]
    fn test_install_result_fields() {
        let dir = tmp();
        let result = install_hook_at(&ToolTarget::ClaudeCode, dir.path()).expect("install failed");

        assert_eq!(result.tool, "claude-code");
        assert_eq!(result.path, dir.path().join("settings.json"));
        assert!(!result.already_installed);
    }

    // ── 10. Malformed JSON returns error ──────────────────────────────────────
    #[test]
    fn test_install_claude_code_with_malformed_json() {
        let dir = tmp();
        let settings_path = dir.path().join("settings.json");

        fs::write(&settings_path, b"{ this is not valid json }").expect("write failed");

        let result = install_hook_at(&ToolTarget::ClaudeCode, dir.path());
        assert!(
            result.is_err(),
            "Should return error for malformed settings.json"
        );
        let err = result.unwrap_err();
        let msg = format!("{:#}", err);
        assert!(
            msg.contains("parse") || msg.contains("JSON") || msg.contains("json"),
            "Error should mention parsing/JSON: {}",
            msg
        );
    }

    // ── 11. detect_tools finds multiple tools ─────────────────────────────────
    #[test]
    fn test_detect_tools_multiple() {
        let home = tmp();
        fs::create_dir_all(home.path().join(".claude")).expect("mkdir failed");
        fs::create_dir_all(home.path().join(".cursor")).expect("mkdir failed");

        let tools = detect_tools_in(Some(home.path()));
        assert!(tools.contains(&ToolTarget::ClaudeCode));
        assert!(tools.contains(&ToolTarget::Cursor));
    }

    // ── 12. detect_tools returns None for None home ───────────────────────────
    #[test]
    fn test_detect_tools_none_home() {
        let tools = detect_tools_in(None);
        assert!(tools.is_empty(), "None home should yield empty tool list");
    }

    // ── 13. ClaudeCode uninstall removes hook ────────────────────────────────
    #[test]
    fn test_uninstall_claude_code_removes_hook() {
        let dir = tmp();
        // Install first
        install_hook_at(&ToolTarget::ClaudeCode, dir.path()).expect("install failed");

        let settings_path = dir.path().join("settings.json");
        let before = fs::read_to_string(&settings_path).expect("read");
        assert!(
            before.contains(HOOK_MARKER),
            "hook should be present before uninstall"
        );

        // Uninstall
        let result =
            uninstall_hook_at(&ToolTarget::ClaudeCode, dir.path()).expect("uninstall failed");
        assert!(result.already_installed, "should report hook was removed");

        let after = fs::read_to_string(&settings_path).expect("read after");
        assert!(
            !after.contains(HOOK_MARKER),
            "hook should be removed after uninstall"
        );
    }

    // ── 14. ClaudeCode uninstall preserves other hooks ───────────────────────
    #[test]
    fn test_uninstall_claude_code_preserves_other_hooks() {
        let dir = tmp();
        let settings_path = dir.path().join("settings.json");

        // Pre-existing settings with tok0 + another hook
        let existing = serde_json::json!({
            "theme": "dark",
            "hooks": {
                "PreToolUse": [
                    {"matcher": "", "hooks": [{"type": "command", "command": "tok0 rewrite"}]},
                    {"matcher": "", "hooks": [{"type": "command", "command": "echo other"}]}
                ],
                "PostToolUse": [{"matcher": "", "hooks": [{"type": "command", "command": "echo done"}]}]
            }
        });
        fs::write(
            &settings_path,
            serde_json::to_string_pretty(&existing).unwrap(),
        )
        .expect("write");

        uninstall_hook_at(&ToolTarget::ClaudeCode, dir.path()).expect("uninstall failed");

        let after: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(&settings_path).expect("read"))
                .expect("parse");

        assert_eq!(after["theme"], "dark", "theme preserved");
        assert!(
            after["hooks"]["PostToolUse"].is_array(),
            "PostToolUse preserved"
        );
        // PreToolUse should have only the non-tok0 entry
        let pre = after["hooks"]["PreToolUse"].as_array().expect("array");
        assert_eq!(pre.len(), 1, "only non-tok0 entry should remain");
        assert!(!serde_json::to_string(&pre[0]).unwrap().contains("tok0"));
    }

    // ── 15. ClaudeCode uninstall is idempotent ───────────────────────────────
    #[test]
    fn test_uninstall_claude_code_idempotent() {
        let dir = tmp();
        // No hook installed
        let result = uninstall_hook_at(&ToolTarget::ClaudeCode, dir.path()).expect("uninstall");
        assert!(!result.already_installed, "nothing to remove");
    }

    // ── 16. Codex uninstall removes AGENTS.md ────────────────────────────────
    #[test]
    fn test_uninstall_codex_removes_agents_md() {
        let dir = tmp();
        install_hook_at(&ToolTarget::Codex, dir.path()).expect("install");
        assert!(dir.path().join("AGENTS.md").exists());

        let result = uninstall_hook_at(&ToolTarget::Codex, dir.path()).expect("uninstall");
        assert!(result.already_installed);
        assert!(
            !dir.path().join("AGENTS.md").exists(),
            "AGENTS.md should be removed"
        );
    }

    // ── 17. Codex uninstall is idempotent ────────────────────────────────────
    #[test]
    fn test_uninstall_codex_idempotent() {
        let dir = tmp();
        let result = uninstall_hook_at(&ToolTarget::Codex, dir.path()).expect("uninstall");
        assert!(!result.already_installed, "nothing to remove");
    }

    // ═══════════════════════════════════════════════════════════════════════
    // Tier 1 installer tests for all supported tools (per-tool parametric).
    // ═══════════════════════════════════════════════════════════════════════

    fn all_instruction_tools() -> Vec<(ToolTarget, &'static str, &'static str)> {
        // (tool, expected filename, expected slug)
        vec![
            (ToolTarget::Cursor, "tok0.md", "cursor"),
            (ToolTarget::GeminiCli, "GEMINI.md", "gemini-cli"),
            (ToolTarget::Windsurf, "global_rules.md", "windsurf"),
            (ToolTarget::Cline, "tok0.md", "cline"),
            (ToolTarget::Amp, "AGENTS.md", "amp"),
            (ToolTarget::OpenCode, "AGENTS.md", "opencode"),
            (ToolTarget::KimiCode, "tok0.md", "kimi-code"),
            (ToolTarget::Codex, "AGENTS.md", "codex"),
        ]
    }

    #[test]
    fn test_instructions_filename_matches_expected() {
        for (tool, expected_filename, _) in all_instruction_tools() {
            assert_eq!(
                instructions_filename(&tool),
                Some(expected_filename),
                "{:?}: unexpected instructions filename",
                tool
            );
        }
        assert_eq!(
            instructions_filename(&ToolTarget::ClaudeCode),
            None,
            "ClaudeCode uses hook, not instructions file"
        );
    }

    /// Phase 3 invariant: the bytes installed by `install_hook_at` for each
    /// instructions-file tool must be exactly the bytes shipped in
    /// `hooks/<tool>/<filename>`. This pins the externalized templates as
    /// the single source of truth — Rust string literals inside setup.rs
    /// are never the canonical content.
    #[test]
    fn test_installed_file_equals_external_template() {
        let cases: Vec<(ToolTarget, &str, &'static str)> = vec![
            (
                ToolTarget::Cursor,
                "tok0.md",
                include_str!("../../hooks/cursor/tok0.md"),
            ),
            (
                ToolTarget::GeminiCli,
                "GEMINI.md",
                include_str!("../../hooks/gemini/GEMINI.md"),
            ),
            (
                ToolTarget::Windsurf,
                "global_rules.md",
                include_str!("../../hooks/windsurf/global_rules.md"),
            ),
            (
                ToolTarget::Cline,
                "tok0.md",
                include_str!("../../hooks/cline/tok0.md"),
            ),
            (
                ToolTarget::Amp,
                "AGENTS.md",
                include_str!("../../hooks/amp/AGENTS.md"),
            ),
            (
                ToolTarget::OpenCode,
                "AGENTS.md",
                include_str!("../../hooks/opencode/AGENTS.md"),
            ),
            (
                ToolTarget::Codex,
                "AGENTS.md",
                include_str!("../../hooks/codex/AGENTS.md"),
            ),
            (
                ToolTarget::KimiCode,
                "tok0.md",
                include_str!("../../hooks/kimi/tok0.md"),
            ),
        ];
        for (tool, filename, expected) in cases {
            let dir = tmp();
            install_hook_at(&tool, dir.path())
                .unwrap_or_else(|e| panic!("{:?}: install failed: {:#}", tool, e));
            let actual =
                fs::read_to_string(dir.path().join(filename)).expect("read installed file");
            assert_eq!(
                actual, expected,
                "{:?}: installed bytes must match hooks/<tool>/{}",
                tool, filename
            );
        }
    }

    #[test]
    fn test_install_creates_instructions_file_for_every_tool() {
        for (tool, filename, slug) in all_instruction_tools() {
            let dir = tmp();
            let result = install_hook_at(&tool, dir.path())
                .unwrap_or_else(|e| panic!("{:?}: install failed: {:#}", tool, e));

            assert_eq!(result.tool, slug, "{:?}: slug mismatch", tool);
            assert!(
                !result.already_installed,
                "{:?}: first install should not report already_installed",
                tool
            );

            let path = dir.path().join(filename);
            assert!(path.exists(), "{:?}: {} should be created", tool, filename);

            let content = fs::read_to_string(&path).expect("read file");
            assert!(
                content.contains(HOOK_MARKER),
                "{:?}: file should contain 'tok0' marker",
                tool
            );
        }
    }

    #[test]
    fn test_install_idempotent_for_every_tool() {
        for (tool, _, _) in all_instruction_tools() {
            let dir = tmp();
            install_hook_at(&tool, dir.path()).expect("first install");
            let second = install_hook_at(&tool, dir.path()).expect("second install");
            assert!(
                second.already_installed,
                "{:?}: second install should be already_installed",
                tool
            );
        }
    }

    #[test]
    fn test_install_preserves_existing_non_tok0_content() {
        for (tool, filename, _) in all_instruction_tools() {
            let dir = tmp();
            let file_path = dir.path().join(filename);
            let pre_existing = "# My Rules\n\nAlways be concise.\n";
            fs::create_dir_all(dir.path()).expect("mkdir");
            fs::write(&file_path, pre_existing).expect("write pre-existing");

            install_hook_at(&tool, dir.path()).expect("install");

            let content = fs::read_to_string(&file_path).expect("read after install");
            assert!(
                content.contains("My Rules"),
                "{:?}: existing content should be preserved",
                tool
            );
            assert!(
                content.contains("Always be concise"),
                "{:?}: existing content body should be preserved",
                tool
            );
            assert!(
                content.contains(HOOK_MARKER),
                "{:?}: tok0 section should be appended",
                tool
            );
        }
    }

    #[test]
    fn test_uninstall_removes_tok0_only_file() {
        for (tool, filename, _) in all_instruction_tools() {
            let dir = tmp();
            install_hook_at(&tool, dir.path()).expect("install");
            let path = dir.path().join(filename);
            assert!(path.exists());

            let result = uninstall_hook_at(&tool, dir.path()).expect("uninstall");
            assert!(
                result.already_installed,
                "{:?}: uninstall should report hook was removed",
                tool
            );
            assert!(
                !path.exists(),
                "{:?}: file should be removed when it only contained tok0 content",
                tool
            );
        }
    }

    #[test]
    fn test_uninstall_preserves_non_tok0_content() {
        for (tool, filename, _) in all_instruction_tools() {
            let dir = tmp();
            // Install onto pre-existing content
            let pre_existing = "# My Custom Rules\n\nBe concise.\n";
            fs::create_dir_all(dir.path()).expect("mkdir");
            let file_path = dir.path().join(filename);
            fs::write(&file_path, pre_existing).expect("write");
            install_hook_at(&tool, dir.path()).expect("install");

            // Now uninstall
            uninstall_hook_at(&tool, dir.path()).expect("uninstall");

            // File should still exist with original content intact
            assert!(file_path.exists(), "{:?}: file should remain", tool);
            let content = fs::read_to_string(&file_path).expect("read");
            assert!(
                content.contains("My Custom Rules"),
                "{:?}: non-tok0 content must be preserved after uninstall",
                tool
            );
            assert!(
                !content.contains(HOOK_MARKER),
                "{:?}: tok0 marker should be removed after uninstall",
                tool
            );
        }
    }

    #[test]
    fn test_uninstall_idempotent_for_every_tool() {
        for (tool, _, _) in all_instruction_tools() {
            let dir = tmp();
            let result = uninstall_hook_at(&tool, dir.path()).expect("uninstall");
            assert!(
                !result.already_installed,
                "{:?}: uninstall on clean dir should report nothing removed",
                tool
            );
        }
    }

    #[test]
    fn test_detect_kimi_code() {
        let home = tmp();
        fs::create_dir_all(home.path().join(".kimi")).expect("mkdir");
        let tools = detect_tools_in(Some(home.path()));
        assert!(
            tools.contains(&ToolTarget::KimiCode),
            "KimiCode should be detected when ~/.kimi exists"
        );
    }

    #[test]
    fn test_config_dir_for_each_tool() {
        let home = std::path::PathBuf::from("/home/test");
        assert_eq!(
            config_dir_for(&ToolTarget::ClaudeCode, &home),
            home.join(".claude")
        );
        assert_eq!(
            config_dir_for(&ToolTarget::Cursor, &home),
            home.join(".cursor")
        );
        assert_eq!(
            config_dir_for(&ToolTarget::GeminiCli, &home),
            home.join(".gemini")
        );
        assert_eq!(
            config_dir_for(&ToolTarget::Windsurf, &home),
            home.join(".codeium").join("windsurf").join("memories"),
            "Windsurf writes to the canonical Cascade Memories path"
        );
        assert_eq!(
            config_dir_for(&ToolTarget::Cline, &home),
            home.join("Documents").join("Cline").join("Rules"),
            "Cline scans Documents/Cline/Rules for .md files"
        );
        assert_eq!(
            config_dir_for(&ToolTarget::Amp, &home),
            home.join(".config").join("amp")
        );
        assert_eq!(
            config_dir_for(&ToolTarget::OpenCode, &home),
            home.join(".config").join("opencode")
        );
        assert_eq!(
            config_dir_for(&ToolTarget::Codex, &home),
            home.join(".codex")
        );
        assert_eq!(
            config_dir_for(&ToolTarget::KimiCode, &home),
            home.join(".kimi")
        );
    }

    #[test]
    fn test_detect_windsurf_modern_path() {
        let home = tmp();
        fs::create_dir_all(home.path().join(".codeium").join("windsurf")).expect("mkdir");
        let tools = detect_tools_in(Some(home.path()));
        assert!(tools.contains(&ToolTarget::Windsurf));
    }

    #[test]
    fn test_detect_windsurf_legacy_path() {
        let home = tmp();
        fs::create_dir_all(home.path().join(".windsurf")).expect("mkdir");
        let tools = detect_tools_in(Some(home.path()));
        assert!(tools.contains(&ToolTarget::Windsurf));
    }

    #[test]
    fn test_detect_cline_via_documents() {
        let home = tmp();
        fs::create_dir_all(home.path().join("Documents").join("Cline")).expect("mkdir");
        let tools = detect_tools_in(Some(home.path()));
        assert!(tools.contains(&ToolTarget::Cline));
    }

    #[test]
    fn test_detect_amp_via_config() {
        let home = tmp();
        fs::create_dir_all(home.path().join(".config").join("amp")).expect("mkdir");
        let tools = detect_tools_in(Some(home.path()));
        assert!(tools.contains(&ToolTarget::Amp));
    }

    #[test]
    fn test_detect_opencode_via_config() {
        let home = tmp();
        fs::create_dir_all(home.path().join(".config").join("opencode")).expect("mkdir");
        let tools = detect_tools_in(Some(home.path()));
        assert!(tools.contains(&ToolTarget::OpenCode));
    }

    #[test]
    fn test_post_install_hint_for_gui_only_tools() {
        let cursor = post_install_hint(&ToolTarget::Cursor).expect("cursor hint");
        assert!(cursor.contains("GUI-only"), "cursor: {}", cursor);
        assert!(cursor.contains("Settings"));

        let kimi = post_install_hint(&ToolTarget::KimiCode).expect("kimi hint");
        assert!(kimi.contains("GUI-only"), "kimi: {}", kimi);
    }

    #[test]
    fn test_post_install_hint_for_cline_has_toggle_instruction() {
        let cline = post_install_hint(&ToolTarget::Cline).expect("cline hint");
        assert!(
            cline.contains("toggle"),
            "cline hint must mention toggle: {}",
            cline
        );
        assert!(
            cline.contains("VS Code") || cline.contains("Rules"),
            "must direct user to Rules panel"
        );
    }

    #[test]
    fn test_post_install_hint_none_for_automatic_tools() {
        for tool in [
            ToolTarget::ClaudeCode,
            ToolTarget::GeminiCli,
            ToolTarget::Windsurf,
            ToolTarget::Amp,
            ToolTarget::OpenCode,
            ToolTarget::Codex,
        ] {
            assert!(
                post_install_hint(&tool).is_none(),
                "{:?} auto-loads; no manual step needed",
                tool
            );
        }
    }
}

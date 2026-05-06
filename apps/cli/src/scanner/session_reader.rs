use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct SessionCommand {
    pub command: String,
    pub tool: String,
}

/// Find Claude Code session JSONL files
pub fn find_claude_sessions() -> Vec<PathBuf> {
    let base = crate::bridge::setup::home_dir_or_default()
        .unwrap_or_default()
        .join(".claude")
        .join("projects");
    if !base.exists() {
        return Vec::new();
    }
    walkdir::WalkDir::new(&base)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().is_some_and(|ext| ext == "jsonl"))
        .map(|e| e.path().to_path_buf())
        .collect()
}

/// Extract shell commands from a Claude Code session JSONL file
pub fn extract_commands_from_jsonl(path: &Path) -> Result<Vec<SessionCommand>> {
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read session: {}", path.display()))?;
    let mut commands = Vec::new();
    for line in content.lines() {
        let Ok(val) = serde_json::from_str::<serde_json::Value>(line) else {
            continue;
        };
        // Claude Code format: {"type":"tool_use","name":"Bash","input":{"command":"..."}}
        if val.pointer("/type").and_then(|v| v.as_str()) == Some("tool_use")
            && val.pointer("/name").and_then(|v| v.as_str()) == Some("Bash")
        {
            if let Some(cmd) = val.pointer("/input/command").and_then(|v| v.as_str()) {
                commands.push(SessionCommand {
                    command: cmd.to_string(),
                    tool: "claude".to_string(),
                });
            }
        }
    }
    Ok(commands)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn test_extract_bash_commands() {
        let mut tmp = tempfile::NamedTempFile::new().expect("Failed to create temp file");
        writeln!(
            tmp,
            r#"{{"type":"tool_use","name":"Bash","input":{{"command":"git status"}}}}"#
        )
        .expect("write failed");
        writeln!(
            tmp,
            r#"{{"type":"tool_use","name":"Read","input":{{"file":"foo.rs"}}}}"#
        )
        .expect("write failed");
        writeln!(tmp, r#"{{"type":"text","text":"Hello world"}}"#).expect("write failed");
        writeln!(
            tmp,
            r#"{{"type":"tool_use","name":"Bash","input":{{"command":"cargo test"}}}}"#
        )
        .expect("write failed");
        tmp.flush().expect("flush failed");

        let cmds = extract_commands_from_jsonl(tmp.path()).expect("extract failed");
        assert_eq!(cmds.len(), 2);
        assert_eq!(cmds[0].command, "git status");
        assert_eq!(cmds[0].tool, "claude");
        assert_eq!(cmds[1].command, "cargo test");
    }

    #[test]
    fn test_empty_session() {
        let tmp = tempfile::NamedTempFile::new().expect("Failed to create temp file");
        let cmds = extract_commands_from_jsonl(tmp.path()).expect("extract failed");
        assert!(cmds.is_empty());
    }

    #[test]
    fn test_malformed_lines_skipped() {
        let mut tmp = tempfile::NamedTempFile::new().expect("Failed to create temp file");
        writeln!(tmp, "this is not json at all").expect("write failed");
        writeln!(tmp, "{{{{broken json").expect("write failed");
        writeln!(
            tmp,
            r#"{{"type":"tool_use","name":"Bash","input":{{"command":"ls -la"}}}}"#
        )
        .expect("write failed");
        writeln!(tmp).expect("write failed");
        tmp.flush().expect("flush failed");

        let cmds = extract_commands_from_jsonl(tmp.path()).expect("extract failed");
        assert_eq!(cmds.len(), 1);
        assert_eq!(cmds[0].command, "ls -la");
    }

    #[test]
    fn test_no_command_field() {
        let mut tmp = tempfile::NamedTempFile::new().expect("Failed to create temp file");
        // tool_use with Bash but missing the input.command field
        writeln!(
            tmp,
            r#"{{"type":"tool_use","name":"Bash","input":{{"timeout":5000}}}}"#
        )
        .expect("write failed");
        // tool_use with Bash but empty input object
        writeln!(tmp, r#"{{"type":"tool_use","name":"Bash","input":{{}}}}"#).expect("write failed");
        tmp.flush().expect("flush failed");

        let cmds = extract_commands_from_jsonl(tmp.path()).expect("extract failed");
        assert!(cmds.is_empty());
    }
}

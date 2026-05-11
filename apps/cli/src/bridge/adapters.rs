use anyhow::{Context, Result};
use serde::Deserialize;

// ---------------------------------------------------------------------------
// Claude Code adapter
// Payload shape (Claude Code 2.x, current):
//   {"session_id":"...","tool_name":"Bash","tool_input":{"command":"git status"},
//    "tool_use_id":"...","cwd":"...","permission_mode":"default"}
// Docs: https://code.claude.com/docs/en/hooks.md
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
pub struct ClaudeCodePayload {
    pub tool_name: String,
    pub tool_input: ClaudeCodeInput,
    #[serde(default)]
    pub session_id: Option<String>,
}

#[derive(Deserialize)]
pub struct ClaudeCodeInput {
    pub command: String,
}

/// Decoded hook payload — what the rewriter needs to construct the
/// rewritten command. `session_id` is `None` when the payload omitted
/// the field (older Claude Code releases, or other adapters that don't
/// have a session concept).
#[derive(Debug)]
pub struct RewriteRequest {
    pub command: String,
    pub session_id: Option<String>,
}

/// Parse a Claude Code PreToolUse payload and return the extracted command
/// plus the (optional) session id. Only succeeds when `tool_name == "Bash"`
/// — other tools have different input shapes and would deserialize-fail or
/// yield meaningless commands.
///
/// The session id is preserved so the dispatcher can scope per-tool state
/// (Phase 5) and per-session cross-invocation context.
pub fn translate_claude_code(payload: &str) -> Result<RewriteRequest> {
    let p: ClaudeCodePayload =
        serde_json::from_str(payload).context("Failed to parse Claude Code hook payload")?;
    if p.tool_name != "Bash" {
        anyhow::bail!(
            "tok0 only rewrites Bash tool calls; got tool_name={}",
            p.tool_name
        );
    }
    Ok(RewriteRequest {
        command: p.tool_input.command,
        session_id: p.session_id,
    })
}

// ---------------------------------------------------------------------------
// Gemini / Copilot / argv adapters
//
// Defined for future hook integrations. Today only the Claude Code
// adapter is wired into production (see main.rs::run_rewrite); the
// others stay behind #[allow(dead_code)] so the implementations don't
// rot, but the dead-code lint doesn't fire on every build.
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
#[allow(dead_code)]
struct GeminiPayload {
    tool: String,
    args: Vec<String>,
}

/// Parse a Gemini hook payload and return the extracted command string.
/// Payload shape: `{"tool":"shell","args":["git","status"]}`.
#[allow(dead_code)]
pub fn translate_gemini(payload: &str) -> Result<String> {
    let p: GeminiPayload =
        serde_json::from_str(payload).context("Failed to parse Gemini hook payload")?;
    Ok(p.args.join(" "))
}

#[derive(Deserialize)]
#[allow(dead_code)]
struct CopilotPayload {
    command: String,
}

/// Parse a Copilot hook payload and return the extracted command string.
/// Payload shape: `{"command":"git status"}`.
#[allow(dead_code)]
pub fn translate_copilot(payload: &str) -> Result<String> {
    let p: CopilotPayload =
        serde_json::from_str(payload).context("Failed to parse Copilot hook payload")?;
    Ok(p.command)
}

/// Join argv-style args into a single command string.
#[allow(dead_code)]
pub fn translate_argv(args: &[String]) -> String {
    args.join(" ")
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // --- Claude Code ---

    #[test]
    fn test_translate_claude_code_valid() {
        let payload = r#"{"session_id":"abc","tool_name":"Bash",
            "tool_input":{"command":"git status"},"tool_use_id":"x",
            "cwd":"/p","permission_mode":"default"}"#;
        let result = translate_claude_code(payload).expect("should parse");
        assert_eq!(result.command, "git status");
        assert_eq!(result.session_id.as_deref(), Some("abc"));
    }

    #[test]
    fn test_translate_claude_code_minimal_valid() {
        // Only the load-bearing fields. Extra payload fields are ignored.
        let payload = r#"{"tool_name":"Bash","tool_input":{"command":"ls"}}"#;
        let result = translate_claude_code(payload).expect("should parse");
        assert_eq!(result.command, "ls");
        assert_eq!(result.session_id, None);
    }

    #[test]
    fn test_translate_claude_code_malformed() {
        let result = translate_claude_code("not json");
        assert!(result.is_err());
    }

    #[test]
    fn test_translate_claude_code_missing_field() {
        let payload = r#"{"tool_name":"Bash"}"#;
        let result = translate_claude_code(payload);
        assert!(result.is_err());
    }

    #[test]
    fn test_translate_claude_code_non_bash_tool_rejected() {
        // tok0 only rewrites Bash; Write/Edit/etc. have different schemas
        // and don't need a tok0 prefix.
        let payload = r#"{"tool_name":"Write","tool_input":{"command":"…"}}"#;
        let err = translate_claude_code(payload).expect_err("should reject non-Bash");
        assert!(format!("{:#}", err).contains("only rewrites Bash"));
    }

    // --- Gemini ---

    #[test]
    fn test_translate_gemini_valid() {
        let payload = r#"{"tool":"shell","args":["git","status"]}"#;
        let result = translate_gemini(payload).expect("should parse");
        assert_eq!(result, "git status");
    }

    #[test]
    fn test_translate_gemini_malformed() {
        let result = translate_gemini("{bad json}");
        assert!(result.is_err());
    }

    #[test]
    fn test_translate_gemini_empty_args() {
        let payload = r#"{"tool":"shell","args":[]}"#;
        let result = translate_gemini(payload).expect("should parse");
        assert_eq!(result, "");
    }

    // --- Copilot ---

    #[test]
    fn test_translate_copilot_valid() {
        let payload = r#"{"command":"cargo build --release"}"#;
        let result = translate_copilot(payload).expect("should parse");
        assert_eq!(result, "cargo build --release");
    }

    #[test]
    fn test_translate_copilot_malformed() {
        let result = translate_copilot("");
        assert!(result.is_err());
    }

    // --- argv ---

    #[test]
    fn test_translate_argv_joins() {
        let args: Vec<String> = ["git", "log", "--oneline"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        assert_eq!(translate_argv(&args), "git log --oneline");
    }

    #[test]
    fn test_translate_argv_empty() {
        assert_eq!(translate_argv(&[]), "");
    }

    // ═══════════════════════════════════════════════════════════════════════
    // Property tests — arbitrary bytes / malformed inputs must never panic.
    // ═══════════════════════════════════════════════════════════════════════
    use proptest::prelude::*;

    proptest! {
        /// Adapters must never panic on arbitrary input — they return Err on
        /// malformed JSON, never blow up.
        #[test]
        fn prop_claude_code_never_panics(payload in any::<String>()) {
            let _ = translate_claude_code(&payload);
        }

        #[test]
        fn prop_gemini_never_panics(payload in any::<String>()) {
            let _ = translate_gemini(&payload);
        }

        #[test]
        fn prop_copilot_never_panics(payload in any::<String>()) {
            let _ = translate_copilot(&payload);
        }

        #[test]
        fn prop_argv_never_panics(args in proptest::collection::vec(any::<String>(), 0..20)) {
            let _ = translate_argv(&args);
        }

        /// Round-trip: a valid Claude Code payload built from an arbitrary
        /// command string extracts back that exact command.
        #[test]
        fn prop_claude_code_round_trip(cmd in "[a-zA-Z0-9 _./\\-]{0,60}") {
            let payload = serde_json::json!({
                "tool_name": "Bash",
                "tool_input": {"command": cmd.clone()}
            })
            .to_string();
            let extracted = translate_claude_code(&payload).expect("valid round-trip");
            prop_assert_eq!(extracted.command, cmd);
        }

        /// Round-trip: a valid Gemini payload with arbitrary args joins them
        /// back into the command string with single-space delimiters.
        #[test]
        fn prop_gemini_round_trip(
            args in proptest::collection::vec("[a-zA-Z0-9_./\\-]{1,20}", 0..6)
        ) {
            let payload = serde_json::json!({
                "tool": "shell",
                "args": args.clone()
            })
            .to_string();
            let extracted = translate_gemini(&payload).expect("valid round-trip");
            prop_assert_eq!(extracted, args.join(" "));
        }

        /// Oversized payloads (up to ~100KB) are handled gracefully — no OOM,
        /// no panic, either valid parse or clean error.
        #[test]
        fn prop_large_payload_handled(size in 0usize..100_000) {
            let payload = "x".repeat(size);
            let _ = translate_claude_code(&payload);
            let _ = translate_gemini(&payload);
            let _ = translate_copilot(&payload);
        }
    }
}

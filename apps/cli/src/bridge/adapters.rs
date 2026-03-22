use anyhow::{Context, Result};
use serde::Deserialize;

// ---------------------------------------------------------------------------
// Claude Code adapter
// Payload shape: {"tool":"Bash","input":{"command":"git status"}}
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
struct ClaudeCodePayload {
    #[allow(dead_code)]
    tool: String,
    input: ClaudeCodeInput,
}

#[derive(Deserialize)]
struct ClaudeCodeInput {
    command: String,
}

/// Parse a Claude Code hook payload and return the extracted command string.
pub fn translate_claude_code(payload: &str) -> Result<String> {
    let p: ClaudeCodePayload =
        serde_json::from_str(payload).context("Failed to parse Claude Code hook payload")?;
    Ok(p.input.command)
}

// ---------------------------------------------------------------------------
// Gemini adapter
// Payload shape: {"tool":"shell","args":["git","status"]}
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
struct GeminiPayload {
    #[allow(dead_code)]
    tool: String,
    args: Vec<String>,
}

/// Parse a Gemini hook payload and return the extracted command string.
pub fn translate_gemini(payload: &str) -> Result<String> {
    let p: GeminiPayload =
        serde_json::from_str(payload).context("Failed to parse Gemini hook payload")?;
    Ok(p.args.join(" "))
}

// ---------------------------------------------------------------------------
// Copilot adapter
// Payload shape: {"command":"git status"}
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
struct CopilotPayload {
    command: String,
}

/// Parse a Copilot hook payload and return the extracted command string.
pub fn translate_copilot(payload: &str) -> Result<String> {
    let p: CopilotPayload =
        serde_json::from_str(payload).context("Failed to parse Copilot hook payload")?;
    Ok(p.command)
}

// ---------------------------------------------------------------------------
// argv adapter — no JSON, just space-join
// ---------------------------------------------------------------------------

/// Join argv-style args into a single command string.
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
        let payload = r#"{"tool":"Bash","input":{"command":"git status"}}"#;
        let result = translate_claude_code(payload).expect("should parse");
        assert_eq!(result, "git status");
    }

    #[test]
    fn test_translate_claude_code_malformed() {
        let result = translate_claude_code("not json");
        assert!(result.is_err());
    }

    #[test]
    fn test_translate_claude_code_missing_field() {
        // `input` key is absent
        let payload = r#"{"tool":"Bash"}"#;
        let result = translate_claude_code(payload);
        assert!(result.is_err());
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
                "tool": "Bash",
                "input": {"command": cmd.clone()}
            })
            .to_string();
            let extracted = translate_claude_code(&payload).expect("valid round-trip");
            prop_assert_eq!(extracted, cmd);
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

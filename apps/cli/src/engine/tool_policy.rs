//! Per-AI-tool compression policy.
//!
//! Different AI tools have different context-window budgets and different
//! tolerances for terse output. Claude Code (200k context) can absorb
//! richer output than Cursor (smaller context). Phase 4 of the aggressive
//! token-reduction work threads tool identity from the bridge hook all
//! the way to the dispatcher so each tool gets a calibrated
//! `apply_output_cap` budget.
//!
//! Identity arrives via the `TOK0_TOOL` env var, inlined into the
//! rewritten command by `bridge::rewriter` (the rewriter is the only
//! place that knows which adapter sent the command — see
//! `bridge::adapters`). The env var must be inlined into the command
//! prefix rather than exported in the hook script: PreToolUse hooks
//! substitute the command string, which then runs in a fresh shell that
//! does not inherit the hook script's env mutations.
//!
//! Unknown / unset `TOK0_TOOL` → `balanced()` (matches the pre-Phase-4
//! hardcoded `30_000 / 60 / 30` defaults exactly, so this module is
//! backward-compatible by construction).

#[derive(Clone, Copy, Debug)]
#[allow(dead_code)] // `id` is exposed for tests/diagnostics; dispatcher reads only the caps.
pub struct ToolPolicy {
    pub id: &'static str,
    /// Global hard cap on output chars before head/tail truncation kicks in.
    pub max_chars: usize,
    /// Head lines kept when `max_chars` is exceeded.
    pub head_lines: usize,
    /// Tail lines kept when `max_chars` is exceeded.
    pub tail_lines: usize,
}

/// Default ("unknown") policy. Identical to Phase 1's hardcoded constants
/// so any environment without TOK0_TOOL sees no behavior change.
pub const BALANCED: ToolPolicy = ToolPolicy {
    id: "unknown",
    max_chars: 30_000,
    head_lines: 60,
    tail_lines: 30,
};

const CLAUDE_CODE: ToolPolicy = ToolPolicy {
    id: "claude_code",
    max_chars: 200_000,
    head_lines: 300,
    tail_lines: 150,
};

const CODEX: ToolPolicy = ToolPolicy {
    id: "codex",
    max_chars: 150_000,
    head_lines: 250,
    tail_lines: 120,
};

const GEMINI: ToolPolicy = ToolPolicy {
    id: "gemini",
    max_chars: 100_000,
    head_lines: 200,
    tail_lines: 100,
};

/// Resolve a tool id (case-sensitive) to its policy. Unknown ids fall
/// back to `BALANCED`.
pub fn for_tool(id: &str) -> &'static ToolPolicy {
    match id {
        "claude_code" => &CLAUDE_CODE,
        "codex" => &CODEX,
        "gemini" => &GEMINI,
        // Tools that share Cursor's profile (smaller context, default
        // catchall caps): cursor, cline, windsurf, amp, opencode, kimi.
        // Listed in the design doc but not differentiated yet — all map
        // to the balanced default.
        _ => &BALANCED,
    }
}

/// The policy for the current process, derived from `TOK0_TOOL`. Reads
/// the env var on every call: this is once per dispatch (not per line),
/// and cheap. We deliberately don't cache via `OnceLock` so tests can
/// override the env without process restart.
pub fn current() -> &'static ToolPolicy {
    let tool = std::env::var("TOK0_TOOL").unwrap_or_default();
    for_tool(&tool)
}

/// The balanced default — exposed for callers that explicitly want the
/// catchall behavior regardless of env. Currently used only in tests; the
/// dispatcher reads `current()` exclusively.
#[allow(dead_code)]
pub fn balanced() -> &'static ToolPolicy {
    &BALANCED
}

#[cfg(test)]
mod tests {
    use super::*;

    // Use a mutex to serialize tests that set TOK0_TOOL — otherwise
    // parallel test runs race on the shared process env.
    use std::sync::Mutex;
    static ENV_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn test_for_tool_known_ids() {
        assert_eq!(for_tool("claude_code").id, "claude_code");
        assert_eq!(for_tool("codex").id, "codex");
        assert_eq!(for_tool("gemini").id, "gemini");
    }

    #[test]
    fn test_for_tool_unknown_returns_balanced() {
        assert_eq!(for_tool("unknown_tool").id, "unknown");
        assert_eq!(for_tool("").id, "unknown");
        assert_eq!(for_tool("CLAUDE_CODE").id, "unknown"); // case-sensitive
    }

    #[test]
    fn test_balanced_matches_phase1_defaults() {
        // Backward compat: BALANCED must match the pre-Phase-4 hardcoded
        // dispatcher constants exactly.
        assert_eq!(BALANCED.max_chars, 30_000);
        assert_eq!(BALANCED.head_lines, 60);
        assert_eq!(BALANCED.tail_lines, 30);
    }

    #[test]
    fn test_claude_code_is_least_aggressive() {
        // Claude Code has the largest context, so its cap is the loosest.
        let cc = for_tool("claude_code");
        assert!(cc.max_chars > BALANCED.max_chars);
        assert!(cc.head_lines > BALANCED.head_lines);
        assert!(cc.tail_lines > BALANCED.tail_lines);
    }

    #[test]
    fn test_aggression_ordering() {
        // BALANCED ≤ GEMINI ≤ CODEX ≤ CLAUDE_CODE on every cap.
        let order = ["unknown", "gemini", "codex", "claude_code"];
        for pair in order.windows(2) {
            let lo = for_tool(pair[0]);
            let hi = for_tool(pair[1]);
            assert!(
                hi.max_chars >= lo.max_chars,
                "{} max_chars >= {}",
                pair[1],
                pair[0]
            );
            assert!(hi.head_lines >= lo.head_lines);
            assert!(hi.tail_lines >= lo.tail_lines);
        }
    }

    #[test]
    fn test_current_reads_env() {
        let _g = ENV_LOCK.lock().expect("env lock");
        // SAFETY: serialized with ENV_LOCK; no other thread mutates env here.
        unsafe { std::env::set_var("TOK0_TOOL", "claude_code") };
        assert_eq!(current().id, "claude_code");
        unsafe { std::env::set_var("TOK0_TOOL", "gemini") };
        assert_eq!(current().id, "gemini");
        unsafe { std::env::remove_var("TOK0_TOOL") };
        assert_eq!(current().id, "unknown");
    }

    #[test]
    fn test_current_unset_returns_balanced() {
        let _g = ENV_LOCK.lock().expect("env lock");
        unsafe { std::env::remove_var("TOK0_TOOL") };
        let p = current();
        assert_eq!(p.id, "unknown");
        assert_eq!(p.max_chars, BALANCED.max_chars);
    }
}

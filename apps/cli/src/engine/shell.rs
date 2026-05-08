use anyhow::{Context, Result};
use lazy_static::lazy_static;
use regex::Regex;
use std::process::{Command, Output};
use std::sync::atomic::{AtomicBool, Ordering};

/// Process-wide flag set once at CLI parse time. Read by the dispatcher
/// post-pass to decide whether to strip trailing `Done in …` lines and
/// collapse fully-noisy output to `ok`. Threading a bool through every
/// run_proxy call site is more invasive than this single atomic.
static ULTRA_COMPACT: AtomicBool = AtomicBool::new(false);

/// Set the ultra-compact flag. Called once from main::run().
pub fn set_ultra_compact(enabled: bool) {
    ULTRA_COMPACT.store(enabled, Ordering::Relaxed);
}

/// Read the ultra-compact flag.
pub fn ultra_compact_enabled() -> bool {
    ULTRA_COMPACT.load(Ordering::Relaxed)
}

lazy_static! {
    static ref ANSI_RE: Regex = Regex::new(r"\x1b\[[0-9;]*[a-zA-Z]").unwrap();
    static ref NODE_DEPRECATION_RE: Regex =
        Regex::new(r"^\(node:\d+\)\s+\[(?:DEP|EXP)\d+\]|^\(Use `node --trace-").unwrap();
    static ref ULTRA_DONE_RE: Regex = Regex::new(
        r"^(?:Done in \d+(?:\.\d+)?\s*(?:ms|s|m|h)|Saved lockfile|success Saved lockfile)"
    )
    .unwrap();
}

/// Remove ANSI escape codes from input. Returns borrowed input if no codes present.
pub fn strip_ansi(input: &str) -> std::borrow::Cow<'_, str> {
    if input.contains('\x1b') {
        std::borrow::Cow::Owned(ANSI_RE.replace_all(input, "").to_string())
    } else {
        std::borrow::Cow::Borrowed(input)
    }
}

/// Count whitespace-delimited tokens (proxy for LLM token count).
pub fn count_tokens(text: &str) -> usize {
    text.split_whitespace().count()
}

/// Execute a command and capture its output.
pub fn execute_command(cmd: &str, args: &[&str]) -> Result<Output> {
    Command::new(cmd)
        .args(args)
        .output()
        .with_context(|| format!("Failed to execute: {} {}", cmd, args.join(" ")))
}

/// Truncate a line to max_chars, appending "..." if truncated. Zero-copy when no truncation needed.
pub fn truncate_line(line: &str, max_chars: usize) -> std::borrow::Cow<'_, str> {
    if line.len() <= max_chars {
        std::borrow::Cow::Borrowed(line)
    } else {
        std::borrow::Cow::Owned(format!("{}...", &line[..max_chars.saturating_sub(3)]))
    }
}

/// Keep first `head` and last `tail` lines, dropping middle with a marker.
pub fn head_tail(input: &str, head: usize, tail: usize) -> String {
    let lines: Vec<&str> = input.lines().collect();
    let total = lines.len();
    if total <= head + tail {
        return input.to_string();
    }
    let mut result: Vec<&str> = Vec::with_capacity(head + tail + 1);
    result.extend_from_slice(&lines[..head]);
    let skipped = total - head - tail;
    let marker = format!("... ({} lines omitted) ...", skipped);
    let tail_lines = &lines[total - tail..];
    let mut output = result.join("\n");
    output.push('\n');
    output.push_str(&marker);
    output.push('\n');
    output.push_str(&tail_lines.join("\n"));
    output
}

/// Strip Node.js runtime deprecation footer lines (`(node:NNN) [DEP…]`
/// and `(Use \`node --trace-deprecation …\`)`). Universal: applied as a
/// post-pass on every compressed output regardless of which compressor
/// or TOML rule produced it. Returns borrowed input when no Node footer
/// is present.
pub fn strip_node_deprecation_footer(input: &str) -> std::borrow::Cow<'_, str> {
    if !input.contains("(node:") && !input.contains("(Use `node --trace-") {
        return std::borrow::Cow::Borrowed(input);
    }
    let kept: Vec<&str> = input
        .lines()
        .filter(|line| !NODE_DEPRECATION_RE.is_match(line))
        .collect();
    std::borrow::Cow::Owned(kept.join("\n"))
}

/// Apply the `--ultra-compact` extra strip pass: drop trailing completion
/// markers (`Done in Xs`, `Saved lockfile`). If the result is empty after
/// trimming, returns the literal `ok` so the caller still has a non-empty
/// success indicator.
pub fn apply_ultra_compact(input: &str) -> String {
    let kept: Vec<&str> = input
        .lines()
        .filter(|line| !ULTRA_DONE_RE.is_match(line.trim_start()))
        .collect();
    let joined = kept.join("\n");
    if joined.trim().is_empty() {
        "ok".to_string()
    } else {
        joined
    }
}

/// Hard cap on total characters.
pub fn hard_cap(input: &str, max_chars: usize) -> String {
    if input.len() <= max_chars {
        input.to_string()
    } else {
        format!("{}... (truncated)", &input[..max_chars.saturating_sub(16)])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strip_ansi_with_codes() {
        let input = "\x1b[32mhello\x1b[0m world";
        assert_eq!(strip_ansi(input).as_ref(), "hello world");
    }

    #[test]
    fn test_strip_ansi_without_codes() {
        let input = "hello world";
        // Should return borrowed (zero-copy)
        let result = strip_ansi(input);
        assert_eq!(result.as_ref(), "hello world");
        assert!(matches!(result, std::borrow::Cow::Borrowed(_)));
    }

    #[test]
    fn test_strip_ansi_empty() {
        assert_eq!(strip_ansi("").as_ref(), "");
    }

    #[test]
    fn test_count_tokens() {
        assert_eq!(count_tokens("hello world foo"), 3);
        assert_eq!(count_tokens(""), 0);
        assert_eq!(count_tokens("   "), 0);
        assert_eq!(count_tokens("one"), 1);
    }

    #[test]
    fn test_truncate_line_short() {
        assert_eq!(truncate_line("hello", 10), "hello");
    }

    #[test]
    fn test_truncate_line_long() {
        let result = truncate_line("this is a very long line", 15);
        assert!(result.len() <= 15);
        assert!(result.ends_with("..."));
    }

    #[test]
    fn test_head_tail_small_input() {
        let input = "line1\nline2\nline3";
        assert_eq!(head_tail(input, 5, 5), input);
    }

    #[test]
    fn test_head_tail_large_input() {
        let lines: Vec<String> = (1..=20).map(|i| format!("line{}", i)).collect();
        let input = lines.join("\n");
        let result = head_tail(&input, 3, 2);
        assert!(result.contains("line1"));
        assert!(result.contains("line3"));
        assert!(result.contains("line19"));
        assert!(result.contains("line20"));
        assert!(result.contains("omitted"));
    }

    #[test]
    fn test_strip_node_deprecation_footer_removes_dep() {
        let input = "+ react 19.2.6\n(node:13518) [DEP0169] DeprecationWarning: url.parse() is deprecated\n(Use `node --trace-deprecation ...` to show where the warning was created)\nDone in 9.3s";
        let out = strip_node_deprecation_footer(input);
        assert!(!out.contains("DeprecationWarning"));
        assert!(!out.contains("trace-deprecation"));
        assert!(out.contains("react 19.2.6"));
        assert!(out.contains("Done in 9.3s"));
    }

    #[test]
    fn test_strip_node_deprecation_footer_removes_exp() {
        let input = "(node:42) [EXP0001] ExperimentalWarning: foo\nactual content";
        let out = strip_node_deprecation_footer(input);
        assert!(!out.contains("ExperimentalWarning"));
        assert!(out.contains("actual content"));
    }

    #[test]
    fn test_strip_node_deprecation_footer_zero_copy_when_clean() {
        let input = "no deprecation here\njust output";
        let result = strip_node_deprecation_footer(input);
        assert!(matches!(result, std::borrow::Cow::Borrowed(_)));
    }

    #[test]
    fn test_strip_node_deprecation_footer_does_not_match_unrelated_parens() {
        // "(node:" must be followed by digits — bare parens elsewhere stay.
        let input = "log message (node attached)\n(other stuff)";
        let result = strip_node_deprecation_footer(input);
        assert_eq!(result.as_ref(), input);
    }

    #[test]
    fn test_apply_ultra_compact_strips_done_line() {
        let input = "+ react 19.2.6\nDone in 9.3s using pnpm v10.8.0";
        let out = apply_ultra_compact(input);
        assert!(out.contains("react 19.2.6"));
        assert!(!out.contains("Done in"));
    }

    #[test]
    fn test_apply_ultra_compact_collapses_to_ok_when_empty() {
        let input = "Done in 302ms using pnpm v10.8.0";
        let out = apply_ultra_compact(input);
        assert_eq!(out, "ok");
    }

    #[test]
    fn test_apply_ultra_compact_keeps_real_content() {
        let input = "ERR_FOO: something failed\nDone in 1s";
        let out = apply_ultra_compact(input);
        assert!(out.contains("ERR_FOO"));
        assert!(!out.contains("Done in"));
    }

    #[test]
    fn test_hard_cap_short() {
        assert_eq!(hard_cap("hello", 100), "hello");
    }

    #[test]
    fn test_hard_cap_long() {
        let input = "a".repeat(200);
        let result = hard_cap(&input, 50);
        assert!(result.len() <= 60); // 50 - 16 + "... (truncated)"
        assert!(result.contains("truncated"));
    }
}

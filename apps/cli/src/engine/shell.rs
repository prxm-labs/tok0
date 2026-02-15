use anyhow::{Context, Result};
use lazy_static::lazy_static;
use regex::Regex;
use std::process::{Command, Output};

lazy_static! {
    static ref ANSI_RE: Regex = Regex::new(r"\x1b\[[0-9;]*[a-zA-Z]").unwrap();
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

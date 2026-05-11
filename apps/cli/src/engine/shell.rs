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
    /// Runs of ≥2 box-drawing / spinner characters that show up in progress
    /// bars after ANSI is stripped. Single occurrences (`│` in a table cell)
    /// pass through.
    static ref SPINNER_RUN_RE: Regex =
        Regex::new(r"[█▓▒░╱╲│─━═]{2,}").unwrap();
}

/// Box-drawing / spinner characters that appear as progress-bar fill.
const SPINNER_CHARS: &[char] = &['█', '▓', '▒', '░', '╱', '╲', '│', '─', '━', '═'];

/// Remove ANSI escape codes from input. Returns borrowed input if no codes present.
pub fn strip_ansi(input: &str) -> std::borrow::Cow<'_, str> {
    if input.contains('\x1b') {
        std::borrow::Cow::Owned(ANSI_RE.replace_all(input, "").to_string())
    } else {
        std::borrow::Cow::Borrowed(input)
    }
}

/// Strip terminal progress-bar noise: collapse `\r`-rewritten lines (keep
/// only the segment after the final `\r` on each logical line) and remove
/// runs of ≥2 box-drawing characters. Returns borrowed input when nothing
/// to strip.
///
/// Run after `strip_ansi`. This handles the residual noise that survives
/// pure ANSI removal — `npm install` / `pip install` / `cargo build`
/// progress lines, pnpm fetch animations, brew download spinners.
pub fn strip_progress_noise(input: &str) -> std::borrow::Cow<'_, str> {
    let has_cr = input.contains('\r');
    let has_spinner = input.contains(SPINNER_CHARS);
    if !has_cr && !has_spinner {
        return std::borrow::Cow::Borrowed(input);
    }
    let collapsed: String = if has_cr {
        input
            .lines()
            .map(|line| match line.rfind('\r') {
                Some(idx) => &line[idx + 1..],
                None => line,
            })
            .collect::<Vec<_>>()
            .join("\n")
    } else {
        input.to_string()
    };
    if has_spinner {
        std::borrow::Cow::Owned(SPINNER_RUN_RE.replace_all(&collapsed, "").into_owned())
    } else {
        std::borrow::Cow::Owned(collapsed)
    }
}

/// Count whitespace-delimited tokens (proxy for LLM token count).
pub fn count_tokens(text: &str) -> usize {
    text.split_whitespace().count()
}

/// Execute a command and capture its output. Stdin is inherited so
/// pipelines like `cat file | tok0 proxy jq .field` work — `Command::
/// output()` would otherwise feed the child an empty stdin.
pub fn execute_command(cmd: &str, args: &[&str]) -> Result<Output> {
    let child = Command::new(cmd)
        .args(args)
        .stdin(std::process::Stdio::inherit())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .with_context(|| format!("Failed to spawn: {} {}", cmd, args.join(" ")))?;
    child
        .wait_with_output()
        .with_context(|| format!("Failed to wait for: {} {}", cmd, args.join(" ")))
}

/// Execute a command with stdin/stdout/stderr fully inherited from the
/// parent process. Used for TUI editors, pagers, sudo prompts, REPLs —
/// anything that needs a real TTY for rendering or password entry.
/// No compression, no metering, no capture: pure passthrough.
///
/// Returns the child's exit code (defaulting to 1 if the process was
/// terminated by signal). Errors only on spawn failure (binary not
/// found, etc.).
pub fn execute_inherit_tty(cmd: &str, args: &[&str]) -> Result<i32> {
    let status = Command::new(cmd)
        .args(args)
        .stdin(std::process::Stdio::inherit())
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit())
        .status()
        .with_context(|| format!("Failed to execute: {} {}", cmd, args.join(" ")))?;
    Ok(status.code().unwrap_or(1))
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

/// Keep enough head and tail lines to fit within `max_tokens`, allocating
/// `head_ratio` of the budget to head and the remainder to tail. Lines are
/// taken whole — partial lines are never emitted.
///
/// Unlike `head_tail`, which counts lines, this counts tokens (whitespace-
/// split words). It's the right axis when output mixes short status lines
/// with long error blocks: a 300-token failure stays intact even if it
/// would have been one of 50 lines dropped by `head_tail`.
///
/// `head_ratio` must be in `[0.0, 1.0]`. Values outside that range are
/// clamped. If the input fits inside `max_tokens`, the input is returned
/// unchanged.
pub fn head_tail_by_tokens(input: &str, max_tokens: usize, head_ratio: f32) -> String {
    let total_tokens = count_tokens(input);
    if total_tokens <= max_tokens || input.is_empty() {
        return input.to_string();
    }

    let ratio = head_ratio.clamp(0.0, 1.0);
    let head_budget = (max_tokens as f32 * ratio).round() as usize;
    let tail_budget = max_tokens.saturating_sub(head_budget);

    let lines: Vec<&str> = input.lines().collect();

    let mut head_used = 0usize;
    let mut head_end = 0usize;
    for (idx, line) in lines.iter().enumerate() {
        let line_tokens = count_tokens(line);
        if head_used + line_tokens > head_budget {
            break;
        }
        head_used += line_tokens;
        head_end = idx + 1;
    }

    let mut tail_used = 0usize;
    let mut tail_start = lines.len();
    if tail_budget > 0 {
        for (idx, line) in lines.iter().enumerate().rev() {
            let line_tokens = count_tokens(line);
            if tail_used + line_tokens > tail_budget {
                break;
            }
            tail_used += line_tokens;
            tail_start = idx;
        }
    }

    // If budgets cover everything (overlap or adjacency), return input.
    if tail_start <= head_end {
        return input.to_string();
    }

    let omitted = tail_start - head_end;
    let head_str = lines[..head_end].join("\n");
    let tail_str = lines[tail_start..].join("\n");

    if head_str.is_empty() && tail_str.is_empty() {
        // Single line longer than budget — keep it whole rather than emit
        // an empty result. Callers can hard-cap downstream if needed.
        return input.to_string();
    }
    if head_str.is_empty() {
        return format!("... ({} lines omitted) ...\n{}", omitted, tail_str);
    }
    if tail_str.is_empty() {
        return format!("{}\n... ({} lines omitted) ...", head_str, omitted);
    }
    format!(
        "{}\n... ({} lines omitted) ...\n{}",
        head_str, omitted, tail_str
    )
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

    // ── Phase 1: head_tail_by_tokens ──────────────────────────────────

    #[test]
    fn test_head_tail_by_tokens_under_budget_unchanged() {
        let input = "alpha\nbeta\ngamma";
        let result = head_tail_by_tokens(input, 100, 0.6);
        assert_eq!(result, input);
    }

    #[test]
    fn test_head_tail_by_tokens_empty_input() {
        assert_eq!(head_tail_by_tokens("", 100, 0.6), "");
    }

    #[test]
    fn test_head_tail_by_tokens_over_budget_keeps_head_and_tail() {
        // 30 single-token lines, budget 10 → 6 head + 4 tail.
        let lines: Vec<String> = (0..30).map(|i| format!("l{i}")).collect();
        let input = lines.join("\n");
        let result = head_tail_by_tokens(&input, 10, 0.6);
        assert!(result.contains("l0"));
        assert!(result.contains("l1"));
        assert!(result.contains("l29"));
        assert!(result.contains("omitted"));
        // Should NOT contain middle-ish line.
        assert!(!result.contains("l15"));
    }

    #[test]
    fn test_head_tail_by_tokens_giant_single_line_kept() {
        // One line of 1000 tokens, budget 100 — line is bigger than budget,
        // we still keep it (function falls back to "input fits" branch when
        // a single line exceeds head budget; defining behavior: keep input).
        let input = "word ".repeat(1000).trim_end().to_string();
        let result = head_tail_by_tokens(&input, 100, 0.6);
        // Either kept as-is or returned with marker — must not panic and
        // must contain at least one "word".
        assert!(result.contains("word"));
    }

    #[test]
    fn test_head_tail_by_tokens_unicode_lines() {
        let lines: Vec<String> = (0..30).map(|i| format!("行{i} 名前")).collect();
        let input = lines.join("\n");
        let result = head_tail_by_tokens(&input, 8, 0.5);
        assert!(result.contains("行0"));
        assert!(result.contains("行29"));
    }

    #[test]
    fn test_head_tail_by_tokens_ansi_input_unaffected_by_function() {
        // head_tail_by_tokens does not strip ANSI; callers do that in
        // strip_ansi first. Just verify no panic on ANSI input.
        let input = "\x1b[32mline1\x1b[0m\nline2\nline3";
        let result = head_tail_by_tokens(input, 2, 0.5);
        assert!(!result.is_empty());
    }

    #[test]
    fn test_head_tail_by_tokens_large_input_no_panic() {
        let lines: Vec<String> = (0..10_000).map(|i| format!("token{i} content")).collect();
        let input = lines.join("\n");
        let result = head_tail_by_tokens(&input, 200, 0.6);
        assert!(result.contains("token0"));
        assert!(result.contains("token9999"));
        assert!(result.contains("omitted"));
    }

    // ── Phase 1: strip_progress_noise ─────────────────────────────────

    #[test]
    fn test_strip_progress_noise_zero_copy_on_clean_input() {
        let input = "hello world\nno noise here";
        let result = strip_progress_noise(input);
        assert!(matches!(result, std::borrow::Cow::Borrowed(_)));
        assert_eq!(result.as_ref(), input);
    }

    #[test]
    fn test_strip_progress_noise_collapses_cr_rewrites() {
        // Three progress states overwritten via \r; only final remains.
        let input = "[..  ] 30%\r[....] 50%\r[......] 80%\r[........] 100%";
        let result = strip_progress_noise(input);
        assert_eq!(result.as_ref(), "[........] 100%");
    }

    #[test]
    fn test_strip_progress_noise_cr_preserves_other_lines() {
        let input = "starting build\nprogress: 10%\rprogress: 100%\nbuild done";
        let result = strip_progress_noise(input);
        let out = result.as_ref();
        assert!(out.contains("starting build"));
        assert!(out.contains("progress: 100%"));
        assert!(!out.contains("progress: 10%"));
        assert!(out.contains("build done"));
    }

    #[test]
    fn test_strip_progress_noise_strips_box_drawing_runs() {
        let input = "downloading ████████░░░░ 60%";
        let result = strip_progress_noise(input);
        let out = result.as_ref();
        assert!(out.contains("downloading"));
        assert!(out.contains("60%"));
        assert!(!out.contains('█'));
        assert!(!out.contains('░'));
    }

    #[test]
    fn test_strip_progress_noise_keeps_single_box_char() {
        // A single box-drawing char in normal text — not a progress bar.
        // Pattern requires 2+ consecutive runs to strip.
        let input = "table │ cell content";
        let result = strip_progress_noise(input);
        assert!(result.as_ref().contains('│'));
    }

    #[test]
    fn test_strip_progress_noise_empty() {
        assert_eq!(strip_progress_noise("").as_ref(), "");
    }

    #[test]
    fn test_strip_progress_noise_unicode_preserved() {
        let input = "名前 アイウ\n他のコンテンツ";
        let result = strip_progress_noise(input);
        assert!(result.as_ref().contains("名前"));
        assert!(result.as_ref().contains("アイウ"));
        assert!(result.as_ref().contains("他のコンテンツ"));
    }

    #[test]
    fn test_strip_progress_noise_npm_install_style() {
        // npm/pnpm install often emit \r-rewritten progress lines.
        let input = "[1/4] Resolving packages...\n[2/4] Fetching packages...\rdone\n[3/4] Linking dependencies...\rdone\n[4/4] Building fresh packages...\rdone\nDone in 12.3s";
        let result = strip_progress_noise(input);
        let out = result.as_ref();
        assert!(out.contains("Done in 12.3s"));
        assert!(out.contains("[1/4]"));
        // The Fetching/Linking/Building lines should have collapsed to "done"
        let done_count = out.matches("done").count();
        assert_eq!(
            done_count, 3,
            "expected 3 collapsed 'done' lines, got: {out}"
        );
    }

    #[test]
    fn test_strip_progress_noise_large_input_no_panic() {
        let input = ("progress\r".repeat(10_000)) + "final";
        let result = strip_progress_noise(&input);
        // Should collapse all \r-overwritten progress noise to just "final".
        assert!(result.as_ref().contains("final"));
    }

    #[test]
    fn test_head_tail_by_tokens_head_ratio_one_skips_tail() {
        let lines: Vec<String> = (0..20).map(|i| format!("l{i}")).collect();
        let input = lines.join("\n");
        let result = head_tail_by_tokens(&input, 5, 1.0);
        // All budget on head; tail should not be present
        assert!(result.contains("l0"));
        assert!(!result.contains("l19"));
    }
}

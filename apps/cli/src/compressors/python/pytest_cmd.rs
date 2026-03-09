use lazy_static::lazy_static;
use regex::Regex;

lazy_static! {
    /// Matches the final summary line: "X passed", "X failed", "X error", etc.
    static ref SUMMARY_RE: Regex =
        Regex::new(r"^=+\s+\d+ (passed|failed|error)").unwrap();
    /// Matches a FAILED marker line (in the short test summary section)
    static ref FAILED_MARKER_RE: Regex =
        Regex::new(r"^FAILED ").unwrap();
    /// Matches a failure section header (underscores separator + test name)
    static ref FAILURE_HEADER_RE: Regex =
        Regex::new(r"^_{5,}\s+\S").unwrap();
    /// Matches the start of the FAILURES block
    static ref FAILURES_BLOCK_RE: Regex =
        Regex::new(r"^=+\s+FAILURES\s+=+$").unwrap();
    /// Matches the short test summary info header
    static ref SHORT_SUMMARY_RE: Regex =
        Regex::new(r"^=+\s+short test summary info\s+=+$").unwrap();
    /// Matches the session header (platform, rootdir, configfile, plugins, collected)
    static ref SESSION_HEADER_RE: Regex =
        Regex::new(r"^={5,} test session starts ={5,}$|^platform |^rootdir:|^configfile:|^plugins:|^collected \d+").unwrap();
    /// Matches test progress lines (dots/letters indicating pass/fail per file)
    static ref PROGRESS_LINE_RE: Regex =
        Regex::new(r"^tests/.*\s+\[[ \d]+%\]$").unwrap();
    /// Matches noise: warnings, Captured output blocks, docs links, separator lines
    static ref NOISE_LINE_RE: Regex =
        Regex::new(r"^\s*/.*\.py:\d+:|^-- Docs:|^=+\s+warnings summary|^-{5,}|^WARNING\s|^ERROR\s+\S+:\S+\.\w+:\d+").unwrap();
    /// Matches assertion error lines (the E-prefixed lines)
    static ref ASSERTION_LINE_RE: Regex =
        Regex::new(r"^E\s+\S").unwrap();
    /// Matches the file:line: AssertionError traceback pointer
    static ref TRACEBACK_FILE_RE: Regex =
        Regex::new(r"^[A-Za-z].*\.py:\d+: \w").unwrap();
    /// Matches ">" lines showing the offending code statement
    static ref CODE_LINE_RE: Regex =
        Regex::new(r"^>\s+\S").unwrap();
    /// Matches "Captured" output section headers to skip
    static ref CAPTURED_RE: Regex =
        Regex::new(r"^-{5,}\s+Captured").unwrap();
}

/// Filter pytest output.
///
/// - All-pass: returns only the final summary line.
/// - With failures: strips session headers, progress dots, warnings, captured output,
///   and verbose tracebacks.  Keeps only the FAILED marker lines + the summary.
///   Target: 85% savings.
pub fn filter_pytest(input: &str) -> String {
    if input.trim().is_empty() {
        return String::new();
    }

    // Check if any failures exist
    let has_failures = input
        .lines()
        .any(|l| FAILED_MARKER_RE.is_match(l) || FAILURE_HEADER_RE.is_match(l));

    if !has_failures {
        // All-pass: just return the summary line
        if let Some(summary) = input.lines().find(|l| SUMMARY_RE.is_match(l)) {
            return summary.trim().to_string();
        }
        // Fallback: return last non-empty line
        if let Some(last) = input.lines().rev().find(|l| !l.trim().is_empty()) {
            return last.trim().to_string();
        }
        return input.to_string();
    }

    // With failures: build a compact report from two sources:
    // 1. The "FAILED tests/..." lines from the short summary section
    // 2. The final summary line ("N passed, M failed in Xs")
    let mut failed_markers: Vec<&str> = Vec::new();
    let mut summary_line: Option<&str> = None;
    let mut in_short_summary = false;

    for line in input.lines() {
        if SHORT_SUMMARY_RE.is_match(line) {
            in_short_summary = true;
            continue;
        }

        if SUMMARY_RE.is_match(line) {
            summary_line = Some(line);
            in_short_summary = false;
            continue;
        }

        if in_short_summary && FAILED_MARKER_RE.is_match(line) {
            failed_markers.push(line);
        }
    }

    // If we found FAILED markers use them; otherwise fall back to the full input
    if failed_markers.is_empty() && summary_line.is_none() {
        return input.to_string();
    }

    let mut lines_out: Vec<&str> = failed_markers;
    if let Some(s) = summary_line {
        lines_out.push(s);
    }

    let result = lines_out.join("\n");
    if result.trim().is_empty() {
        input.to_string()
    } else {
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use insta::assert_snapshot;

    fn count_tokens(s: &str) -> usize {
        s.split_whitespace().count()
    }

    // --- all-pass fixture ---

    #[test]
    fn test_pytest_pass_format() {
        let input = include_str!("../../../tests/fixtures/python/pytest_raw.txt");
        let output = filter_pytest(input);
        assert_snapshot!(output);
    }

    #[test]
    fn test_pytest_pass_savings() {
        let input = include_str!("../../../tests/fixtures/python/pytest_raw.txt");
        let output = filter_pytest(input);
        let input_t = count_tokens(input);
        let output_t = count_tokens(&output);
        let savings = 100.0 - (output_t as f64 / input_t as f64 * 100.0);
        assert!(
            savings >= 85.0,
            "Expected >=85% savings, got {:.1}% ({} -> {} tokens)",
            savings,
            input_t,
            output_t
        );
    }

    #[test]
    fn test_pytest_pass_only_summary() {
        let input = include_str!("../../../tests/fixtures/python/pytest_raw.txt");
        let output = filter_pytest(input);
        // All-pass output should be a single line containing the summary
        assert!(
            output.contains("passed"),
            "Should contain 'passed': {}",
            output
        );
        assert_eq!(
            output.lines().count(),
            1,
            "All-pass should collapse to 1 line"
        );
    }

    // --- failures fixture ---

    #[test]
    fn test_pytest_failures_format() {
        let input = include_str!("../../../tests/fixtures/python/pytest_failures_raw.txt");
        let output = filter_pytest(input);
        assert_snapshot!(output);
    }

    #[test]
    fn test_pytest_failures_savings() {
        let input = include_str!("../../../tests/fixtures/python/pytest_failures_raw.txt");
        let output = filter_pytest(input);
        let input_t = count_tokens(input);
        let output_t = count_tokens(&output);
        let savings = 100.0 - (output_t as f64 / input_t as f64 * 100.0);
        assert!(
            savings >= 85.0,
            "Expected >=85% savings, got {:.1}% ({} -> {} tokens)",
            savings,
            input_t,
            output_t
        );
    }

    #[test]
    fn test_pytest_failures_keeps_failed_markers() {
        let input = include_str!("../../../tests/fixtures/python/pytest_failures_raw.txt");
        let output = filter_pytest(input);
        assert!(
            output.contains("FAILED"),
            "Should keep FAILED markers: {}",
            output
        );
    }

    #[test]
    fn test_pytest_failures_keeps_assertions() {
        let input = include_str!("../../../tests/fixtures/python/pytest_failures_raw.txt");
        let output = filter_pytest(input);
        assert!(
            output.contains("AssertionError"),
            "Should keep assertion errors: {}",
            output
        );
    }

    #[test]
    fn test_pytest_failures_keeps_summary() {
        let input = include_str!("../../../tests/fixtures/python/pytest_failures_raw.txt");
        let output = filter_pytest(input);
        assert!(
            output.contains("failed"),
            "Should keep summary with 'failed': {}",
            output
        );
    }

    // --- edge cases ---

    #[test]
    fn test_pytest_empty() {
        assert_eq!(filter_pytest(""), "");
        assert_eq!(filter_pytest("   \n  "), "");
    }

    #[test]
    fn test_pytest_malformed() {
        let output = filter_pytest("not pytest output\nsome random text");
        assert!(!output.is_empty(), "Should return input on malformed data");
    }
}

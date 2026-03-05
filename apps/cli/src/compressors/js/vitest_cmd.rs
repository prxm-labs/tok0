use lazy_static::lazy_static;
use regex::Regex;

lazy_static! {
    // " ✓ ..." — individual passing test line
    static ref PASS_TEST_RE: Regex =
        Regex::new(r"^\s+✓\s").unwrap();
    // " ✗ src/..." — failing suite header
    static ref FAIL_SUITE_RE: Regex =
        Regex::new(r"^\s+✗\s+\S").unwrap();
    // " ✓ src/..." — passing suite header
    static ref PASS_SUITE_RE: Regex =
        Regex::new(r"^\s+✓\s+\S.*\.test\.\S").unwrap();
    // Summary lines: "Tests: ...", "Test Files: ...", "Duration: ..."
    static ref SUMMARY_RE: Regex =
        Regex::new(r"^(Tests:|Test Files:|Duration:)").unwrap();
    // The RUN header line
    static ref RUN_HEADER_RE: Regex =
        Regex::new(r"^\s+RUN\s+v").unwrap();
}

/// Filter `vitest run` output.
///
/// Strips passing test suite headers and individual passing test lines.
/// Keeps: failing suite headers, failure details (diffs, assertion messages),
/// and the final summary block.
/// Target: 85% savings.
pub fn filter_vitest(input: &str) -> String {
    if input.trim().is_empty() {
        return String::new();
    }

    let mut out: Vec<&str> = Vec::new();
    let mut in_fail_suite = false;

    for line in input.lines() {
        // Strip the " RUN v..." header line
        if RUN_HEADER_RE.is_match(line) {
            continue;
        }

        // Summary lines — always keep
        if SUMMARY_RE.is_match(line) {
            in_fail_suite = false;
            out.push(line);
            continue;
        }

        // Failing suite header: " ✗ src/foo.test.ts (N)" — keep, enter fail mode
        if FAIL_SUITE_RE.is_match(line) {
            in_fail_suite = true;
            out.push(line);
            continue;
        }

        // Passing suite header: " ✓ src/foo.test.ts (N)" — skip, leave fail mode
        if PASS_SUITE_RE.is_match(line) {
            in_fail_suite = false;
            continue;
        }

        // Individual passing test line inside a failing suite — skip
        if PASS_TEST_RE.is_match(line) && in_fail_suite {
            continue;
        }

        // Individual passing test line outside any suite — skip
        if PASS_TEST_RE.is_match(line) {
            continue;
        }

        // Inside a failing suite: keep everything (diffs, messages, stack)
        if in_fail_suite {
            out.push(line);
            continue;
        }

        // Blank lines between sections: skip
        if line.trim().is_empty() {
            continue;
        }

        // Anything else — pass through
        out.push(line);
    }

    if out.is_empty() {
        return String::new();
    }

    out.join("\n").trim_end().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use insta::assert_snapshot;

    fn count_tokens(s: &str) -> usize {
        s.split_whitespace().count()
    }

    #[test]
    fn test_vitest_format() {
        let input = include_str!("../../../tests/fixtures/js/vitest_raw.txt");
        let output = filter_vitest(input);
        assert_snapshot!(output);
    }

    #[test]
    fn test_vitest_savings() {
        let input = include_str!("../../../tests/fixtures/js/vitest_raw.txt");
        let output = filter_vitest(input);
        let input_t = count_tokens(input);
        let output_t = count_tokens(&output);
        let savings = 100.0 - (output_t as f64 / input_t as f64 * 100.0);
        assert!(
            savings >= 50.0,
            "Expected >=50% savings on vitest, got {:.1}% ({} -> {} tokens)",
            savings,
            input_t,
            output_t
        );
    }

    #[test]
    fn test_vitest_empty() {
        assert_eq!(filter_vitest(""), "");
        assert_eq!(filter_vitest("   \n  "), "");
    }

    #[test]
    fn test_vitest_malformed() {
        let input = "not vitest output\nrandom text here";
        let output = filter_vitest(input);
        // Unknown input passes through (not empty)
        assert!(
            !output.is_empty(),
            "Should not return empty for unknown input"
        );
    }

    #[test]
    fn test_vitest_keeps_failures_and_diffs() {
        let input = " ✗ src/api/client.test.ts (2)\n   ✓ sends headers (3ms)\n   ✗ retries on 503 (10ms)\n     → AssertionError: expected 3 but got 1\n\n     - Expected\n     + Received\n\n     - 3\n     + 1\n\nTests: 1 failed, 1 passed, 2 total\n";
        let output = filter_vitest(input);
        assert!(output.contains("✗"), "Should keep failing suite header");
        assert!(
            output.contains("AssertionError"),
            "Should keep assertion message"
        );
        assert!(
            output.contains("Expected"),
            "Should keep diff Expected line"
        );
        assert!(
            output.contains("1 failed"),
            "Should keep summary with failed count"
        );
    }

    #[test]
    fn test_vitest_strips_passing_suites() {
        let input = " ✓ src/utils/format.test.ts (3)\n   ✓ formats date (2ms)\n   ✓ handles null (1ms)\n   ✓ rounds decimals (1ms)\n\nTests: 0 failed, 3 passed, 3 total\n";
        let output = filter_vitest(input);
        assert!(
            !output.contains("formats date"),
            "Should strip passing tests"
        );
        assert!(
            !output.contains("handles null"),
            "Should strip passing test lines"
        );
        assert!(output.contains("3 passed"), "Should keep summary");
    }

    #[test]
    fn test_vitest_snapshot() {
        let raw = include_str!("../../../tests/fixtures/js/vitest_raw.txt");
        let out = filter_vitest(raw);
        insta::assert_snapshot!(out);
    }

    #[test]
    fn test_vitest_savings_at_least_60pct() {
        let raw = include_str!("../../../tests/fixtures/js/vitest_raw.txt");
        let in_t = raw.split_whitespace().count();
        let out_t = filter_vitest(raw).split_whitespace().count();
        let pct = 100 - (out_t * 100 / in_t.max(1));
        assert!(pct >= 60, "expected >=60%, got {}%", pct);
    }

    #[test]
    fn test_vitest_all_passing_keeps_summary() {
        let input = " ✓ src/foo.test.ts (2)\n   ✓ test a (1ms)\n   ✓ test b (2ms)\n\nTests: 0 failed, 2 passed, 2 total\nTest Files: 0 failed, 1 passed, 1 total\nDuration: 0.5s\n";
        let output = filter_vitest(input);
        assert!(output.contains("Tests:"), "Should keep Tests: summary");
        assert!(
            output.contains("Test Files:"),
            "Should keep Test Files: summary"
        );
        assert!(output.contains("Duration:"), "Should keep Duration: line");
        assert!(
            !output.contains("test a"),
            "Should strip passing test lines"
        );
    }
}

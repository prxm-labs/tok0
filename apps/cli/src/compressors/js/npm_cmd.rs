use lazy_static::lazy_static;
use regex::Regex;

lazy_static! {
    // npm install: lines to strip
    static ref PROGRESS_RE: Regex = Regex::new(r"^[⠙⠹⠸⠼⠴⠦⠧⠇⠏⠋]").unwrap();
    static ref REIFY_RE: Regex = Regex::new(r"^\s*(reify:|timing\s)").unwrap();
    static ref DEPRECATED_WARN_RE: Regex =
        Regex::new(r"^npm warn deprecated ").unwrap();
    static ref PEER_WARN_RE: Regex =
        Regex::new(r"^npm warn peer ").unwrap();
    static ref FUNDING_RE: Regex =
        Regex::new(r"^(\d+ packages? are looking for funding|  run `npm fund`)").unwrap();
    static ref TIMING_RE: Regex =
        Regex::new(r"^npm timing ").unwrap();
    static ref ADDED_RE: Regex =
        Regex::new(r"^added \d+ packages?").unwrap();
    static ref AUDIT_VULN_RE: Regex =
        Regex::new(r"^\d+ vulnerabilit").unwrap();
    static ref AUDIT_FIX_RE: Regex =
        Regex::new(r"^\s*(To address|Run `npm audit|npm audit fix)").unwrap();
    static ref AUDITED_RE: Regex =
        Regex::new(r"^(\d+ packages?, and audited|and audited)").unwrap();

    // npm test: lines to keep / strip
    static ref TEST_PASS_LINE_RE: Regex =
        Regex::new(r"^\s+✓ ").unwrap();
    static ref TEST_SUITE_PASS_RE: Regex =
        Regex::new(r"^ PASS  ").unwrap();
    static ref TEST_SUITE_FAIL_RE: Regex =
        Regex::new(r"^ FAIL  ").unwrap();
    static ref TEST_SUMMARY_RE: Regex =
        Regex::new(r"^(Test Suites:|Tests:|Snapshots:|Time:|Ran all)").unwrap();
    static ref TEST_SCRIPT_RE: Regex =
        Regex::new(r"^> .+ test$").unwrap();
    static ref TEST_RUNNER_RE: Regex =
        Regex::new(r"^> jest").unwrap();
}

/// Filter `npm install` / `npm ci` output.
/// Keeps: summary line, audit vuln counts, high-sev guidance.
/// Strips: progress spinners, deprecation warnings, funding blurbs, timing.
pub fn filter_npm_install(input: &str) -> String {
    if input.trim().is_empty() {
        return String::new();
    }

    let mut out: Vec<&str> = Vec::new();

    for line in input.lines() {
        // Always strip pure noise
        if PROGRESS_RE.is_match(line)
            || REIFY_RE.is_match(line)
            || DEPRECATED_WARN_RE.is_match(line)
            || TIMING_RE.is_match(line)
            || FUNDING_RE.is_match(line)
        {
            continue;
        }

        // Keep important lines
        if ADDED_RE.is_match(line)
            || AUDIT_VULN_RE.is_match(line)
            || AUDIT_FIX_RE.is_match(line)
            || AUDITED_RE.is_match(line)
            || PEER_WARN_RE.is_match(line)
        {
            out.push(line);
            continue;
        }

        // Skip blank lines — they're just spacing around stripped content
        if line.trim().is_empty() {
            continue;
        }

        // Pass through anything else (e.g. real errors)
        out.push(line);
    }

    if out.is_empty() {
        return String::new();
    }

    out.join("\n").trim_end().to_string()
}

/// Filter `npm test` / `npm run test` output.
/// Keeps: FAIL suite headers, failure details, summary block.
/// Strips: PASS suite headers, individual passing test lines.
pub fn filter_npm_test(input: &str) -> String {
    if input.trim().is_empty() {
        return String::new();
    }

    let mut out: Vec<&str> = Vec::new();
    let mut in_failure_block = false;

    for line in input.lines() {
        // Strip script invocation noise
        if TEST_SCRIPT_RE.is_match(line) || TEST_RUNNER_RE.is_match(line) {
            continue;
        }

        // Strip individual passing test lines
        if TEST_PASS_LINE_RE.is_match(line) {
            continue;
        }

        // FAIL suite lines — keep and enter failure mode
        if TEST_SUITE_FAIL_RE.is_match(line) {
            in_failure_block = true;
            out.push(line);
            continue;
        }

        // PASS suite lines — collapse to nothing (saved by summary)
        if TEST_SUITE_PASS_RE.is_match(line) {
            in_failure_block = false;
            continue;
        }

        // Summary block always kept
        if TEST_SUMMARY_RE.is_match(line) {
            in_failure_block = false;
            out.push(line);
            continue;
        }

        // Inside failure block: keep everything (error messages, stack traces)
        if in_failure_block {
            out.push(line);
            continue;
        }

        // Blank lines between sections: skip
        if line.trim().is_empty() {
            continue;
        }

        // Anything else (runner header, etc.) — pass through
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

    // ── npm install tests ──────────────────────────────────────────────────

    #[test]
    fn test_npm_install_format() {
        let input = include_str!("../../../tests/fixtures/js/npm_install_raw.txt");
        let output = filter_npm_install(input);
        assert_snapshot!(output);
    }

    #[test]
    fn test_npm_install_savings() {
        let input = include_str!("../../../tests/fixtures/js/npm_install_raw.txt");
        let output = filter_npm_install(input);
        let input_t = count_tokens(input);
        let output_t = count_tokens(&output);
        let savings = 100.0 - (output_t as f64 / input_t as f64 * 100.0);
        assert!(
            savings >= 70.0,
            "Expected >=70% savings on npm install, got {:.1}% ({} -> {} tokens)",
            savings,
            input_t,
            output_t
        );
    }

    #[test]
    fn test_npm_install_empty() {
        assert_eq!(filter_npm_install(""), "");
        assert_eq!(filter_npm_install("   \n  "), "");
    }

    #[test]
    fn test_npm_install_malformed() {
        // Non-npm output should pass through unchanged (no crash)
        let input = "some random text\nnot npm output at all";
        let output = filter_npm_install(input);
        assert!(
            !output.is_empty(),
            "Should not return empty for unknown input"
        );
    }

    #[test]
    fn test_npm_install_snapshot() {
        let raw = include_str!("../../../tests/fixtures/js/npm_install_raw.txt");
        let out = filter_npm_install(raw);
        insta::assert_snapshot!(out);
    }

    #[test]
    fn test_npm_install_savings_at_least_60pct() {
        let raw = include_str!("../../../tests/fixtures/js/npm_install_raw.txt");
        let in_t = raw.split_whitespace().count();
        let out_t = filter_npm_install(raw).split_whitespace().count();
        let pct = 100 - (out_t * 100 / in_t.max(1));
        assert!(pct >= 60, "expected >=60%, got {}%", pct);
    }

    #[test]
    fn test_npm_install_keeps_added_summary() {
        let input = "npm warn deprecated foo@1.0.0: deprecated\nadded 42 packages, and audited 43 packages in 5s\n";
        let output = filter_npm_install(input);
        assert!(
            output.contains("added 42 packages"),
            "Should keep summary line"
        );
        assert!(
            !output.contains("deprecated"),
            "Should strip deprecation warning"
        );
    }

    #[test]
    fn test_npm_install_keeps_audit_vulns() {
        let input = "added 10 packages, and audited 11 packages in 1s\n\n8 vulnerabilities (3 low, 2 moderate, 3 high)\n\n  To address issues that do not require attention, run:\n    npm audit fix\n";
        let output = filter_npm_install(input);
        assert!(output.contains("vulnerabilit"), "Should keep vuln count");
    }

    #[test]
    fn test_npm_install_strips_progress_spinners() {
        let input = "⠙ reify:lodash: http fetch GET 200 ...\nadded 5 packages in 1s\n";
        let output = filter_npm_install(input);
        assert!(
            !output.contains("reify:"),
            "Should strip spinner/reify lines"
        );
        assert!(output.contains("added 5 packages"));
    }

    // ── npm test tests ─────────────────────────────────────────────────────

    #[test]
    fn test_npm_test_format() {
        let input = include_str!("../../../tests/fixtures/js/npm_test_raw.txt");
        let output = filter_npm_test(input);
        assert_snapshot!(output);
    }

    #[test]
    fn test_npm_test_savings() {
        let input = include_str!("../../../tests/fixtures/js/npm_test_raw.txt");
        let output = filter_npm_test(input);
        let input_t = count_tokens(input);
        let output_t = count_tokens(&output);
        let savings = 100.0 - (output_t as f64 / input_t as f64 * 100.0);
        assert!(
            savings >= 45.0,
            "Expected >=45% savings on npm test, got {:.1}% ({} -> {} tokens)",
            savings,
            input_t,
            output_t
        );
    }

    #[test]
    fn test_npm_test_empty() {
        assert_eq!(filter_npm_test(""), "");
        assert_eq!(filter_npm_test("  \n  "), "");
    }

    #[test]
    fn test_npm_test_malformed() {
        let input = "not jest output\nrandom text here";
        let output = filter_npm_test(input);
        assert!(
            !output.is_empty(),
            "Should not return empty for unknown input"
        );
    }

    #[test]
    fn test_npm_test_keeps_failures() {
        let input = " FAIL  src/foo.test.ts\n  ● foo › fails\n\n    Expected: 1\n    Received: 0\n\nTests: 1 failed, 0 passed, 1 total\n";
        let output = filter_npm_test(input);
        assert!(output.contains("FAIL"), "Should keep FAIL suite header");
        assert!(
            output.contains("Expected: 1"),
            "Should keep failure details"
        );
        assert!(output.contains("1 failed"), "Should keep test summary");
    }

    #[test]
    fn test_npm_test_strips_passing_lines() {
        let input = " PASS  src/foo.test.ts\n  foo\n    ✓ passes (5 ms)\n    ✓ also passes (3 ms)\n\nTests: 0 failed, 2 passed, 2 total\n";
        let output = filter_npm_test(input);
        assert!(!output.contains("✓"), "Should strip passing test lines");
        assert!(!output.contains("PASS "), "Should strip PASS suite header");
        assert!(output.contains("2 passed"), "Should keep summary");
    }

    #[test]
    fn test_npm_test_all_passing() {
        let input = " PASS  src/a.test.ts\n    ✓ test one (2 ms)\n\nTest Suites: 0 failed, 1 passed, 1 total\nTests:       0 failed, 1 passed, 1 total\nTime:        1.234 s\n";
        let output = filter_npm_test(input);
        assert!(output.contains("Test Suites:"), "Should keep summary block");
        assert!(output.contains("Tests:"), "Should keep tests count");
        assert!(output.contains("Time:"), "Should keep timing in summary");
    }
}

use lazy_static::lazy_static;
use regex::Regex;

lazy_static! {
    // go test patterns
    static ref GO_TEST_PASS_RE: Regex =
        Regex::new(r"^--- PASS:").unwrap();
    static ref GO_TEST_FAIL_RE: Regex =
        Regex::new(r"^--- FAIL:").unwrap();
    static ref GO_TEST_RUN_RE: Regex =
        Regex::new(r"^=== RUN\s+").unwrap();
    static ref GO_PKG_OK_RE: Regex =
        Regex::new(r"^ok\s+\S").unwrap();
    static ref GO_PKG_FAIL_RE: Regex =
        Regex::new(r"^FAIL\s").unwrap();
    static ref GO_FINAL_FAIL_RE: Regex =
        Regex::new(r"^FAIL\s*$").unwrap();
    static ref GO_FINAL_OK_RE: Regex =
        Regex::new(r"^ok\s*$").unwrap();

    // go build patterns
    static ref GO_BUILD_PKG_HEADER_RE: Regex =
        Regex::new(r"^#\s+\S").unwrap();
    static ref GO_BUILD_DOWNLOAD_RE: Regex =
        Regex::new(r"^go:\s+(downloading|downloaded|verifying|finding)\s").unwrap();
    static ref GO_BUILD_ERROR_RE: Regex =
        Regex::new(r"^\S.*\.go:\d+:\d*:").unwrap();
    static ref GO_BUILD_CONTINUATION_RE: Regex =
        Regex::new(r"^\t").unwrap();

    // golangci-lint patterns
    static ref LINT_FINDING_RE: Regex =
        Regex::new(r"^\S.*\.go:\d+:\d+:").unwrap();
    static ref LINT_NOISE_RE: Regex =
        Regex::new(r"^(Issues found:|Linters used:|Run with|WARN|level=)").unwrap();
}

/// Filter `go test ./...` output.
///
/// Strategy:
/// - Strip all `=== RUN` and `--- PASS` lines (noise when passing).
/// - Keep `--- FAIL` lines with their indented detail lines.
/// - Keep per-package `ok` and `FAIL` summary lines.
/// - Keep the final bare `FAIL` or `ok` exit summary.
///
/// Target: 80% token savings on typical all-pass runs.
pub fn filter_go_test(input: &str) -> String {
    if input.trim().is_empty() {
        return String::new();
    }

    // Detect whether this looks like go test output at all
    let looks_like_go_test = input.lines().any(|l| {
        GO_PKG_OK_RE.is_match(l)
            || GO_PKG_FAIL_RE.is_match(l)
            || GO_TEST_RUN_RE.is_match(l)
            || GO_TEST_PASS_RE.is_match(l)
            || GO_TEST_FAIL_RE.is_match(l)
    });

    if !looks_like_go_test {
        return input.to_string();
    }

    let mut output_lines: Vec<&str> = Vec::new();
    let mut in_fail_detail = false;

    for line in input.lines() {
        if GO_TEST_FAIL_RE.is_match(line) {
            // Start of a failing test — keep the FAIL header
            in_fail_detail = true;
            output_lines.push(line);
        } else if GO_TEST_PASS_RE.is_match(line) || GO_TEST_RUN_RE.is_match(line) {
            // Strip passing test lines and RUN lines
            in_fail_detail = false;
        } else if GO_PKG_OK_RE.is_match(line) || GO_PKG_FAIL_RE.is_match(line) {
            // Per-package summary — always keep
            in_fail_detail = false;
            output_lines.push(line);
        } else if GO_FINAL_FAIL_RE.is_match(line) || GO_FINAL_OK_RE.is_match(line) {
            // Final bare FAIL/ok exit line
            output_lines.push(line);
        } else if in_fail_detail {
            // Indented detail lines following a FAIL header
            output_lines.push(line);
        }
        // Everything else (=== CONT, cached lines already handled, etc.) — skip
    }

    // Remove trailing blank lines
    while output_lines
        .last()
        .is_some_and(|l: &&str| l.trim().is_empty())
    {
        output_lines.pop();
    }

    if output_lines.is_empty() {
        return input.to_string();
    }

    output_lines.join("\n")
}

/// Filter `go build` output.
///
/// Strategy:
/// - Strip `# package` header lines (noise; the errors below identify the package).
/// - Keep all error lines matching `file.go:line:col: message`.
/// - Keep continuation lines (tab-indented context for multi-line errors).
///
/// Target: 70% token savings.
pub fn filter_go_build(input: &str) -> String {
    if input.trim().is_empty() {
        return String::new();
    }

    let looks_like_go_build = input.lines().any(|l| {
        GO_BUILD_PKG_HEADER_RE.is_match(l)
            || GO_BUILD_ERROR_RE.is_match(l)
            || GO_BUILD_DOWNLOAD_RE.is_match(l)
    });

    if !looks_like_go_build {
        return input.to_string();
    }

    let mut output_lines: Vec<&str> = Vec::new();
    let mut prev_was_error = false;

    for line in input.lines() {
        if GO_BUILD_PKG_HEADER_RE.is_match(line) || GO_BUILD_DOWNLOAD_RE.is_match(line) {
            // Strip "# package" headers and "go: downloading" lines
            prev_was_error = false;
        } else if GO_BUILD_ERROR_RE.is_match(line) {
            output_lines.push(line);
            prev_was_error = true;
        } else if prev_was_error && GO_BUILD_CONTINUATION_RE.is_match(line) {
            // Tab-indented continuation of a multi-line error
            output_lines.push(line);
        } else {
            prev_was_error = false;
        }
    }

    // Remove trailing blank lines
    while output_lines
        .last()
        .is_some_and(|l: &&str| l.trim().is_empty())
    {
        output_lines.pop();
    }

    if output_lines.is_empty() {
        return input.to_string();
    }

    output_lines.join("\n")
}

/// Filter `golangci-lint run` output.
///
/// Strategy:
/// - Keep individual linter finding lines (`file:line:col: message (linter)`).
/// - Strip summary counts, WARN lines, "Run with --fix" footer, blank lines.
///
/// Target: 75% token savings.
pub fn filter_golangci_lint(input: &str) -> String {
    if input.trim().is_empty() {
        return String::new();
    }

    let looks_like_lint = input
        .lines()
        .any(|l| LINT_FINDING_RE.is_match(l) || LINT_NOISE_RE.is_match(l));

    if !looks_like_lint {
        return input.to_string();
    }

    let mut output_lines: Vec<&str> = Vec::new();

    for line in input.lines() {
        if LINT_FINDING_RE.is_match(line) {
            output_lines.push(line);
        }
        // Strip: summary counts, WARN, "Run with", blank lines
    }

    // Remove trailing blank lines
    while output_lines
        .last()
        .is_some_and(|l: &&str| l.trim().is_empty())
    {
        output_lines.pop();
    }

    if output_lines.is_empty() {
        return input.to_string();
    }

    output_lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use insta::assert_snapshot;

    fn count_tokens(s: &str) -> usize {
        s.split_whitespace().count()
    }

    // ── go test ───────────────────────────────────────────────────────────────

    #[test]
    fn test_go_test_format() {
        let input = include_str!("../../../tests/fixtures/go/go_test_raw.txt");
        let output = filter_go_test(input);
        assert_snapshot!(output);
    }

    #[test]
    fn test_go_test_savings() {
        let input = include_str!("../../../tests/fixtures/go/go_test_raw.txt");
        let output = filter_go_test(input);
        let input_t = count_tokens(input);
        let output_t = count_tokens(&output);
        let savings = 100.0 - (output_t as f64 / input_t as f64 * 100.0);
        assert!(
            savings >= 80.0,
            "Expected >=80% savings, got {:.1}% ({} -> {})",
            savings,
            input_t,
            output_t
        );
    }

    #[test]
    fn test_go_test_empty() {
        assert_eq!(filter_go_test(""), "");
        assert_eq!(filter_go_test("   \n  "), "");
    }

    #[test]
    fn test_go_test_malformed() {
        let output = filter_go_test("not go test output at all");
        assert!(!output.is_empty());
    }

    #[test]
    fn test_go_test_strips_pass_lines() {
        let input = concat!(
            "=== RUN   TestFoo\n",
            "--- PASS: TestFoo (0.00s)\n",
            "=== RUN   TestBar\n",
            "--- PASS: TestBar (0.00s)\n",
            "ok  \tgithub.com/user/project/pkg/foo\t0.002s\n",
        );
        let output = filter_go_test(input);
        assert!(!output.contains("--- PASS"));
        assert!(!output.contains("=== RUN"));
        assert!(output.contains("ok"));
    }

    #[test]
    fn test_go_test_keeps_fail_details() {
        let input = concat!(
            "=== RUN   TestFoo\n",
            "--- FAIL: TestFoo (0.01s)\n",
            "    foo_test.go:42: expected 1, got 2\n",
            "FAIL\tgithub.com/user/project/pkg/foo\t0.012s\n",
            "FAIL\n",
        );
        let output = filter_go_test(input);
        assert!(output.contains("--- FAIL: TestFoo"));
        assert!(output.contains("foo_test.go:42: expected 1, got 2"));
        assert!(output.contains("FAIL\tgithub.com/user/project/pkg/foo"));
    }

    // ── go build ──────────────────────────────────────────────────────────────

    #[test]
    fn test_go_build_format() {
        let input = include_str!("../../../tests/fixtures/go/go_build_raw.txt");
        let output = filter_go_build(input);
        assert_snapshot!(output);
    }

    #[test]
    fn test_go_build_savings() {
        let input = include_str!("../../../tests/fixtures/go/go_build_raw.txt");
        let output = filter_go_build(input);
        let input_t = count_tokens(input);
        let output_t = count_tokens(&output);
        let savings = 100.0 - (output_t as f64 / input_t as f64 * 100.0);
        assert!(
            savings >= 70.0,
            "Expected >=70% savings, got {:.1}% ({} -> {})",
            savings,
            input_t,
            output_t
        );
    }

    #[test]
    fn test_go_build_empty() {
        assert_eq!(filter_go_build(""), "");
        assert_eq!(filter_go_build("   \n  "), "");
    }

    #[test]
    fn test_go_build_malformed() {
        let output = filter_go_build("not go build output at all");
        assert!(!output.is_empty());
    }

    #[test]
    fn test_go_build_strips_pkg_headers() {
        let input = concat!(
            "# github.com/user/project/pkg/handler\n",
            "pkg/handler/handler.go:23:14: undefined: Foo\n",
            "# github.com/user/project/pkg/db\n",
            "pkg/db/connection.go:18:2: imported and not used: \"fmt\"\n",
        );
        let output = filter_go_build(input);
        assert!(!output.contains("# github.com"));
        assert!(output.contains("handler.go:23:14"));
        assert!(output.contains("connection.go:18:2"));
    }

    #[test]
    fn test_go_build_keeps_continuation_lines() {
        let input = concat!(
            "# github.com/user/project/pkg/handler\n",
            "pkg/handler/handler.go:67:9: too many arguments in call to w.WriteHeader\n",
            "\thave (int, string)\n",
            "\twant (int)\n",
        );
        let output = filter_go_build(input);
        assert!(output.contains("too many arguments"));
        assert!(output.contains("\thave (int, string)"));
        assert!(output.contains("\twant (int)"));
    }

    // ── golangci-lint ─────────────────────────────────────────────────────────

    #[test]
    fn test_golangci_lint_format() {
        let input = include_str!("../../../tests/fixtures/go/golangci_lint_raw.txt");
        let output = filter_golangci_lint(input);
        assert_snapshot!(output);
    }

    #[test]
    fn test_golangci_lint_savings() {
        let input = include_str!("../../../tests/fixtures/go/golangci_lint_raw.txt");
        let output = filter_golangci_lint(input);
        let input_t = count_tokens(input);
        let output_t = count_tokens(&output);
        let savings = 100.0 - (output_t as f64 / input_t as f64 * 100.0);
        assert!(
            savings >= 75.0,
            "Expected >=75% savings, got {:.1}% ({} -> {})",
            savings,
            input_t,
            output_t
        );
    }

    #[test]
    fn test_golangci_lint_empty() {
        assert_eq!(filter_golangci_lint(""), "");
        assert_eq!(filter_golangci_lint("   \n  "), "");
    }

    #[test]
    fn test_golangci_lint_malformed() {
        let output = filter_golangci_lint("not lint output at all");
        assert!(!output.is_empty());
    }

    #[test]
    fn test_golangci_lint_strips_summary() {
        let input = concat!(
            "pkg/handler/handler.go:15:2: \"net/http\" imported and not used (goimports)\n",
            "pkg/user/service.go:45:15: cognitive complexity 24 is high (gocognit)\n",
            "\n",
            "Run with --fix to apply automatic fixes.\n",
            "\n",
            "Issues found: 2\n",
            "Linters used: goimports(1), gocognit(1)\n",
        );
        let output = filter_golangci_lint(input);
        assert!(output.contains("handler.go:15:2"));
        assert!(output.contains("service.go:45:15"));
        assert!(!output.contains("Run with --fix"));
        assert!(!output.contains("Issues found:"));
        assert!(!output.contains("Linters used:"));
    }

    #[test]
    fn test_golangci_lint_keeps_all_findings() {
        let input = concat!(
            "pkg/a/a.go:1:1: first finding (lintA)\n",
            "pkg/b/b.go:2:2: second finding (lintB)\n",
            "pkg/c/c.go:3:3: third finding (lintC)\n",
            "Issues found: 3\n",
        );
        let output = filter_golangci_lint(input);
        assert!(output.contains("a.go:1:1"));
        assert!(output.contains("b.go:2:2"));
        assert!(output.contains("c.go:3:3"));
        assert_eq!(output.lines().count(), 3);
    }
}

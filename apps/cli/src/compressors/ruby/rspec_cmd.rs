use lazy_static::lazy_static;
use regex::Regex;

lazy_static! {
    /// Matches the progress line of dots and F's (e.g. ".......F.........F..")
    static ref PROGRESS_RE: Regex =
        Regex::new(r"^[\.FPE\*]+$").unwrap();
    /// Matches rspec example description lines (indented with spaces)
    static ref DESCRIPTION_RE: Regex =
        Regex::new(r"^  ").unwrap();
    /// Matches a failure block header (numbered, e.g. "  1) Some description")
    static ref FAILURE_HEADER_RE: Regex =
        Regex::new(r"^\s+\d+\) ").unwrap();
    /// Matches the summary line e.g. "20 examples, 2 failures"
    static ref SUMMARY_RE: Regex =
        Regex::new(r"^\d+ examples?,").unwrap();
    /// Matches the "Failed examples:" section header
    static ref FAILED_EXAMPLES_RE: Regex =
        Regex::new(r"^Failed examples:").unwrap();
    /// Matches rspec re-run lines (e.g. "rspec ./spec/... # ...")
    static ref RSPEC_LINE_RE: Regex =
        Regex::new(r"^rspec ").unwrap();
    /// Matches "Finished in X seconds" line
    static ref FINISHED_RE: Regex =
        Regex::new(r"^Finished in ").unwrap();
    /// Matches "Randomized with seed" line
    static ref SEED_RE: Regex =
        Regex::new(r"^Randomized with seed").unwrap();
    /// Matches expected/got diff lines inside a failure block
    static ref DIFF_RE: Regex =
        Regex::new(r"^\s+(expected:|got:|Failure/Error:|expected \d|# \.)").unwrap();
    /// Matches the "Failures:" section header
    static ref FAILURES_SECTION_RE: Regex =
        Regex::new(r"^Failures:$").unwrap();
}

/// Filter rspec output.
///
/// - All-pass: returns only the summary line.
/// - With failures: strips passing dots/descriptions, keeps failure blocks + summary.
///   Target: 80% savings.
pub fn filter_rspec(input: &str) -> String {
    if input.trim().is_empty() {
        return String::new();
    }

    // Detect if there are failures
    let has_failures = input
        .lines()
        .any(|l| FAILURE_HEADER_RE.is_match(l) || FAILURES_SECTION_RE.is_match(l));

    if !has_failures {
        // All-pass: return summary line only
        if let Some(summary) = input.lines().find(|l| SUMMARY_RE.is_match(l)) {
            return summary.trim().to_string();
        }
        // Fallback: last non-empty line
        if let Some(last) = input.lines().rev().find(|l| !l.trim().is_empty()) {
            return last.trim().to_string();
        }
        return input.to_string();
    }

    // With failures: keep failure blocks + summary + failed examples section
    let mut output_lines: Vec<&str> = Vec::new();
    let mut in_failure_block = false;
    let mut in_failed_examples = false;

    for line in input.lines() {
        if FAILURES_SECTION_RE.is_match(line) {
            in_failure_block = true;
            output_lines.push(line);
            continue;
        }

        if FAILED_EXAMPLES_RE.is_match(line) {
            in_failure_block = false;
            in_failed_examples = true;
            output_lines.push(line);
            continue;
        }

        if SUMMARY_RE.is_match(line) {
            in_failure_block = false;
            in_failed_examples = false;
            output_lines.push(line);
            continue;
        }

        if in_failure_block {
            output_lines.push(line);
            continue;
        }

        if in_failed_examples && RSPEC_LINE_RE.is_match(line) {
            output_lines.push(line);
            continue;
        }

        // Skip: seed line, progress dots, description lines, Finished line
        // (everything outside failure/summary sections is noise)
    }

    // Remove leading/trailing blank lines
    while output_lines.first().is_some_and(|l| l.is_empty()) {
        output_lines.remove(0);
    }
    while output_lines.last().is_some_and(|l| l.is_empty()) {
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

    #[test]
    fn test_rspec_failures_format() {
        let input = include_str!("../../../tests/fixtures/ruby/rspec_raw.txt");
        let output = filter_rspec(input);
        assert_snapshot!(output);
    }

    #[test]
    fn test_rspec_failures_savings() {
        let input = include_str!("../../../tests/fixtures/ruby/rspec_raw.txt");
        let output = filter_rspec(input);
        let input_t = count_tokens(input);
        let output_t = count_tokens(&output);
        let savings = 100.0 - (output_t as f64 / input_t as f64 * 100.0);
        assert!(
            savings >= 80.0,
            "Expected >=80% savings, got {:.1}% ({} -> {} tokens)",
            savings,
            input_t,
            output_t
        );
    }

    #[test]
    fn test_rspec_failures_keeps_failure_blocks() {
        let input = include_str!("../../../tests/fixtures/ruby/rspec_raw.txt");
        let output = filter_rspec(input);
        assert!(
            output.contains("expected: 99.99"),
            "Should keep expected/got diff: {}",
            output
        );
        assert!(
            output.contains("got: 89.99"),
            "Should keep got value: {}",
            output
        );
    }

    #[test]
    fn test_rspec_failures_keeps_summary() {
        let input = include_str!("../../../tests/fixtures/ruby/rspec_raw.txt");
        let output = filter_rspec(input);
        assert!(
            output.contains("examples, 2 failures"),
            "Should keep summary: {}",
            output
        );
    }

    #[test]
    fn test_rspec_failures_strips_descriptions() {
        let input = include_str!("../../../tests/fixtures/ruby/rspec_raw.txt");
        let output = filter_rspec(input);
        // The passing description lines like "  logs in successfully" should be stripped
        assert!(
            !output.contains("logs in successfully"),
            "Should strip passing descriptions: {}",
            output
        );
        assert!(
            !output.contains("creates the order"),
            "Should strip passing descriptions: {}",
            output
        );
    }

    #[test]
    fn test_rspec_all_pass() {
        let input = concat!(
            "Randomized with seed 99999\n",
            "\n",
            "UserService\n",
            "  creates a user\n",
            "  finds a user\n",
            "\n",
            "...\n",
            "\n",
            "Finished in 0.12345 seconds (files took 0.5 seconds to load)\n",
            "3 examples, 0 failures\n"
        );
        let output = filter_rspec(input);
        assert_eq!(output, "3 examples, 0 failures");
    }

    #[test]
    fn test_rspec_empty() {
        assert_eq!(filter_rspec(""), "");
        assert_eq!(filter_rspec("   \n  "), "");
    }

    #[test]
    fn test_rspec_malformed() {
        let output = filter_rspec("not rspec output\nsome random text");
        assert!(!output.is_empty(), "Should return input on malformed data");
    }
}

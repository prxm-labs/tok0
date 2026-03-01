use lazy_static::lazy_static;
use regex::Regex;

lazy_static! {
    // cargo test patterns
    static ref TEST_RESULT_RE: Regex =
        Regex::new(r"^test result:").unwrap();
    static ref TEST_PASS_RE: Regex =
        Regex::new(r"^test .+ \.\.\. ok$").unwrap();
    static ref TEST_FAIL_RE: Regex =
        Regex::new(r"^test .+ \.\.\. FAILED$").unwrap();
    static ref TEST_IGNORED_RE: Regex =
        Regex::new(r"^test .+ \.\.\. ignored$").unwrap();

    // cargo build patterns
    static ref COMPILING_RE: Regex =
        Regex::new(r"^\s*Compiling ").unwrap();
    static ref DOWNLOADING_RE: Regex =
        Regex::new(r"^\s*(Downloading|Downloaded|Fetching|Fetch) ").unwrap();
    static ref FINISHED_RE: Regex =
        Regex::new(r"^\s*Finished ").unwrap();
    static ref ERROR_RE: Regex =
        Regex::new(r"^error").unwrap();
    static ref WARNING_LINE_RE: Regex =
        Regex::new(r"^warning:").unwrap();
    static ref ARROW_RE: Regex =
        Regex::new(r"^\s+-->").unwrap();
    static ref PIPE_RE: Regex =
        Regex::new(r"^\s+\|").unwrap();
    static ref NOTE_RE: Regex =
        Regex::new(r"^\s+= (note|help):").unwrap();
    static ref BLANK_RE: Regex =
        Regex::new(r"^\s*$").unwrap();

    // cargo clippy patterns
    static ref CHECKING_RE: Regex =
        Regex::new(r"^\s*Checking ").unwrap();
    static ref GENERATED_WARNINGS_RE: Regex =
        Regex::new(r"^warning: `\S+` \((bin|lib|test) ").unwrap();
    static ref ERROR_FULL_RE: Regex =
        Regex::new(r"^error(\[E\d+\])?:").unwrap();
}

/// Filter cargo test output: keep only summary lines and failing tests with output.
pub fn filter_cargo_test(input: &str) -> String {
    if input.trim().is_empty() {
        return String::new();
    }

    let mut output_lines: Vec<&str> = Vec::new();
    let mut in_failure_output = false;
    let mut found_any_result = false;

    for line in input.lines() {
        if TEST_RESULT_RE.is_match(line) {
            found_any_result = true;
            // Add blank line separator before result if we have content
            if !output_lines.is_empty() && !output_lines.last().is_some_and(|l: &&str| l.is_empty())
            {
                output_lines.push("");
            }
            output_lines.push(line);
            in_failure_output = false;
        } else if TEST_FAIL_RE.is_match(line) {
            output_lines.push(line);
        } else if TEST_PASS_RE.is_match(line) || TEST_IGNORED_RE.is_match(line) {
            // skip passing/ignored test lines
        } else if line.starts_with("failures:") || line.starts_with("---- ") {
            // Start of failure output section
            in_failure_output = true;
            output_lines.push(line);
        } else if in_failure_output {
            output_lines.push(line);
        } else if line.starts_with("running ") {
            // Keep "running N tests" summary
            output_lines.push(line);
        } else if WARNING_LINE_RE.is_match(line)
            || ARROW_RE.is_match(line)
            || PIPE_RE.is_match(line)
            || NOTE_RE.is_match(line)
        {
            // Strip compilation warnings
        } else if line.contains("generated") && line.contains("warning") {
            // Strip warning summary lines
        }
        // Everything else: skip (compilation output, Running lines, Doc-tests, etc.)
    }

    if !found_any_result {
        // Input doesn't look like cargo test output — pass through
        return input.to_string();
    }

    // Remove leading blank lines
    while output_lines.first().is_some_and(|l| l.is_empty()) {
        output_lines.remove(0);
    }
    // Remove trailing blank lines
    while output_lines.last().is_some_and(|l| l.is_empty()) {
        output_lines.pop();
    }

    output_lines.join("\n")
}

/// Filter cargo build output: keep errors and the final Finished line (plus own-crate Compiling).
pub fn filter_cargo_build(input: &str) -> String {
    if input.trim().is_empty() {
        return String::new();
    }

    let mut output_lines: Vec<&str> = Vec::new();
    let mut in_error_block = false;
    let mut found_build_line = false;
    let mut compiling_queue: Vec<&str> = Vec::new();

    for line in input.lines() {
        if FINISHED_RE.is_match(line) {
            found_build_line = true;
            in_error_block = false;
            // Flush last compiling entry (own crate)
            if let Some(last) = compiling_queue.last() {
                output_lines.push(last);
            }
            compiling_queue.clear();
            output_lines.push(line);
        } else if DOWNLOADING_RE.is_match(line) {
            // Skip dependency downloading
        } else if COMPILING_RE.is_match(line) {
            found_build_line = true;
            // Queue compiling lines; only the last (own crate) will be kept
            compiling_queue.push(line);
        } else if ERROR_RE.is_match(line) || line.starts_with("error[") {
            // Flush queued compiling before showing error context
            if let Some(last) = compiling_queue.last() {
                output_lines.push(last);
            }
            compiling_queue.clear();
            in_error_block = true;
            output_lines.push(line);
        } else if in_error_block {
            // Keep all error block content (location, code context, notes)
            if BLANK_RE.is_match(line) {
                in_error_block = false;
                output_lines.push(line);
            } else {
                output_lines.push(line);
            }
        } else if WARNING_LINE_RE.is_match(line)
            || ARROW_RE.is_match(line)
            || PIPE_RE.is_match(line)
            || NOTE_RE.is_match(line)
        {
            // Strip warning noise
        } else if line.contains("generated") && line.contains("warning") {
            // Strip warning summary lines
        }
        // Everything else: skip
    }

    if !found_build_line {
        return input.to_string();
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

/// Filter cargo clippy output: keep warnings/errors with file:line context, strip Checking lines.
///
/// Strategy: for each diagnostic block keep only:
///   - the `warning: ...` / `error: ...` headline
///   - the `  --> file:line:col` location line
///   - `  = note:` and `  = help:` summary lines (actionable)
///
/// Strip: code context lines (`  |`), source excerpts, URL help links.
pub fn filter_cargo_clippy(input: &str) -> String {
    if input.trim().is_empty() {
        return String::new();
    }

    let mut output_lines: Vec<&str> = Vec::new();
    let mut in_diagnostic_block = false;
    let mut found_clippy_line = false;

    for line in input.lines() {
        if CHECKING_RE.is_match(line) || DOWNLOADING_RE.is_match(line) {
            found_clippy_line = true;
            // Skip Checking/Downloading lines
        } else if FINISHED_RE.is_match(line) {
            found_clippy_line = true;
            in_diagnostic_block = false;
            // Don't include Finished line — clippy output is just the findings
        } else if GENERATED_WARNINGS_RE.is_match(line) {
            // Keep summary counts e.g. "warning: `tok0` (lib) generated 3 warnings"
            in_diagnostic_block = false;
            output_lines.push(line);
        } else if WARNING_LINE_RE.is_match(line) || ERROR_FULL_RE.is_match(line) {
            in_diagnostic_block = true;
            output_lines.push(line);
        } else if in_diagnostic_block {
            if BLANK_RE.is_match(line) {
                // End of diagnostic block
                in_diagnostic_block = false;
            } else if ARROW_RE.is_match(line) {
                // Keep `  --> file:line:col` location line
                output_lines.push(line);
            }
            // Strip: `  |` code context, numbered source lines,
            // `  = note:` and `  = help: visit` lines
        }
        // Lines outside diagnostic blocks are skipped
    }

    if !found_clippy_line {
        return input.to_string();
    }

    // Remove leading/trailing blank lines
    while output_lines.first().is_some_and(|l| l.is_empty()) {
        output_lines.remove(0);
    }
    while output_lines.last().is_some_and(|l| l.is_empty()) {
        output_lines.pop();
    }

    if output_lines.is_empty() {
        return String::from("ok");
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

    // ── cargo test ────────────────────────────────────────────────────────────

    #[test]
    fn test_cargo_test_format() {
        let input = include_str!("../../../tests/fixtures/rust/cargo_test_raw.txt");
        let output = filter_cargo_test(input);
        assert_snapshot!(output);
    }

    #[test]
    fn test_cargo_test_savings() {
        let input = include_str!("../../../tests/fixtures/rust/cargo_test_raw.txt");
        let output = filter_cargo_test(input);
        let input_t = count_tokens(input);
        let output_t = count_tokens(&output);
        let savings = 100.0 - (output_t as f64 / input_t as f64 * 100.0);
        assert!(
            savings >= 85.0,
            "Expected >=85% savings, got {:.1}% ({} -> {})",
            savings,
            input_t,
            output_t
        );
    }

    #[test]
    fn test_cargo_test_empty() {
        assert_eq!(filter_cargo_test(""), "");
    }

    #[test]
    fn test_cargo_test_malformed() {
        let output = filter_cargo_test("not cargo test output at all");
        assert!(!output.is_empty());
    }

    #[test]
    fn test_cargo_test_all_pass() {
        let input = concat!(
            "running 3 tests\n",
            "test foo ... ok\n",
            "test bar ... ok\n",
            "test baz ... ok\n",
            "\n",
            "test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s\n"
        );
        let output = filter_cargo_test(input);
        assert!(output.contains("test result: ok. 3 passed"));
        assert!(!output.contains("test foo ... ok"));
        assert!(!output.contains("test bar ... ok"));
    }

    #[test]
    fn test_cargo_test_with_failure() {
        let input = concat!(
            "running 2 tests\n",
            "test foo ... ok\n",
            "test bar ... FAILED\n",
            "\n",
            "failures:\n",
            "\n",
            "---- bar stdout ----\n",
            "thread 'bar' panicked at 'assertion failed'\n",
            "\n",
            "failures:\n",
            "    bar\n",
            "\n",
            "test result: FAILED. 1 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s\n"
        );
        let output = filter_cargo_test(input);
        assert!(output.contains("test bar ... FAILED"));
        assert!(output.contains("test result: FAILED"));
        assert!(output.contains("assertion failed"));
        assert!(!output.contains("test foo ... ok"));
    }

    // ── cargo build ───────────────────────────────────────────────────────────

    #[test]
    fn test_cargo_build_format() {
        let input = include_str!("../../../tests/fixtures/rust/cargo_build_raw.txt");
        let output = filter_cargo_build(input);
        assert_snapshot!(output);
    }

    #[test]
    fn test_cargo_build_savings() {
        let input = include_str!("../../../tests/fixtures/rust/cargo_build_raw.txt");
        let output = filter_cargo_build(input);
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
    fn test_cargo_build_empty() {
        assert_eq!(filter_cargo_build(""), "");
    }

    #[test]
    fn test_cargo_build_malformed() {
        let output = filter_cargo_build("not cargo build output at all");
        assert!(!output.is_empty());
    }

    #[test]
    fn test_cargo_build_finished_line_kept() {
        let input = concat!(
            "   Compiling myapp v0.1.0\n",
            "   Finished `dev` profile [unoptimized + debuginfo] target(s) in 1.23s\n"
        );
        let output = filter_cargo_build(input);
        assert!(output.contains("Finished"));
    }

    #[test]
    fn test_cargo_build_strips_deps() {
        let input = concat!(
            "   Compiling libc v0.2.1\n",
            "   Compiling anyhow v1.0.0\n",
            "   Compiling myapp v0.1.0\n",
            "   Finished `dev` profile target(s) in 2.0s\n"
        );
        let output = filter_cargo_build(input);
        // Should keep only last compiling (myapp) and finished
        assert!(output.contains("myapp"));
        assert!(output.contains("Finished"));
        assert!(!output.contains("libc"));
        assert!(!output.contains("anyhow"));
    }

    // ── cargo clippy ──────────────────────────────────────────────────────────

    #[test]
    fn test_cargo_clippy_format() {
        let input = include_str!("../../../tests/fixtures/rust/cargo_clippy_raw.txt");
        let output = filter_cargo_clippy(input);
        assert_snapshot!(output);
    }

    #[test]
    fn test_cargo_clippy_savings() {
        let input = include_str!("../../../tests/fixtures/rust/cargo_clippy_raw.txt");
        let output = filter_cargo_clippy(input);
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
    fn test_cargo_clippy_empty() {
        assert_eq!(filter_cargo_clippy(""), "");
    }

    #[test]
    fn test_cargo_clippy_malformed() {
        let output = filter_cargo_clippy("not cargo clippy output at all");
        assert!(!output.is_empty());
    }

    #[test]
    fn test_cargo_clippy_no_warnings_returns_ok() {
        let input = concat!(
            "    Checking myapp v0.1.0\n",
            "    Finished `dev` profile target(s) in 0.05s\n"
        );
        let output = filter_cargo_clippy(input);
        assert_eq!(output, "ok");
    }

    #[test]
    fn test_cargo_clippy_strips_checking() {
        let input = concat!(
            "    Checking foo v1.0\n",
            "    Checking bar v2.0\n",
            "warning: unused variable\n",
            "  --> src/main.rs:1:5\n",
            "   |\n",
            "1  |     let x = 5;\n",
            "   |         ^ help: ...\n",
            "   |\n",
            "   = note: `#[warn(unused)]`\n",
            "\n",
            "    Finished `dev` profile target(s) in 0.1s\n"
        );
        let output = filter_cargo_clippy(input);
        assert!(!output.contains("Checking foo"));
        assert!(!output.contains("Checking bar"));
        assert!(output.contains("warning: unused variable"));
        assert!(output.contains("--> src/main.rs:1:5"));
    }
}

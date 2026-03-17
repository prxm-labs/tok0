use lazy_static::lazy_static;
use regex::Regex;

lazy_static! {
    /// Matches "Determining projects to restore..." noise line
    static ref RESTORE_RE: Regex =
        Regex::new(r"^Determining projects to restore").unwrap();
    /// Matches "  Restored /path/to/project.csproj" noise lines
    static ref RESTORED_RE: Regex =
        Regex::new(r"^\s+Restored ").unwrap();
    /// Matches "Build started ..." noise lines
    static ref BUILD_STARTED_RE: Regex =
        Regex::new(r"^Build started ").unwrap();
    /// Matches MSBuild project node lines (e.g. "     1>Project ...")
    static ref MSBUILD_RE: Regex =
        Regex::new(r"^\s+\d+>").unwrap();
    /// Matches "Build succeeded." or "Build FAILED."
    static ref BUILD_RESULT_RE: Regex =
        Regex::new(r"^Build (succeeded|FAILED)").unwrap();
    /// Matches warning/error count lines ("    0 Warning(s)")
    static ref BUILD_COUNT_RE: Regex =
        Regex::new(r"^\s+\d+ (Warning|Error)\(s\)").unwrap();
    /// Matches "Time Elapsed ..." line
    static ref TIME_ELAPSED_RE: Regex =
        Regex::new(r"^Time Elapsed ").unwrap();
    /// Matches the test results summary line ("  Passed! - Failed: 0, Passed: 25 ...")
    static ref TEST_SUMMARY_RE: Regex =
        Regex::new(r"(Passed!|Failed!)\s*-\s*Failed:\s*\d+").unwrap();
    /// Matches "A total of N test files matched" line
    static ref TEST_FILES_RE: Regex =
        Regex::new(r"^A total of \d+ test files? matched").unwrap();
    /// Matches test failure details (stack traces, assertion messages)
    static ref FAILED_TEST_RE: Regex =
        Regex::new(r"^\s+X ").unwrap();
    /// Matches error output lines
    static ref ERROR_RE: Regex =
        Regex::new(r"^\s*(Error|error)\b").unwrap();
}

/// Filter dotnet test output.
///
/// Strips restore/build noise lines, keeps test results summary and any failure details.
/// Target: 80% savings.
pub fn filter_dotnet_test(input: &str) -> String {
    if input.trim().is_empty() {
        return String::new();
    }

    let mut output_lines: Vec<&str> = Vec::new();
    let mut found_test_line = false;

    for line in input.lines() {
        // Skip restore noise
        if RESTORE_RE.is_match(line)
            || RESTORED_RE.is_match(line)
            || BUILD_STARTED_RE.is_match(line)
            || MSBUILD_RE.is_match(line)
            || TIME_ELAPSED_RE.is_match(line)
        {
            found_test_line = true;
            continue;
        }

        // Keep build result
        if BUILD_RESULT_RE.is_match(line) {
            found_test_line = true;
            // Only keep "Build FAILED" — suppress "Build succeeded" since test summary says enough
            if line.contains("FAILED") {
                output_lines.push(line);
            }
            continue;
        }

        // Skip build warning/error counts that follow "Build succeeded"
        if BUILD_COUNT_RE.is_match(line) {
            continue;
        }

        // Keep test results summary (e.g. "Passed! - Failed: 0, Passed: 25, ...")
        if TEST_SUMMARY_RE.is_match(line) {
            found_test_line = true;
            output_lines.push(line);
            continue;
        }

        // Keep "A total of N test files" line
        if TEST_FILES_RE.is_match(line) {
            found_test_line = true;
            continue;
        }

        // Keep individual failed test lines
        if FAILED_TEST_RE.is_match(line) {
            output_lines.push(line);
            continue;
        }

        // Keep error output lines
        if ERROR_RE.is_match(line) {
            output_lines.push(line);
            continue;
        }

        // Skip blank lines between noise sections (keep only at boundaries of kept content)
        // Everything else is skipped
    }

    if !found_test_line {
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

#[cfg(test)]
mod tests {
    use super::*;
    use insta::assert_snapshot;

    fn count_tokens(s: &str) -> usize {
        s.split_whitespace().count()
    }

    #[test]
    fn test_dotnet_test_format() {
        let input = include_str!("../../../tests/fixtures/dotnet/dotnet_test_raw.txt");
        let output = filter_dotnet_test(input);
        assert_snapshot!(output);
    }

    #[test]
    fn test_dotnet_test_savings() {
        let input = include_str!("../../../tests/fixtures/dotnet/dotnet_test_raw.txt");
        let output = filter_dotnet_test(input);
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
    fn test_dotnet_test_keeps_summary() {
        let input = include_str!("../../../tests/fixtures/dotnet/dotnet_test_raw.txt");
        let output = filter_dotnet_test(input);
        assert!(
            output.contains("Passed!"),
            "Should keep test result summary: {}",
            output
        );
        assert!(
            output.contains("Passed:  25"),
            "Should keep passed count: {}",
            output
        );
    }

    #[test]
    fn test_dotnet_test_strips_restore() {
        let input = include_str!("../../../tests/fixtures/dotnet/dotnet_test_raw.txt");
        let output = filter_dotnet_test(input);
        assert!(
            !output.contains("Determining projects to restore"),
            "Should strip restore lines: {}",
            output
        );
        assert!(
            !output.contains("Restored /"),
            "Should strip Restored lines: {}",
            output
        );
    }

    #[test]
    fn test_dotnet_test_strips_build_noise() {
        let input = include_str!("../../../tests/fixtures/dotnet/dotnet_test_raw.txt");
        let output = filter_dotnet_test(input);
        assert!(
            !output.contains("Build started"),
            "Should strip build started: {}",
            output
        );
        assert!(
            !output.contains("Time Elapsed"),
            "Should strip time elapsed: {}",
            output
        );
    }

    #[test]
    fn test_dotnet_test_empty() {
        assert_eq!(filter_dotnet_test(""), "");
        assert_eq!(filter_dotnet_test("   \n  "), "");
    }

    #[test]
    fn test_dotnet_test_malformed() {
        let output = filter_dotnet_test("not dotnet output\nsome random text");
        assert!(!output.is_empty(), "Should return input on malformed data");
    }

    #[test]
    fn test_dotnet_test_with_failure() {
        let input = concat!(
            "Determining projects to restore...\n",
            "  Restored /home/dev/myapp/tests/MyApp.Tests.csproj (200ms).\n",
            "Build succeeded.\n",
            "    0 Warning(s)\n",
            "    0 Error(s)\n",
            "Time Elapsed 00:00:03.00\n",
            "\n",
            "A total of 1 test files matched the specified pattern.\n",
            "  Failed!  - Failed:   1, Passed:  24, Skipped:   0, Total:  25, Duration: 500 ms - MyApp.Tests.dll (net8.0)\n"
        );
        let output = filter_dotnet_test(input);
        assert!(output.contains("Failed!"), "Should keep failure result");
        assert!(output.contains("Failed:   1"), "Should show failure count");
        assert!(
            !output.contains("Determining"),
            "Should strip restore noise"
        );
    }
}

use lazy_static::lazy_static;
use regex::Regex;

lazy_static! {
    /// Matches task lines for up-to-date tasks (e.g. "> Task :foo UP-TO-DATE")
    static ref TASK_UP_TO_DATE_RE: Regex =
        Regex::new(r"^> Task :.+ UP-TO-DATE$").unwrap();
    /// Matches task lines for no-source tasks (e.g. "> Task :foo NO-SOURCE")
    static ref TASK_NO_SOURCE_RE: Regex =
        Regex::new(r"^> Task :.+ NO-SOURCE$").unwrap();
    /// Matches task lines that executed (no suffix or FAILED)
    static ref TASK_EXECUTED_RE: Regex =
        Regex::new(r"^> Task :\S+$").unwrap();
    /// Matches FAILED task lines (e.g. "> Task :foo FAILED")
    static ref TASK_FAILED_RE: Regex =
        Regex::new(r"^> Task :.+ FAILED$").unwrap();
    /// Matches "BUILD SUCCESSFUL" or "BUILD FAILED"
    static ref BUILD_RESULT_RE: Regex =
        Regex::new(r"^BUILD (SUCCESSFUL|FAILED)").unwrap();
    /// Matches the timing line (e.g. "BUILD SUCCESSFUL in 12s")
    static ref BUILD_TIMING_RE: Regex =
        Regex::new(r"^BUILD (SUCCESSFUL|FAILED) in \d+").unwrap();
    /// Matches the actionable tasks summary line
    static ref ACTIONABLE_RE: Regex =
        Regex::new(r"^\d+ actionable tasks:").unwrap();
    /// Matches Gradle daemon startup lines
    static ref DAEMON_RE: Regex =
        Regex::new(r"^(Starting Gradle Daemon|Gradle Daemon started)").unwrap();
    /// Matches deprecation warning block lines
    static ref DEPRECATION_RE: Regex =
        Regex::new(r"^Deprecated Gradle features|^Use '--warning-mode|^See https://docs\.gradle").unwrap();
    /// Matches configure project lines
    static ref CONFIGURE_RE: Regex =
        Regex::new(r"^> Configure project").unwrap();
    /// Matches FAILURE section header
    static ref FAILURE_SECTION_RE: Regex =
        Regex::new(r"^FAILURE:").unwrap();
    /// Matches "* What went wrong:" and similar gradle error sections
    static ref ERROR_SECTION_RE: Regex =
        Regex::new(r"^\* (What went wrong|Try:|Where:)").unwrap();
}

/// Filter gradle build output.
///
/// Strips UP-TO-DATE/NO-SOURCE task lines, daemon startup, deprecation warnings,
/// and configure lines. Keeps executed tasks + BUILD result + timing + actionable summary.
/// Target: 75% savings.
pub fn filter_gradle(input: &str) -> String {
    if input.trim().is_empty() {
        return String::new();
    }

    let mut output_lines: Vec<&str> = Vec::new();
    let mut found_build_line = false;
    let mut in_error_section = false;

    for line in input.lines() {
        // Skip daemon startup
        if DAEMON_RE.is_match(line) {
            found_build_line = true;
            continue;
        }

        // Skip configure project lines
        if CONFIGURE_RE.is_match(line) {
            found_build_line = true;
            continue;
        }

        // Skip deprecation warnings
        if DEPRECATION_RE.is_match(line) {
            continue;
        }

        // Skip UP-TO-DATE and NO-SOURCE tasks
        if TASK_UP_TO_DATE_RE.is_match(line) || TASK_NO_SOURCE_RE.is_match(line) {
            found_build_line = true;
            continue;
        }

        // Keep FAILED tasks
        if TASK_FAILED_RE.is_match(line) {
            found_build_line = true;
            output_lines.push(line);
            continue;
        }

        // Keep executed task lines
        if TASK_EXECUTED_RE.is_match(line) {
            found_build_line = true;
            output_lines.push(line);
            continue;
        }

        // Keep BUILD result line (includes timing if on same line)
        if BUILD_TIMING_RE.is_match(line) {
            found_build_line = true;
            output_lines.push(line);
            continue;
        }

        if BUILD_RESULT_RE.is_match(line) {
            found_build_line = true;
            output_lines.push(line);
            continue;
        }

        // Keep actionable tasks summary
        if ACTIONABLE_RE.is_match(line) {
            output_lines.push(line);
            continue;
        }

        // Keep FAILURE section and error details
        if FAILURE_SECTION_RE.is_match(line) || ERROR_SECTION_RE.is_match(line) {
            in_error_section = true;
            output_lines.push(line);
            continue;
        }

        if in_error_section {
            if line.is_empty() {
                in_error_section = false;
            } else {
                output_lines.push(line);
            }
            continue;
        }

        // Everything else: skip (blank lines between sections, misc gradle output)
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

#[cfg(test)]
mod tests {
    use super::*;
    use insta::assert_snapshot;

    fn count_tokens(s: &str) -> usize {
        s.split_whitespace().count()
    }

    #[test]
    fn test_gradle_build_format() {
        let input = include_str!("../../../tests/fixtures/java/gradle_build_raw.txt");
        let output = filter_gradle(input);
        assert_snapshot!(output);
    }

    #[test]
    fn test_gradle_build_savings() {
        let input = include_str!("../../../tests/fixtures/java/gradle_build_raw.txt");
        let output = filter_gradle(input);
        let input_t = count_tokens(input);
        let output_t = count_tokens(&output);
        let savings = 100.0 - (output_t as f64 / input_t as f64 * 100.0);
        assert!(
            savings >= 75.0,
            "Expected >=75% savings, got {:.1}% ({} -> {} tokens)",
            savings,
            input_t,
            output_t
        );
    }

    #[test]
    fn test_gradle_build_keeps_build_result() {
        let input = include_str!("../../../tests/fixtures/java/gradle_build_raw.txt");
        let output = filter_gradle(input);
        assert!(
            output.contains("BUILD SUCCESSFUL"),
            "Should keep BUILD result: {}",
            output
        );
        assert!(output.contains("12s"), "Should keep timing: {}", output);
    }

    #[test]
    fn test_gradle_build_keeps_executed_tasks() {
        let input = include_str!("../../../tests/fixtures/java/gradle_build_raw.txt");
        let output = filter_gradle(input);
        assert!(
            output.contains(":myapp-core:compileJava"),
            "Should keep compileJava task: {}",
            output
        );
    }

    #[test]
    fn test_gradle_build_strips_up_to_date() {
        let input = include_str!("../../../tests/fixtures/java/gradle_build_raw.txt");
        let output = filter_gradle(input);
        assert!(
            !output.contains("UP-TO-DATE"),
            "Should strip UP-TO-DATE tasks: {}",
            output
        );
        assert!(
            !output.contains("NO-SOURCE"),
            "Should strip NO-SOURCE tasks: {}",
            output
        );
    }

    #[test]
    fn test_gradle_build_strips_daemon() {
        let input = include_str!("../../../tests/fixtures/java/gradle_build_raw.txt");
        let output = filter_gradle(input);
        assert!(
            !output.contains("Gradle Daemon"),
            "Should strip daemon startup: {}",
            output
        );
    }

    #[test]
    fn test_gradle_build_empty() {
        assert_eq!(filter_gradle(""), "");
        assert_eq!(filter_gradle("   \n  "), "");
    }

    #[test]
    fn test_gradle_build_malformed() {
        let output = filter_gradle("not gradle output\nsome random text");
        assert!(!output.is_empty(), "Should return input on malformed data");
    }

    #[test]
    fn test_gradle_build_failure() {
        let input = concat!(
            "Starting Gradle Daemon...\n",
            "Gradle Daemon started in 1 s 200 ms\n",
            "\n",
            "> Task :compileJava\n",
            "> Task :test FAILED\n",
            "\n",
            "FAILURE: Build failed with an exception.\n",
            "\n",
            "* What went wrong:\n",
            "Execution failed for task ':test'.\n",
            "> There were failing tests.\n",
            "\n",
            "BUILD FAILED in 5s\n",
            "3 actionable tasks: 3 executed\n"
        );
        let output = filter_gradle(input);
        assert!(output.contains("BUILD FAILED"), "Should show BUILD FAILED");
        assert!(output.contains(":test FAILED"), "Should show failed task");
        assert!(
            !output.contains("Gradle Daemon"),
            "Should strip daemon lines"
        );
    }
}

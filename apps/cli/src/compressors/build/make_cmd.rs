use lazy_static::lazy_static;
use regex::Regex;

lazy_static! {
    /// Matches recipe echo lines: compiler invocations (gcc/g++/cc/clang/ld/ar/as/ranlib)
    static ref RECIPE_ECHO_RE: Regex =
        Regex::new(r"^(gcc|g\+\+|cc|c\+\+|clang|clang\+\+|ld|ar|as|ranlib|objcopy|strip|cmake|ninja)\b").unwrap();
    /// Matches compiler warning lines (file:line: warning: ... OR file:line:col: warning: ...)
    static ref WARNING_RE: Regex =
        Regex::new(r"^[^\s:][^:]*:\d+(?::\d+)?: warning:").unwrap();
    /// Matches compiler error lines (file:line: error: ... OR file:line:col: error: ...)
    static ref ERROR_RE: Regex =
        Regex::new(r"^[^\s:][^:]*:\d+(?::\d+)?: error:").unwrap();
    /// Matches make fatal error / termination lines
    static ref MAKE_ERROR_RE: Regex =
        Regex::new(r"^make(?:\[\d+\])?: \*\*\*").unwrap();
    /// Matches make "Nothing to be done" or "is up to date" informational lines
    static ref MAKE_NOTHING_RE: Regex =
        Regex::new(r"^make(?:\[\d+\])?: (Nothing to be done|.+ is up to date)").unwrap();
    /// Matches linker / library error lines (undefined reference, cannot find, multiple definition)
    static ref LINKER_ERROR_RE: Regex =
        Regex::new(r"(undefined reference|cannot find|multiple definition|collect2:|ld returned)").unwrap();
    /// Matches "Entering directory" / "Leaving directory" make noise
    static ref MAKE_DIR_RE: Regex =
        Regex::new(r"^make(?:\[\d+\])?: (Entering|Leaving) directory").unwrap();
    /// Matches source context lines that follow an error (lines starting with spaces or containing ^ caret markers)
    static ref SOURCE_CONTEXT_RE: Regex =
        Regex::new(r"^\s+\^|^\s{4,}[^\s]").unwrap();
}

/// Filter `make` output.
///
/// Strips recipe echo lines (compiler invocations), keeps:
/// - Compiler warning lines (file:line: warning: ...)
/// - Compiler error lines (file:line: error: ...)
/// - Source context lines immediately following an error (caret markers)
/// - Linker errors
/// - make *** fatal lines
/// - "Nothing to be done" / "is up to date" informational lines
///
/// Appends a summary: "N compiled, M warnings, K errors"
/// Target: 75% savings.
pub fn filter_make(input: &str) -> String {
    if input.trim().is_empty() {
        return String::new();
    }

    let mut kept_lines: Vec<&str> = Vec::new();
    let mut compiled = 0usize;
    let mut warnings = 0usize;
    let mut errors = 0usize;
    let mut after_error = false;
    let mut found_make_line = false;

    for line in input.lines() {
        if MAKE_DIR_RE.is_match(line) {
            // Strip entering/leaving directory noise
            continue;
        }

        if RECIPE_ECHO_RE.is_match(line) {
            compiled += 1;
            found_make_line = true;
            after_error = false;
            continue;
        }

        if ERROR_RE.is_match(line) {
            errors += 1;
            found_make_line = true;
            kept_lines.push(line);
            after_error = true;
            continue;
        }

        if LINKER_ERROR_RE.is_match(line) {
            errors += 1;
            found_make_line = true;
            kept_lines.push(line);
            after_error = false;
            continue;
        }

        if after_error && SOURCE_CONTEXT_RE.is_match(line) {
            // Keep source context / caret markers immediately after an error
            kept_lines.push(line);
            continue;
        }

        after_error = false;

        if WARNING_RE.is_match(line) {
            warnings += 1;
            found_make_line = true;
            kept_lines.push(line);
            continue;
        }

        if MAKE_ERROR_RE.is_match(line) {
            found_make_line = true;
            kept_lines.push(line);
            continue;
        }

        if MAKE_NOTHING_RE.is_match(line) {
            found_make_line = true;
            kept_lines.push(line);
            continue;
        }

        // Skip all other lines (unknown noise)
    }

    if !found_make_line {
        // Doesn't look like make output — pass through unchanged
        return input.to_string();
    }

    // Build summary line
    let summary = format!(
        "{} compiled, {} warnings, {} errors",
        compiled, warnings, errors
    );

    // Assemble output: kept diagnostic lines + summary
    let mut output_lines: Vec<&str> = kept_lines;

    // Remove leading/trailing blank lines
    while output_lines.first().is_some_and(|l| l.is_empty()) {
        output_lines.remove(0);
    }
    while output_lines.last().is_some_and(|l| l.is_empty()) {
        output_lines.pop();
    }

    if output_lines.is_empty() {
        summary
    } else {
        format!("{}\n{}", output_lines.join("\n"), summary)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use insta::assert_snapshot;

    fn count_tokens(s: &str) -> usize {
        s.split_whitespace().count()
    }

    // ── snapshot ──────────────────────────────────────────────────────────────

    #[test]
    fn test_make_format() {
        let input = include_str!("../../../tests/fixtures/build/make_raw.txt");
        let output = filter_make(input);
        assert_snapshot!(output);
    }

    #[test]
    fn test_make_error_format() {
        let input = include_str!("../../../tests/fixtures/build/make_error_raw.txt");
        let output = filter_make(input);
        assert_snapshot!(output);
    }

    // ── savings ───────────────────────────────────────────────────────────────

    #[test]
    fn test_make_savings() {
        let input = include_str!("../../../tests/fixtures/build/make_raw.txt");
        let output = filter_make(input);
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
    fn test_make_error_savings() {
        let input = include_str!("../../../tests/fixtures/build/make_error_raw.txt");
        let output = filter_make(input);
        let input_t = count_tokens(input);
        let output_t = count_tokens(&output);
        // Error output is already compact — just ensure we don't balloon it
        assert!(
            output_t <= input_t,
            "Output should not exceed input tokens ({} -> {})",
            input_t,
            output_t
        );
    }

    // ── empty / malformed ────────────────────────────────────────────────────

    #[test]
    fn test_make_empty() {
        assert_eq!(filter_make(""), "");
        assert_eq!(filter_make("   \n  "), "");
    }

    #[test]
    fn test_make_malformed() {
        let output = filter_make("not make output\njust some random text");
        assert!(!output.is_empty(), "Should return input on malformed data");
    }

    // ── clean build (no errors) ───────────────────────────────────────────────

    #[test]
    fn test_make_clean_build_strips_recipes() {
        let input = concat!(
            "gcc -c -o obj/main.o src/main.c\n",
            "gcc -c -o obj/util.o src/util.c\n",
            "gcc -o bin/app obj/main.o obj/util.o -lm\n",
        );
        let output = filter_make(input);
        assert!(
            !output.contains("gcc"),
            "Should strip recipe echo lines; got: {}",
            output
        );
        assert!(
            output.contains("3 compiled"),
            "Should report compiled count; got: {}",
            output
        );
        assert!(
            output.contains("0 warnings"),
            "Should report zero warnings; got: {}",
            output
        );
        assert!(
            output.contains("0 errors"),
            "Should report zero errors; got: {}",
            output
        );
    }

    #[test]
    fn test_make_clean_build_summary_only() {
        let input = concat!(
            "gcc -c -o obj/a.o src/a.c\n",
            "gcc -c -o obj/b.o src/b.c\n",
            "gcc -o bin/app obj/a.o obj/b.o\n",
        );
        let output = filter_make(input);
        assert_eq!(output, "3 compiled, 0 warnings, 0 errors");
    }

    // ── build with warnings ───────────────────────────────────────────────────

    #[test]
    fn test_make_keeps_warnings() {
        let input = concat!(
            "gcc -c -o obj/util.o src/util.c\n",
            "src/util.c:45: warning: unused variable 'x'\n",
            "gcc -o bin/app obj/util.o\n",
        );
        let output = filter_make(input);
        assert!(
            output.contains("warning: unused variable"),
            "Should keep warning line; got: {}",
            output
        );
        assert!(
            output.contains("1 warnings"),
            "Should report 1 warning; got: {}",
            output
        );
    }

    // ── build with errors ─────────────────────────────────────────────────────

    #[test]
    fn test_make_keeps_errors() {
        let input = concat!(
            "gcc -c -o obj/main.o src/main.c\n",
            "src/main.c:12:5: error: use of undeclared identifier 'foo'\n",
            "    foo(argc, argv);\n",
            "    ^\n",
            "make: *** [obj/main.o] Error 1\n",
        );
        let output = filter_make(input);
        assert!(
            output.contains("error: use of undeclared identifier"),
            "Should keep error line; got: {}",
            output
        );
        assert!(
            output.contains("make: ***"),
            "Should keep make fatal line; got: {}",
            output
        );
        assert!(
            output.contains("1 errors"),
            "Should report 1 error; got: {}",
            output
        );
        assert!(
            !output.contains("gcc -c"),
            "Should strip recipe echo; got: {}",
            output
        );
    }

    #[test]
    fn test_make_only_errors() {
        let input = concat!(
            "src/a.c:1:1: error: expected ';'\n",
            "src/b.c:5:10: error: undeclared 'x'\n",
            "make: *** [all] Error 2\n",
        );
        let output = filter_make(input);
        assert!(output.contains("2 errors"), "got: {}", output);
        assert!(output.contains("0 compiled"), "got: {}", output);
        assert!(output.contains("src/a.c:1:1: error"), "got: {}", output);
        assert!(output.contains("src/b.c:5:10: error"), "got: {}", output);
    }

    // ── "Nothing to be done" ──────────────────────────────────────────────────

    #[test]
    fn test_make_nothing_to_be_done() {
        let input = "make: Nothing to be done for 'all'.\n";
        let output = filter_make(input);
        assert!(
            output.contains("Nothing to be done"),
            "Should keep nothing-to-do message; got: {}",
            output
        );
        assert!(
            output.contains("0 compiled"),
            "Should report 0 compiled; got: {}",
            output
        );
    }

    #[test]
    fn test_make_up_to_date() {
        let input = "make[1]: 'libfoo.a' is up to date\n";
        let output = filter_make(input);
        assert!(
            output.contains("is up to date"),
            "Should keep up-to-date message; got: {}",
            output
        );
    }
}

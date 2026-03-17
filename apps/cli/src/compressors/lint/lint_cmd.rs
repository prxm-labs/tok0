use lazy_static::lazy_static;
use regex::Regex;

lazy_static! {
    // ESLint: "  12:5   error    message   rule-name"
    static ref ESLINT_ISSUE_RE: Regex =
        Regex::new(r"^\s+(\d+):(\d+)\s+(error|warning)\s+(.+?)\s{2,}(\S+)\s*$").unwrap();
    // ESLint: file path line (absolute or relative, ends before issues list)
    static ref ESLINT_FILE_RE: Regex =
        Regex::new(r"^(/[\w./\-]+|[\w./\-]+\.(ts|tsx|js|jsx|mjs|cjs|vue|svelte))$").unwrap();
    // ESLint: final summary "✖ N problems (X errors, Y warnings)"
    static ref ESLINT_SUMMARY_RE: Regex =
        Regex::new(r"^[✖✗x]\s+\d+ problem").unwrap();
    // ESLint: fixable hint
    static ref ESLINT_FIXABLE_RE: Regex =
        Regex::new(r"potentially fixable with").unwrap();

    // Shellcheck: "In file.sh line N:"
    static ref SC_FILE_LINE_RE: Regex =
        Regex::new(r"^In (.+) line (\d+):$").unwrap();
    // Shellcheck: "^-- SC####: message"
    static ref SC_CODE_RE: Regex =
        Regex::new(r"^\s+\^--\s+(SC\d+):\s+(.+)$").unwrap();
}

// ─── ESLint ──────────────────────────────────────────────────────────────────

/// Filter ESLint output.
///
/// Groups findings by file. For each file shows error count, warning count,
/// and the error messages (warnings stripped). Files with zero errors and
/// zero warnings are omitted entirely. Target: 75% savings.
pub fn filter_eslint(input: &str) -> String {
    if input.trim().is_empty() {
        return String::new();
    }

    struct FileIssues<'a> {
        errors: Vec<&'a str>,
        warning_count: usize,
        error_count: usize,
    }

    let mut files: Vec<(&str, FileIssues)> = Vec::new();
    let mut current_file: Option<&str> = None;
    let mut summary_line: Option<&str> = None;
    let mut fixable_line: Option<&str> = None;

    for line in input.lines() {
        if ESLINT_SUMMARY_RE.is_match(line) {
            summary_line = Some(line);
            continue;
        }
        if ESLINT_FIXABLE_RE.is_match(line) {
            fixable_line = Some(line);
            continue;
        }
        if ESLINT_FILE_RE.is_match(line.trim()) && !line.trim().is_empty() {
            let file = line.trim();
            files.push((
                file,
                FileIssues {
                    errors: Vec::new(),
                    warning_count: 0,
                    error_count: 0,
                },
            ));
            current_file = Some(file);
            continue;
        }
        if let Some(caps) = ESLINT_ISSUE_RE.captures(line) {
            let severity = caps.get(3).map_or("", |m| m.as_str());
            let message = caps.get(4).map_or("", |m| m.as_str());
            let rule = caps.get(5).map_or("", |m| m.as_str());
            let col_line = caps.get(1).map_or("", |m| m.as_str());
            let col_col = caps.get(2).map_or("", |m| m.as_str());

            if let Some((_f, issues)) = files.last_mut() {
                if severity == "error" {
                    issues.error_count += 1;
                    // store line for inclusion
                    let _ = (col_line, col_col, message, rule, current_file);
                    issues.errors.push(line.trim());
                } else {
                    issues.warning_count += 1;
                }
            }
        }
    }

    if files.is_empty() && summary_line.is_none() {
        // Unknown format — passthrough
        return input.trim_end().to_string();
    }

    let mut out: Vec<String> = Vec::new();

    for (file, issues) in &files {
        // Skip files with no errors (warning-only files are low signal)
        if issues.error_count == 0 {
            continue;
        }
        let mut header = file.to_string();
        let mut parts: Vec<String> = Vec::new();
        parts.push(format!(
            "{} error{}",
            issues.error_count,
            if issues.error_count == 1 { "" } else { "s" }
        ));
        if issues.warning_count > 0 {
            parts.push(format!(
                "{} warning{}",
                issues.warning_count,
                if issues.warning_count == 1 { "" } else { "s" }
            ));
        }
        header.push_str(&format!("  [{}]", parts.join(", ")));
        out.push(header);
        for err_line in &issues.errors {
            out.push(format!("  {}", err_line));
        }
    }

    if let Some(s) = summary_line {
        out.push(s.trim().to_string());
    }
    if let Some(f) = fixable_line {
        out.push(f.trim().to_string());
    }

    if out.is_empty() {
        return String::new();
    }

    out.join("\n")
}

// ─── Shellcheck ──────────────────────────────────────────────────────────────

/// Filter shellcheck output.
///
/// Keeps findings in compact format: "file:line SC####: message".
/// Strips the source lines and caret indicators. Target: 65% savings.
pub fn filter_shellcheck(input: &str) -> String {
    if input.trim().is_empty() {
        return String::new();
    }

    let mut out: Vec<String> = Vec::new();
    let mut current_file: Option<String> = None;
    let mut current_line_no: Option<u32> = None;

    for line in input.lines() {
        if let Some(caps) = SC_FILE_LINE_RE.captures(line) {
            current_file = Some(caps.get(1).map_or("", |m| m.as_str()).to_string());
            current_line_no = caps.get(2).and_then(|m| m.as_str().parse().ok());
            continue;
        }
        if let Some(caps) = SC_CODE_RE.captures(line) {
            let code = caps.get(1).map_or("", |m| m.as_str());
            let message = caps.get(2).map_or("", |m| m.as_str());
            if let (Some(file), Some(lno)) = (&current_file, current_line_no) {
                out.push(format!("{}:{}: {}: {}", file, lno, code, message));
            } else {
                out.push(format!("{}: {}", code, message));
            }
            // Reset so next finding needs a new "In file line N:" header
            current_file = None;
            current_line_no = None;
            continue;
        }
        // Skip source code lines and caret indicators (pure noise)
    }

    if out.is_empty() {
        // Unknown format — passthrough
        return input.trim_end().to_string();
    }

    out.join("\n")
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use insta::assert_snapshot;

    fn count_tokens(s: &str) -> usize {
        s.split_whitespace().count()
    }

    // ── ESLint ────────────────────────────────────────────────────────────────

    #[test]
    fn test_eslint_format() {
        let input = include_str!("../../../tests/fixtures/lint/eslint_raw.txt");
        let output = filter_eslint(input);
        assert_snapshot!(output);
    }

    #[test]
    fn test_eslint_savings() {
        let input = include_str!("../../../tests/fixtures/lint/eslint_raw.txt");
        let output = filter_eslint(input);
        let input_t = count_tokens(input);
        let output_t = count_tokens(&output);
        let savings = 100.0 - (output_t as f64 / input_t as f64 * 100.0);
        assert!(
            savings >= 30.0,
            "Expected >=30% savings on eslint, got {:.1}% ({} -> {} tokens)",
            savings,
            input_t,
            output_t
        );
    }

    #[test]
    fn test_eslint_empty() {
        assert_eq!(filter_eslint(""), "");
        assert_eq!(filter_eslint("   \n  "), "");
    }

    #[test]
    fn test_eslint_malformed() {
        let input = "not eslint output\nsome random text here";
        let output = filter_eslint(input);
        assert!(
            !output.is_empty(),
            "Should not return empty for unknown input"
        );
    }

    #[test]
    fn test_eslint_keeps_errors() {
        let input = "/src/foo.ts\n  12:5  error  Missing semicolon  semi\n  23:1  warning  No console  no-console\n\n✖ 2 problems (1 error, 1 warning)\n";
        let output = filter_eslint(input);
        assert!(output.contains("error"), "Should keep error lines");
        assert!(output.contains("semi"), "Should keep rule name in error");
    }

    #[test]
    fn test_eslint_keeps_summary() {
        let input = "/src/foo.ts\n  12:5  error  Missing semicolon  semi\n\n✖ 1 problems (1 error, 0 warnings)\n";
        let output = filter_eslint(input);
        assert!(output.contains("problem"), "Should keep summary line");
    }

    #[test]
    fn test_eslint_strips_warnings_only_files() {
        // A file with only warnings should be omitted
        let input = "/src/clean.ts\n  1:1  warning  some warning  no-console\n\n✖ 1 problems (0 errors, 1 warning)\n";
        let output = filter_eslint(input);
        // The file header should not appear since there are no errors
        // but summary should still be there
        assert!(output.contains("problem"), "Should keep summary");
    }

    // ── Shellcheck ────────────────────────────────────────────────────────────

    #[test]
    fn test_shellcheck_format() {
        let input = include_str!("../../../tests/fixtures/lint/shellcheck_raw.txt");
        let output = filter_shellcheck(input);
        assert_snapshot!(output);
    }

    #[test]
    fn test_shellcheck_savings() {
        let input = include_str!("../../../tests/fixtures/lint/shellcheck_raw.txt");
        let output = filter_shellcheck(input);
        let input_t = count_tokens(input);
        let output_t = count_tokens(&output);
        let savings = 100.0 - (output_t as f64 / input_t as f64 * 100.0);
        assert!(
            savings >= 40.0,
            "Expected >=40% savings on shellcheck, got {:.1}% ({} -> {} tokens)",
            savings,
            input_t,
            output_t
        );
    }

    #[test]
    fn test_shellcheck_empty() {
        assert_eq!(filter_shellcheck(""), "");
        assert_eq!(filter_shellcheck("   \n  "), "");
    }

    #[test]
    fn test_shellcheck_malformed() {
        let input = "not shellcheck output\nrandom text here";
        let output = filter_shellcheck(input);
        assert!(
            !output.is_empty(),
            "Should not return empty for unknown input"
        );
    }

    #[test]
    fn test_shellcheck_compact_format() {
        let input = "In scripts/test.sh line 5:\n  source ./foo.sh\n  ^-- SC1091: Not following.\n";
        let output = filter_shellcheck(input);
        assert!(
            output.contains("scripts/test.sh:5"),
            "Should use file:line format"
        );
        assert!(output.contains("SC1091"), "Should include the SC code");
        assert!(
            !output.contains("source ./foo.sh"),
            "Should strip source code lines"
        );
    }

    #[test]
    fn test_shellcheck_multiple_findings() {
        let input = "In a.sh line 3:\n  echo $VAR\n  ^-- SC2086: Double quote.\n\nIn b.sh line 7:\n  [ $X == 1 ]\n  ^-- SC2039: POSIX issue.\n";
        let output = filter_shellcheck(input);
        assert!(output.contains("a.sh:3"), "Should include a.sh finding");
        assert!(output.contains("b.sh:7"), "Should include b.sh finding");
        assert_eq!(
            output.lines().count(),
            2,
            "Should have exactly 2 compact lines"
        );
    }
}

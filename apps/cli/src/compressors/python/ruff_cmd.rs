use lazy_static::lazy_static;
use regex::Regex;
use std::collections::HashMap;

lazy_static! {
    // "src/file.py:line:col: EXXX [*] message"  (the [*] fixable marker is optional)
    static ref RUFF_ISSUE_RE: Regex =
        Regex::new(r"^(.+):(\d+):\d+: ([A-Z]\d+)(?:\s+\[\*\])?\s+(.+)$").unwrap();
    // "Found N errors (M fixable)."
    static ref RUFF_SUMMARY_RE: Regex =
        Regex::new(r"^Found \d+ errors?").unwrap();
}

/// Filter `ruff check .` output.
///
/// Groups findings by rule code (F401, E501, etc.). Shows the first instance
/// of each rule with file:line and message, then "+N more" if there are
/// additional occurrences. Appends the summary line.
/// Target: 70% savings.
pub fn filter_ruff(input: &str) -> String {
    if input.trim().is_empty() {
        return String::new();
    }

    // rule_code -> (file, line_no, message)
    let mut rule_first: HashMap<String, (String, u32, String)> = HashMap::new();
    let mut rule_count: HashMap<String, usize> = HashMap::new();
    let mut rule_order: Vec<String> = Vec::new();
    let mut summary_line: Option<&str> = None;

    for line in input.lines() {
        if RUFF_SUMMARY_RE.is_match(line) {
            summary_line = Some(line);
            continue;
        }
        if let Some(caps) = RUFF_ISSUE_RE.captures(line) {
            let file = caps.get(1).map_or("", |m| m.as_str()).to_string();
            let line_no: u32 = caps
                .get(2)
                .and_then(|m| m.as_str().parse().ok())
                .unwrap_or(0);
            let code = caps.get(3).map_or("", |m| m.as_str()).to_string();
            let message = caps.get(4).map_or("", |m| m.as_str()).to_string();

            let count = rule_count.entry(code.clone()).or_insert(0);
            if *count == 0 {
                rule_order.push(code.clone());
                rule_first.insert(code.clone(), (file, line_no, message));
            }
            *count += 1;
        }
    }

    if rule_order.is_empty() && summary_line.is_none() {
        // Unknown format — passthrough
        return input.trim_end().to_string();
    }

    let mut out: Vec<String> = Vec::new();

    for code in &rule_order {
        if let (Some((file, line_no, message)), Some(&count)) =
            (rule_first.get(code), rule_count.get(code))
        {
            if count == 1 {
                out.push(format!("{} {}:{}: {}", code, file, line_no, message));
            } else {
                out.push(format!(
                    "{} {}:{}: {} (+{} more)",
                    code,
                    file,
                    line_no,
                    message,
                    count - 1
                ));
            }
        }
    }

    if let Some(s) = summary_line {
        out.push(s.to_string());
    }

    if out.is_empty() {
        return String::new();
    }

    out.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use insta::assert_snapshot;

    fn count_tokens(s: &str) -> usize {
        s.split_whitespace().count()
    }

    #[test]
    fn test_ruff_format() {
        let input = include_str!("../../../tests/fixtures/python/ruff_check_raw.txt");
        let output = filter_ruff(input);
        assert_snapshot!(output);
    }

    #[test]
    fn test_ruff_savings() {
        let input = include_str!("../../../tests/fixtures/python/ruff_check_raw.txt");
        let output = filter_ruff(input);
        let input_t = count_tokens(input);
        let output_t = count_tokens(&output);
        let savings = 100.0 - (output_t as f64 / input_t as f64 * 100.0);
        assert!(
            savings >= 30.0,
            "Expected >=30% savings on ruff, got {:.1}% ({} -> {} tokens)",
            savings,
            input_t,
            output_t
        );
    }

    #[test]
    fn test_ruff_empty() {
        assert_eq!(filter_ruff(""), "");
        assert_eq!(filter_ruff("   \n  "), "");
    }

    #[test]
    fn test_ruff_malformed() {
        let input = "not ruff output\nrandom text here";
        let output = filter_ruff(input);
        assert!(
            !output.is_empty(),
            "Should not return empty for unknown input"
        );
    }

    #[test]
    fn test_ruff_groups_by_rule() {
        let input = "src/a.py:1:1: F401 `os` imported but unused\nsrc/b.py:5:1: F401 `sys` imported but unused\nsrc/c.py:3:80: E501 Line too long\nFound 3 errors.\n";
        let output = filter_ruff(input);
        assert!(output.contains("F401"), "Should show F401 rule");
        assert!(
            output.contains("+1 more"),
            "Should show additional count for F401"
        );
        assert!(output.contains("E501"), "Should show E501 rule");
        assert!(output.contains("Found 3 errors"), "Should keep summary");
    }

    #[test]
    fn test_ruff_fixable_marker_stripped() {
        // The [*] fixable marker should not appear in output
        let input =
            "src/foo.py:5:1: F401 [*] `os` imported but unused\nFound 1 error (1 fixable).\n";
        let output = filter_ruff(input);
        assert!(output.contains("F401"), "Should show rule code");
        assert!(!output.contains("[*]"), "Should strip [*] fixable marker");
        assert!(output.contains("Found 1 error"), "Should keep summary");
    }

    #[test]
    fn test_ruff_keeps_summary() {
        let input = "src/foo.py:1:1: E501 Line too long\nFound 1 error.\n";
        let output = filter_ruff(input);
        assert!(output.contains("Found 1 error"), "Should keep summary line");
    }
}

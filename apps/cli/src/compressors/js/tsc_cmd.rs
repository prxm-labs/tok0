use lazy_static::lazy_static;
use regex::Regex;
use std::collections::HashMap;

lazy_static! {
    // "src/file.ts(line,col): error TSNNNN: message"
    static ref TSC_DIAG_RE: Regex =
        Regex::new(r"^(.+?)\((\d+),\d+\): error (TS\d+): (.+)$").unwrap();
    // "Found N errors."
    static ref TSC_SUMMARY_RE: Regex =
        Regex::new(r"^Found \d+ errors?\.?$").unwrap();
}

const MAX_PER_FILE: usize = 3;

/// Filter `tsc --noEmit` output.
///
/// Groups diagnostics by file. For each file shows the first 3 errors and
/// "...N more" if there are additional errors. Appends the summary line.
/// Target: 70% savings.
pub fn filter_tsc(input: &str) -> String {
    if input.trim().is_empty() {
        return String::new();
    }

    // file -> Vec<(line_no, ts_code, message)>
    let mut file_errors: HashMap<String, Vec<(u32, String, String)>> = HashMap::new();
    let mut file_order: Vec<String> = Vec::new();
    let mut summary_line: Option<&str> = None;

    for line in input.lines() {
        if TSC_SUMMARY_RE.is_match(line) {
            summary_line = Some(line);
            continue;
        }
        if let Some(caps) = TSC_DIAG_RE.captures(line) {
            let file = caps.get(1).map_or("", |m| m.as_str()).to_string();
            let line_no: u32 = caps
                .get(2)
                .and_then(|m| m.as_str().parse().ok())
                .unwrap_or(0);
            let ts_code = caps.get(3).map_or("", |m| m.as_str()).to_string();
            let message = caps.get(4).map_or("", |m| m.as_str()).to_string();

            if !file_errors.contains_key(&file) {
                file_order.push(file.clone());
                file_errors.insert(file.clone(), Vec::new());
            }
            if let Some(errs) = file_errors.get_mut(&file) {
                errs.push((line_no, ts_code, message));
            }
        }
    }

    if file_errors.is_empty() && summary_line.is_none() {
        // Unknown format — passthrough
        return input.trim_end().to_string();
    }

    let mut out: Vec<String> = Vec::new();

    for file in &file_order {
        if let Some(errors) = file_errors.get(file) {
            let total = errors.len();
            out.push(format!(
                "{} ({} error{})",
                file,
                total,
                if total == 1 { "" } else { "s" }
            ));
            for (line_no, ts_code, message) in errors.iter().take(MAX_PER_FILE) {
                out.push(format!("  {}:{}: {}", line_no, ts_code, message));
            }
            if total > MAX_PER_FILE {
                out.push(format!("  ...{} more", total - MAX_PER_FILE));
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
    fn test_tsc_format() {
        let input = include_str!("../../../tests/fixtures/js/tsc_raw.txt");
        let output = filter_tsc(input);
        assert_snapshot!(output);
    }

    #[test]
    fn test_tsc_savings() {
        let input = include_str!("../../../tests/fixtures/js/tsc_raw.txt");
        let output = filter_tsc(input);
        let input_t = count_tokens(input);
        let output_t = count_tokens(&output);
        let savings = 100.0 - (output_t as f64 / input_t as f64 * 100.0);
        assert!(
            savings >= 10.0,
            "Expected >=10% savings on tsc, got {:.1}% ({} -> {} tokens)",
            savings,
            input_t,
            output_t
        );
    }

    #[test]
    fn test_tsc_empty() {
        assert_eq!(filter_tsc(""), "");
        assert_eq!(filter_tsc("   \n  "), "");
    }

    #[test]
    fn test_tsc_malformed() {
        let input = "not tsc output\nsome random text";
        let output = filter_tsc(input);
        assert!(
            !output.is_empty(),
            "Should not return empty for unknown input"
        );
    }

    #[test]
    fn test_tsc_groups_by_file() {
        let input = "src/foo.ts(1,1): error TS2322: Type mismatch.\nsrc/foo.ts(2,5): error TS2339: Property missing.\nsrc/bar.ts(3,1): error TS7006: Implicit any.\n\nFound 3 errors.\n";
        let output = filter_tsc(input);
        assert!(output.contains("src/foo.ts"), "Should include foo.ts group");
        assert!(output.contains("src/bar.ts"), "Should include bar.ts group");
        assert!(output.contains("Found 3 errors"), "Should keep summary");
    }

    #[test]
    fn test_tsc_truncates_per_file() {
        let input = "src/foo.ts(1,1): error TS2322: Error one.\nsrc/foo.ts(2,2): error TS2339: Error two.\nsrc/foo.ts(3,3): error TS7006: Error three.\nsrc/foo.ts(4,4): error TS2305: Error four.\nsrc/foo.ts(5,5): error TS2307: Error five.\n\nFound 5 errors.\n";
        let output = filter_tsc(input);
        assert!(
            output.contains("...2 more"),
            "Should show '...N more' for truncated errors"
        );
    }

    #[test]
    fn test_tsc_keeps_summary() {
        let input = "src/foo.ts(1,1): error TS2322: Type mismatch.\n\nFound 1 error.\n";
        let output = filter_tsc(input);
        assert!(output.contains("Found 1 error"), "Should keep summary line");
    }
}

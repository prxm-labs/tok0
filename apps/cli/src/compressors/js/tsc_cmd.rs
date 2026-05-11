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

/// When a `(ts_code, message)` pair appears in this many distinct files, it
/// is consolidated into a single line rather than repeated per-file. Saves
/// a lot of tokens on mass-rename / shared-type migrations.
const CROSS_FILE_DEDUP_THRESHOLD: usize = 3;
const CROSS_FILE_SAMPLE: usize = 4;

/// Filter `tsc --noEmit` output.
///
/// Groups diagnostics by file. For each file shows the first 3 errors and
/// "...N more" if there are additional errors. Appends the summary line.
///
/// Cross-file dedup: when the same `(error code, message)` appears in ≥3
/// distinct files (mass-rename / shared-type migration), it is consolidated
/// into a single line with a sample of file:line locations instead of
/// repeating the same error in every file's per-file section.
///
/// Target: 70% savings on typical input; ≥90% on mass-rename scenarios.
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

    // Cross-file dedup: collect (code, message) → Vec<(file, line)>.
    type ErrorKey = (String, String);
    type FileLine = (String, u32);
    let mut by_error: HashMap<ErrorKey, Vec<FileLine>> = HashMap::new();
    for file in &file_order {
        if let Some(errs) = file_errors.get(file) {
            for (line_no, code, msg) in errs {
                by_error
                    .entry((code.clone(), msg.clone()))
                    .or_default()
                    .push((file.clone(), *line_no));
            }
        }
    }

    let mut consolidated: Vec<(ErrorKey, Vec<FileLine>)> = by_error
        .into_iter()
        .filter(|(_, occs)| {
            occs.iter()
                .map(|(f, _)| f.as_str())
                .collect::<std::collections::HashSet<_>>()
                .len()
                >= CROSS_FILE_DEDUP_THRESHOLD
        })
        .collect();
    // Stable order: by code, then message.
    consolidated.sort_by(|a, b| a.0.cmp(&b.0));

    // Remove consolidated errors from per-file groups.
    let dedup_keys: std::collections::HashSet<ErrorKey> =
        consolidated.iter().map(|(k, _)| k.clone()).collect();
    for errs in file_errors.values_mut() {
        errs.retain(|(_, code, msg)| !dedup_keys.contains(&(code.clone(), msg.clone())));
    }

    let mut out: Vec<String> = Vec::new();

    for ((code, message), occs) in &consolidated {
        let distinct_files: Vec<&String> = {
            let mut seen = std::collections::HashSet::new();
            let mut v: Vec<&String> = Vec::new();
            for (f, _) in occs {
                if seen.insert(f.as_str()) {
                    v.push(f);
                }
            }
            v
        };
        out.push(format!(
            "{} in {} files: {}",
            code,
            distinct_files.len(),
            message
        ));
        let sample: Vec<String> = occs
            .iter()
            .take(CROSS_FILE_SAMPLE)
            .map(|(f, l)| format!("{f}:{l}"))
            .collect();
        let more = occs.len().saturating_sub(CROSS_FILE_SAMPLE);
        let tail = if more > 0 {
            format!(" (+{more} more)")
        } else {
            String::new()
        };
        out.push(format!("  {}{}", sample.join(", "), tail));
    }

    for file in &file_order {
        if let Some(errors) = file_errors.get(file) {
            if errors.is_empty() {
                continue;
            }
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

    // ── Phase 3: cross-file dedup for mass-rename scenarios ───────────

    #[test]
    fn test_tsc_collapses_same_error_across_files() {
        // 5 files, all with the same TS2322 message — should consolidate.
        let input = "\
src/a.ts(1,1): error TS2322: Type 'X' is not assignable to type 'Y'.
src/b.ts(2,1): error TS2322: Type 'X' is not assignable to type 'Y'.
src/c.ts(3,1): error TS2322: Type 'X' is not assignable to type 'Y'.
src/d.ts(4,1): error TS2322: Type 'X' is not assignable to type 'Y'.
src/e.ts(5,1): error TS2322: Type 'X' is not assignable to type 'Y'.

Found 5 errors.
";
        let output = filter_tsc(input);
        // Consolidated form should NOT have 5 separate per-file sections.
        assert!(
            !output.contains("src/a.ts (1 error)"),
            "expected consolidated form, not per-file groups, got:\n{output}"
        );
        // Should mention the error code + count across files
        assert!(
            output.contains("TS2322") && output.contains("5 files"),
            "expected 'TS2322 ... 5 files' consolidation, got:\n{output}"
        );
        // Summary preserved
        assert!(output.contains("Found 5 errors"));
    }

    #[test]
    fn test_tsc_preserves_per_file_when_unique() {
        // All errors unique — should NOT consolidate.
        let input = "\
src/a.ts(1,1): error TS2322: Type 'X' is not assignable to type 'Y'.
src/b.ts(2,1): error TS2339: Property 'foo' missing.
src/c.ts(3,1): error TS7006: Implicit any.

Found 3 errors.
";
        let output = filter_tsc(input);
        assert!(output.contains("src/a.ts"));
        assert!(output.contains("src/b.ts"));
        assert!(output.contains("src/c.ts"));
        assert!(
            !output.contains("files:"),
            "should not consolidate uniques: {output}"
        );
    }

    #[test]
    fn test_tsc_mixed_consolidates_dups_keeps_uniques() {
        // 3 duplicates across files + 1 unique.
        let input = "\
src/a.ts(1,1): error TS2322: Type 'X' is not assignable to type 'Y'.
src/b.ts(2,1): error TS2322: Type 'X' is not assignable to type 'Y'.
src/c.ts(3,1): error TS2322: Type 'X' is not assignable to type 'Y'.
src/unique.ts(99,1): error TS7006: Parameter implicitly any.

Found 4 errors.
";
        let output = filter_tsc(input);
        assert!(
            output.contains("TS2322") && output.contains("3 files"),
            "should consolidate the 3 TS2322 dups, got:\n{output}"
        );
        assert!(
            output.contains("src/unique.ts"),
            "should keep unique error in per-file section, got:\n{output}"
        );
        assert!(output.contains("Found 4 errors"));
    }
}

use lazy_static::lazy_static;
use regex::Regex;
use std::collections::BTreeMap;

lazy_static! {
    /// Match `wc -l` per-file lines: "   123 path/to/file.ext"
    static ref WC_FILE_RE: Regex = Regex::new(r"^\s*(\d+)\s+(.+)$").unwrap();
    /// Match the "total" summary line
    static ref WC_TOTAL_RE: Regex = Regex::new(r"^\s*(\d+)\s+total$").unwrap();
    /// Extract file extension
    static ref EXT_RE: Regex = Regex::new(r"\.([a-zA-Z0-9]+)$").unwrap();
}

/// Filter `wc -l` output: summarize total line count, group by extension for many files.
pub fn filter_wc(input: &str) -> String {
    if input.trim().is_empty() {
        return String::new();
    }

    let lines: Vec<&str> = input.lines().collect();

    // Separate total line from file lines
    let mut total_count: Option<u64> = None;
    let mut file_entries: Vec<(u64, &str)> = Vec::new();

    for line in &lines {
        if let Some(caps) = WC_TOTAL_RE.captures(line) {
            total_count = caps[1].parse().ok();
        } else if let Some(caps) = WC_FILE_RE.captures(line) {
            let count: u64 = caps[1].parse().unwrap_or(0);
            let path = line[caps.get(1).unwrap().end()..].trim();
            file_entries.push((count, path));
        }
    }

    // If only one file (or no total line), just return compact form
    if file_entries.len() <= 1 {
        if let Some((count, path)) = file_entries.first() {
            return format!("{}: {} lines", path, count);
        }
        // Passthrough if we couldn't parse anything meaningful
        return input.to_string();
    }

    let total = total_count.unwrap_or_else(|| file_entries.iter().map(|(c, _)| c).sum());

    // For many files, group by extension
    if file_entries.len() > 10 {
        let mut by_ext: BTreeMap<&str, (u64, usize)> = BTreeMap::new();
        for (count, path) in &file_entries {
            let ext = EXT_RE
                .captures(path)
                .map(|c| c.get(1).unwrap().as_str())
                .unwrap_or("(no ext)");
            let entry = by_ext.entry(ext).or_insert((0, 0));
            entry.0 += count;
            entry.1 += 1;
        }

        let mut result = format!(
            "total: {} lines across {} files\n\nby extension:\n",
            total,
            file_entries.len()
        );

        // Sort by line count descending
        let mut ext_vec: Vec<(&str, u64, usize)> = by_ext
            .iter()
            .map(|(ext, (lines, files))| (*ext, *lines, *files))
            .collect();
        ext_vec.sort_by_key(|e| std::cmp::Reverse(e.1));

        for (ext, lines, files) in &ext_vec {
            result.push_str(&format!(
                "  .{:<12} {:>8} lines  ({} files)\n",
                ext, lines, files
            ));
        }

        // Show top 5 largest individual files
        let mut sorted = file_entries.clone();
        sorted.sort_by_key(|s| std::cmp::Reverse(s.0));
        result.push_str("\ntop files:\n");
        for (count, path) in sorted.iter().take(5) {
            result.push_str(&format!("  {:>8}  {}\n", count, path));
        }

        return result.trim_end().to_string();
    }

    // For moderate file counts (2-10), list all with total
    let mut result = format!("total: {} lines\n", total);
    for (count, path) in &file_entries {
        result.push_str(&format!("  {:>8}  {}\n", count, path));
    }
    result.trim_end().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use insta::assert_snapshot;

    fn count_tokens(s: &str) -> usize {
        s.split_whitespace().count()
    }

    /// Generate a large wc -l output: 50 files across many extensions.
    fn make_large_wc_input() -> String {
        let exts = [
            "rs", "ts", "js", "py", "go", "md", "toml", "json", "yaml", "sh",
        ];
        let mut lines: Vec<String> = Vec::new();
        let mut grand_total: u64 = 0;
        for i in 0..50usize {
            let ext = exts[i % exts.len()];
            let count = ((i + 1) * 50) as u64;
            grand_total += count;
            // Realistic path with long directory names to boost input token count
            lines.push(format!(
                "  {}  src/some_long_module_name_{}/another_directory/file_{}.{}",
                count, i, i, ext
            ));
        }
        lines.push(format!("  {}  total", grand_total));
        lines.join("\n")
    }

    #[test]
    fn test_wc_empty() {
        assert_eq!(filter_wc(""), "");
    }

    #[test]
    fn test_wc_single_file() {
        let input = "  42 src/main.rs";
        let output = filter_wc(input);
        assert!(output.contains("42"), "should contain line count");
        assert!(output.contains("main.rs"), "should contain filename");
    }

    #[test]
    fn test_wc_few_files() {
        let input = "  100 src/main.rs\n   50 src/lib.rs\n  150 total";
        let output = filter_wc(input);
        assert!(output.contains("150"), "should contain total");
        assert!(output.contains("main.rs"));
        assert!(output.contains("lib.rs"));
    }

    #[test]
    fn test_wc_many_files_snapshot() {
        let input = make_large_wc_input();
        let output = filter_wc(&input);
        assert_snapshot!(output);
    }

    #[test]
    fn test_wc_many_files_savings() {
        let input = make_large_wc_input();
        let output = filter_wc(&input);
        let input_t = count_tokens(&input);
        let output_t = count_tokens(&output);
        let savings = 100.0 - (output_t as f64 / input_t as f64 * 100.0);
        assert!(
            savings >= 0.0,
            "Expected non-negative savings, got {:.1}% ({} -> {} tokens)",
            savings,
            input_t,
            output_t
        );
    }

    #[test]
    fn test_wc_groups_by_extension() {
        let input = make_large_wc_input();
        let output = filter_wc(&input);
        assert!(
            output.contains("by extension:"),
            "should group by extension"
        );
        assert!(output.contains("top files:"), "should show top files");
    }

    #[test]
    fn test_wc_passthrough_unparseable() {
        let input = "not wc output at all";
        let output = filter_wc(input);
        assert!(
            !output.is_empty(),
            "should not return empty for unparseable input"
        );
    }
}

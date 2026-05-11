use lazy_static::lazy_static;
use regex::Regex;
use std::collections::BTreeMap;

lazy_static! {
    /// Match grep -n output: file:line:content
    static ref GREP_LINE_RE: Regex = Regex::new(r"^([^:]+):(\d+):(.*)$").unwrap();
}

/// Filter grep output: group by file, limit matches per file, show counts.
pub fn filter_grep(input: &str, max_results: usize, max_per_file: usize) -> String {
    if input.trim().is_empty() {
        return String::new();
    }

    let lines: Vec<&str> = input.lines().filter(|l| !l.trim().is_empty()).collect();
    let total = lines.len();

    if total <= max_results {
        return input.to_string();
    }

    // Group by file
    let mut by_file: BTreeMap<&str, Vec<(&str, &str)>> = BTreeMap::new();
    for line in &lines {
        if let Some(caps) = GREP_LINE_RE.captures(line) {
            let file = caps.get(1).unwrap().as_str();
            let lineno = caps.get(2).unwrap().as_str();
            let content = caps.get(3).unwrap().as_str();
            by_file.entry(file).or_default().push((lineno, content));
        }
    }

    let mut result = format!("{} matches across {} files\n", total, by_file.len());
    let mut shown = 0;

    for (file, matches) in &by_file {
        if shown >= max_results {
            break;
        }
        result.push_str(&format!("\n{}  ({} matches)\n", file, matches.len()));
        let to_show = matches.len().min(max_per_file);
        for (lineno, content) in matches.iter().take(to_show) {
            result.push_str(&format!("  {}:{}\n", lineno, content.trim()));
            shown += 1;
        }
        if matches.len() > max_per_file {
            result.push_str(&format!("  ... +{} more\n", matches.len() - max_per_file));
        }
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

    #[test]
    fn test_grep_output_format() {
        let input = include_str!("../../../tests/fixtures/system/grep_raw.txt");
        let output = filter_grep(input, 100, 10);
        assert_snapshot!(output);
    }

    #[test]
    fn test_grep_savings() {
        let input = include_str!("../../../tests/fixtures/system/grep_raw.txt");
        let output = filter_grep(input, 100, 25);
        let input_t = count_tokens(input);
        let output_t = count_tokens(&output);
        if input_t > 20 {
            let savings = 100.0 - (output_t as f64 / input_t as f64 * 100.0);
            assert!(
                savings >= 30.0,
                "Expected >=30% savings, got {:.1}% ({} -> {} tokens)",
                savings,
                input_t,
                output_t
            );
        }
    }

    #[test]
    fn test_grep_empty() {
        assert_eq!(filter_grep("", 100, 10), "");
    }

    #[test]
    fn test_grep_small_results() {
        let input = "src/main.rs:1:fn main() {";
        assert_eq!(filter_grep(input, 100, 10), input);
    }

    #[test]
    fn test_grep_unicode() {
        let input = "src/test.rs:1:// 日本語コメント";
        let output = filter_grep(input, 100, 10);
        assert!(output.contains("日本語"));
    }
}

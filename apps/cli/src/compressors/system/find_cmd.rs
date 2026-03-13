use lazy_static::lazy_static;
use regex::Regex;
use std::collections::BTreeMap;

lazy_static! {
    /// Match file extension from path
    static ref EXT_RE: Regex = Regex::new(r"\.([a-zA-Z0-9]+)$").unwrap();
}

/// Filter find output: group by extension, show counts, truncate long lists.
pub fn filter_find(input: &str) -> String {
    if input.trim().is_empty() {
        return String::new();
    }

    let lines: Vec<&str> = input.lines().filter(|l| !l.trim().is_empty()).collect();
    let total = lines.len();

    if total <= 10 {
        return input.to_string();
    }

    // Group by extension
    let mut by_ext: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    for line in &lines {
        let ext = EXT_RE
            .captures(line)
            .map(|c| c.get(1).unwrap().as_str())
            .unwrap_or("(no ext)");
        by_ext.entry(ext).or_default().push(line);
    }

    let mut result = format!("{} files found\n", total);
    for (ext, files) in &by_ext {
        if files.len() <= 5 {
            result.push_str(&format!("\n.{} ({}):\n", ext, files.len()));
            for f in files {
                result.push_str(&format!("  {}\n", f));
            }
        } else {
            result.push_str(&format!("\n.{} ({}):\n", ext, files.len()));
            for f in files.iter().take(3) {
                result.push_str(&format!("  {}\n", f));
            }
            result.push_str(&format!("  ... +{} more\n", files.len() - 3));
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
    fn test_find_output_format() {
        let input = include_str!("../../../tests/fixtures/system/find_raw.txt");
        let output = filter_find(input);
        assert_snapshot!(output);
    }

    #[test]
    fn test_find_savings() {
        let input = include_str!("../../../tests/fixtures/system/find_raw.txt");
        let output = filter_find(input);
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
    fn test_find_empty() {
        assert_eq!(filter_find(""), "");
    }

    #[test]
    fn test_find_small_list() {
        let input = "./src/main.rs\n./src/lib.rs";
        assert_eq!(filter_find(input), input);
    }

    #[test]
    fn test_find_unicode() {
        let input = "./src/名前.rs\n./src/αβγ.rs";
        let output = filter_find(input);
        assert!(!output.is_empty());
    }
}

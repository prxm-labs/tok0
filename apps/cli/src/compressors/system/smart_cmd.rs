use lazy_static::lazy_static;
use regex::Regex;

lazy_static! {
    /// Match common code constructs
    static ref FN_RE: Regex = Regex::new(r"^\s*(pub\s+)?(fn|async fn|def|function|class|struct|enum|trait|impl|interface|type)\s+").unwrap();
    static ref IMPORT_RE: Regex = Regex::new(r"^\s*(use|import|from|require|include|#include)\s+").unwrap();
    static ref COMMENT_RE: Regex = Regex::new(r"^\s*(//|#|/\*|\*|<!--)").unwrap();
    static ref BLANK_RE: Regex = Regex::new(r"^\s*$").unwrap();
}

/// Smart filter: extract code structure (definitions, imports), strip blanks and comments.
pub fn filter_smart(input: &str) -> String {
    if input.trim().is_empty() {
        return String::new();
    }

    let lines: Vec<&str> = input.lines().collect();
    let total = lines.len();

    if total <= 30 {
        return input.to_string();
    }

    let mut definitions: Vec<String> = Vec::new();
    let mut imports: Vec<&str> = Vec::new();
    let mut blank_count: usize = 0;
    let mut comment_count: usize = 0;

    for (i, line) in lines.iter().enumerate() {
        if BLANK_RE.is_match(line) {
            blank_count += 1;
        } else if COMMENT_RE.is_match(line) {
            comment_count += 1;
        } else if FN_RE.is_match(line) {
            definitions.push(format!("  L{}: {}", i + 1, line.trim()));
        } else if IMPORT_RE.is_match(line) {
            imports.push(line.trim());
        }
    }

    let code_lines = total - blank_count - comment_count;
    let mut result = format!(
        "{} lines ({} code, {} blank, {} comments)\n",
        total, code_lines, blank_count, comment_count,
    );

    if !imports.is_empty() {
        result.push_str(&format!("\nImports ({}):\n", imports.len()));
        for imp in imports.iter().take(10) {
            result.push_str(&format!("  {}\n", imp));
        }
        if imports.len() > 10 {
            result.push_str(&format!("  ... +{} more\n", imports.len() - 10));
        }
    }

    if !definitions.is_empty() {
        result.push_str(&format!("\nDefinitions ({}):\n", definitions.len()));
        for def in &definitions {
            result.push_str(&format!("{}\n", def));
        }
    }

    result.trim_end().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn count_tokens(s: &str) -> usize {
        s.split_whitespace().count()
    }

    #[test]
    fn test_smart_small_file() {
        let input = "fn main() {\n    println!(\"hello\");\n}";
        assert_eq!(filter_smart(input), input);
    }

    #[test]
    fn test_smart_large_file() {
        let mut lines = vec![
            "use anyhow::Result;".to_string(),
            "use std::io;".to_string(),
            "".to_string(),
            "// This is a comment".to_string(),
            "pub fn main() {".to_string(),
        ];
        for i in 0..40 {
            lines.push(format!("    let x{} = {};", i, i));
        }
        lines.push("}".to_string());
        lines.push("".to_string());
        lines.push("pub fn helper() {".to_string());
        lines.push("}".to_string());

        let input = lines.join("\n");
        let output = filter_smart(&input);

        assert!(output.contains("lines"));
        assert!(output.contains("Definitions"));
        assert!(output.contains("pub fn main"));
        assert!(output.contains("pub fn helper"));

        let savings = 100.0 - (count_tokens(&output) as f64 / count_tokens(&input) as f64 * 100.0);
        assert!(
            savings >= 40.0,
            "Expected >=40% savings, got {:.1}%",
            savings
        );
    }

    #[test]
    fn test_smart_empty() {
        assert_eq!(filter_smart(""), "");
    }

    #[test]
    fn test_smart_unicode() {
        let input = "// 日本語コメント\nfn テスト() {}";
        let output = filter_smart(input);
        assert!(!output.is_empty());
    }
}

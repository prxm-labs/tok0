use crate::engine::shell;

/// Filter file read output: add line numbers, truncate long lines, head/tail for large files.
pub fn filter_read(input: &str, path: &str) -> String {
    if input.trim().is_empty() {
        return format!("(empty file: {})", path);
    }

    let lines: Vec<&str> = input.lines().collect();
    let total = lines.len();
    let max_lines = 100;
    let max_line_chars = 200;

    if total <= max_lines {
        // Small file: add line numbers
        let numbered: Vec<String> = lines
            .iter()
            .enumerate()
            .map(|(i, line)| {
                let truncated = shell::truncate_line(line, max_line_chars);
                format!("{:>4}  {}", i + 1, truncated)
            })
            .collect();
        return format!("{}  ({} lines)\n{}", path, total, numbered.join("\n"));
    }

    // Large file: head/tail with summary
    let head = 40;
    let tail = 20;
    let mut result = format!(
        "{}  ({} lines, showing first {} + last {})\n",
        path, total, head, tail
    );

    for (i, line) in lines.iter().take(head).enumerate() {
        let truncated = shell::truncate_line(line, max_line_chars);
        result.push_str(&format!("{:>4}  {}\n", i + 1, truncated));
    }

    result.push_str(&format!(
        "\n... ({} lines omitted) ...\n\n",
        total - head - tail
    ));

    for (i, line) in lines.iter().skip(total - tail).enumerate() {
        let lineno = total - tail + i + 1;
        let truncated = shell::truncate_line(line, max_line_chars);
        result.push_str(&format!("{:>4}  {}\n", lineno, truncated));
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
    fn test_read_small_file() {
        let input = "line one\nline two\nline three";
        let output = filter_read(input, "test.txt");
        assert!(output.contains("test.txt"));
        assert!(output.contains("3 lines"));
        assert!(output.contains("line one"));
    }

    #[test]
    fn test_read_large_file() {
        let lines: Vec<String> = (1..=200).map(|i| format!("content line {}", i)).collect();
        let input = lines.join("\n");
        let output = filter_read(&input, "big.txt");
        assert!(output.contains("200 lines"));
        assert!(output.contains("omitted"));
        assert!(output.contains("content line 1"));
        assert!(output.contains("content line 200"));

        let savings = 100.0 - (count_tokens(&output) as f64 / count_tokens(&input) as f64 * 100.0);
        assert!(
            savings >= 50.0,
            "Expected >=50% savings for large file, got {:.1}%",
            savings
        );
    }

    #[test]
    fn test_read_empty() {
        let output = filter_read("", "empty.txt");
        assert!(output.contains("empty"));
    }

    #[test]
    fn test_read_unicode() {
        let input = "日本語テキスト\nαβγδ\n🎉";
        let output = filter_read(input, "unicode.txt");
        assert!(output.contains("日本語") || output.contains("αβγ"));
    }

    #[test]
    fn test_read_long_lines() {
        let long_line = "x".repeat(500);
        let output = filter_read(&long_line, "long.txt");
        assert!(output.len() < 500 + 50); // line number prefix overhead
    }
}

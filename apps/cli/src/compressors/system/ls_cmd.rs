use lazy_static::lazy_static;
use regex::Regex;

lazy_static! {
    /// Matches ls -la output lines: permissions, links, owner, group, size, date, name
    static ref LS_LINE_RE: Regex = Regex::new(
        r"^([drwx\-lsStT@+]{10,})\s+\d+\s+\S+\s+\S+\s+(\d+)\s+\w+\s+\d+\s+[\d:]+\s+(.+)$"
    ).unwrap();
    /// Matches the "total N" header line
    static ref TOTAL_RE: Regex = Regex::new(r"^total \d+").unwrap();
}

/// Filter ls output to a compact tree view: strip permissions/owner/date, keep name + size for files.
pub fn filter_ls(input: &str, path: &str) -> String {
    if input.trim().is_empty() {
        return String::new();
    }

    let lines: Vec<&str> = input.lines().collect();
    let mut entries: Vec<String> = Vec::new();
    let mut dir_count: usize = 0;
    let mut file_count: usize = 0;

    for line in &lines {
        if TOTAL_RE.is_match(line) {
            continue;
        }
        if let Some(caps) = LS_LINE_RE.captures(line) {
            let perms = &caps[1];
            let size: u64 = caps[2].parse().unwrap_or(0);
            let name = &caps[3];

            // Skip . and ..
            if name == "." || name == ".." {
                continue;
            }

            let is_dir = perms.starts_with('d');
            let is_link = perms.starts_with('l');

            if is_dir {
                dir_count += 1;
                entries.push(format!("  {}/", name));
            } else if is_link {
                entries.push(format!(
                    "  {} ->",
                    name.split(" -> ").next().unwrap_or(name)
                ));
            } else {
                file_count += 1;
                entries.push(format!("  {} ({})", name, format_size(size)));
            }
        }
    }

    if entries.is_empty() {
        return input.to_string();
    }

    let mut result = format!("{}  ({} dirs, {} files)\n", path, dir_count, file_count);
    result.push_str(&entries.join("\n"));
    result
}

fn format_size(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{}B", bytes)
    } else if bytes < 1024 * 1024 {
        format!("{:.1}K", bytes as f64 / 1024.0)
    } else if bytes < 1024 * 1024 * 1024 {
        format!("{:.1}M", bytes as f64 / (1024.0 * 1024.0))
    } else {
        format!("{:.1}G", bytes as f64 / (1024.0 * 1024.0 * 1024.0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use insta::assert_snapshot;

    fn count_tokens(s: &str) -> usize {
        s.split_whitespace().count()
    }

    #[test]
    fn test_ls_output_format() {
        let input = include_str!("../../../tests/fixtures/system/ls_raw.txt");
        let output = filter_ls(input, ".");
        assert_snapshot!(output);
    }

    #[test]
    fn test_ls_savings() {
        let input = include_str!("../../../tests/fixtures/system/ls_raw.txt");
        let output = filter_ls(input, ".");
        let input_t = count_tokens(input);
        let output_t = count_tokens(&output);
        if input_t > 10 {
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
    fn test_ls_empty() {
        assert_eq!(filter_ls("", "."), "");
    }

    #[test]
    fn test_ls_malformed() {
        let output = filter_ls("not ls output\nrandom text", ".");
        assert!(!output.is_empty(), "Should return input on malformed data");
    }

    #[test]
    fn test_ls_unicode() {
        let input = "total 0\ndrwxr-xr-x  2 user staff  64 Jan  1 00:00 名前\n-rw-r--r--  1 user staff  42 Jan  1 00:00 αβγ.txt";
        let output = filter_ls(input, ".");
        assert!(output.contains("名前") || output.contains("αβγ"));
    }

    #[test]
    fn test_ls_ansi() {
        let input = "\x1b[32mtotal 0\x1b[0m\n-rw-r--r--  1 user staff  42 Jan  1 00:00 file.txt";
        let stripped = crate::engine::shell::strip_ansi(input);
        let output = filter_ls(&stripped, ".");
        assert!(output.contains("file.txt"));
    }

    #[test]
    fn test_format_size() {
        assert_eq!(format_size(0), "0B");
        assert_eq!(format_size(512), "512B");
        assert_eq!(format_size(1024), "1.0K");
        assert_eq!(format_size(1048576), "1.0M");
    }
}

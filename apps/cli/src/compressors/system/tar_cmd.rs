use lazy_static::lazy_static;
use regex::Regex;
use std::collections::BTreeMap;

lazy_static! {
    /// Match a `tar -tvf` row: "perms <links?> owner group size <date...> name"
    /// macOS BSD tar: `-rw-r--r--  0 theodorevorillas staff     314 May  1 01:10 ./ava.toml`
    /// GNU tar:       `-rw-r--r-- user/group         314 2026-05-01 01:10 ./ava.toml`
    /// Captures (perms, size, name).
    static ref ROW_RE: Regex = Regex::new(
        r"^([dl\-][rwxstST\-]{9})\s+\S+\s+\S+(?:\s+\S+)?\s+(\d+)\s+\S+\s+\S+(?:\s+\S+)?\s+(.+)$"
    ).unwrap();
    static ref EXT_RE: Regex = Regex::new(r"\.([a-zA-Z0-9]+)$").unwrap();
}

/// Filter `tar -tvf` / `tar -tzf` output to a compact form: count + size summary,
/// list a sample of paths. For huge archives, group by extension.
pub fn filter_tar(input: &str) -> String {
    if input.trim().is_empty() {
        return String::new();
    }

    let entries: Vec<(u64, String, bool)> = input
        .lines()
        .filter_map(|line| {
            let caps = ROW_RE.captures(line)?;
            let size: u64 = caps[2].parse().ok()?;
            let perms = &caps[1];
            let is_dir = perms.starts_with('d');
            Some((size, caps[3].to_string(), is_dir))
        })
        .collect();

    if entries.is_empty() {
        return input.to_string();
    }

    let dirs = entries.iter().filter(|(_, _, d)| *d).count();
    let files = entries.len() - dirs;
    let total_bytes: u64 = entries
        .iter()
        .filter(|(_, _, d)| !*d)
        .map(|(s, _, _)| s)
        .sum();

    let mut out = format!(
        "archive: {} files, {} dirs, {}\n",
        files,
        dirs,
        format_size(total_bytes)
    );

    // For huge archives, group by extension
    if entries.len() > 50 {
        let mut by_ext: BTreeMap<&str, (u64, usize)> = BTreeMap::new();
        for (size, name, is_dir) in &entries {
            if *is_dir {
                continue;
            }
            let ext = EXT_RE
                .captures(name)
                .and_then(|c| c.get(1))
                .map(|m| m.as_str())
                .unwrap_or("(no ext)");
            let entry = by_ext.entry(ext).or_insert((0, 0));
            entry.0 += size;
            entry.1 += 1;
        }

        let mut ext_vec: Vec<(&str, u64, usize)> = by_ext
            .iter()
            .map(|(ext, (sz, n))| (*ext, *sz, *n))
            .collect();
        ext_vec.sort_by_key(|e| std::cmp::Reverse(e.2));

        out.push_str("\nby extension:\n");
        for (ext, sz, n) in &ext_vec {
            out.push_str(&format!(
                "  .{:<12} {:>5} files  {}\n",
                ext,
                n,
                format_size(*sz)
            ));
        }

        // Top 5 largest files
        let mut sorted: Vec<&(u64, String, bool)> =
            entries.iter().filter(|(_, _, d)| !*d).collect();
        sorted.sort_by_key(|(s, _, _)| std::cmp::Reverse(*s));
        out.push_str("\ntop files:\n");
        for (sz, name, _) in sorted.iter().take(5) {
            out.push_str(&format!("  {:>10}  {}\n", format_size(*sz), name));
        }

        return out.trim_end().to_string();
    }

    // Moderate-size archive: list entries by name (sized).
    out.push_str("\nentries:\n");
    for (sz, name, is_dir) in &entries {
        if *is_dir {
            out.push_str(&format!("  {}/\n", name.trim_end_matches('/')));
        } else {
            out.push_str(&format!("  {} ({})\n", name, format_size(*sz)));
        }
    }
    out.trim_end().to_string()
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
    fn test_tar_tvf_format() {
        let input = include_str!("../../../tests/fixtures/system/tar_tvf_raw.txt");
        let output = filter_tar(input);
        assert_snapshot!(output);
    }

    #[test]
    fn test_tar_tvf_savings() {
        let input = include_str!("../../../tests/fixtures/system/tar_tvf_raw.txt");
        let output = filter_tar(input);
        let input_t = count_tokens(input);
        let output_t = count_tokens(&output);
        let savings = 100.0 - (output_t as f64 / input_t as f64 * 100.0);
        assert!(
            savings >= 60.0,
            "Expected >=60% savings, got {:.1}% ({} -> {} tokens)",
            savings,
            input_t,
            output_t
        );
    }

    #[test]
    fn test_tar_empty() {
        assert_eq!(filter_tar(""), "");
    }

    #[test]
    fn test_tar_groups_by_extension_for_large_archive() {
        let input = include_str!("../../../tests/fixtures/system/tar_tvf_raw.txt");
        let output = filter_tar(input);
        // Fixture has >50 entries → expect summary view
        assert!(output.contains("by extension"));
        assert!(output.contains("top files"));
        assert!(output.contains(".toml") || output.contains("toml"));
    }

    #[test]
    fn test_tar_small_archive_lists_entries() {
        let input = "-rw-r--r--  0 user staff     100 May  1 12:00 ./a.txt\n-rw-r--r--  0 user staff     200 May  1 12:01 ./b.txt\n";
        let output = filter_tar(input);
        assert!(output.contains("a.txt"));
        assert!(output.contains("b.txt"));
        assert!(output.contains("2 files"));
    }

    #[test]
    fn test_tar_counts_dirs_separately() {
        let input = "drwxr-xr-x  0 user staff       0 May  1 12:00 ./d1/\n-rw-r--r--  0 user staff     100 May  1 12:00 ./d1/a.txt\n";
        let output = filter_tar(input);
        assert!(output.contains("1 files"));
        assert!(output.contains("1 dirs"));
    }

    #[test]
    fn test_tar_malformed_passthrough() {
        let output = filter_tar("garbage\nnot a tar listing\n");
        assert!(!output.is_empty());
    }

    #[test]
    fn test_tar_unicode_filenames() {
        let input = "-rw-r--r--  0 user staff     100 May  1 12:00 ./名前.txt\n-rw-r--r--  0 user staff      50 May  1 12:01 ./αβγ.md\n";
        let output = filter_tar(input);
        assert!(output.contains("名前") || output.contains("αβγ"));
    }
}

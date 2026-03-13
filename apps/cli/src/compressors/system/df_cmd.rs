use lazy_static::lazy_static;
use regex::Regex;

lazy_static! {
    /// Match virtual / noise filesystem types to skip
    static ref SKIP_FS_RE: Regex = Regex::new(
        r"(?i)^(tmpfs|devfs|devtmpfs|sysfs|proc|cgroup|overlay|none|udev|map\s|/snap/|loop\d|nsfs|hugetlbfs|mqueue|debugfs|tracefs|pstore|securityfs|fusectl|binfmt_misc|efivarfs|configfs|devpts|ramfs|rpc_pipefs|sunrpc|nfsd|autofs)"
    ).unwrap();
    /// Match noisy internal macOS volume paths
    static ref SKIP_MOUNT_RE: Regex = Regex::new(
        r"^(/System/Volumes/(VM|Preboot|Update|iSCPreboot|xarts|Hardware)|/private/var/folders|/dev$|map\s)"
    ).unwrap();
}

/// Filter `df -h` output: keep header + real mounts only (skip tmpfs, devfs, snap, loop).
pub fn filter_df(input: &str) -> String {
    if input.trim().is_empty() {
        return String::new();
    }

    let mut lines = input.lines();

    // Always keep the header (first line)
    let header = match lines.next() {
        Some(h) => h,
        None => return String::new(),
    };

    let mut result_lines: Vec<&str> = vec![header];
    let mut skipped = 0usize;

    for line in lines {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        // The filesystem name is always the first token
        let fs = trimmed.split_whitespace().next().unwrap_or("");

        // The mount point is typically the last token
        let mount = trimmed.split_whitespace().last().unwrap_or("");

        if SKIP_FS_RE.is_match(fs) || SKIP_FS_RE.is_match(mount) {
            skipped += 1;
            continue;
        }

        // Skip noisy internal mount paths
        if SKIP_MOUNT_RE.is_match(mount) {
            skipped += 1;
            continue;
        }

        result_lines.push(line);
    }

    if result_lines.len() <= 1 {
        // Only header — fallback to full input
        return input.to_string();
    }

    let mut result = result_lines.join("\n");
    if skipped > 0 {
        result.push_str(&format!(
            "\n# ({} virtual/internal mounts omitted)",
            skipped
        ));
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use insta::assert_snapshot;

    fn count_tokens(s: &str) -> usize {
        s.split_whitespace().count()
    }

    #[test]
    fn test_df_empty() {
        assert_eq!(filter_df(""), "");
    }

    #[test]
    fn test_df_macos_fixture_snapshot() {
        let input = include_str!("../../../tests/fixtures/system/df_raw.txt");
        let output = filter_df(input);
        assert_snapshot!(output);
    }

    #[test]
    fn test_df_linux_tmpfs_removed() {
        let input = "\
Filesystem      Size  Used Avail Use% Mounted on
/dev/sda1        50G   20G   28G  42% /
tmpfs           7.8G     0  7.8G   0% /dev/shm
tmpfs           1.6G  1.7M  1.6G   1% /run
/dev/sda2       200G   90G  100G  48% /home
tmpfs           5.0M     0  5.0M   0% /run/lock";
        let output = filter_df(input);
        assert!(!output.contains("tmpfs"), "tmpfs lines should be removed");
        assert!(output.contains("/dev/sda1"), "real disk should be kept");
        assert!(output.contains("/dev/sda2"), "real disk should be kept");
    }

    #[test]
    fn test_df_keeps_header() {
        let input = "\
Filesystem      Size  Used Avail Use% Mounted on
/dev/sda1        50G   20G   28G  42% /";
        let output = filter_df(input);
        assert!(
            output.starts_with("Filesystem"),
            "header should be first line"
        );
    }

    #[test]
    fn test_df_snap_mounts_removed() {
        let input = "\
Filesystem      Size  Used Avail Use% Mounted on
/dev/sda1        50G   20G   28G  42% /
/dev/loop0       56M   56M     0 100% /snap/core18/2284
/dev/loop1       64M   64M     0 100% /snap/core20/1328
/dev/sdb1       100G   10G   90G  10% /data";
        let output = filter_df(input);
        assert!(!output.contains("/snap/"), "snap mounts should be removed");
        assert!(output.contains("/dev/sdb1"), "real disk should be kept");
    }

    #[test]
    fn test_df_savings() {
        let input = include_str!("../../../tests/fixtures/system/df_raw.txt");
        let output = filter_df(input);
        let input_t = count_tokens(input);
        let output_t = count_tokens(&output);
        if input_t > 20 {
            let savings = 100.0 - (output_t as f64 / input_t as f64 * 100.0);
            assert!(
                savings >= 40.0,
                "Expected >=40% savings, got {:.1}% ({} -> {} tokens)",
                savings,
                input_t,
                output_t
            );
        }
    }

    #[test]
    fn test_df_fallback_when_all_filtered() {
        // If everything gets filtered, return original input
        let input = "\
Filesystem      Size  Used Avail Use% Mounted on
tmpfs           7.8G     0  7.8G   0% /dev/shm";
        let output = filter_df(input);
        assert!(!output.is_empty(), "should not be empty");
        // Fallback: returns original
        assert_eq!(
            output, input,
            "should fall back to input when only header survives"
        );
    }
}

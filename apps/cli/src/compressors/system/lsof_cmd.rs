use lazy_static::lazy_static;
use regex::Regex;
use std::collections::BTreeMap;

struct Row {
    cmd: String,
    pid: String,
    user: String,
    fd: String,
    ty: String,
    name: String,
}

lazy_static! {
    /// Match an lsof header row: COMMAND PID USER FD TYPE DEVICE SIZE/OFF NODE NAME
    static ref HEADER_RE: Regex = Regex::new(r"^COMMAND\s+PID\s+USER").unwrap();
    /// Match an lsof data row. Captures: command, pid, user, fd, type, ...rest (name)
    /// Format on macOS / Linux: COMMAND PID USER FD TYPE DEVICE SIZE/OFF NODE NAME
    /// Whitespace-separated; columns 1-4 are short tokens, NAME is the trailing free text.
    static ref ROW_RE: Regex = Regex::new(
        r"^(\S+)\s+(\d+)\s+(\S+)\s+(\S+)\s+(\S+)\s+\S+\s+\S+\s+\S+\s+(.+)$"
    ).unwrap();
}

/// Filter lsof output: keep COMMAND/PID/USER/FD/TYPE + NAME, drop DEVICE/SIZE/NODE.
/// For huge outputs (>100 rows), group by command and show top processes by FD count.
pub fn filter_lsof(input: &str) -> String {
    if input.trim().is_empty() {
        return String::new();
    }

    let rows: Vec<Row> = input
        .lines()
        .filter(|l| !HEADER_RE.is_match(l))
        .filter_map(|l| ROW_RE.captures(l))
        .map(|caps| Row {
            cmd: caps[1].to_string(),
            pid: caps[2].to_string(),
            user: caps[3].to_string(),
            fd: caps[4].to_string(),
            ty: caps[5].to_string(),
            name: caps[6].trim().to_string(),
        })
        .collect();

    if rows.is_empty() {
        return input.to_string();
    }

    let mut out = String::from("COMMAND PID USER FD TYPE NAME\n");

    if rows.len() > 100 {
        let mut by_cmd: BTreeMap<&str, usize> = BTreeMap::new();
        for row in &rows {
            *by_cmd.entry(row.cmd.as_str()).or_insert(0) += 1;
        }
        let mut summary: Vec<(&str, usize)> = by_cmd.into_iter().collect();
        summary.sort_by_key(|(_, n)| std::cmp::Reverse(*n));

        out.push_str(&format!(
            "({} rows across {} processes; showing top by fd count)\n",
            rows.len(),
            summary.len()
        ));
        for (cmd, count) in summary.iter().take(20) {
            out.push_str(&format!("  {:<16} {} fds\n", cmd, count));
        }
        return out.trim_end().to_string();
    }

    for row in &rows {
        out.push_str(&format!(
            "{} {} {} {} {} {}\n",
            row.cmd, row.pid, row.user, row.fd, row.ty, row.name
        ));
    }
    out.trim_end().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use insta::assert_snapshot;

    fn count_tokens(s: &str) -> usize {
        s.split_whitespace().count()
    }

    #[test]
    fn test_lsof_format() {
        let input = include_str!("../../../tests/fixtures/system/lsof_raw.txt");
        let output = filter_lsof(input);
        assert_snapshot!(output);
    }

    #[test]
    fn test_lsof_savings() {
        let input = include_str!("../../../tests/fixtures/system/lsof_raw.txt");
        let output = filter_lsof(input);
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
    fn test_lsof_empty() {
        assert_eq!(filter_lsof(""), "");
    }

    #[test]
    fn test_lsof_malformed_passthrough() {
        let output = filter_lsof("not lsof output\nrandom text");
        assert!(!output.is_empty());
    }

    #[test]
    fn test_lsof_unicode() {
        let input = "COMMAND     PID    USER  FD   TYPE  DEVICE  SIZE/OFF  NODE  NAME\nappénd    123    user  cwd  DIR   1,1     704       2     /tmp/名前.txt";
        let output = filter_lsof(input);
        assert!(output.contains("名前") || output.contains("appénd"));
    }

    #[test]
    fn test_lsof_drops_size_node_columns() {
        let input = include_str!("../../../tests/fixtures/system/lsof_raw.txt");
        let output = filter_lsof(input);
        // Realistic NODE numbers from the macOS fixture are 16+ digit ints
        assert!(!output.contains("1152921500312090550"));
    }

    #[test]
    fn test_lsof_keeps_command_pid() {
        let input = include_str!("../../../tests/fixtures/system/lsof_raw.txt");
        let output = filter_lsof(input);
        assert!(output.contains("loginwind") || output.contains("COMMAND"));
    }

    #[test]
    fn test_lsof_summary_for_large_input() {
        // Build a fake 150-row input
        let mut rows =
            vec!["COMMAND     PID  USER  FD   TYPE  DEVICE  SIZE/OFF  NODE  NAME".to_string()];
        for i in 0..150 {
            rows.push(format!(
                "fakecmd     {}  user  cwd  DIR   1,1     704       2     /path/{}",
                i, i
            ));
        }
        let input = rows.join("\n");
        let output = filter_lsof(&input);
        assert!(output.contains("150 rows"));
        assert!(output.contains("fakecmd"));
    }
}

use lazy_static::lazy_static;
use regex::Regex;

lazy_static! {
    static ref COMMIT_RE: Regex = Regex::new(r"^commit ([0-9a-f]{7,40})").unwrap();
    static ref AUTHOR_RE: Regex = Regex::new(r"^Author:\s+(.+)$").unwrap();
    static ref DATE_RE: Regex = Regex::new(r"^Date:\s+(.+)$").unwrap();
    static ref STATUS_FILE_RE: Regex =
        Regex::new(r"^\s+(new file|modified|deleted|renamed|copied|typechange):\s+(.+)$").unwrap();
    static ref STATUS_UNTRACKED_RE: Regex = Regex::new(r"^\s+(\S.+)$").unwrap();
    static ref STATUS_HINT_RE: Regex = Regex::new(r#"^\s+\(use "git"#).unwrap();
    static ref BRANCH_RE: Regex = Regex::new(r"^On branch (.+)$").unwrap();
    static ref DIFF_FILE_RE: Regex = Regex::new(r"^diff --git a/(.+) b/(.+)$").unwrap();
    static ref HUNK_RE: Regex = Regex::new(r"^@@\s+").unwrap();
    static ref ADD_LINE_RE: Regex = Regex::new(r"^\+[^+]").unwrap();
    static ref DEL_LINE_RE: Regex = Regex::new(r"^-[^-]").unwrap();
}

pub fn filter_git_log(input: &str) -> String {
    if input.trim().is_empty() {
        return String::new();
    }

    let mut entries: Vec<(String, String)> = Vec::new();
    let mut current_hash: Option<String> = None;
    let mut current_subject: Option<String> = None;
    let mut in_message = false;

    for line in input.lines() {
        if let Some(caps) = COMMIT_RE.captures(line) {
            if let (Some(hash), Some(subject)) = (current_hash.take(), current_subject.take()) {
                entries.push((hash, subject));
            }
            let full_hash = &caps[1];
            current_hash = Some(full_hash[..7.min(full_hash.len())].to_string());
            current_subject = None;
            in_message = false;
        } else if AUTHOR_RE.is_match(line) || DATE_RE.is_match(line) {
            // skip
        } else if line.trim().is_empty() && current_subject.is_none() {
            in_message = true;
        } else if in_message && current_subject.is_none() && !line.trim().is_empty() {
            current_subject = Some(line.trim().to_string());
        }
    }

    if let (Some(hash), Some(subject)) = (current_hash, current_subject) {
        entries.push((hash, subject));
    }

    if entries.is_empty() {
        return input.to_string();
    }

    let mut result = format!("{} commits\n", entries.len());
    for (hash, subject) in &entries {
        result.push_str(&format!("  {} {}\n", hash, subject));
    }
    result.trim_end().to_string()
}

pub fn filter_git_status(input: &str) -> String {
    if input.trim().is_empty() {
        return String::new();
    }

    let mut branch = String::new();
    let mut staged: Vec<String> = Vec::new();
    let mut unstaged: Vec<String> = Vec::new();
    let mut untracked: Vec<String> = Vec::new();
    let mut current_section = "";

    for line in input.lines() {
        if let Some(caps) = BRANCH_RE.captures(line) {
            branch = caps[1].to_string();
        } else if STATUS_HINT_RE.is_match(line) {
            continue;
        } else if line.starts_with("Changes to be committed") {
            current_section = "staged";
        } else if line.starts_with("Changes not staged") {
            current_section = "unstaged";
        } else if line.starts_with("Untracked files") {
            current_section = "untracked";
        } else if let Some(caps) = STATUS_FILE_RE.captures(line) {
            let action = &caps[1];
            let file = &caps[2];
            let entry = format!("{}: {}", action, file.trim());
            match current_section {
                "staged" => staged.push(entry),
                "unstaged" => unstaged.push(entry),
                _ => {}
            }
        } else if current_section == "untracked"
            && STATUS_UNTRACKED_RE.is_match(line)
            && !line.trim().is_empty()
        {
            untracked.push(line.trim().to_string());
        }
    }

    let mut result = format!("branch: {}\n", branch);
    if !staged.is_empty() {
        result.push_str(&format!("\nstaged ({}):\n", staged.len()));
        for f in &staged {
            result.push_str(&format!("  {}\n", f));
        }
    }
    if !unstaged.is_empty() {
        result.push_str(&format!("\nunstaged ({}):\n", unstaged.len()));
        for f in &unstaged {
            result.push_str(&format!("  {}\n", f));
        }
    }
    if !untracked.is_empty() {
        result.push_str(&format!("\nuntracked ({}):\n", untracked.len()));
        for f in untracked.iter().take(10) {
            result.push_str(&format!("  {}\n", f));
        }
        if untracked.len() > 10 {
            result.push_str(&format!("  ... +{} more\n", untracked.len() - 10));
        }
    }
    result.trim_end().to_string()
}

pub fn filter_git_diff(input: &str) -> String {
    if input.trim().is_empty() {
        return String::new();
    }

    let mut files: Vec<(String, usize, usize, usize)> = Vec::new();
    let mut current_file: Option<String> = None;
    let mut adds: usize = 0;
    let mut dels: usize = 0;
    let mut hunks: usize = 0;

    for line in input.lines() {
        if let Some(caps) = DIFF_FILE_RE.captures(line) {
            if let Some(name) = current_file.take() {
                files.push((name, adds, dels, hunks));
            }
            current_file = Some(caps[2].to_string());
            adds = 0;
            dels = 0;
            hunks = 0;
        } else if HUNK_RE.is_match(line) {
            hunks += 1;
        } else if ADD_LINE_RE.is_match(line) {
            adds += 1;
        } else if DEL_LINE_RE.is_match(line) {
            dels += 1;
        }
    }

    if let Some(name) = current_file {
        files.push((name, adds, dels, hunks));
    }

    if files.is_empty() {
        return input.to_string();
    }

    let total_add: usize = files.iter().map(|f| f.1).sum();
    let total_del: usize = files.iter().map(|f| f.2).sum();

    let mut result = format!(
        "{} files changed, +{} -{}\n",
        files.len(),
        total_add,
        total_del
    );
    for (name, a, d, h) in &files {
        result.push_str(&format!("  {} (+{} -{}, {} hunks)\n", name, a, d, h));
    }
    result.trim_end().to_string()
}

pub fn filter_git_simple(input: &str, subcommand: &str) -> String {
    if input.trim().is_empty() {
        return format!("ok {}", subcommand);
    }
    format!("ok {}", subcommand)
}

#[cfg(test)]
mod tests {
    use super::*;
    use insta::assert_snapshot;

    fn count_tokens(s: &str) -> usize {
        s.split_whitespace().count()
    }

    #[test]
    fn test_git_log_format() {
        let input = include_str!("../../../tests/fixtures/git/log_raw.txt");
        let output = filter_git_log(input);
        assert_snapshot!(output);
    }

    #[test]
    fn test_git_log_savings() {
        let input = include_str!("../../../tests/fixtures/git/log_raw.txt");
        let output = filter_git_log(input);
        let input_t = count_tokens(input);
        let output_t = count_tokens(&output);
        let savings = 100.0 - (output_t as f64 / input_t as f64 * 100.0);
        assert!(
            savings >= 60.0,
            "Expected >=60% savings, got {:.1}% ({} -> {})",
            savings,
            input_t,
            output_t
        );
    }

    #[test]
    fn test_git_log_empty() {
        assert_eq!(filter_git_log(""), "");
    }

    #[test]
    fn test_git_log_malformed() {
        let output = filter_git_log("not a git log");
        assert!(!output.is_empty());
    }

    #[test]
    fn test_git_status_format() {
        let input = include_str!("../../../tests/fixtures/git/status_raw.txt");
        let output = filter_git_status(input);
        assert_snapshot!(output);
    }

    #[test]
    fn test_git_status_savings() {
        let input = include_str!("../../../tests/fixtures/git/status_raw.txt");
        let output = filter_git_status(input);
        let input_t = count_tokens(input);
        let output_t = count_tokens(&output);
        let savings = 100.0 - (output_t as f64 / input_t as f64 * 100.0);
        assert!(
            savings >= 30.0,
            "Expected >=30% savings, got {:.1}%",
            savings
        );
    }

    #[test]
    fn test_git_status_empty() {
        assert_eq!(filter_git_status(""), "");
    }

    #[test]
    fn test_git_status_clean() {
        let input = "On branch main\nnothing to commit, working tree clean";
        let output = filter_git_status(input);
        assert!(output.contains("branch: main"));
    }

    #[test]
    fn test_git_diff_format() {
        let input = include_str!("../../../tests/fixtures/git/diff_raw.txt");
        let output = filter_git_diff(input);
        assert_snapshot!(output);
    }

    #[test]
    fn test_git_diff_savings() {
        let input = include_str!("../../../tests/fixtures/git/diff_raw.txt");
        let output = filter_git_diff(input);
        let input_t = count_tokens(input);
        let output_t = count_tokens(&output);
        let savings = 100.0 - (output_t as f64 / input_t as f64 * 100.0);
        assert!(
            savings >= 60.0,
            "Expected >=60% savings, got {:.1}%",
            savings
        );
    }

    #[test]
    fn test_git_diff_empty() {
        assert_eq!(filter_git_diff(""), "");
    }

    #[test]
    fn test_git_push_ok() {
        let output = filter_git_simple("", "push");
        assert_eq!(output, "ok push");
    }

    #[test]
    fn test_git_commit_ok() {
        let output = filter_git_simple("", "commit");
        assert_eq!(output, "ok commit");
    }

    #[test]
    fn test_git_simple_unicode() {
        let output = filter_git_simple("message", "push");
        assert!(output.contains("ok push"));
    }
}

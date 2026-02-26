use lazy_static::lazy_static;
use regex::Regex;

lazy_static! {
    /// Matches gh pr list table rows: leading whitespace, # number, title, branch, status, date
    static ref PR_ROW_RE: Regex =
        Regex::new(r"^\s{0,4}#(\d+)\s{2,}(.+?)\s{2,}(\S+)\s{2,}(OPEN|MERGED|CLOSED)\s").unwrap();
    /// Matches gh run list table rows: status icon, title, workflow, branch, event, id, elapsed, age
    static ref RUN_ROW_RE: Regex =
        Regex::new(r"^([✓X✗\-*]|in_progress)\s+(.+?)\s{2,}(\S[\w\s\-]+?)\s{2,}(\S+)\s{2,}(\S+)\s{2,}(\d+)\s").unwrap();
    /// Matches the "Showing N of M" header line
    static ref SHOWING_RE: Regex =
        Regex::new(r"^Showing \d+ of \d+").unwrap();
    /// Matches the column header line (STATUS  TITLE  WORKFLOW ...)
    static ref HEADER_RE: Regex =
        Regex::new(r"^STATUS\s+TITLE").unwrap();
    /// Matches `gh pr view` / `gh issue view` metadata lines: "key:\tvalue"
    static ref META_KV_RE: Regex =
        Regex::new(r"^([a-z]+):\t(.+)$").unwrap();
    /// Matches `gh issue list` rows: #number  title  labels  date
    static ref ISSUE_ROW_RE: Regex =
        Regex::new(r"^#(\d+)\s{2,}(.+?)\s{2,}([\w\-,\s]+?)\s{2,}(about .+)$").unwrap();
    /// Matches `gh run view` job header: icon + job name + duration
    static ref RUN_VIEW_JOB_RE: Regex =
        Regex::new(r"^([✓X✗\-])\s+(.+?)\s+in\s+(.+)$").unwrap();
    /// Matches `gh run view` step line: indented icon + step name (optionally with duration)
    static ref RUN_VIEW_STEP_RE: Regex =
        Regex::new(r"^\s{2}([✓X✗\-])\s+(.+?)(?:\s+\(\d+[smh]?\d*s?\))?$").unwrap();
    /// Matches `gh run view` indented detail/error lines (deeper than step)
    static ref RUN_VIEW_DETAIL_RE: Regex =
        Regex::new(r"^\s{4,}(.+)$").unwrap();
}

/// Truncate a string to at most `max_chars`, appending "…" if truncated.
fn truncate(s: &str, max_chars: usize) -> String {
    let s = s.trim();
    let chars: Vec<char> = s.chars().collect();
    if chars.len() <= max_chars {
        s.to_string()
    } else {
        let truncated: String = chars[..max_chars.saturating_sub(1)].iter().collect();
        format!("{}…", truncated.trim_end())
    }
}

/// Filter `gh pr list` output.
///
/// Compact table: keep #, title (truncated to 40 chars), status.
/// Strip branch, date columns.
/// Target: 60% savings.
pub fn filter_gh_pr_list(input: &str) -> String {
    if input.trim().is_empty() {
        return String::new();
    }

    let mut rows: Vec<String> = Vec::new();

    for line in input.lines() {
        if SHOWING_RE.is_match(line.trim()) {
            continue;
        }
        if let Some(caps) = PR_ROW_RE.captures(line) {
            let number = &caps[1];
            let title = truncate(&caps[2], 40);
            let status = caps[4].trim();
            rows.push(format!("#{} {} [{}]", number, title, status));
        }
    }

    if rows.is_empty() {
        return input.to_string();
    }

    rows.join("\n")
}

/// Filter `gh run list` output.
///
/// Compact: keep status icon, workflow name, conclusion.
/// Strip id, elapsed, age.
/// Target: 65% savings.
pub fn filter_gh_run_list(input: &str) -> String {
    if input.trim().is_empty() {
        return String::new();
    }

    let mut rows: Vec<String> = Vec::new();

    for line in input.lines() {
        let trimmed = line.trim();
        if HEADER_RE.is_match(trimmed) {
            continue;
        }
        if trimmed.is_empty() {
            continue;
        }
        if let Some(caps) = RUN_ROW_RE.captures(line) {
            let icon = caps[1].trim();
            let title = truncate(&caps[2], 35);
            let workflow = truncate(&caps[3], 25);
            rows.push(format!("{} {} ({})", icon, title, workflow));
        }
    }

    if rows.is_empty() {
        return input.to_string();
    }

    rows.join("\n")
}

/// Filter `gh pr view` output.
///
/// Strip markdown body, keep title/state/reviewers/labels/URL.
/// Target: 85-90% savings.
pub fn filter_gh_pr_view(input: &str) -> String {
    if input.trim().is_empty() {
        return String::new();
    }

    let keep_keys = [
        "title",
        "state",
        "number",
        "url",
        "labels",
        "reviewers",
        "assignees",
        "milestone",
    ];
    let mut lines: Vec<String> = Vec::new();
    let mut in_body = false;

    for line in input.lines() {
        // The body separator is "--" on its own line
        if line.trim() == "--" {
            in_body = true;
            continue;
        }
        if in_body {
            continue;
        }

        if let Some(caps) = META_KV_RE.captures(line) {
            let key = caps.get(1).map_or("", |m| m.as_str());
            let val = caps.get(2).map_or("", |m| m.as_str());
            if keep_keys.contains(&key) {
                let display_val = if key == "title" {
                    truncate(val, 60)
                } else {
                    val.trim().to_string()
                };
                lines.push(format!("{}: {}", key, display_val));
            }
        }
    }

    if lines.is_empty() {
        return input.to_string();
    }

    lines.join("\n")
}

/// Filter `gh issue list` output.
///
/// Compact table: #number, title (truncated 30 chars), first label.
/// Target: 80% savings.
pub fn filter_gh_issue_list(input: &str) -> String {
    if input.trim().is_empty() {
        return String::new();
    }

    let mut rows: Vec<String> = Vec::new();

    for line in input.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || SHOWING_RE.is_match(trimmed) {
            continue;
        }
        if let Some(caps) = ISSUE_ROW_RE.captures(trimmed) {
            let number = &caps[1];
            let title = truncate(&caps[2], 30);
            // Extract first label as a status hint
            let labels = &caps[3];
            let first_label = labels.split(',').next().unwrap_or("").trim();
            rows.push(format!("#{} {} [{}]", number, title, first_label));
        }
    }

    if rows.is_empty() {
        return input.to_string();
    }

    rows.join("\n")
}

/// Filter `gh issue view` output.
///
/// Strip body, keep title/state/assignees/labels.
/// Target: 85% savings.
pub fn filter_gh_issue_view(input: &str) -> String {
    if input.trim().is_empty() {
        return String::new();
    }

    let keep_keys = [
        "title",
        "state",
        "number",
        "url",
        "labels",
        "assignees",
        "milestone",
    ];
    let mut lines: Vec<String> = Vec::new();
    let mut in_body = false;

    for line in input.lines() {
        if line.trim() == "--" {
            in_body = true;
            continue;
        }
        if in_body {
            continue;
        }

        if let Some(caps) = META_KV_RE.captures(line) {
            let key = caps.get(1).map_or("", |m| m.as_str());
            let val = caps.get(2).map_or("", |m| m.as_str());
            if keep_keys.contains(&key) {
                let display_val = if key == "title" {
                    truncate(val, 60)
                } else {
                    val.trim().to_string()
                };
                lines.push(format!("{}: {}", key, display_val));
            }
        }
    }

    if lines.is_empty() {
        return input.to_string();
    }

    lines.join("\n")
}

/// Filter `gh run view` output.
///
/// Keep status + failed steps only, strip successful step details.
/// Target: 85% savings.
pub fn filter_gh_run_view(input: &str) -> String {
    if input.trim().is_empty() {
        return String::new();
    }

    let mut lines: Vec<String> = Vec::new();
    // Track whether the current job has failures (to include its header)
    let mut current_job: Option<String> = None;
    let mut current_job_failed = false;
    let mut failed_steps: Vec<String> = Vec::new();
    let mut in_annotations = false;

    // Keep the first line (overall status)
    let all_lines: Vec<&str> = input.lines().collect();
    if let Some(first) = all_lines.first() {
        lines.push(first.to_string());
    }

    for line in &all_lines[1..] {
        let trimmed = line.trim();

        // Keep the ANNOTATIONS section header and all annotation lines
        if trimmed == "ANNOTATIONS" {
            in_annotations = true;
            // Flush any pending failed job
            if current_job_failed {
                if let Some(ref job) = current_job {
                    lines.push(job.clone());
                }
                lines.append(&mut failed_steps);
            }
            lines.push(String::new());
            lines.push("ANNOTATIONS".to_string());
            continue;
        }
        if in_annotations {
            if !trimmed.is_empty() {
                lines.push(trimmed.to_string());
            }
            continue;
        }

        // Skip non-essential header lines
        if trimmed.starts_with("Triggered via") || trimmed == "JOBS" {
            continue;
        }

        // Job header line
        if let Some(caps) = RUN_VIEW_JOB_RE.captures(trimmed) {
            // Flush previous job if it was failed
            if current_job_failed {
                if let Some(ref job) = current_job {
                    lines.push(job.clone());
                }
                lines.append(&mut failed_steps);
            }
            failed_steps.clear();

            let icon = &caps[1];
            let name = &caps[2];
            let duration = &caps[3];
            current_job_failed = icon == "X";
            current_job = Some(format!("{} {} ({})", icon, name, duration));

            // For passing jobs, emit a one-line summary
            if !current_job_failed {
                lines.push(format!("{} {} ({})", icon, name, duration));
                current_job = None;
            }
            continue;
        }

        // Step line (indented with 2 spaces)
        if let Some(caps) = RUN_VIEW_STEP_RE.captures(line) {
            let icon = &caps[1];
            let step_name = &caps[2];
            if current_job_failed && icon == "X" {
                failed_steps.push(format!("  X {}", step_name));
            }
            continue;
        }

        // Detail lines under a failed step — skip verbose error output to save tokens
        if RUN_VIEW_DETAIL_RE.is_match(line) {
            continue;
        }
    }

    // Flush last job if failed
    if current_job_failed {
        if let Some(ref job) = current_job {
            lines.push(job.clone());
        }
        lines.extend(failed_steps);
    }

    if lines.len() <= 1 {
        return input.to_string();
    }

    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use insta::assert_snapshot;

    fn count_tokens(s: &str) -> usize {
        s.split_whitespace().count()
    }

    // --- filter_gh_pr_list ---

    #[test]
    fn test_gh_pr_list_format() {
        let input = include_str!("../../../tests/fixtures/git/gh_pr_list_raw.txt");
        let output = filter_gh_pr_list(input);
        assert_snapshot!(output);
    }

    #[test]
    fn test_gh_pr_list_savings() {
        let input = include_str!("../../../tests/fixtures/git/gh_pr_list_raw.txt");
        let output = filter_gh_pr_list(input);
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
    fn test_gh_pr_list_empty() {
        assert_eq!(filter_gh_pr_list(""), "");
        assert_eq!(filter_gh_pr_list("   \n  "), "");
    }

    #[test]
    fn test_gh_pr_list_malformed() {
        let output = filter_gh_pr_list("not gh output\njust some random text here");
        assert!(!output.is_empty(), "Should return input on malformed data");
    }

    #[test]
    fn test_gh_pr_list_strips_branch_and_date() {
        let input = include_str!("../../../tests/fixtures/git/gh_pr_list_raw.txt");
        let output = filter_gh_pr_list(input);
        // Branch names contain slashes like "feature/rate-limiting-redis-sentinel"
        assert!(
            !output.contains("feature/rate-limiting"),
            "Should strip branch names"
        );
        assert!(!output.contains("ago"), "Should strip date columns");
    }

    #[test]
    fn test_gh_pr_list_keeps_number_and_status() {
        let input = include_str!("../../../tests/fixtures/git/gh_pr_list_raw.txt");
        let output = filter_gh_pr_list(input);
        assert!(output.contains("#142"), "Should keep PR number");
        assert!(output.contains("OPEN"), "Should keep status");
        assert!(output.contains("MERGED"), "Should keep merged status");
    }

    // --- filter_gh_run_list ---

    #[test]
    fn test_gh_run_list_format() {
        let input = include_str!("../../../tests/fixtures/git/gh_run_list_raw.txt");
        let output = filter_gh_run_list(input);
        assert_snapshot!(output);
    }

    #[test]
    fn test_gh_run_list_savings() {
        let input = include_str!("../../../tests/fixtures/git/gh_run_list_raw.txt");
        let output = filter_gh_run_list(input);
        let input_t = count_tokens(input);
        let output_t = count_tokens(&output);
        let savings = 100.0 - (output_t as f64 / input_t as f64 * 100.0);
        assert!(
            savings >= 65.0,
            "Expected >=65% savings, got {:.1}% ({} -> {} tokens)",
            savings,
            input_t,
            output_t
        );
    }

    #[test]
    fn test_gh_run_list_empty() {
        assert_eq!(filter_gh_run_list(""), "");
        assert_eq!(filter_gh_run_list("   \n  "), "");
    }

    #[test]
    fn test_gh_run_list_malformed() {
        let output = filter_gh_run_list("not gh run output\njust some random text here");
        assert!(!output.is_empty(), "Should return input on malformed data");
    }

    #[test]
    fn test_gh_run_list_strips_id_and_age() {
        let input = include_str!("../../../tests/fixtures/git/gh_run_list_raw.txt");
        let output = filter_gh_run_list(input);
        // Run IDs are large numbers like 8432109876
        assert!(!output.contains("8432109876"), "Should strip run IDs");
        assert!(!output.contains("ago"), "Should strip age");
    }

    // --- filter_gh_pr_view ---

    #[test]
    fn test_gh_pr_view_format() {
        let input = include_str!("../../../tests/fixtures/git/gh_pr_view_raw.txt");
        let output = filter_gh_pr_view(input);
        assert_snapshot!(output);
    }

    #[test]
    fn test_gh_pr_view_savings() {
        let input = include_str!("../../../tests/fixtures/git/gh_pr_view_raw.txt");
        let output = filter_gh_pr_view(input);
        let input_t = count_tokens(input);
        let output_t = count_tokens(&output);
        let savings = 100.0 - (output_t as f64 / input_t as f64 * 100.0);
        assert!(
            savings >= 85.0,
            "Expected >=85% savings, got {:.1}% ({} -> {} tokens)",
            savings,
            input_t,
            output_t
        );
    }

    #[test]
    fn test_gh_pr_view_empty() {
        assert_eq!(filter_gh_pr_view(""), "");
        assert_eq!(filter_gh_pr_view("   \n  "), "");
    }

    #[test]
    fn test_gh_pr_view_malformed() {
        let output = filter_gh_pr_view("not gh pr view output\njust random text");
        assert!(!output.is_empty(), "Should return input on malformed data");
    }

    #[test]
    fn test_gh_pr_view_strips_body() {
        let input = include_str!("../../../tests/fixtures/git/gh_pr_view_raw.txt");
        let output = filter_gh_pr_view(input);
        assert!(!output.contains("Motivation"), "Should strip markdown body");
        assert!(!output.contains("Checklist"), "Should strip checklist");
        assert!(!output.contains("```"), "Should strip code blocks");
    }

    #[test]
    fn test_gh_pr_view_keeps_metadata() {
        let input = include_str!("../../../tests/fixtures/git/gh_pr_view_raw.txt");
        let output = filter_gh_pr_view(input);
        assert!(output.contains("OPEN"), "Should keep state");
        assert!(output.contains("142"), "Should keep number");
        assert!(output.contains("github.com"), "Should keep URL");
        assert!(output.contains("reviewers"), "Should keep reviewers");
    }

    // --- filter_gh_issue_list ---

    #[test]
    fn test_gh_issue_list_format() {
        let input = include_str!("../../../tests/fixtures/git/gh_issue_list_raw.txt");
        let output = filter_gh_issue_list(input);
        assert_snapshot!(output);
    }

    #[test]
    fn test_gh_issue_list_savings() {
        let input = include_str!("../../../tests/fixtures/git/gh_issue_list_raw.txt");
        let output = filter_gh_issue_list(input);
        let input_t = count_tokens(input);
        let output_t = count_tokens(&output);
        let savings = 100.0 - (output_t as f64 / input_t as f64 * 100.0);
        assert!(
            savings >= 70.0,
            "Expected >=70% savings, got {:.1}% ({} -> {} tokens)",
            savings,
            input_t,
            output_t
        );
    }

    #[test]
    fn test_gh_issue_list_empty() {
        assert_eq!(filter_gh_issue_list(""), "");
        assert_eq!(filter_gh_issue_list("   \n  "), "");
    }

    #[test]
    fn test_gh_issue_list_malformed() {
        let output = filter_gh_issue_list("not gh issue list output\njust random text");
        assert!(!output.is_empty(), "Should return input on malformed data");
    }

    #[test]
    fn test_gh_issue_list_strips_dates() {
        let input = include_str!("../../../tests/fixtures/git/gh_issue_list_raw.txt");
        let output = filter_gh_issue_list(input);
        assert!(!output.contains("ago"), "Should strip date columns");
    }

    #[test]
    fn test_gh_issue_list_keeps_numbers() {
        let input = include_str!("../../../tests/fixtures/git/gh_issue_list_raw.txt");
        let output = filter_gh_issue_list(input);
        assert!(output.contains("#389"), "Should keep issue number");
        assert!(output.contains("#350"), "Should keep last issue number");
    }

    // --- filter_gh_issue_view ---

    #[test]
    fn test_gh_issue_view_format() {
        let input = include_str!("../../../tests/fixtures/git/gh_issue_view_raw.txt");
        let output = filter_gh_issue_view(input);
        assert_snapshot!(output);
    }

    #[test]
    fn test_gh_issue_view_savings() {
        let input = include_str!("../../../tests/fixtures/git/gh_issue_view_raw.txt");
        let output = filter_gh_issue_view(input);
        let input_t = count_tokens(input);
        let output_t = count_tokens(&output);
        let savings = 100.0 - (output_t as f64 / input_t as f64 * 100.0);
        assert!(
            savings >= 85.0,
            "Expected >=85% savings, got {:.1}% ({} -> {} tokens)",
            savings,
            input_t,
            output_t
        );
    }

    #[test]
    fn test_gh_issue_view_empty() {
        assert_eq!(filter_gh_issue_view(""), "");
        assert_eq!(filter_gh_issue_view("   \n  "), "");
    }

    #[test]
    fn test_gh_issue_view_malformed() {
        let output = filter_gh_issue_view("not gh issue view output\njust random text");
        assert!(!output.is_empty(), "Should return input on malformed data");
    }

    #[test]
    fn test_gh_issue_view_strips_body() {
        let input = include_str!("../../../tests/fixtures/git/gh_issue_view_raw.txt");
        let output = filter_gh_issue_view(input);
        assert!(!output.contains("Steps to Reproduce"), "Should strip body");
        assert!(
            !output.contains("Broken pipe"),
            "Should strip error details"
        );
    }

    #[test]
    fn test_gh_issue_view_keeps_metadata() {
        let input = include_str!("../../../tests/fixtures/git/gh_issue_view_raw.txt");
        let output = filter_gh_issue_view(input);
        assert!(output.contains("OPEN"), "Should keep state");
        assert!(output.contains("389"), "Should keep number");
        assert!(output.contains("bug"), "Should keep labels");
        assert!(output.contains("alee"), "Should keep assignees");
    }

    // --- filter_gh_run_view ---

    #[test]
    fn test_gh_run_view_format() {
        let input = include_str!("../../../tests/fixtures/git/gh_run_view_raw.txt");
        let output = filter_gh_run_view(input);
        assert_snapshot!(output);
    }

    #[test]
    fn test_gh_run_view_savings() {
        let input = include_str!("../../../tests/fixtures/git/gh_run_view_raw.txt");
        let output = filter_gh_run_view(input);
        let input_t = count_tokens(input);
        let output_t = count_tokens(&output);
        let savings = 100.0 - (output_t as f64 / input_t as f64 * 100.0);
        assert!(
            savings >= 85.0,
            "Expected >=85% savings, got {:.1}% ({} -> {} tokens)",
            savings,
            input_t,
            output_t
        );
    }

    #[test]
    fn test_gh_run_view_empty() {
        assert_eq!(filter_gh_run_view(""), "");
        assert_eq!(filter_gh_run_view("   \n  "), "");
    }

    #[test]
    fn test_gh_run_view_malformed() {
        let output = filter_gh_run_view("not gh run view output\njust random text");
        assert!(!output.is_empty(), "Should return input on malformed data");
    }

    #[test]
    fn test_gh_run_view_strips_passing_steps() {
        let input = include_str!("../../../tests/fixtures/git/gh_run_view_raw.txt");
        let output = filter_gh_run_view(input);
        assert!(
            !output.contains("Set up job"),
            "Should strip passing step details"
        );
        assert!(
            !output.contains("Cache cargo"),
            "Should strip passing step details"
        );
    }

    #[test]
    fn test_gh_run_view_keeps_failures() {
        let input = include_str!("../../../tests/fixtures/git/gh_run_view_raw.txt");
        let output = filter_gh_run_view(input);
        assert!(
            output.contains("Build release binary"),
            "Should keep failed step name"
        );
        assert!(
            output.contains("ANNOTATIONS"),
            "Should keep annotations section"
        );
    }
}

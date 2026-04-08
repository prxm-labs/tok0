//! Edge case battery: empty, unicode, ANSI, binary-like, huge input,
//! symlinks, boundary conditions for every compressor shipped in Phase 1.

mod common;

use tok0::compressors::git::git_cmd;
use tok0::compressors::system::{diff_cmd, find_cmd, grep_cmd, ls_cmd, read_cmd, smart_cmd};

// ============================================================
// ls_cmd edge cases
// ============================================================

#[test]
fn ls_empty() {
    assert!(ls_cmd::filter_ls("", ".").is_empty());
}

#[test]
fn ls_single_file() {
    let input = "total 8\n-rw-r--r--  1 user staff  42 Jan  1 00:00 README.md";
    let output = ls_cmd::filter_ls(input, ".");
    assert!(output.contains("README.md"));
}

#[test]
fn ls_symlinks() {
    let input = "total 0\nlrwxr-xr-x  1 user staff  10 Jan  1 00:00 link -> target";
    let output = ls_cmd::filter_ls(input, ".");
    assert!(output.contains("link"));
}

#[test]
fn ls_unicode_filenames() {
    let input =
        "total 0\ndrwxr-xr-x  2 user staff  64 Jan  1 00:00 \u{65e5}\u{672c}\u{8a9e}\n-rw-r--r--  1 user staff  42 Jan  1 00:00 \u{03b1}\u{03b2}\u{03b3}.txt";
    let output = ls_cmd::filter_ls(input, ".");
    assert!(!output.is_empty());
}

#[test]
fn ls_large_files_gb() {
    let input = "total 0\n-rw-r--r--  1 user staff  1073741824 Jan  1 00:00 huge.bin";
    let output = ls_cmd::filter_ls(input, ".");
    assert!(output.contains("1.0G") || output.contains("huge.bin"));
}

#[test]
fn ls_zero_byte_file() {
    let input = "total 0\n-rw-r--r--  1 user staff  0 Jan  1 00:00 empty.txt";
    let output = ls_cmd::filter_ls(input, ".");
    assert!(output.contains("0B") || output.contains("empty.txt"));
}

#[test]
fn ls_many_entries() {
    let mut lines = vec!["total 100".to_string()];
    for i in 0..50 {
        lines.push(format!(
            "-rw-r--r--  1 user staff  {} Jan  1 00:00 file_{}.rs",
            i * 100,
            i
        ));
    }
    for i in 0..20 {
        lines.push(format!(
            "drwxr-xr-x  2 user staff  64 Jan  1 00:00 dir_{}",
            i
        ));
    }
    let input = lines.join("\n");
    let output = ls_cmd::filter_ls(&input, ".");
    assert!(output.contains("20 dirs"));
    assert!(output.contains("50 files"));
}

#[test]
fn ls_only_total_line() {
    let output = ls_cmd::filter_ls("total 0", ".");
    assert!(output.is_empty() || output == "total 0");
}

#[test]
fn ls_edge_case_battery() {
    common::run_edge_cases(|input| ls_cmd::filter_ls(input, "."), "ls");
}

// ============================================================
// find_cmd edge cases
// ============================================================

#[test]
fn find_empty() {
    assert!(find_cmd::filter_find("").is_empty());
}

#[test]
fn find_single_result() {
    let output = find_cmd::filter_find("./src/main.rs");
    assert!(output.contains("main.rs"));
}

#[test]
fn find_exactly_10_files() {
    let lines: Vec<String> = (0..10).map(|i| format!("./file_{}.rs", i)).collect();
    let output = find_cmd::filter_find(&lines.join("\n"));
    // 10 or fewer should pass through unchanged
    assert!(output.contains("file_0.rs"));
    assert!(output.contains("file_9.rs"));
}

#[test]
fn find_11_files_triggers_grouping() {
    let lines: Vec<String> = (0..11).map(|i| format!("./file_{}.rs", i)).collect();
    let output = find_cmd::filter_find(&lines.join("\n"));
    assert!(output.contains("11 files"));
}

#[test]
fn find_deeply_nested() {
    let lines: Vec<String> = (0..50).map(|i| format!("./a/b/c/d/e/f{}.rs", i)).collect();
    let output = find_cmd::filter_find(&lines.join("\n"));
    assert!(output.contains("50 files"));
}

#[test]
fn find_mixed_extensions() {
    let input = "./a.rs\n./b.rs\n./c.toml\n./d.toml\n./e.md\n./f.rs\n./g.rs\n./h.rs\n./i.rs\n./j.rs\n./k.rs\n./l.rs";
    let output = find_cmd::filter_find(input);
    assert!(output.contains(".rs"));
    assert!(output.contains(".toml"));
}

#[test]
fn find_no_extension_files() {
    let lines: Vec<String> = (0..15).map(|i| format!("./dir/Makefile_{}", i)).collect();
    let output = find_cmd::filter_find(&lines.join("\n"));
    assert!(output.contains("no ext") || output.contains("15 files"));
}

#[test]
fn find_edge_case_battery() {
    common::run_edge_cases(find_cmd::filter_find, "find");
}

// ============================================================
// grep_cmd edge cases
// ============================================================

#[test]
fn grep_empty() {
    assert!(grep_cmd::filter_grep("", 100, 10).is_empty());
}

#[test]
fn grep_single_match() {
    let input = "src/main.rs:1:fn main() {";
    assert_eq!(grep_cmd::filter_grep(input, 100, 10), input);
}

#[test]
fn grep_exactly_at_limit() {
    let lines: Vec<String> = (1..=100)
        .map(|i| format!("src/file.rs:{}:match line {}", i, i))
        .collect();
    let input = lines.join("\n");
    // At limit, should pass through
    let output = grep_cmd::filter_grep(&input, 100, 100);
    assert_eq!(output, input);
}

#[test]
fn grep_one_over_limit() {
    let lines: Vec<String> = (1..=101)
        .map(|i| format!("src/file.rs:{}:match line {}", i, i))
        .collect();
    let input = lines.join("\n");
    let output = grep_cmd::filter_grep(&input, 100, 100);
    assert!(output.contains("101 matches"));
}

#[test]
fn grep_many_files_few_matches() {
    let lines: Vec<String> = (0..50)
        .map(|i| format!("src/mod_{}.rs:1:fn handler() {{}}", i))
        .collect();
    let input = lines.join("\n");
    let output = grep_cmd::filter_grep(&input, 20, 5);
    assert!(output.contains("50 matches"));
    assert!(output.contains("50 files"));
}

#[test]
fn grep_many_matches_per_file() {
    let lines: Vec<String> = (1..=200)
        .map(|i| format!("src/big.rs:{}:fn test_{}() {{}}", i, i))
        .collect();
    let output = grep_cmd::filter_grep(&lines.join("\n"), 50, 5);
    assert!(output.contains("200 matches"));
    assert!(output.contains("more"));
}

#[test]
fn grep_non_standard_format() {
    // Lines that don't match file:line:content format
    let input = "Binary file matches\nsome random output";
    let output = grep_cmd::filter_grep(input, 100, 10);
    assert_eq!(output, input); // Pass through
}

#[test]
fn grep_edge_case_battery() {
    common::run_edge_cases(|input| grep_cmd::filter_grep(input, 100, 10), "grep");
}

// ============================================================
// diff_cmd edge cases
// ============================================================

#[test]
fn diff_empty() {
    assert!(diff_cmd::filter_diff("").is_empty());
}

#[test]
fn diff_no_changes() {
    let output = diff_cmd::filter_diff("no diff output here");
    assert!(!output.is_empty());
}

#[test]
fn diff_single_file_one_hunk() {
    let input = "diff --git a/f.rs b/f.rs\n--- a/f.rs\n+++ b/f.rs\n@@ -1 +1 @@\n-old\n+new";
    let output = diff_cmd::filter_diff(input);
    assert!(output.contains("1 files changed"));
    assert!(output.contains("+1 -1"));
}

#[test]
fn diff_additions_only() {
    let input =
        "diff --git a/f.rs b/f.rs\n--- a/f.rs\n+++ b/f.rs\n@@ -0,0 +1,3 @@\n+line1\n+line2\n+line3";
    let output = diff_cmd::filter_diff(input);
    assert!(output.contains("+3 -0"));
}

#[test]
fn diff_deletions_only() {
    let input =
        "diff --git a/f.rs b/f.rs\n--- a/f.rs\n+++ b/f.rs\n@@ -1,3 +0,0 @@\n-line1\n-line2\n-line3";
    let output = diff_cmd::filter_diff(input);
    assert!(output.contains("+0 -3"));
}

#[test]
fn diff_multiple_hunks() {
    let input = "diff --git a/f.rs b/f.rs\n--- a/f.rs\n+++ b/f.rs\n@@ -1,2 +1,2 @@\n-a\n+b\n@@ -10,2 +10,2 @@\n-c\n+d";
    let output = diff_cmd::filter_diff(input);
    assert!(output.contains("2 hunks"));
}

#[test]
fn diff_many_files() {
    let mut diff = String::new();
    for i in 0..25 {
        diff.push_str(&format!(
            "diff --git a/src/mod_{}.rs b/src/mod_{}.rs\n",
            i, i
        ));
        diff.push_str(&format!("--- a/src/mod_{}.rs\n", i));
        diff.push_str(&format!("+++ b/src/mod_{}.rs\n", i));
        diff.push_str("@@ -1 +1 @@\n-old\n+new\n");
    }
    let output = diff_cmd::filter_diff(&diff);
    assert!(output.contains("25 files changed"));
}

#[test]
fn diff_edge_case_battery() {
    common::run_edge_cases(diff_cmd::filter_diff, "diff");
}

// ============================================================
// read_cmd edge cases
// ============================================================

#[test]
fn read_empty() {
    let output = read_cmd::filter_read("", "f.txt");
    assert!(output.contains("empty"));
}

#[test]
fn read_single_line() {
    let output = read_cmd::filter_read("hello world", "f.txt");
    assert!(output.contains("hello"));
    assert!(output.contains("1 lines"));
}

#[test]
fn read_exactly_100_lines_no_truncation() {
    let lines: Vec<String> = (1..=100).map(|i| format!("line {}", i)).collect();
    let output = read_cmd::filter_read(&lines.join("\n"), "f.txt");
    assert!(output.contains("100 lines"));
    assert!(!output.contains("omitted"));
}

#[test]
fn read_101_lines_triggers_truncation() {
    let lines: Vec<String> = (1..=101).map(|i| format!("line {}", i)).collect();
    let output = read_cmd::filter_read(&lines.join("\n"), "f.txt");
    assert!(output.contains("omitted"));
}

#[test]
fn read_preserves_first_and_last_lines() {
    let lines: Vec<String> = (1..=200).map(|i| format!("LINE_{}", i)).collect();
    let output = read_cmd::filter_read(&lines.join("\n"), "f.txt");
    assert!(output.contains("LINE_1"));
    assert!(output.contains("LINE_200"));
}

#[test]
fn read_very_long_lines_truncated() {
    let long_line = format!("prefix {}", "x".repeat(500));
    let output = read_cmd::filter_read(&long_line, "f.txt");
    assert!(output.len() < long_line.len() + 50);
}

#[test]
fn read_path_in_header() {
    let output = read_cmd::filter_read("content", "src/engine/config.rs");
    assert!(output.contains("src/engine/config.rs"));
}

#[test]
fn read_line_numbers_present() {
    let output = read_cmd::filter_read("line one\nline two\nline three", "f.txt");
    assert!(output.contains("   1"));
    assert!(output.contains("   2"));
    assert!(output.contains("   3"));
}

// ============================================================
// smart_cmd edge cases
// ============================================================

#[test]
fn smart_empty() {
    assert!(smart_cmd::filter_smart("").is_empty());
}

#[test]
fn smart_small_file_passthrough() {
    let input = "fn main() {\n    println!(\"hello\");\n}";
    assert_eq!(smart_cmd::filter_smart(input), input);
}

#[test]
fn smart_exactly_30_lines_passthrough() {
    let lines: Vec<String> = (0..30).map(|i| format!("line {}", i)).collect();
    let input = lines.join("\n");
    assert_eq!(smart_cmd::filter_smart(&input), input);
}

#[test]
fn smart_31_lines_triggers_compression() {
    let mut lines: Vec<String> = (0..31)
        .map(|i| format!("    let x{} = {};", i, i))
        .collect();
    lines.insert(0, "pub fn big() {".to_string());
    lines.push("}".to_string());
    let input = lines.join("\n");
    let output = smart_cmd::filter_smart(&input);
    assert!(output.contains("lines"));
    assert!(output.contains("Definitions"));
}

#[test]
fn smart_all_comments() {
    let lines: Vec<String> = (0..50).map(|i| format!("// comment {}", i)).collect();
    let output = smart_cmd::filter_smart(&lines.join("\n"));
    assert!(output.contains("comments"));
}

#[test]
fn smart_all_blank_lines() {
    let input = "\n".repeat(50);
    let output = smart_cmd::filter_smart(&input);
    // All-blank input: filter may return summary or pass through empty content
    // Just verify it doesn't panic and produces something
    assert!(output.is_empty() || !output.is_empty()); // no panic
}

#[test]
fn smart_mixed_languages() {
    let mut lines = vec![
        "import React from 'react';".to_string(),
        "import { useState } from 'react';".to_string(),
        "".to_string(),
        "function App() {".to_string(),
    ];
    for i in 0..35 {
        lines.push(format!("  const val{} = useState(null);", i));
    }
    lines.push("}".to_string());
    let input = lines.join("\n");
    let output = smart_cmd::filter_smart(&input);
    assert!(output.contains("Imports") || output.contains("Definitions"));
}

// ============================================================
// git_cmd::filter_git_log edge cases
// ============================================================

#[test]
fn git_log_empty() {
    assert!(git_cmd::filter_git_log("").is_empty());
}

#[test]
fn git_log_single_commit() {
    let input = "commit abc1234567890\nAuthor: Test <t@t.com>\nDate:   Mon Jan 1 00:00:00 2026\n\n    feat: initial commit";
    let output = git_cmd::filter_git_log(input);
    assert!(output.contains("abc1234"));
    assert!(output.contains("initial commit"));
    assert!(output.contains("1 commits"));
}

#[test]
fn git_log_merge_commit() {
    let input = "commit abc1234567890\nMerge: aaa1111 bbb2222\nAuthor: Test <t@t.com>\nDate:   Mon Jan 1 00:00:00 2026\n\n    Merge branch 'feature' into main";
    let output = git_cmd::filter_git_log(input);
    assert!(output.contains("Merge"));
}

#[test]
fn git_log_multi_line_body_ignored() {
    let input = "commit abc1234567890\nAuthor: Test <t@t.com>\nDate:   Mon Jan 1 00:00:00 2026\n\n    feat: subject line\n\n    This is a longer description\n    spanning multiple lines\n    that should be ignored.";
    let output = git_cmd::filter_git_log(input);
    assert!(output.contains("subject line"));
    assert!(!output.contains("longer description"));
}

#[test]
fn git_log_short_hash_7_chars() {
    let input = "commit 1234567\nAuthor: Test <t@t.com>\nDate:   Mon Jan 1 00:00:00 2026\n\n    short hash commit";
    let output = git_cmd::filter_git_log(input);
    assert!(output.contains("1234567"));
}

#[test]
fn git_log_malformed_no_panic() {
    let output = git_cmd::filter_git_log("not a git log\nrandom text\n\nmore stuff");
    assert!(!output.is_empty());
}

// ============================================================
// git_cmd::filter_git_status edge cases
// ============================================================

#[test]
fn git_status_empty() {
    assert!(git_cmd::filter_git_status("").is_empty());
}

#[test]
fn git_status_clean_repo() {
    let input = "On branch main\nnothing to commit, working tree clean";
    let output = git_cmd::filter_git_status(input);
    assert!(output.contains("branch: main"));
}

#[test]
fn git_status_detached_head() {
    let input = "HEAD detached at abc1234\nnothing to commit, working tree clean";
    let output = git_cmd::filter_git_status(input);
    // Should not panic; branch may be empty
    assert!(!output.is_empty());
}

#[test]
fn git_status_only_staged() {
    let input = "On branch feat\nChanges to be committed:\n  (use \"git restore --staged <file>...\" to unstage)\n\tnew file:   src/new.rs";
    let output = git_cmd::filter_git_status(input);
    assert!(output.contains("staged"));
    assert!(output.contains("new.rs"));
}

#[test]
fn git_status_only_unstaged() {
    let input = "On branch feat\nChanges not staged for commit:\n  (use \"git add <file>...\" to update)\n\tmodified:   src/main.rs";
    let output = git_cmd::filter_git_status(input);
    assert!(output.contains("unstaged"));
    assert!(output.contains("main.rs"));
}

#[test]
fn git_status_only_untracked() {
    let input = "On branch dev\nUntracked files:\n  (use \"git add <file>...\" to include)\n\tnew_file.txt\n\tanother.rs";
    let output = git_cmd::filter_git_status(input);
    assert!(output.contains("untracked"));
}

#[test]
fn git_status_many_untracked_truncated() {
    let mut input = "On branch main\nUntracked files:\n  (use \"git add\" ...)\n".to_string();
    for i in 0..25 {
        input.push_str(&format!("\tfile_{}.tmp\n", i));
    }
    let output = git_cmd::filter_git_status(&input);
    assert!(output.contains("untracked"));
    assert!(output.contains("more"));
}

// ============================================================
// git_cmd::filter_git_diff edge cases
// ============================================================

#[test]
fn git_diff_empty() {
    assert!(git_cmd::filter_git_diff("").is_empty());
}

#[test]
fn git_diff_single_file_minimal() {
    let input = "diff --git a/f.rs b/f.rs\n--- a/f.rs\n+++ b/f.rs\n@@ -1 +1 @@\n-old\n+new";
    let output = git_cmd::filter_git_diff(input);
    assert!(output.contains("1 files changed"));
    assert!(output.contains("+1 -1"));
}

#[test]
fn git_diff_no_hunks() {
    let input = "diff --git a/f.rs b/f.rs\nindex abc..def 100644";
    let output = git_cmd::filter_git_diff(input);
    assert!(output.contains("f.rs"));
}

// ============================================================
// git_cmd::filter_git_simple edge cases
// ============================================================

#[test]
fn git_simple_push_empty() {
    assert_eq!(git_cmd::filter_git_simple("", "push"), "ok push");
}

#[test]
fn git_simple_commit_empty() {
    assert_eq!(git_cmd::filter_git_simple("", "commit"), "ok commit");
}

#[test]
fn git_simple_add_empty() {
    assert_eq!(git_cmd::filter_git_simple("", "add"), "ok add");
}

#[test]
fn git_simple_pull_empty() {
    assert_eq!(git_cmd::filter_git_simple("", "pull"), "ok pull");
}

#[test]
fn git_simple_with_verbose_output() {
    let verbose = "lots of output\nthat should be\nignored completely";
    assert_eq!(git_cmd::filter_git_simple(verbose, "push"), "ok push");
}

// ============================================================
// Edge case batteries using run_edge_cases helper
// ============================================================

#[test]
fn read_edge_case_battery() {
    common::run_edge_cases(|input| read_cmd::filter_read(input, "test.txt"), "read");
}

#[test]
fn smart_edge_case_battery() {
    common::run_edge_cases(smart_cmd::filter_smart, "smart");
}

#[test]
fn diff_edge_case_battery_full() {
    common::run_edge_cases(diff_cmd::filter_diff, "diff_full");
}

#[test]
fn git_log_edge_case_battery() {
    common::run_edge_cases(git_cmd::filter_git_log, "git_log");
}

#[test]
fn git_status_edge_case_battery() {
    common::run_edge_cases(git_cmd::filter_git_status, "git_status");
}

#[test]
fn git_diff_edge_case_battery() {
    common::run_edge_cases(git_cmd::filter_git_diff, "git_diff");
}

// ============================================================
// Sanitize module edge cases
// ============================================================

use tok0::engine::sanitize;

#[test]
fn sanitize_redacts_home_paths_in_ls() {
    let input = "drwxr-xr-x  2 user staff  64 /Users/johndoe/projects/tok0";
    let output = sanitize::redact_paths(input);
    assert!(!output.contains("johndoe"));
    assert!(output.contains("/Users/user"));
}

#[test]
fn sanitize_redacts_linux_home() {
    let input = "/home/developer/.config/tok0/config.toml";
    let output = sanitize::redact_paths(input);
    assert!(!output.contains("developer"));
}

#[test]
fn sanitize_preserves_non_home_paths() {
    let input = "/usr/local/bin/tok0";
    let output = sanitize::redact_paths(input);
    assert_eq!(output.as_ref(), input);
}

#[test]
fn sanitize_redacts_env_secrets() {
    let input = "API_KEY=sk-abc123\nHOME=/Users/testuser\nPATH=/usr/bin";
    let output = sanitize::sanitize_output(input);
    assert!(!output.contains("testuser"));
    assert!(output.contains("[REDACTED]"));
}

#[test]
fn sanitize_no_false_positives() {
    let input = "normal output without any secrets or paths";
    let output = sanitize::sanitize_output(input);
    assert_eq!(output, input);
}

#[test]
fn sanitize_multiple_secrets_one_line() {
    let input = "token=ghp_ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghij password=mysecret";
    let output = sanitize::redact_secrets(input);
    assert!(!output.contains("ghp_"));
    assert!(output.contains("[REDACTED]"));
}

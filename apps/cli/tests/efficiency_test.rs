//! Validates that all compressors meet minimum savings thresholds
//! against real fixture data and synthetic large inputs.

mod common;

use tok0::compressors::git::git_cmd;
use tok0::compressors::system::{diff_cmd, find_cmd, grep_cmd, ls_cmd, read_cmd, smart_cmd};

// === System compressor efficiency ===

#[test]
fn efficiency_ls() {
    let input = include_str!("fixtures/system/ls_raw.txt");
    let output = ls_cmd::filter_ls(input, ".");
    common::assert_savings(input, &output, 30.0, "ls");
}

#[test]
fn efficiency_find() {
    let input = include_str!("fixtures/system/find_raw.txt");
    let output = find_cmd::filter_find(input);
    common::assert_savings(input, &output, 30.0, "find");
}

#[test]
fn efficiency_grep() {
    let input = include_str!("fixtures/system/grep_raw.txt");
    let output = grep_cmd::filter_grep(input, 100, 10);
    common::assert_savings(input, &output, 10.0, "grep (100 max)");
}

#[test]
fn efficiency_grep_limited() {
    let input = include_str!("fixtures/system/grep_raw.txt");
    let output = grep_cmd::filter_grep(input, 20, 3);
    common::assert_savings(input, &output, 20.0, "grep (20 max, 3 per file)");
}

// === Git compressor efficiency ===

#[test]
fn efficiency_git_log() {
    let input = include_str!("fixtures/git/log_raw.txt");
    let output = git_cmd::filter_git_log(input);
    common::assert_savings(input, &output, 60.0, "git log");
}

#[test]
fn efficiency_git_status() {
    let input = include_str!("fixtures/git/status_raw.txt");
    let output = git_cmd::filter_git_status(input);
    common::assert_savings(input, &output, 30.0, "git status");
}

#[test]
fn efficiency_git_diff() {
    let input = include_str!("fixtures/git/diff_raw.txt");
    let output = git_cmd::filter_git_diff(input);
    common::assert_savings(input, &output, 60.0, "git diff");
}

// === Synthetic large-input efficiency ===

#[test]
fn efficiency_read_large_file() {
    let lines: Vec<String> = (1..=500)
        .map(|i| format!("line {} content here with some data and more words", i))
        .collect();
    let input = lines.join("\n");
    let output = read_cmd::filter_read(&input, "big.rs");
    common::assert_savings(&input, &output, 50.0, "read 500-line file");
}

#[test]
fn efficiency_read_huge_file() {
    let lines: Vec<String> = (1..=2000)
        .map(|i| format!("fn test_{}() {{ assert!(true); }}", i))
        .collect();
    let input = lines.join("\n");
    let output = read_cmd::filter_read(&input, "huge.rs");
    common::assert_savings(&input, &output, 70.0, "read 2000-line file");
}

#[test]
fn efficiency_smart_large_rust_file() {
    let mut lines = vec![
        "use std::io;".to_string(),
        "use anyhow::Result;".to_string(),
        "use serde::Deserialize;".to_string(),
        "".to_string(),
        "/// Main entry point".to_string(),
        "pub fn main() -> Result<()> {".to_string(),
    ];
    for i in 0..60 {
        lines.push(format!("    let x{} = compute_{}();", i, i));
    }
    lines.push("}".to_string());
    lines.push("".to_string());
    lines.push("pub struct Config {".to_string());
    lines.push("    pub name: String,".to_string());
    lines.push("}".to_string());
    lines.push("".to_string());
    lines.push("impl Config {".to_string());
    lines.push("    pub fn new() -> Self {".to_string());
    lines.push("        Self { name: String::new() }".to_string());
    lines.push("    }".to_string());
    lines.push("}".to_string());
    lines.push("".to_string());
    lines.push("pub fn helper() -> String {".to_string());
    lines.push("    \"hello\".to_string()".to_string());
    lines.push("}".to_string());
    let input = lines.join("\n");
    let output = smart_cmd::filter_smart(&input);
    common::assert_savings(&input, &output, 40.0, "smart large Rust file");
}

#[test]
fn efficiency_find_large_project() {
    let mut paths = Vec::new();
    for dir in &["src", "tests", "benches", "examples"] {
        for i in 0..30 {
            paths.push(format!("./{}/module_{}.rs", dir, i));
        }
    }
    for i in 0..20 {
        paths.push(format!("./docs/guide_{}.md", i));
    }
    for i in 0..10 {
        paths.push(format!("./config/setting_{}.toml", i));
    }
    let input = paths.join("\n");
    let output = find_cmd::filter_find(&input);
    common::assert_savings(&input, &output, 40.0, "find 150-file project");
}

#[test]
fn efficiency_grep_many_matches() {
    let lines: Vec<String> = (1..=300)
        .map(|i| {
            let file = format!("src/mod_{}.rs", i % 20);
            format!("{}:{}:    fn handler_{}() {{}}", file, i, i)
        })
        .collect();
    let input = lines.join("\n");
    let output = grep_cmd::filter_grep(&input, 50, 5);
    common::assert_savings(&input, &output, 50.0, "grep 300 matches across 20 files");
}

#[test]
fn efficiency_diff_multi_file() {
    let mut diff = String::new();
    for i in 0..10 {
        diff.push_str(&format!(
            "diff --git a/src/file_{}.rs b/src/file_{}.rs\n",
            i, i
        ));
        diff.push_str(&format!("--- a/src/file_{}.rs\n", i));
        diff.push_str(&format!("+++ b/src/file_{}.rs\n", i));
        diff.push_str("@@ -1,5 +1,8 @@\n");
        for j in 0..5 {
            diff.push_str(&format!("-old line {} in file {}\n", j, i));
        }
        for j in 0..8 {
            diff.push_str(&format!("+new line {} in file {}\n", j, i));
        }
    }
    let output = diff_cmd::filter_diff(&diff);
    common::assert_savings(&diff, &output, 60.0, "diff 10 files with 13 changes each");
}

#[test]
fn efficiency_git_log_20_commits() {
    let mut log = String::new();
    for i in 0..20 {
        log.push_str(&format!("commit {}abcdef1234567890abcdef1234567890ab\n", i));
        log.push_str(&format!("Author: Dev {} <dev{}@example.com>\n", i, i));
        log.push_str(&format!("Date:   Mon Apr {} 12:00:00 2026 +0300\n", i + 1));
        log.push('\n');
        log.push_str(&format!(
            "    feat: implement feature number {} with improvements\n",
            i
        ));
        log.push('\n');
        log.push_str(&format!(
            "    This commit adds feature {} which improves the system\n",
            i
        ));
        log.push_str("    by reducing complexity and improving performance.\n");
        log.push('\n');
    }
    let output = git_cmd::filter_git_log(&log);
    common::assert_savings(&log, &output, 70.0, "git log 20 commits with bodies");
}

#[test]
fn efficiency_git_status_busy_repo() {
    let mut status = String::from("On branch feature/big-refactor\n");
    status.push_str("Changes to be committed:\n");
    status.push_str("  (use \"git restore --staged <file>...\" to unstage)\n");
    for i in 0..15 {
        status.push_str(&format!("\tnew file:   src/new_module_{}.rs\n", i));
    }
    for i in 0..10 {
        status.push_str(&format!("\tmodified:   src/existing_{}.rs\n", i));
    }
    status.push_str("\nChanges not staged for commit:\n");
    status.push_str("  (use \"git add <file>...\" to update what will be committed)\n");
    for i in 0..5 {
        status.push_str(&format!("\tmodified:   tests/test_{}.rs\n", i));
    }
    status.push_str("\nUntracked files:\n");
    status.push_str("  (use \"git add <file>...\" to include in what will be committed)\n");
    for i in 0..20 {
        status.push_str(&format!("\ttmp/scratch_{}.txt\n", i));
    }
    let output = git_cmd::filter_git_status(&status);
    common::assert_savings(&status, &output, 25.0, "git status busy repo (50 files)");
}

#[test]
fn efficiency_git_simple_commands() {
    // Simple commands should achieve ~90%+ savings by returning "ok"
    let verbose_push = "Enumerating objects: 5, done.\nCounting objects: 100% (5/5), done.\nDelta compression using up to 10 threads\nCompressing objects: 100% (3/3), done.\nWriting objects: 100% (3/3), 1.23 KiB | 1.23 MiB/s, done.\nTotal 3 (delta 2), reused 0 (delta 0), pack-reused 0\nTo github.com:user/repo.git\n   abc1234..def5678  main -> main";
    let output = git_cmd::filter_git_simple(verbose_push, "push");
    let input_tokens = common::count_tokens(verbose_push);
    let output_tokens = common::count_tokens(&output);
    assert!(
        output_tokens <= 3,
        "git push should compress to ~2 tokens, got {} (from {})",
        output_tokens,
        input_tokens
    );
}

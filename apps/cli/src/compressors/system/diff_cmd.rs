use lazy_static::lazy_static;
use regex::Regex;

lazy_static! {
    /// Match diff file header
    static ref DIFF_FILE_RE: Regex = Regex::new(r"^diff --git a/(.+) b/(.+)$").unwrap();
    /// Match hunk header
    static ref HUNK_RE: Regex = Regex::new(r"^@@\s+\-\d+(?:,\d+)?\s+\+\d+(?:,\d+)?\s+@@(.*)$").unwrap();
    /// Match +/- lines
    static ref ADD_RE: Regex = Regex::new(r"^\+[^+]").unwrap();
    static ref DEL_RE: Regex = Regex::new(r"^\-[^-]").unwrap();
}

/// Filter diff output: show file summary + condensed hunks.
pub fn filter_diff(input: &str) -> String {
    if input.trim().is_empty() {
        return String::new();
    }

    let lines: Vec<&str> = input.lines().collect();
    let mut files: Vec<FileDiff> = Vec::new();
    let mut current_file: Option<String> = None;
    let mut additions: usize = 0;
    let mut deletions: usize = 0;
    let mut hunk_count: usize = 0;
    let mut change_lines: Vec<String> = Vec::new();

    for line in &lines {
        if let Some(caps) = DIFF_FILE_RE.captures(line) {
            // Save previous file
            if let Some(name) = current_file.take() {
                files.push(FileDiff {
                    name,
                    additions,
                    deletions,
                    hunks: hunk_count,
                    sample_changes: std::mem::take(&mut change_lines),
                });
            }
            current_file = Some(caps[2].to_string());
            additions = 0;
            deletions = 0;
            hunk_count = 0;
        } else if HUNK_RE.is_match(line) {
            hunk_count += 1;
        } else if ADD_RE.is_match(line) {
            additions += 1;
            if change_lines.len() < 5 {
                change_lines.push(line.to_string());
            }
        } else if DEL_RE.is_match(line) {
            deletions += 1;
            if change_lines.len() < 5 {
                change_lines.push(line.to_string());
            }
        }
    }

    // Save last file
    if let Some(name) = current_file {
        files.push(FileDiff {
            name,
            additions,
            deletions,
            hunks: hunk_count,
            sample_changes: change_lines,
        });
    }

    if files.is_empty() {
        return input.to_string();
    }

    let total_add: usize = files.iter().map(|f| f.additions).sum();
    let total_del: usize = files.iter().map(|f| f.deletions).sum();

    let mut result = format!(
        "{} files changed, +{} -{}\n",
        files.len(),
        total_add,
        total_del,
    );

    for file in &files {
        result.push_str(&format!(
            "\n{}  (+{} -{}, {} hunks)\n",
            file.name, file.additions, file.deletions, file.hunks
        ));
        for change in file.sample_changes.iter().take(3) {
            result.push_str(&format!("  {}\n", change));
        }
        let total_changes = file.additions + file.deletions;
        if total_changes > 3 {
            result.push_str(&format!("  ... +{} more changes\n", total_changes - 3));
        }
    }

    result.trim_end().to_string()
}

struct FileDiff {
    name: String,
    additions: usize,
    deletions: usize,
    hunks: usize,
    sample_changes: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use insta::assert_snapshot;

    fn count_tokens(s: &str) -> usize {
        s.split_whitespace().count()
    }

    #[test]
    fn test_diff_synthetic() {
        let input = r#"diff --git a/src/main.rs b/src/main.rs
index abc1234..def5678 100644
--- a/src/main.rs
+++ b/src/main.rs
@@ -1,5 +1,7 @@
 use anyhow::Result;
+use clap::Parser;

 fn main() {
-    println!("hello");
+    let cli = Cli::parse();
+    run(cli).unwrap();
 }
diff --git a/src/lib.rs b/src/lib.rs
index 1234567..abcdefg 100644
--- a/src/lib.rs
+++ b/src/lib.rs
@@ -1 +1,3 @@
+pub mod engine;
+pub mod compressors;
"#;
        let output = filter_diff(input);
        assert_snapshot!(output);
    }

    #[test]
    fn test_diff_savings() {
        let input = r#"diff --git a/file.rs b/file.rs
--- a/file.rs
+++ b/file.rs
@@ -1,10 +1,10 @@
-line1
-line2
-line3
-line4
-line5
+new1
+new2
+new3
+new4
+new5
+new6
+new7
+new8
+new9
+new10
"#;
        let output = filter_diff(input);
        let input_t = count_tokens(input);
        let output_t = count_tokens(&output);
        let savings = 100.0 - (output_t as f64 / input_t as f64 * 100.0);
        assert!(
            savings >= 20.0,
            "Expected >=20% savings, got {:.1}%",
            savings
        );
    }

    #[test]
    fn test_diff_empty() {
        assert_eq!(filter_diff(""), "");
    }

    #[test]
    fn test_diff_malformed() {
        let output = filter_diff("not a diff");
        assert!(!output.is_empty());
    }

    #[test]
    fn test_diff_unicode() {
        let input = "diff --git a/日本語.rs b/日本語.rs\n--- a/日本語.rs\n+++ b/日本語.rs\n@@ -1 +1 @@\n-old\n+new";
        let output = filter_diff(input);
        assert!(output.contains("日本語"));
    }
}

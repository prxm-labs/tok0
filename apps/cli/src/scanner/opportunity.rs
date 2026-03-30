use super::command_catalog::{classify, is_compressible};
use super::session_reader::{extract_commands_from_jsonl, find_claude_sessions};
use anyhow::{Context, Result};
use std::collections::HashMap;

#[derive(Debug)]
pub struct MissedOpportunity {
    pub command: String,
    pub category: &'static str,
    pub count: usize,
}

pub fn scan_sessions() -> Result<Vec<MissedOpportunity>> {
    let sessions = find_claude_sessions();
    let mut counts: HashMap<String, usize> = HashMap::new();

    for session_path in &sessions {
        let cmds = extract_commands_from_jsonl(session_path).unwrap_or_default();
        for sc in cmds {
            // Skip already-proxied commands
            if sc.command.starts_with("tok0 ") {
                continue;
            }
            // Extract base command (first word or pipe-separated first command)
            let base = sc.command.split('|').next().unwrap_or(&sc.command).trim();
            if is_compressible(base) {
                *counts.entry(base.to_string()).or_insert(0) += 1;
            }
        }
    }

    let mut opps: Vec<MissedOpportunity> = counts
        .into_iter()
        .filter_map(|(cmd, count)| {
            classify(&cmd).map(|cat| MissedOpportunity {
                command: cmd,
                category: cat,
                count,
            })
        })
        .collect();
    opps.sort_by_key(|o| std::cmp::Reverse(o.count));
    Ok(opps)
}

pub fn run() -> Result<()> {
    let opps = scan_sessions().context("Failed to scan session history")?;
    if opps.is_empty() {
        println!("No missed opportunities found — tok0 is covering all detected commands.");
        return Ok(());
    }
    println!("{:>5}  {:10}  COMMAND", "COUNT", "CATEGORY");
    println!("{}", "-".repeat(60));
    for opp in opps.iter().take(20) {
        println!("{:>5}  {:10}  {}", opp.count, opp.category, opp.command);
    }
    let total: usize = opps.iter().map(|o| o.count).sum();
    println!(
        "\nTotal missed: {} commands across {} unique patterns",
        total,
        opps.len()
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::super::session_reader::extract_commands_from_jsonl;
    use super::*;
    use std::io::Write;

    #[test]
    fn test_scan_empty_dir() {
        // Create an empty temp file — simulates a session with no commands
        let tmp = tempfile::NamedTempFile::new().expect("Failed to create temp file");
        let cmds = extract_commands_from_jsonl(tmp.path()).expect("extract failed");
        assert!(cmds.is_empty());
    }

    #[test]
    fn test_skips_proxied_commands() {
        let mut tmp = tempfile::NamedTempFile::new().expect("Failed to create temp file");
        // Already proxied via tok0
        writeln!(
            tmp,
            r#"{{"type":"tool_use","name":"Bash","input":{{"command":"tok0 git status"}}}}"#
        )
        .expect("write failed");
        // NOT proxied — should be detected
        writeln!(
            tmp,
            r#"{{"type":"tool_use","name":"Bash","input":{{"command":"git log --oneline"}}}}"#
        )
        .expect("write failed");
        tmp.flush().expect("flush failed");

        let cmds = extract_commands_from_jsonl(tmp.path()).expect("extract failed");
        assert_eq!(cmds.len(), 2);

        // Filter like scan_sessions does
        let missed: Vec<_> = cmds
            .iter()
            .filter(|sc| !sc.command.starts_with("tok0 "))
            .collect();
        assert_eq!(missed.len(), 1);
        assert_eq!(missed[0].command, "git log --oneline");
    }

    #[test]
    fn test_counts_aggregation() {
        let mut tmp = tempfile::NamedTempFile::new().expect("Failed to create temp file");
        for _ in 0..5 {
            writeln!(
                tmp,
                r#"{{"type":"tool_use","name":"Bash","input":{{"command":"git status"}}}}"#
            )
            .expect("write failed");
        }
        for _ in 0..3 {
            writeln!(
                tmp,
                r#"{{"type":"tool_use","name":"Bash","input":{{"command":"cargo test"}}}}"#
            )
            .expect("write failed");
        }
        tmp.flush().expect("flush failed");

        let cmds = extract_commands_from_jsonl(tmp.path()).expect("extract failed");
        assert_eq!(cmds.len(), 8);

        // Simulate aggregation logic from scan_sessions
        let mut counts: HashMap<String, usize> = HashMap::new();
        for sc in &cmds {
            if !sc.command.starts_with("tok0 ") {
                let base = sc.command.split('|').next().unwrap_or(&sc.command).trim();
                if is_compressible(base) {
                    *counts.entry(base.to_string()).or_insert(0) += 1;
                }
            }
        }
        assert_eq!(counts.get("git status"), Some(&5));
        assert_eq!(counts.get("cargo test"), Some(&3));
    }
}

//! `tok0 rule list|show|test` — rule management CLI.
//!
//! Surfaces what rules are currently active (built-in + extensions +
//! project-local), lets the user inspect a single rule's config, and
//! exercises a rule against stdin so they can see exactly what tok0
//! would compress it to.

use anyhow::{Context, Result};
use std::io::Read;

use crate::engine::rules::{apply_filter_config, FilterConfig};

/// Source of a rule — shown in `rule list` output for disambiguation.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RuleSource {
    Builtin,
    Extension,
    Project,
}

impl RuleSource {
    fn label(&self) -> &'static str {
        match self {
            RuleSource::Builtin => "builtin",
            RuleSource::Extension => "extension",
            RuleSource::Project => "project",
        }
    }
}

/// A rule with provenance.
pub struct LocatedRule {
    pub source: RuleSource,
    pub config: FilterConfig,
}

/// Enumerate every rule the engine would use, in the same priority order
/// as the live dispatch path: project (trust-gated) > extensions > builtin.
/// Called by both `rule list` and `rule show`.
pub fn enumerate_rules(project_trusted: bool) -> Vec<LocatedRule> {
    let mut out = Vec::new();

    // Project-local rules, only if trusted (mirrors main.rs behavior)
    if project_trusted {
        if let Ok(pwd) = std::env::current_dir() {
            let dir = pwd.join(".tok0").join("filters");
            if dir.is_dir() {
                if let Ok(rules) = crate::engine::rules::load_rules_from_dir(&dir, None) {
                    for r in rules {
                        out.push(LocatedRule {
                            source: RuleSource::Project,
                            config: r.filter,
                        });
                    }
                }
            }
        }
    }

    // Extensions
    if let Ok(rules) = crate::extensions::loader::load_extension_rules() {
        for r in rules {
            out.push(LocatedRule {
                source: RuleSource::Extension,
                config: r.filter,
            });
        }
    }

    // Builtins
    for r in crate::engine::builtin_rules::builtin_rules() {
        out.push(LocatedRule {
            source: RuleSource::Builtin,
            config: r.clone(),
        });
    }

    out
}

/// Render the `rule list` output. Pure + testable.
pub fn format_list(rules: &[LocatedRule]) -> String {
    if rules.is_empty() {
        return "No rules active.\n".to_string();
    }
    let mut out = String::new();
    out.push_str(&format!("{} rule(s) active:\n\n", rules.len()));
    out.push_str(&format!(
        "  {:<9}  {:<32}  {}\n",
        "SOURCE", "NAME", "COMMANDS"
    ));
    for r in rules {
        let commands = r.config.commands.join(", ");
        out.push_str(&format!(
            "  {:<9}  {:<32}  {}\n",
            r.source.label(),
            truncate(&r.config.name, 32),
            truncate(&commands, 60)
        ));
    }
    out
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}\u{2026}", &s[..max.saturating_sub(1)])
    }
}

/// Render detailed info for a single rule.
pub fn format_show(rule: &LocatedRule) -> String {
    let cfg = &rule.config;
    let mut out = String::new();
    out.push_str(&format!("name:     {}\n", cfg.name));
    out.push_str(&format!("source:   {}\n", rule.source.label()));
    out.push_str(&format!("commands: {}\n", cfg.commands.join(", ")));
    if let Some(ref ext) = cfg.extends {
        out.push_str(&format!("extends:  {}\n", ext));
    }
    if !cfg.strip_patterns.is_empty() {
        out.push_str(&format!("strip_patterns ({}):\n", cfg.strip_patterns.len()));
        for p in &cfg.strip_patterns {
            out.push_str(&format!("  - {}\n", p));
        }
    }
    if let Some(h) = cfg.head_lines {
        out.push_str(&format!("head_lines:    {}\n", h));
    }
    if let Some(t) = cfg.tail_lines {
        out.push_str(&format!("tail_lines:    {}\n", t));
    }
    if let Some(m) = cfg.max_line_chars {
        out.push_str(&format!("max_line_chars: {}\n", m));
    }
    if let Some(ref m) = cfg.empty_message {
        out.push_str(&format!("empty_message:  {:?}\n", m));
    }
    out
}

/// Apply a rule to input and render a before/after summary with savings.
pub fn format_test(rule: &LocatedRule, input: &str) -> String {
    let before_bytes = input.len();
    let before_lines = input.lines().count();
    let output = apply_filter_config(input, &rule.config);
    let after_bytes = output.len();
    let after_lines = output.lines().count();
    let saved_bytes = before_bytes.saturating_sub(after_bytes);
    let pct = if before_bytes == 0 {
        0.0
    } else {
        (saved_bytes as f64 / before_bytes as f64) * 100.0
    };

    let mut out = String::new();
    out.push_str(&format!(
        "rule: {} ({})\n",
        rule.config.name,
        rule.source.label()
    ));
    out.push_str(&format!(
        "input:  {} bytes, {} line(s)\n",
        before_bytes, before_lines
    ));
    out.push_str(&format!(
        "output: {} bytes, {} line(s)  ({:.1}% saved)\n",
        after_bytes, after_lines, pct
    ));
    out.push_str("\n--- output ---\n");
    out.push_str(&output);
    if !out.ends_with('\n') {
        out.push('\n');
    }
    out
}

// ─── CLI entry points ───────────────────────────────────────────────────

/// Detect whether the current working directory is a trusted project.
fn is_cwd_trusted() -> bool {
    std::env::current_dir()
        .ok()
        .and_then(|p| crate::bridge::trust::is_trusted(&p).ok())
        .unwrap_or(false)
}

pub fn run_list() -> Result<()> {
    let rules = enumerate_rules(is_cwd_trusted());
    print!("{}", format_list(&rules));
    Ok(())
}

pub fn run_show(name: &str) -> Result<()> {
    let rules = enumerate_rules(is_cwd_trusted());
    let found = rules
        .into_iter()
        .find(|r| r.config.name == name)
        .with_context(|| format!("No rule named `{}` found", name))?;
    print!("{}", format_show(&found));
    Ok(())
}

pub fn run_test(name: &str) -> Result<()> {
    let rules = enumerate_rules(is_cwd_trusted());
    let found = rules
        .into_iter()
        .find(|r| r.config.name == name)
        .with_context(|| format!("No rule named `{}` found", name))?;
    let mut input = String::new();
    std::io::stdin()
        .read_to_string(&mut input)
        .context("Failed to read stdin")?;
    print!("{}", format_test(&found, &input));
    Ok(())
}

// ─── Tests ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_rule(name: &str, source: RuleSource) -> LocatedRule {
        LocatedRule {
            source,
            config: FilterConfig {
                name: name.to_string(),
                commands: vec!["test".to_string()],
                strip_patterns: vec!["^noise".to_string()],
                head_lines: Some(5),
                tail_lines: Some(2),
                ..Default::default()
            },
        }
    }

    #[test]
    fn test_format_list_empty() {
        let out = format_list(&[]);
        assert!(out.contains("No rules active"));
    }

    #[test]
    fn test_format_list_includes_source_name_commands() {
        let rules = vec![
            make_rule("brew-install", RuleSource::Builtin),
            make_rule("my-rule", RuleSource::Project),
        ];
        let out = format_list(&rules);
        assert!(out.contains("2 rule(s) active"));
        assert!(out.contains("builtin"));
        assert!(out.contains("project"));
        assert!(out.contains("brew-install"));
        assert!(out.contains("my-rule"));
        assert!(out.contains("SOURCE"));
        assert!(out.contains("NAME"));
        assert!(out.contains("COMMANDS"));
    }

    #[test]
    fn test_format_list_truncates_long_names() {
        let rules = vec![make_rule(&"a".repeat(100), RuleSource::Builtin)];
        let out = format_list(&rules);
        assert!(
            out.contains("\u{2026}"),
            "should include ellipsis for truncated name"
        );
    }

    #[test]
    fn test_format_show_lists_all_fields() {
        let r = make_rule("my-rule", RuleSource::Extension);
        let out = format_show(&r);
        assert!(out.contains("name:     my-rule"));
        assert!(out.contains("source:   extension"));
        assert!(out.contains("commands: test"));
        assert!(out.contains("strip_patterns"));
        assert!(out.contains("^noise"));
        assert!(out.contains("head_lines:    5"));
        assert!(out.contains("tail_lines:    2"));
    }

    #[test]
    fn test_format_show_hides_empty_fields() {
        let r = LocatedRule {
            source: RuleSource::Builtin,
            config: FilterConfig {
                name: "minimal".to_string(),
                commands: vec!["x".to_string()],
                ..Default::default()
            },
        };
        let out = format_show(&r);
        assert!(out.contains("name:     minimal"));
        assert!(!out.contains("strip_patterns"));
        assert!(!out.contains("head_lines"));
        assert!(!out.contains("tail_lines"));
        assert!(!out.contains("extends"));
    }

    #[test]
    fn test_format_test_shows_before_after_and_savings() {
        let mut cfg = FilterConfig {
            name: "drop-noise".to_string(),
            commands: vec!["x".to_string()],
            strip_patterns: vec!["^noise".to_string()],
            ..Default::default()
        };
        cfg.compile_patterns();
        let r = LocatedRule {
            source: RuleSource::Builtin,
            config: cfg,
        };

        let input = "noise line 1\nreal content\nnoise line 2\nmore content\n";
        let out = format_test(&r, input);

        assert!(out.contains("drop-noise"));
        assert!(out.contains("input:"));
        assert!(out.contains("output:"));
        assert!(out.contains("saved"));
        assert!(out.contains("real content"));
        assert!(out.contains("more content"));
        assert!(!out.contains("noise line 1"));
    }

    #[test]
    fn test_format_test_empty_input_does_not_divide_by_zero() {
        let r = make_rule("x", RuleSource::Builtin);
        let out = format_test(&r, "");
        assert!(out.contains("0.0% saved") || out.contains("0% saved"));
    }

    #[test]
    fn test_enumerate_rules_includes_builtins() {
        // is_cwd_trusted irrelevant — we're not in a project.
        let rules = enumerate_rules(false);
        // Should have many builtins (>20)
        assert!(
            rules.len() >= 20,
            "expected at least 20 builtin rules, got {}",
            rules.len()
        );
        assert!(
            rules.iter().all(|r| r.source != RuleSource::Project),
            "without trust, no project rules should appear"
        );
    }

    #[test]
    fn test_truncate_preserves_short_strings() {
        assert_eq!(truncate("hello", 10), "hello");
    }

    #[test]
    fn test_truncate_ellipsizes_long_strings() {
        let out = truncate("this is too long", 8);
        assert!(out.ends_with('\u{2026}'));
        assert_eq!(out.chars().count(), 8);
    }
}

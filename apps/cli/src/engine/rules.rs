use anyhow::{Context, Result};
use regex::Regex;
use std::collections::HashMap;
use std::path::Path;

#[derive(Debug, Clone, serde::Deserialize)]
pub struct FilterRule {
    pub filter: FilterConfig,
}

#[derive(Debug, Clone, serde::Deserialize, Default)]
#[allow(dead_code)]
pub struct FilterConfig {
    pub name: String,
    pub commands: Vec<String>,
    #[serde(default)]
    pub extends: Option<String>,
    #[serde(default)]
    pub strip_patterns: Vec<String>,
    #[serde(default)]
    pub head_lines: Option<usize>,
    #[serde(default)]
    pub tail_lines: Option<usize>,
    #[serde(default)]
    pub max_line_chars: Option<usize>,
    #[serde(default)]
    pub empty_message: Option<String>,
    /// Pre-compiled regexes for strip_patterns (populated by compile_patterns).
    #[serde(skip)]
    pub(crate) compiled_patterns: Vec<Regex>,
}

/// Reject any pattern source longer than this. Anything bigger is almost
/// certainly an attack — legitimate strip patterns are <100 chars.
const MAX_PATTERN_SOURCE_BYTES: usize = 4096;

/// Cap the size of compiled regex programs and DFA cache. Both limits
/// are advisory — the regex crate refuses to compile if exceeded rather
/// than running unbounded.
const MAX_REGEX_PROGRAM_BYTES: usize = 1 << 20; // 1 MiB
const MAX_REGEX_DFA_BYTES: usize = 1 << 20; // 1 MiB

impl FilterConfig {
    /// Compile strip_patterns into Regex objects for reuse.
    ///
    /// Each pattern is bounded by:
    ///   - source length (`MAX_PATTERN_SOURCE_BYTES`)
    ///   - compiled program size (`RegexBuilder::size_limit`)
    ///   - DFA cache size (`RegexBuilder::dfa_size_limit`)
    ///
    /// Anything that fails any of these is silently dropped — same
    /// behaviour as before for plain invalid patterns. This blocks
    /// extension-supplied TOML rules from inducing pathological
    /// regex cost. ROADMAP T1.1.
    pub fn compile_patterns(&mut self) {
        self.compiled_patterns = self
            .strip_patterns
            .iter()
            .filter(|p| p.len() <= MAX_PATTERN_SOURCE_BYTES)
            .filter_map(|p| {
                regex::RegexBuilder::new(p)
                    .size_limit(MAX_REGEX_PROGRAM_BYTES)
                    .dfa_size_limit(MAX_REGEX_DFA_BYTES)
                    .build()
                    .ok()
            })
            .collect();
    }

    /// Return compiled regexes, compiling on-demand if needed.
    fn compiled_strip_patterns(&self) -> Vec<Regex> {
        if !self.compiled_patterns.is_empty() || self.strip_patterns.is_empty() {
            return self.compiled_patterns.clone();
        }
        // Fallback: compile on the fly with the same caps as compile_patterns.
        self.strip_patterns
            .iter()
            .filter(|p| p.len() <= MAX_PATTERN_SOURCE_BYTES)
            .filter_map(|p| {
                regex::RegexBuilder::new(p)
                    .size_limit(MAX_REGEX_PROGRAM_BYTES)
                    .dfa_size_limit(MAX_REGEX_DFA_BYTES)
                    .build()
                    .ok()
            })
            .collect()
    }
}

/// Parse a TOML string into a FilterConfig.
/// Pre-compiles strip_patterns into regexes.
#[allow(dead_code)]
pub fn parse_filter_config(toml_str: &str) -> Result<FilterConfig> {
    let mut rule: FilterRule =
        toml::from_str(toml_str).context("Failed to parse filter rule TOML")?;
    rule.filter.compile_patterns();
    Ok(rule.filter)
}

/// Load a single rule file from disk.
/// Pre-compiles strip_patterns into regexes.
pub fn load_rule_file(path: &Path) -> Result<FilterRule> {
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read rule file: {}", path.display()))?;
    let mut rule: FilterRule = toml::from_str(&content)
        .with_context(|| format!("Failed to parse rule file: {}", path.display()))?;
    rule.filter.compile_patterns();
    Ok(rule)
}

/// Load all .toml rule files from a directory, optionally using a cache.
pub fn load_rules_from_dir(
    dir: &Path,
    mut cache: Option<&mut super::rules_cache::RulesCache>,
) -> Result<Vec<FilterRule>> {
    let mut rules = Vec::new();
    for entry in std::fs::read_dir(dir)
        .with_context(|| format!("Failed to read rules dir: {}", dir.display()))?
    {
        let entry = entry.context("Failed to read dir entry")?;
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("toml") {
            continue;
        }
        let rule = if let Some(ref mut c) = cache {
            c.load_or_parse(&path)?.clone()
        } else {
            load_rule_file(&path)?
        };
        rules.push(rule);
    }
    Ok(rules)
}

/// Resolve `extends` inheritance: merge parent fields into child.
/// Child fields override parent; strip_patterns are appended.
#[allow(dead_code)]
pub fn resolve_extends(rule: &FilterConfig, registry: &[FilterConfig]) -> Result<FilterConfig> {
    let Some(parent_name) = &rule.extends else {
        return Ok(rule.clone());
    };
    let parent = registry
        .iter()
        .find(|r| &r.name == parent_name)
        .with_context(|| {
            format!(
                "Rule '{}' extends unknown base '{}'",
                rule.name, parent_name
            )
        })?;

    let mut merged = parent.clone();
    merged.name = rule.name.clone();
    merged.commands = rule.commands.clone();
    merged.extends = None;
    // Append child strip_patterns to parent's
    merged
        .strip_patterns
        .extend(rule.strip_patterns.iter().cloned());
    if rule.head_lines.is_some() {
        merged.head_lines = rule.head_lines;
    }
    if rule.tail_lines.is_some() {
        merged.tail_lines = rule.tail_lines;
    }
    if rule.max_line_chars.is_some() {
        merged.max_line_chars = rule.max_line_chars;
    }
    if rule.empty_message.is_some() {
        merged.empty_message = rule.empty_message.clone();
    }
    merged.compile_patterns();
    Ok(merged)
}

/// Apply a FilterConfig's rules to raw output.
pub fn apply_filter_config(input: &str, config: &FilterConfig) -> String {
    if input.trim().is_empty() {
        return config.empty_message.clone().unwrap_or_default();
    }

    // Stage 1: strip_patterns — remove lines matching any pattern
    let mut lines: Vec<&str> = input.lines().collect();
    let compiled = config.compiled_strip_patterns();
    for re in &compiled {
        lines.retain(|line| !re.is_match(line));
    }

    // Stage 2: head/tail window
    let lines = apply_head_tail(lines, config.head_lines, config.tail_lines);

    // Stage 3: max_line_chars truncation
    let result = if let Some(max) = config.max_line_chars {
        lines
            .iter()
            .map(|l| if l.len() > max { &l[..max] } else { l })
            .collect::<Vec<_>>()
            .join("\n")
    } else {
        lines.join("\n")
    };

    // Stage 4: empty_message fallback
    if result.trim().is_empty() {
        config.empty_message.clone().unwrap_or_default()
    } else {
        result
    }
}

fn apply_head_tail(lines: Vec<&str>, head: Option<usize>, tail: Option<usize>) -> Vec<&str> {
    match (head, tail) {
        (Some(h), Some(t)) if lines.len() > h + t => {
            let mut result: Vec<&str> = lines[..h].to_vec();
            result.extend_from_slice(&lines[lines.len() - t..]);
            result
        }
        (Some(h), None) if lines.len() > h => lines[..h].to_vec(),
        (None, Some(t)) if lines.len() > t => lines[lines.len() - t..].to_vec(),
        _ => lines,
    }
}

/// Find the first rule whose commands match the given command string.
#[allow(dead_code)]
pub fn find_matching_rule<'a>(
    command: &str,
    rules: &'a [FilterConfig],
) -> Option<&'a FilterConfig> {
    rules.iter().find(|rule| {
        rule.commands
            .iter()
            .any(|cmd_pattern| command.starts_with(cmd_pattern.as_str()))
    })
}

/// Pre-indexed rule lookup for O(1) matching by first command word.
/// Use this when dispatching many commands against the same rule set.
pub struct RuleIndex<'a> {
    /// Maps first word of command pattern → list of (full pattern, rule ref)
    by_first_word: HashMap<String, Vec<(&'a str, &'a FilterConfig)>>,
}

impl<'a> RuleIndex<'a> {
    /// Build an index from a slice of FilterConfigs.
    /// Candidates are sorted by pattern length (longest first) so that
    /// more specific rules match before generic catch-alls.
    pub fn build(rules: &'a [FilterConfig]) -> Self {
        let mut by_first_word: HashMap<String, Vec<(&'a str, &'a FilterConfig)>> = HashMap::new();
        for rule in rules {
            for cmd_pattern in &rule.commands {
                let first_word = cmd_pattern.split_whitespace().next().unwrap_or(cmd_pattern);
                by_first_word
                    .entry(first_word.to_string())
                    .or_default()
                    .push((cmd_pattern.as_str(), rule));
            }
        }
        // Sort each bucket: longest patterns first (most specific wins)
        for candidates in by_first_word.values_mut() {
            candidates.sort_by_key(|c| std::cmp::Reverse(c.0.len()));
        }
        Self { by_first_word }
    }

    /// Find the first matching rule for a command string. O(1) lookup by
    /// first word, then linear scan on the (small) candidate set.
    /// Most specific (longest) pattern matches first.
    pub fn find(&self, command: &str) -> Option<&'a FilterConfig> {
        let first_word = command.split_whitespace().next().unwrap_or(command);
        let candidates = self.by_first_word.get(first_word)?;
        candidates
            .iter()
            .find(|(pattern, _)| command.starts_with(*pattern))
            .map(|(_, rule)| *rule)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_parse_filter_config_basic() {
        let toml_str = r#"
[filter]
name = "test-rule"
commands = ["test"]
strip_patterns = ["^debug:"]
head_lines = 10
empty_message = "ok"
"#;
        let config = parse_filter_config(toml_str).expect("should parse");
        assert_eq!(config.name, "test-rule");
        assert_eq!(config.commands, vec!["test"]);
        assert_eq!(config.strip_patterns, vec!["^debug:"]);
        assert_eq!(config.head_lines, Some(10));
        assert_eq!(config.empty_message, Some("ok".to_string()));
    }

    #[test]
    fn test_parse_filter_config_minimal() {
        let toml_str = r#"
[filter]
name = "minimal"
commands = ["foo"]
"#;
        let config = parse_filter_config(toml_str).expect("should parse");
        assert_eq!(config.name, "minimal");
        assert!(config.strip_patterns.is_empty());
        assert!(config.head_lines.is_none());
        assert!(config.empty_message.is_none());
    }

    #[test]
    fn test_load_rule_file() {
        let tmp = TempDir::new().expect("tmp dir");
        let path = tmp.path().join("test.toml");
        std::fs::write(
            &path,
            r#"
[filter]
name = "file-rule"
commands = ["test"]
"#,
        )
        .expect("write");
        let rule = load_rule_file(&path).expect("load");
        assert_eq!(rule.filter.name, "file-rule");
    }

    #[test]
    fn test_load_rules_from_dir() {
        let tmp = TempDir::new().expect("tmp dir");
        std::fs::write(
            tmp.path().join("a.toml"),
            r#"
[filter]
name = "rule-a"
commands = ["a"]
"#,
        )
        .expect("write a");
        std::fs::write(
            tmp.path().join("b.toml"),
            r#"
[filter]
name = "rule-b"
commands = ["b"]
"#,
        )
        .expect("write b");
        // Non-toml file should be skipped
        std::fs::write(tmp.path().join("readme.txt"), "not a rule").expect("write txt");

        let rules = load_rules_from_dir(tmp.path(), None).expect("load dir");
        assert_eq!(rules.len(), 2);
    }

    #[test]
    fn test_parse_invalid_toml_errors() {
        let result = parse_filter_config("not valid toml {{{}}}");
        assert!(result.is_err());
    }

    #[test]
    fn test_extends_merges_parent_strips() {
        let parent_toml = r#"
[filter]
name = "base-git"
commands = ["git"]
strip_patterns = ["^Counting objects", "^Compressing"]
empty_message = "ok"
"#;
        let child_toml = r#"
[filter]
name = "git-fetch"
commands = ["git fetch"]
extends = "base-git"
strip_patterns = ["^remote:"]
"#;
        let registry = vec![
            parse_filter_config(parent_toml).expect("parse parent"),
            parse_filter_config(child_toml).expect("parse child"),
        ];
        let resolved = resolve_extends(&registry[1], &registry).expect("resolve");
        assert!(resolved
            .strip_patterns
            .contains(&"^Counting objects".to_string()));
        assert!(resolved.strip_patterns.contains(&"^remote:".to_string()));
        assert_eq!(resolved.empty_message, Some("ok".to_string()));
    }

    #[test]
    fn test_child_overrides_parent_empty_message() {
        let parent_toml = r#"
[filter]
name = "base"
commands = []
empty_message = "parent msg"
"#;
        let child_toml = r#"
[filter]
name = "child"
commands = ["foo"]
extends = "base"
empty_message = "child msg"
"#;
        let registry = vec![
            parse_filter_config(parent_toml).expect("parse parent"),
            parse_filter_config(child_toml).expect("parse child"),
        ];
        let resolved = resolve_extends(&registry[1], &registry).expect("resolve");
        assert_eq!(resolved.empty_message, Some("child msg".to_string()));
    }

    #[test]
    fn test_extends_unknown_base_errors() {
        let child_toml = r#"
[filter]
name = "orphan"
commands = ["foo"]
extends = "nonexistent"
"#;
        let registry = vec![parse_filter_config(child_toml).expect("parse child")];
        assert!(resolve_extends(&registry[0], &registry).is_err());
    }

    #[test]
    fn test_no_extends_returns_clone() {
        let toml_str = r#"
[filter]
name = "standalone"
commands = ["test"]
strip_patterns = ["^debug:"]
"#;
        let config = parse_filter_config(toml_str).expect("parse");
        let resolved = resolve_extends(&config, std::slice::from_ref(&config)).expect("resolve");
        assert_eq!(resolved.name, "standalone");
        assert_eq!(resolved.strip_patterns, vec!["^debug:"]);
    }

    #[test]
    fn test_compile_patterns_rejects_oversized_regex() {
        // Patterns > 4096 chars are silently dropped — no compiled regex
        // shows up in the cache, so they apply nothing at runtime.
        let mut cfg = FilterConfig {
            name: "evil".to_string(),
            commands: vec!["evil".to_string()],
            strip_patterns: vec!["a".repeat(10_000)],
            ..Default::default()
        };
        cfg.compile_patterns();
        assert!(
            cfg.compiled_patterns.is_empty(),
            "oversized regex pattern must be rejected"
        );
    }

    #[test]
    fn test_compile_patterns_caps_dfa_size() {
        // A pathological pattern that would normally explode the DFA cache
        // must either compile under the 1 MiB cap or be dropped — never
        // succeed unbounded. RegexBuilder.size_limit + dfa_size_limit
        // enforce both.
        let mut cfg = FilterConfig {
            name: "evil".to_string(),
            commands: vec!["evil".to_string()],
            // Long alternation with backtracky structure
            strip_patterns: vec!["(a|aa|aaa|aaaa|aaaaa)+b".repeat(50)],
            ..Default::default()
        };
        cfg.compile_patterns();
        // No assertion on count — the goal is no panic / unbounded growth.
        let start = std::time::Instant::now();
        let input = "a".repeat(80) + "x";
        let _ = apply_filter_config(&input, &cfg);
        assert!(
            start.elapsed().as_millis() < 500,
            "pattern application must never take > 500 ms on adversarial input"
        );
    }

    #[test]
    fn test_apply_rule_strips_patterns() {
        let mut config = FilterConfig {
            name: "test".to_string(),
            commands: vec!["test".to_string()],
            strip_patterns: vec!["^noise".to_string(), "^debug".to_string()],
            ..Default::default()
        };
        config.compile_patterns();
        let input = "noise: blah\nreal output\ndebug: stuff\nmore output";
        let output = apply_filter_config(input, &config);
        assert!(!output.contains("noise"));
        assert!(!output.contains("debug"));
        assert!(output.contains("real output"));
        assert!(output.contains("more output"));
    }

    #[test]
    fn test_apply_rule_head_tail() {
        let config = FilterConfig {
            name: "test".to_string(),
            commands: vec!["test".to_string()],
            head_lines: Some(2),
            tail_lines: Some(1),
            ..Default::default()
        };
        let input = "line1\nline2\nline3\nline4\nline5";
        let output = apply_filter_config(input, &config);
        assert!(output.contains("line1"));
        assert!(output.contains("line2"));
        assert!(output.contains("line5"));
        assert!(!output.contains("line3"));
    }

    #[test]
    fn test_apply_rule_empty_message() {
        let mut config = FilterConfig {
            name: "test".to_string(),
            commands: vec!["test".to_string()],
            strip_patterns: vec![".*".to_string()],
            empty_message: Some("ok".to_string()),
            ..Default::default()
        };
        config.compile_patterns();
        let input = "all noise\nmore noise";
        let output = apply_filter_config(input, &config);
        assert_eq!(output, "ok");
    }

    #[test]
    fn test_apply_rule_max_line_chars() {
        let config = FilterConfig {
            name: "test".to_string(),
            commands: vec!["test".to_string()],
            max_line_chars: Some(10),
            ..Default::default()
        };
        let input = "short\nthis is a very long line that should be truncated";
        let output = apply_filter_config(input, &config);
        assert!(output.contains("short"));
        // Long line should be truncated to 10 chars
        for line in output.lines() {
            assert!(line.len() <= 10, "Line too long: '{}'", line);
        }
    }

    #[test]
    fn test_apply_rule_empty_input() {
        let config = FilterConfig {
            name: "test".to_string(),
            commands: vec!["test".to_string()],
            empty_message: Some("nothing".to_string()),
            ..Default::default()
        };
        let output = apply_filter_config("", &config);
        assert_eq!(output, "nothing");
    }

    #[test]
    fn test_find_matching_rule() {
        let rules = vec![
            FilterConfig {
                name: "git-log".to_string(),
                commands: vec!["git log".to_string()],
                ..Default::default()
            },
            FilterConfig {
                name: "npm-install".to_string(),
                commands: vec!["npm install".to_string()],
                ..Default::default()
            },
        ];
        let matched = find_matching_rule("npm install foo", &rules);
        assert!(matched.is_some());
        assert_eq!(matched.expect("should match").name, "npm-install");
    }

    #[test]
    fn test_find_matching_rule_no_match() {
        let rules = vec![FilterConfig {
            name: "git-log".to_string(),
            commands: vec!["git log".to_string()],
            ..Default::default()
        }];
        assert!(find_matching_rule("cargo build", &rules).is_none());
    }
}

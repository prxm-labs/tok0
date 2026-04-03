use anyhow::{Context, Result};

use crate::engine::rules::load_rules_from_dir;
use crate::engine::rules::FilterRule;

use super::catalog::extensions_dir;

/// Load all rule files from installed extensions.
/// Each extension is expected to have a `rules/` subdirectory with `.toml` files.
pub fn load_extension_rules() -> Result<Vec<FilterRule>> {
    load_extension_rules_from(&extensions_dir())
}

/// Load all rule files from a specific extensions directory.
pub fn load_extension_rules_from(ext_dir: &std::path::Path) -> Result<Vec<FilterRule>> {
    if !ext_dir.exists() {
        return Ok(vec![]);
    }
    let mut all_rules = Vec::new();
    for entry in std::fs::read_dir(ext_dir)
        .with_context(|| format!("Failed to read extensions dir: {}", ext_dir.display()))?
    {
        let entry = entry.context("Failed to read entry")?;
        let rules_dir = entry.path().join("rules");
        if rules_dir.is_dir() {
            let rules = load_rules_from_dir(&rules_dir, None).with_context(|| {
                format!(
                    "Failed to load rules from extension: {}",
                    entry.path().display()
                )
            })?;
            all_rules.extend(rules);
        }
    }
    Ok(all_rules)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_load_extension_rules_no_dir() {
        let tmp = TempDir::new().expect("tmp dir");
        let ext_dir = tmp.path().join("extensions");
        // extensions dir doesn't exist
        let rules = load_extension_rules_from(&ext_dir).expect("load");
        assert!(rules.is_empty());
    }

    #[test]
    fn test_load_extension_rules_with_rules_dir() {
        let tmp = TempDir::new().expect("tmp dir");
        let ext_dir = tmp.path().join("extensions");
        let rules_dir = ext_dir.join("test-pack").join("rules");
        std::fs::create_dir_all(&rules_dir).expect("create rules dir");
        std::fs::write(
            rules_dir.join("test.toml"),
            r#"
[filter]
name = "ext-rule"
commands = ["ext-cmd"]
"#,
        )
        .expect("write rule");

        let rules = load_extension_rules_from(&ext_dir).expect("load");
        assert_eq!(rules.len(), 1);
        assert_eq!(rules[0].filter.name, "ext-rule");
    }

    #[test]
    fn test_skips_extensions_without_rules_dir() {
        let tmp = TempDir::new().expect("tmp dir");
        let ext_dir = tmp.path().join("extensions");
        // Extension with no rules/ subdir
        std::fs::create_dir_all(ext_dir.join("no-rules-pack")).expect("create dir");
        let rules = load_extension_rules_from(&ext_dir).expect("load");
        assert!(rules.is_empty());
    }
}

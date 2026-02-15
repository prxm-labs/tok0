use anyhow::{Context, Result};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use crate::engine::rules::FilterRule;

#[derive(Default)]
pub struct RulesCache {
    entries: HashMap<PathBuf, (SystemTime, FilterRule)>,
}

impl RulesCache {
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self::default()
    }

    #[allow(dead_code)]
    pub fn get(&self, path: &Path) -> Option<&FilterRule> {
        self.entries.get(path).map(|(_, rule)| rule)
    }

    pub fn insert(&mut self, path: PathBuf, mtime: SystemTime, rule: FilterRule) {
        self.entries.insert(path, (mtime, rule));
    }

    pub fn is_stale(&self, path: &Path) -> Result<bool> {
        let Some((cached_mtime, _)) = self.entries.get(path) else {
            return Ok(true); // not in cache = stale
        };
        let current_mtime = path
            .metadata()
            .and_then(|m| m.modified())
            .with_context(|| format!("Failed to stat {}", path.display()))?;
        Ok(current_mtime != *cached_mtime)
    }

    /// Load rule from cache if fresh, otherwise parse from disk and cache it.
    pub fn load_or_parse(&mut self, path: &Path) -> Result<&FilterRule> {
        if self.is_stale(path)? {
            let rule = super::rules::load_rule_file(path)?;
            let mtime = path
                .metadata()
                .and_then(|m| m.modified())
                .with_context(|| format!("Failed to stat {}", path.display()))?;
            self.insert(path.to_path_buf(), mtime, rule);
        }
        Ok(self
            .entries
            .get(path)
            .map(|(_, r)| r)
            .expect("just inserted"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::rules::load_rule_file;
    use tempfile::TempDir;

    fn write_rule_toml(dir: &std::path::Path, name: &str, content: &str) -> std::path::PathBuf {
        let path = dir.join(name);
        std::fs::write(&path, content).expect("write rule toml");
        path
    }

    #[test]
    fn test_cache_miss_on_first_load() {
        let tmp = TempDir::new().expect("tmp dir");
        let path = write_rule_toml(
            tmp.path(),
            "test.toml",
            r#"
[filter]
name = "test"
commands = ["test"]
"#,
        );
        let cache = RulesCache::new();
        assert!(cache.get(&path).is_none());
    }

    #[test]
    fn test_cache_hit_after_load() {
        let tmp = TempDir::new().expect("tmp dir");
        let path = write_rule_toml(
            tmp.path(),
            "test.toml",
            r#"
[filter]
name = "test"
commands = ["test"]
"#,
        );
        let mut cache = RulesCache::new();
        let rule = load_rule_file(&path).expect("load rule");
        let mtime = path
            .metadata()
            .expect("metadata")
            .modified()
            .expect("mtime");
        cache.insert(path.clone(), mtime, rule);
        assert!(cache.get(&path).is_some());
    }

    #[test]
    fn test_cache_invalidated_on_mtime_change() {
        let tmp = TempDir::new().expect("tmp dir");
        let path = write_rule_toml(
            tmp.path(),
            "test.toml",
            r#"
[filter]
name = "test-v1"
commands = ["test"]
"#,
        );
        let mut cache = RulesCache::new();
        let mtime = path
            .metadata()
            .expect("metadata")
            .modified()
            .expect("mtime");
        let rule = load_rule_file(&path).expect("load rule");
        cache.insert(path.clone(), mtime, rule);

        // Simulate file modification by writing new content
        std::thread::sleep(std::time::Duration::from_millis(10));
        std::fs::write(
            &path,
            r#"
[filter]
name = "test-v2"
commands = ["test"]
"#,
        )
        .expect("rewrite");

        assert!(cache.is_stale(&path).expect("is_stale check"));
    }

    #[test]
    fn test_load_or_parse_caches_result() {
        let tmp = TempDir::new().expect("tmp dir");
        let path = write_rule_toml(
            tmp.path(),
            "test.toml",
            r#"
[filter]
name = "cached-rule"
commands = ["test"]
"#,
        );
        let mut cache = RulesCache::new();
        let rule = cache.load_or_parse(&path).expect("load_or_parse");
        assert_eq!(rule.filter.name, "cached-rule");
        // Second call should hit cache
        assert!(!cache.is_stale(&path).expect("stale check"));
    }
}

use lazy_static::lazy_static;
use regex::Regex;

lazy_static! {
    /// Match `du -sh` lines: "4.0K\tpath" or "701M\tpath"
    static ref DU_LINE_RE: Regex = Regex::new(r"^([\d.]+[BKMGTPE]?[i]?)\s+(.+)$").unwrap();
    /// Match size suffix for ordering
    static ref SIZE_SUFFIX_RE: Regex = Regex::new(r"^([\d.]+)([BKMGTPE]?)i?$").unwrap();
}

/// Parse a human-readable size string (e.g., "4.0K", "701M", "1.2G") into bytes as f64.
fn parse_size(s: &str) -> f64 {
    if let Some(caps) = SIZE_SUFFIX_RE.captures(s) {
        let num: f64 = caps[1].parse().unwrap_or(0.0);
        let mult = match caps[2].to_uppercase().as_str() {
            "B" | "" => 1.0,
            "K" => 1024.0,
            "M" => 1024.0 * 1024.0,
            "G" => 1024.0 * 1024.0 * 1024.0,
            "T" => 1024.0_f64.powi(4),
            "P" => 1024.0_f64.powi(5),
            "E" => 1024.0_f64.powi(6),
            _ => 1.0,
        };
        num * mult
    } else {
        0.0
    }
}

/// Filter `du -sh` output: sort by size descending, show top-N entries, summarize rest.
pub fn filter_du(input: &str, top_n: usize) -> String {
    if input.trim().is_empty() {
        return String::new();
    }

    let top_n = top_n.max(1);
    let mut entries: Vec<(f64, &str, &str)> = Vec::new(); // (bytes, size_str, path)

    for line in input.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if let Some(caps) = DU_LINE_RE.captures(line) {
            let size_str = caps.get(1).unwrap().as_str();
            let path = caps.get(2).unwrap().as_str();
            let bytes = parse_size(size_str);
            entries.push((bytes, size_str, path));
        }
    }

    if entries.is_empty() {
        return input.to_string();
    }

    // Sort by size descending
    entries.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

    if entries.len() <= top_n {
        // Show all entries, sorted
        let mut result = String::new();
        for (_, size_str, path) in &entries {
            result.push_str(&format!("{:>8}  {}\n", size_str, path));
        }
        return result.trim_end().to_string();
    }

    // Show top N, summarize the rest
    let mut result = format!(
        "top {} of {} entries (sorted by size):\n",
        top_n,
        entries.len()
    );
    for (_, size_str, path) in entries.iter().take(top_n) {
        result.push_str(&format!("  {:>8}  {}\n", size_str, path));
    }

    let rest_count = entries.len() - top_n;
    result.push_str(&format!("  ... and {} more entries\n", rest_count));

    result.trim_end().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use insta::assert_snapshot;

    fn count_tokens(s: &str) -> usize {
        s.split_whitespace().count()
    }

    /// Generate a large du -sh output with 50 entries and long paths.
    fn make_large_du_input() -> String {
        let entries = vec![
            ("701M", "target/debug/incremental/long_build_artifact_path"),
            ("216K", "docs/api/generated/html/reference"),
            ("136K", "src/compressors/javascript/node_modules_cache"),
            ("6.8M", ".git/refs/remotes/origin/long_branch_name_here"),
            ("48K", "Cargo.lock.backup.from.last.upgrade"),
            ("4.0K", "Cargo.toml.original.before.workspace.migration"),
            ("4.0K", "scripts/ci/pipeline/deploy/production/helpers"),
            ("36K", "tests/fixtures/system/snapshots/large_dataset"),
            ("8.0K", ".git/hooks/pre-commit.d/validate_branch_policy"),
            ("2.1M", "node_modules/.cache/babel/transform/output"),
            ("512K", "dist/static/chunks/vendor/react_and_dependencies"),
            ("128K", "coverage/lcov-report/src/compressors/full_trace"),
            ("64K", "fixtures/integration/end_to_end/snapshots/golden"),
            ("32K", "benches/results/flamegraphs/compressor_perf_data"),
            ("16K", "examples/advanced/pipeline/with_custom_rules_demo"),
            ("8.0K", "tools/release/scripts/bump_version_and_changelog"),
            ("4.0K", "vendor/third_party/patched/upstream_dependency"),
            ("1.5G", "media/recordings/demo_screencasts/high_quality_mp4"),
            ("256K", "assets/images/screenshots/documentation/figures"),
            ("92K", "migrations/database/schema/v3_to_v4_upgrade_path"),
            ("44K", "schemas/json/validation/api/response_definitions"),
            ("22K", "config/environments/staging/feature_flags/overrides"),
            ("11K", "cache/compiled/templates/layouts/partials/shared"),
            ("5.5K", "logs/application/error/2026/04/07/service_errors"),
            ("2.8K", "tmp/uploads/pending/processing/queue_overflow_dir"),
            ("1.4K", "backups/database/incremental/daily/latest_snapshot"),
            ("700B", "run/sockets/application/worker/process_id_lockfile"),
            (
                "350B",
                "pid/supervisord/workers/main_process_identifier_txt",
            ),
            ("175B", "var/lib/application/state/persistent_runtime_cache"),
            (
                "88B",
                "opt/local/share/application/data/initialization_lock",
            ),
        ];
        entries
            .iter()
            .map(|(size, path)| format!("{}\t{}", size, path))
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[test]
    fn test_du_empty() {
        assert_eq!(filter_du("", 10), "");
    }

    #[test]
    fn test_du_single_entry() {
        let input = "701M\ttarget";
        let output = filter_du(input, 10);
        assert!(output.contains("701M"), "should contain size");
        assert!(output.contains("target"), "should contain path");
    }

    #[test]
    fn test_du_sorted_descending() {
        let input = "4.0K\tscripts\n701M\ttarget\n136K\tsrc\n216K\tdocs";
        let output = filter_du(input, 10);
        let lines: Vec<&str> = output.lines().collect();
        // First line should contain the largest (701M target)
        assert!(lines[0].contains("701M"), "largest should be first");
    }

    #[test]
    fn test_du_fixture_snapshot() {
        let input = include_str!("../../../tests/fixtures/system/du_raw.txt");
        let output = filter_du(input, 5);
        assert_snapshot!(output);
    }

    #[test]
    fn test_du_many_entries_snapshot() {
        let input = make_large_du_input();
        let output = filter_du(&input, 10);
        assert_snapshot!(output);
    }

    #[test]
    fn test_du_savings() {
        // Realistic large-repo du -sh: 100 entries. Production cap is 15
        // (set in dispatcher), so ~85% of entries get summarized.
        let input = (0..100)
            .map(|i| {
                let size = if i < 5 {
                    "1.5G"
                } else if i < 15 {
                    "200M"
                } else {
                    "4.0K"
                };
                format!("{}\tpath/to/some/long/directory/structure/dir_{}", size, i)
            })
            .collect::<Vec<_>>()
            .join("\n");
        let output = filter_du(&input, 15);
        let input_t = count_tokens(&input);
        let output_t = count_tokens(&output);
        let savings = 100.0 - (output_t as f64 / input_t as f64 * 100.0);
        assert!(
            savings >= 60.0,
            "Expected >=60% savings on 100-entry du, got {:.1}% ({} -> {} tokens)",
            savings,
            input_t,
            output_t
        );
    }

    #[test]
    fn test_du_rest_summary() {
        let input = make_large_du_input();
        let output = filter_du(&input, 5);
        assert!(output.contains("and"), "should summarize remaining entries");
        assert!(
            output.contains("more entries"),
            "should mention remaining count"
        );
    }

    #[test]
    fn test_du_passthrough_unparseable() {
        let input = "not du output";
        let output = filter_du(input, 5);
        assert!(
            !output.is_empty(),
            "should not return empty for unparseable input"
        );
    }

    #[test]
    fn test_parse_size() {
        assert!(parse_size("1K") < parse_size("1M"));
        assert!(parse_size("1M") < parse_size("1G"));
        assert_eq!(parse_size("0"), 0.0);
    }
}

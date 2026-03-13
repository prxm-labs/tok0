use lazy_static::lazy_static;
use regex::Regex;

lazy_static! {
    // psql separator line: "-----+------+------" etc.
    static ref PSQL_SEP_RE: Regex =
        Regex::new(r"^[-+]+$").unwrap();

    // psql footer: "(N rows)"
    static ref PSQL_FOOTER_RE: Regex =
        Regex::new(r"^\(\d+ rows?\)$").unwrap();

    // redis section header: "# SectionName"
    static ref REDIS_SECTION_RE: Regex =
        Regex::new(r"^# \w+").unwrap();

    // redis key metrics to keep
    static ref REDIS_KEEP_KEY_RE: Regex =
        Regex::new(
            r"^(?:used_memory_human|used_memory_peak_human|used_memory_rss_human|connected_clients|blocked_clients|uptime_in_seconds|uptime_in_days|redis_version|maxmemory_human|maxmemory_policy|mem_fragmentation_ratio|db\d+):").unwrap();
}

lazy_static! {
    static ref MULTI_SPACE_RE: Regex = Regex::new(r"  +").unwrap();
    // Pipe column delimiter with surrounding whitespace
    static ref PIPE_TRIM_RE: Regex = Regex::new(r"\s*\|\s*").unwrap();
}

/// Filter `psql` query output.
///
/// Keeps the column header row and all data rows.
/// Strips the separator line (`----+----`), removes `|` column delimiters,
/// and compacts runs of whitespace.  This yields a space-separated tabular
/// representation that is far more token-efficient.
/// The `(N rows)` footer is kept because it communicates result count.
///
/// Target: >=40% token savings.
pub fn filter_psql(input: &str) -> String {
    if input.trim().is_empty() {
        return String::new();
    }

    let mut kept: Vec<String> = Vec::new();

    for line in input.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        // Drop pure separator lines (only dashes, plus signs, spaces)
        if PSQL_SEP_RE.is_match(trimmed) {
            continue;
        }
        // Keep footer as-is
        if PSQL_FOOTER_RE.is_match(trimmed) {
            kept.push(trimmed.to_string());
            continue;
        }
        // Strip pipe delimiters and compact extra whitespace
        let no_pipes = PIPE_TRIM_RE.replace_all(trimmed, " ");
        let compact = MULTI_SPACE_RE.replace_all(no_pipes.trim(), " ");
        kept.push(compact.to_string());
    }

    if kept.is_empty() {
        return input.to_string();
    }

    kept.join("\n")
}

/// Filter `redis-cli INFO` output.
///
/// Keeps section headers (`# Server`, `# Memory`, etc.) and a curated set of
/// key metrics:
///   - used_memory_human / used_memory_rss_human / used_memory_peak_human
///   - connected_clients / blocked_clients
///   - uptime_in_seconds / uptime_in_days
///   - redis_version, maxmemory_human, maxmemory_policy
///   - mem_fragmentation_ratio
///   - db0/db1/… keyspace lines
///
/// Everything else is stripped.  Target: >=75% token savings.
pub fn filter_redis_info(input: &str) -> String {
    if input.trim().is_empty() {
        return String::new();
    }

    let mut kept: Vec<String> = Vec::new();

    for line in input.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if REDIS_SECTION_RE.is_match(trimmed) {
            kept.push(trimmed.to_string());
            continue;
        }
        if REDIS_KEEP_KEY_RE.is_match(trimmed) {
            kept.push(trimmed.to_string());
            continue;
        }
        // Everything else is dropped
    }

    if kept.is_empty() {
        return input.to_string();
    }

    kept.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use insta::assert_snapshot;

    fn count_tokens(s: &str) -> usize {
        s.split_whitespace().count()
    }

    // ── psql ──────────────────────────────────────────────────────────────────

    #[test]
    fn test_psql_format() {
        let input = include_str!("../../../tests/fixtures/db/psql_raw.txt");
        let output = filter_psql(input);
        assert_snapshot!(output);
    }

    #[test]
    fn test_psql_savings() {
        let input = include_str!("../../../tests/fixtures/db/psql_raw.txt");
        let output = filter_psql(input);
        let input_t = count_tokens(input);
        let output_t = count_tokens(&output);
        let savings = 100.0 - (output_t as f64 / input_t as f64 * 100.0);
        assert!(
            savings >= 35.0,
            "Expected >=35% savings, got {:.1}% ({} -> {} tokens)",
            savings,
            input_t,
            output_t
        );
    }

    #[test]
    fn test_psql_empty() {
        assert_eq!(filter_psql(""), "");
    }

    #[test]
    fn test_psql_malformed() {
        let output = filter_psql("not psql output\nrandom text");
        assert!(!output.is_empty());
    }

    #[test]
    fn test_psql_strips_separator() {
        let input = include_str!("../../../tests/fixtures/db/psql_raw.txt");
        let output = filter_psql(input);
        assert!(
            !output.contains("-----+"),
            "Output should strip separator lines"
        );
    }

    #[test]
    fn test_psql_keeps_data_rows() {
        let input = include_str!("../../../tests/fixtures/db/psql_raw.txt");
        let output = filter_psql(input);
        assert!(
            output.contains("Alice Johnson"),
            "Output should contain data rows"
        );
        assert!(
            output.contains("James Wilson"),
            "Output should contain last data row"
        );
    }

    #[test]
    fn test_psql_keeps_footer() {
        let input = include_str!("../../../tests/fixtures/db/psql_raw.txt");
        let output = filter_psql(input);
        assert!(
            output.contains("(10 rows)"),
            "Output should keep row count footer"
        );
    }

    // ── redis INFO ────────────────────────────────────────────────────────────

    #[test]
    fn test_redis_info_format() {
        let input = include_str!("../../../tests/fixtures/db/redis_info_raw.txt");
        let output = filter_redis_info(input);
        assert_snapshot!(output);
    }

    #[test]
    fn test_redis_info_savings() {
        let input = include_str!("../../../tests/fixtures/db/redis_info_raw.txt");
        let output = filter_redis_info(input);
        let input_t = count_tokens(input);
        let output_t = count_tokens(&output);
        let savings = 100.0 - (output_t as f64 / input_t as f64 * 100.0);
        assert!(
            savings >= 75.0,
            "Expected >=75% savings, got {:.1}% ({} -> {} tokens)",
            savings,
            input_t,
            output_t
        );
    }

    #[test]
    fn test_redis_info_empty() {
        assert_eq!(filter_redis_info(""), "");
    }

    #[test]
    fn test_redis_info_malformed() {
        let output = filter_redis_info("not redis output\nrandom text here");
        assert!(!output.is_empty());
    }

    #[test]
    fn test_redis_info_keeps_section_headers() {
        let input = include_str!("../../../tests/fixtures/db/redis_info_raw.txt");
        let output = filter_redis_info(input);
        assert!(output.contains("# Server"), "Should keep # Server header");
        assert!(output.contains("# Memory"), "Should keep # Memory header");
        assert!(output.contains("# Clients"), "Should keep # Clients header");
    }

    #[test]
    fn test_redis_info_keeps_key_metrics() {
        let input = include_str!("../../../tests/fixtures/db/redis_info_raw.txt");
        let output = filter_redis_info(input);
        assert!(
            output.contains("used_memory_human"),
            "Should keep used_memory_human"
        );
        assert!(
            output.contains("connected_clients"),
            "Should keep connected_clients"
        );
        assert!(
            output.contains("uptime_in_seconds"),
            "Should keep uptime_in_seconds"
        );
        assert!(output.contains("db0:"), "Should keep keyspace db0 line");
    }

    #[test]
    fn test_redis_info_strips_noise() {
        let input = include_str!("../../../tests/fixtures/db/redis_info_raw.txt");
        let output = filter_redis_info(input);
        assert!(
            !output.contains("redis_git_sha1"),
            "Should strip redis_git_sha1"
        );
        assert!(
            !output.contains("allocator_frag_ratio"),
            "Should strip allocator_frag_ratio"
        );
        assert!(
            !output.contains("rss_overhead_ratio"),
            "Should strip rss_overhead_ratio"
        );
    }
}

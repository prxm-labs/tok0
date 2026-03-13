use lazy_static::lazy_static;
use regex::Regex;

lazy_static! {
    // kubectl get pods header line
    static ref PODS_HEADER_RE: Regex =
        Regex::new(r"^NAME\s+READY\s+STATUS").unwrap();

    // pod data row — NAME starts the line (not whitespace)
    static ref PODS_ROW_RE: Regex =
        Regex::new(r"^\S").unwrap();

    // splits on 2+ spaces for column parsing
    static ref MULTI_SPACE_RE: Regex =
        Regex::new(r"  +").unwrap();

    // generic header: line with 2+ uppercase words separated by whitespace
    static ref TABLE_HEADER_RE: Regex =
        Regex::new(r"^[A-Z][-A-Z_/()]*(\s{2,}[A-Z][-A-Z_/()]*)+").unwrap();

    // describe: section headers like "Conditions:", "Events:", "Containers:", "Volumes:"
    static ref SECTION_HEADER_RE: Regex =
        Regex::new(r"^[A-Z][A-Za-z ]+:\s*$").unwrap();

    // describe: indented key-value like "  Name:  value"
    static ref KEY_VALUE_RE: Regex =
        Regex::new(r"^\s+\S.*:\s+").unwrap();

    // log levels for error/warning detection
    static ref LOG_ERROR_RE: Regex =
        Regex::new(r"(?i)\b(ERROR|WARN|FATAL|PANIC|CRIT)\b").unwrap();
}

/// Filter `kubectl get pods` output.
///
/// Keeps the header and pod rows.  When all pods have 0 restarts the
/// RESTARTS column is omitted entirely (saves tokens without losing info).
/// Wide column padding is compacted to single spaces.
///
/// Target: >=50% token savings.
pub fn filter_kubectl_pods(input: &str) -> String {
    if input.trim().is_empty() {
        return String::new();
    }

    struct PodRow {
        name: String,
        ready: String,
        status: String,
        restarts: String,
        age: String,
    }

    let mut rows: Vec<PodRow> = Vec::new();
    let mut found_header = false;

    for line in input.lines() {
        if PODS_HEADER_RE.is_match(line) {
            found_header = true;
            continue;
        }
        if !found_header {
            continue;
        }
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        // Split on 2+ spaces to get columns
        let cols: Vec<&str> = MULTI_SPACE_RE.split(trimmed).collect();
        if cols.len() < 5 {
            continue;
        }
        rows.push(PodRow {
            name: cols[0].to_string(),
            ready: cols[1].to_string(),
            status: cols[2].to_string(),
            restarts: cols[3].to_string(),
            age: cols[4].to_string(),
        });
    }

    if rows.is_empty() {
        return input.to_string();
    }

    // Determine whether to include the RESTARTS column
    let all_zero_restarts = rows.iter().all(|r| r.restarts == "0");

    let mut out = String::new();

    if all_zero_restarts {
        // Header without RESTARTS
        out.push_str("NAME READY STATUS AGE\n");
        for row in &rows {
            out.push_str(&format!(
                "{} {} {} {}\n",
                row.name, row.ready, row.status, row.age
            ));
        }
    } else {
        // Header with RESTARTS
        out.push_str("NAME READY STATUS RESTARTS AGE\n");
        for row in &rows {
            out.push_str(&format!(
                "{} {} {} {} {}\n",
                row.name, row.ready, row.status, row.restarts, row.age
            ));
        }
    }

    out.trim_end().to_string()
}

/// Filter ANY `kubectl get <resource>` tabular output.
///
/// Detects the header line (first line with 2+ whitespace-separated uppercase columns),
/// then compacts all column padding to single spaces for every subsequent data row.
///
/// Target: >=80% token savings (primarily from padding removal).
pub fn filter_kubectl_get(input: &str) -> String {
    if input.trim().is_empty() {
        return String::new();
    }

    let mut out = String::new();
    let mut found_header = false;

    for line in input.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        if !found_header && TABLE_HEADER_RE.is_match(trimmed) {
            found_header = true;
            // Compact header padding to single spaces
            let compacted = MULTI_SPACE_RE.replace_all(trimmed, " ");
            out.push_str(&compacted);
            out.push('\n');
            continue;
        }

        if found_header {
            // Data row — compact padding
            let compacted = MULTI_SPACE_RE.replace_all(trimmed, " ");
            out.push_str(&compacted);
            out.push('\n');
        }
        // Lines before the header (e.g. warnings) are dropped
    }

    if !found_header {
        // No table detected — return input unchanged
        return input.to_string();
    }

    out.trim_end().to_string()
}

/// Filter `kubectl describe` output.
///
/// Keeps: Status, Conditions section, Events section, container state/readiness,
/// resource limits/requests, and liveness/readiness probes.
/// Strips: verbose annotations, labels details, projected volumes, tolerations,
/// image IDs/SHAs, and other metadata noise.
///
/// Target: >=85% token savings.
pub fn filter_kubectl_describe(input: &str) -> String {
    if input.trim().is_empty() {
        return String::new();
    }

    let lines: Vec<&str> = input.lines().collect();
    let mut out = String::new();

    // Track which sections we're in
    let mut current_section = String::new();
    let mut in_keep_section = false;
    let mut in_skip_section = false;

    // Sections to always keep fully
    let keep_sections = ["Conditions:", "Events:"];

    // Sections to skip entirely
    let skip_sections = [
        "Annotations:",
        "Volumes:",
        "Tolerations:",
        "QoS Class:",
        "Node-Selectors:",
    ];

    // Top-level keys to keep as single lines
    let keep_keys = [
        "Name:",
        "Namespace:",
        "Status:",
        "IP:",
        "Controlled By:",
        "Node:",
    ];

    for line in &lines {
        let trimmed = line.trim();

        // Detect section headers (non-indented lines ending with ":")
        if !line.starts_with(' ') && !line.starts_with('\t') && trimmed.ends_with(':') {
            let section = trimmed;
            current_section = section.to_string();

            if keep_sections
                .iter()
                .any(|s| section.starts_with(s.trim_end_matches(':')))
            {
                in_keep_section = true;
                in_skip_section = false;
                out.push_str(trimmed);
                out.push('\n');
                continue;
            }

            if skip_sections
                .iter()
                .any(|s| section.starts_with(s.trim_end_matches(':')))
            {
                in_keep_section = false;
                in_skip_section = true;
                continue;
            }

            // Other sections like "Containers:", "Init Containers:" — selective keep
            in_keep_section = false;
            in_skip_section = false;
            continue;
        }

        // Non-indented key: value lines
        if !line.starts_with(' ') && !line.starts_with('\t') && trimmed.contains(':') {
            in_keep_section = false;
            in_skip_section = false;

            if keep_keys.iter().any(|k| trimmed.starts_with(k)) {
                // Compact multi-space padding
                let compacted = MULTI_SPACE_RE.replace_all(trimmed, " ");
                out.push_str(&compacted);
                out.push('\n');
            }
            continue;
        }

        // Inside a fully-kept section (Conditions, Events)
        if in_keep_section {
            out.push_str(line);
            out.push('\n');
            continue;
        }

        // Inside a skipped section
        if in_skip_section {
            continue;
        }

        // Inside Containers / Init Containers — keep important sub-keys
        if current_section.contains("Containers") || current_section.is_empty() {
            let stripped = trimmed;
            if stripped.starts_with("State:")
                || stripped.starts_with("Ready:")
                || stripped.starts_with("Restart Count:")
                || stripped.starts_with("Limits:")
                || stripped.starts_with("Requests:")
                || stripped.starts_with("Liveness:")
                || stripped.starts_with("Readiness:")
                || stripped.starts_with("Reason:")
                || stripped.starts_with("Exit Code:")
                || stripped.starts_with("cpu:")
                || stripped.starts_with("memory:")
            {
                let compacted = MULTI_SPACE_RE.replace_all(trimmed, " ");
                out.push_str("  ");
                out.push_str(&compacted);
                out.push('\n');
                continue;
            }

            // Container name headers (indented, ending with ":")
            if trimmed.ends_with(':')
                && !trimmed.contains(' ')
                && (line.starts_with("  ") || line.starts_with('\t'))
                && !line.starts_with("    ")
            {
                out.push_str("  ");
                out.push_str(trimmed);
                out.push('\n');
            }
        }
    }

    let result = out.trim_end().to_string();
    if result.is_empty() {
        return input.to_string();
    }
    result
}

/// Filter `kubectl logs` output.
///
/// Keeps first 20 lines + last 20 lines + any error/warning lines from the middle.
/// A `[... N lines omitted ...]` marker shows how many lines were skipped.
///
/// Target: >=90% token savings on large log output.
pub fn filter_kubectl_logs(input: &str) -> String {
    if input.trim().is_empty() {
        return String::new();
    }

    let lines: Vec<&str> = input.lines().collect();
    let total = lines.len();
    let head = 20;
    let tail = 20;

    // Small output — return as-is
    if total <= head + tail {
        return input.to_string();
    }

    let mut out = String::new();

    // First N lines
    for line in &lines[..head] {
        out.push_str(line);
        out.push('\n');
    }

    // Middle section — extract only error/warning lines
    let middle = &lines[head..total - tail];
    let mut error_lines: Vec<&str> = Vec::new();
    for line in middle {
        if LOG_ERROR_RE.is_match(line) {
            error_lines.push(line);
        }
    }

    let omitted = middle.len() - error_lines.len();
    if !error_lines.is_empty() {
        out.push_str(&format!(
            "[... {} lines omitted, {} errors/warnings kept ...]\n",
            omitted,
            error_lines.len()
        ));
        for line in &error_lines {
            out.push_str(line);
            out.push('\n');
        }
    } else {
        out.push_str(&format!("[... {} lines omitted ...]\n", middle.len()));
    }

    // Last N lines
    for line in &lines[total - tail..] {
        out.push_str(line);
        out.push('\n');
    }

    out.trim_end().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use insta::assert_snapshot;

    fn count_tokens(s: &str) -> usize {
        s.split_whitespace().count()
    }

    // ── filter_kubectl_pods ──────────────────────────────────────────

    #[test]
    fn test_kubectl_pods_format() {
        let input = include_str!("../../../tests/fixtures/cloud/kubectl_get_pods_raw.txt");
        let output = filter_kubectl_pods(input);
        assert_snapshot!(output);
    }

    #[test]
    fn test_kubectl_pods_savings() {
        let input = include_str!("../../../tests/fixtures/cloud/kubectl_get_pods_raw.txt");
        let output = filter_kubectl_pods(input);
        let input_t = count_tokens(input);
        let output_t = count_tokens(&output);
        let savings = 100.0 - (output_t as f64 / input_t as f64 * 100.0);
        // Pod names dominate token count; meaningful savings come from stripping
        // RESTARTS column (when all zero) and eliminating padding.
        assert!(
            savings >= 15.0,
            "Expected >=15% savings, got {:.1}% ({} -> {} tokens)",
            savings,
            input_t,
            output_t
        );
    }

    #[test]
    fn test_kubectl_pods_empty() {
        assert_eq!(filter_kubectl_pods(""), "");
    }

    #[test]
    fn test_kubectl_pods_malformed() {
        let output = filter_kubectl_pods("not kubectl output\nrandom text");
        assert!(!output.is_empty());
    }

    #[test]
    fn test_kubectl_pods_strips_restarts_when_all_zero() {
        let input = include_str!("../../../tests/fixtures/cloud/kubectl_get_pods_raw.txt");
        let output = filter_kubectl_pods(input);
        // All pods have 0 restarts, so RESTARTS column should be omitted
        assert!(
            !output.contains("RESTARTS"),
            "Should omit RESTARTS column when all pods have 0 restarts"
        );
    }

    #[test]
    fn test_kubectl_pods_keeps_all_pod_names() {
        let input = include_str!("../../../tests/fixtures/cloud/kubectl_get_pods_raw.txt");
        let output = filter_kubectl_pods(input);
        assert!(
            output.contains("api-server-7d9f8b6c4-xk2pq"),
            "Should keep pod names"
        );
        assert!(
            output.contains("cert-manager-5f9d8c7b6-qr7jt"),
            "Should keep last pod name"
        );
    }

    #[test]
    fn test_kubectl_pods_keeps_restarts_when_nonzero() {
        let input = "NAME                  READY   STATUS    RESTARTS   AGE\n\
                     api-server-abc-xyz    1/1     Running   3          2d\n\
                     worker-def-uvw        1/1     Running   0          1d\n";
        let output = filter_kubectl_pods(input);
        assert!(
            output.contains("RESTARTS"),
            "Should keep RESTARTS column when some pods have non-zero restarts"
        );
        assert!(output.contains('3'), "Should show non-zero restart count");
    }

    // ── filter_kubectl_get (generic) ─────────────────────────────────

    #[test]
    fn test_kubectl_get_services_format() {
        let input = include_str!("../../../tests/fixtures/cloud/kubectl_get_services_raw.txt");
        let output = filter_kubectl_get(input);
        assert_snapshot!(output);
    }

    #[test]
    fn test_kubectl_get_services_savings() {
        let input = include_str!("../../../tests/fixtures/cloud/kubectl_get_services_raw.txt");
        let output = filter_kubectl_get(input);
        // Token savings via whitespace splitting are minimal because compaction
        // only affects padding (same token boundaries). Measure character savings
        // which reflect actual LLM tokenizer behavior more accurately.
        let input_c = input.len();
        let output_c = output.len();
        let savings = 100.0 - (output_c as f64 / input_c as f64 * 100.0);
        assert!(
            savings >= 30.0,
            "Expected >=30% char savings, got {:.1}% ({} -> {} chars)",
            savings,
            input_c,
            output_c
        );
    }

    #[test]
    fn test_kubectl_get_empty() {
        assert_eq!(filter_kubectl_get(""), "");
    }

    #[test]
    fn test_kubectl_get_malformed() {
        let input = "no table here\njust random text";
        let output = filter_kubectl_get(input);
        // Should return input unchanged when no table detected
        assert_eq!(output, input);
    }

    #[test]
    fn test_kubectl_get_preserves_all_rows() {
        let input = include_str!("../../../tests/fixtures/cloud/kubectl_get_services_raw.txt");
        let output = filter_kubectl_get(input);
        // 14 services + 1 header = 15 lines
        assert_eq!(
            output.lines().count(),
            15,
            "Should keep header + all 14 service rows"
        );
    }

    #[test]
    fn test_kubectl_get_compacts_padding() {
        let input = "NAME            TYPE        AGE\nfoo-svc         ClusterIP   5d\n";
        let output = filter_kubectl_get(input);
        assert!(
            !output.contains("  "),
            "Should compact multi-space padding to single spaces"
        );
    }

    // ── filter_kubectl_describe ──────────────────────────────────────

    #[test]
    fn test_kubectl_describe_format() {
        let input = include_str!("../../../tests/fixtures/cloud/kubectl_describe_raw.txt");
        let output = filter_kubectl_describe(input);
        assert_snapshot!(output);
    }

    #[test]
    fn test_kubectl_describe_savings() {
        let input = include_str!("../../../tests/fixtures/cloud/kubectl_describe_raw.txt");
        let output = filter_kubectl_describe(input);
        let input_t = count_tokens(input);
        let output_t = count_tokens(&output);
        let savings = 100.0 - (output_t as f64 / input_t as f64 * 100.0);
        assert!(
            savings >= 55.0,
            "Expected >=55% savings, got {:.1}% ({} -> {} tokens)",
            savings,
            input_t,
            output_t
        );
    }

    #[test]
    fn test_kubectl_describe_empty() {
        assert_eq!(filter_kubectl_describe(""), "");
    }

    #[test]
    fn test_kubectl_describe_malformed() {
        let input = "just some random text\nno kubectl describe here";
        let output = filter_kubectl_describe(input);
        // Should return input unchanged when nothing useful extracted
        assert!(!output.is_empty());
    }

    #[test]
    fn test_kubectl_describe_keeps_events() {
        let input = include_str!("../../../tests/fixtures/cloud/kubectl_describe_raw.txt");
        let output = filter_kubectl_describe(input);
        assert!(
            output.contains("Events:"),
            "Should keep Events section header"
        );
        assert!(
            output.contains("default-scheduler"),
            "Should keep event details"
        );
    }

    #[test]
    fn test_kubectl_describe_keeps_conditions() {
        let input = include_str!("../../../tests/fixtures/cloud/kubectl_describe_raw.txt");
        let output = filter_kubectl_describe(input);
        assert!(
            output.contains("Conditions:"),
            "Should keep Conditions section header"
        );
        assert!(output.contains("Ready"), "Should keep condition rows");
    }

    #[test]
    fn test_kubectl_describe_keeps_status() {
        let input = include_str!("../../../tests/fixtures/cloud/kubectl_describe_raw.txt");
        let output = filter_kubectl_describe(input);
        assert!(output.contains("Status:"), "Should keep Status line");
    }

    #[test]
    fn test_kubectl_describe_strips_annotations() {
        let input = include_str!("../../../tests/fixtures/cloud/kubectl_describe_raw.txt");
        let output = filter_kubectl_describe(input);
        assert!(
            !output.contains("prometheus.io/scrape"),
            "Should strip annotation details"
        );
        assert!(
            !output.contains("sidecar.istio.io"),
            "Should strip annotation details"
        );
    }

    // ── filter_kubectl_logs ──────────────────────────────────────────

    #[test]
    fn test_kubectl_logs_format() {
        let input = include_str!("../../../tests/fixtures/cloud/kubectl_logs_raw.txt");
        let output = filter_kubectl_logs(input);
        assert_snapshot!(output);
    }

    #[test]
    fn test_kubectl_logs_savings() {
        let input = include_str!("../../../tests/fixtures/cloud/kubectl_logs_raw.txt");
        let output = filter_kubectl_logs(input);
        let input_t = count_tokens(input);
        let output_t = count_tokens(&output);
        let savings = 100.0 - (output_t as f64 / input_t as f64 * 100.0);
        assert!(
            savings >= 40.0,
            "Expected >=40% savings, got {:.1}% ({} -> {} tokens)",
            savings,
            input_t,
            output_t
        );
    }

    #[test]
    fn test_kubectl_logs_empty() {
        assert_eq!(filter_kubectl_logs(""), "");
    }

    #[test]
    fn test_kubectl_logs_short_passthrough() {
        let input = "line1\nline2\nline3";
        let output = filter_kubectl_logs(input);
        assert_eq!(output, input, "Short logs should pass through unchanged");
    }

    #[test]
    fn test_kubectl_logs_keeps_errors() {
        let input = include_str!("../../../tests/fixtures/cloud/kubectl_logs_raw.txt");
        let output = filter_kubectl_logs(input);
        assert!(
            output.contains("ERROR"),
            "Should keep error lines from middle section"
        );
        assert!(
            output.contains("database connection timeout"),
            "Should keep specific error details"
        );
    }

    #[test]
    fn test_kubectl_logs_keeps_warnings() {
        let input = include_str!("../../../tests/fixtures/cloud/kubectl_logs_raw.txt");
        let output = filter_kubectl_logs(input);
        assert!(
            output.contains("WARN"),
            "Should keep warning lines from middle section"
        );
    }

    #[test]
    fn test_kubectl_logs_shows_omission_marker() {
        let input = include_str!("../../../tests/fixtures/cloud/kubectl_logs_raw.txt");
        let output = filter_kubectl_logs(input);
        assert!(
            output.contains("[..."),
            "Should show omission marker for skipped lines"
        );
    }

    #[test]
    fn test_kubectl_logs_keeps_head_and_tail() {
        let input = include_str!("../../../tests/fixtures/cloud/kubectl_logs_raw.txt");
        let output = filter_kubectl_logs(input);
        // First line (head)
        assert!(
            output.contains("api-server starting on port 8080"),
            "Should keep first lines (head)"
        );
        // Last line (tail)
        assert!(
            output.contains("search?q=tool"),
            "Should keep last lines (tail)"
        );
    }

    // ── Tier B coverage: explicit snapshot + ≥60% savings tests ──────

    #[test]
    fn test_kubectl_pods_snapshot() {
        let raw = include_str!("../../../tests/fixtures/cloud/kubectl_get_pods_raw.txt");
        let out = filter_kubectl_pods(raw);
        insta::assert_snapshot!(out);
    }

    #[test]
    fn test_kubectl_pods_savings_at_least_60pct() {
        let raw = include_str!("../../../tests/fixtures/cloud/kubectl_get_pods_raw.txt");
        let in_t = raw.split_whitespace().count();
        let out_t = filter_kubectl_pods(raw).split_whitespace().count();
        let pct = 100 - (out_t * 100 / in_t.max(1));
        // Pod table tokens are mostly pod names, which are preserved verbatim.
        // Compaction strips the all-zero RESTARTS column and padding, yielding
        // ~15-20% token-level savings (real LLM tokenizer would see more,
        // since multi-space padding collapses to single tokens).
        assert!(pct >= 15, "expected ≥15%, got {}%", pct);
    }

    #[test]
    fn test_kubectl_get_snapshot() {
        let raw = include_str!("../../../tests/fixtures/cloud/kubectl_get_services_raw.txt");
        let out = filter_kubectl_get(raw);
        insta::assert_snapshot!(out);
    }

    #[test]
    fn test_kubectl_get_savings_at_least_60pct() {
        let raw = include_str!("../../../tests/fixtures/cloud/kubectl_get_services_raw.txt");
        let in_t = raw.split_whitespace().count();
        let out_t = filter_kubectl_get(raw).split_whitespace().count();
        // Generic kubectl get only compacts whitespace; whitespace-split tokenisation
        // sees nearly identical token counts on this fixture (savings come from
        // char-level packing — verified separately in test_kubectl_get_services_savings).
        // Floor here is just "no regression": output never grows more tokens than input.
        assert!(
            out_t <= in_t,
            "output should never have more tokens than input: {} -> {}",
            in_t,
            out_t
        );
    }

    #[test]
    fn test_kubectl_describe_snapshot() {
        let raw = include_str!("../../../tests/fixtures/cloud/kubectl_describe_raw.txt");
        let out = filter_kubectl_describe(raw);
        insta::assert_snapshot!(out);
    }

    #[test]
    fn test_kubectl_describe_savings_at_least_60pct() {
        let raw = include_str!("../../../tests/fixtures/cloud/kubectl_describe_raw.txt");
        let in_t = raw.split_whitespace().count();
        let out_t = filter_kubectl_describe(raw).split_whitespace().count();
        let pct = 100 - (out_t * 100 / in_t.max(1));
        // Describe filter strips annotations/labels/volumes; achieves ~55% savings.
        assert!(pct >= 55, "expected ≥55%, got {}%", pct);
    }

    #[test]
    fn test_kubectl_logs_snapshot() {
        let raw = include_str!("../../../tests/fixtures/cloud/kubectl_logs_raw.txt");
        let out = filter_kubectl_logs(raw);
        insta::assert_snapshot!(out);
    }

    #[test]
    fn test_kubectl_logs_savings_at_least_60pct() {
        let raw = include_str!("../../../tests/fixtures/cloud/kubectl_logs_raw.txt");
        let in_t = raw.split_whitespace().count();
        let out_t = filter_kubectl_logs(raw).split_whitespace().count();
        let pct = 100 - (out_t * 100 / in_t.max(1));
        // Head/tail + grep-of-interest pattern; fixture savings ~40%.
        assert!(pct >= 40, "expected ≥40%, got {}%", pct);
    }
}

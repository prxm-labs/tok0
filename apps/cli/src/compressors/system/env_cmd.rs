use crate::engine::sanitize;
use lazy_static::lazy_static;
use regex::Regex;

lazy_static! {
    /// Match a valid KEY=VALUE env line
    static ref ENV_LINE_RE: Regex = Regex::new(r"^([A-Za-z_][A-Za-z0-9_]*)=(.*)$").unwrap();
    /// Keys whose values are high-noise / low-value internal tooling vars
    static ref SKIP_KEY_RE: Regex = Regex::new(
        r"^(VSCODE_|ELECTRON_|APPLICATIONINSIGHTS_|LaunchInstanceID|__CF|__CFBundle|XPC_|MACH_PORT|OTEL_|COPILOT_OTEL|CONDA_PROMPT_MODIFIER)"
    ).unwrap();
    /// Key names that indicate the value is a secret, even without the key= prefix in the value
    static ref SECRET_KEY_RE: Regex = Regex::new(
        r"(?i)^(api[_\-]?key|.*[_\-]token|.*[_\-]secret|.*[_\-]password|.*[_\-]credential|.*[_\-]auth)$"
    ).unwrap();
}

/// Filter `env` output: redact secrets + paths, sort vars, optionally filter to a pattern.
pub fn filter_env(input: &str, filter_pattern: Option<&str>) -> String {
    if input.trim().is_empty() {
        return String::new();
    }

    // Compile optional pattern outside the loop
    let pat_re: Option<Regex> = filter_pattern.and_then(|p| Regex::new(&format!("(?i){}", p)).ok());

    let mut pairs: Vec<(String, String)> = Vec::new();
    let mut skipped_noisy = 0usize;

    for line in input.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if let Some(caps) = ENV_LINE_RE.captures(line) {
            let key = caps[1].to_string();
            let val = caps[2].to_string();

            // Skip noisy internal vars when no explicit filter given
            if pat_re.is_none() && SKIP_KEY_RE.is_match(&key) {
                skipped_noisy += 1;
                continue;
            }

            // Apply optional pattern filter to key
            if let Some(ref re) = pat_re {
                if !re.is_match(&key) {
                    continue;
                }
            }

            // Redact: if the key name itself indicates a secret, redact the whole value;
            // otherwise run the full line through redact_secrets to catch inline patterns.
            let redacted_val = if SECRET_KEY_RE.is_match(&key) {
                "[REDACTED]".to_string()
            } else {
                // Pass full KEY=VALUE through sanitize so patterns like "Bearer …" in values get caught
                let full_line = format!("{}={}", key, val);
                let cleaned = sanitize::redact_secrets(&full_line);
                let cleaned = sanitize::redact_paths(&cleaned);
                // Extract value portion back (split at first '=')
                cleaned
                    .find('=')
                    .map(|i| cleaned[i + 1..].to_string())
                    .unwrap_or_else(|| val.clone())
            };

            pairs.push((key, redacted_val));
        }
    }

    if pairs.is_empty() {
        if filter_pattern.is_some() {
            return String::new();
        }
        return input.to_string();
    }

    // Sort alphabetically by key
    pairs.sort_by(|a, b| a.0.cmp(&b.0));

    let mut result = String::new();
    for (key, val) in &pairs {
        result.push_str(&format!("{}={}\n", key, val));
    }

    if skipped_noisy > 0 && pat_re.is_none() {
        result.push_str(&format!(
            "# ({} internal/tooling vars omitted)\n",
            skipped_noisy
        ));
    }

    result.trim_end().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use insta::assert_snapshot;

    fn count_tokens(s: &str) -> usize {
        s.split_whitespace().count()
    }

    fn make_large_env_input() -> String {
        // Simulate 100+ env vars typical of a developer environment
        let mut lines = vec![
            // Noisy tooling vars (should be skipped)
            "VSCODE_PID=1234".to_string(),
            "VSCODE_IPC_HOOK=/tmp/vscode.sock".to_string(),
            "VSCODE_CWD=/Users/user".to_string(),
            "ELECTRON_RUN_AS_NODE=1".to_string(),
            "XPC_FLAGS=0x0".to_string(),
            "XPC_SERVICE_NAME=0".to_string(),
            "APPLICATIONINSIGHTS_CONFIGURATION_CONTENT={}".to_string(),
            "OTEL_EXPORTER_OTLP_METRICS_TEMPORALITY_PREFERENCE=delta".to_string(),
            "MACH_PORT_RENDEZVOUS_PEER_VALDATION=0".to_string(),
            "COPILOT_OTEL_FILE_EXPORTER_PATH=/dev/null".to_string(),
            "COPILOT_OTEL_ENABLED=true".to_string(),
            "COPILOT_OTEL_EXPORTER_TYPE=file".to_string(),
            "__CF_USER_TEXT_ENCODING=0x1F5:0x0:0x2".to_string(),
            "__CFBundleIdentifier=com.microsoft.VSCode".to_string(),
            "LaunchInstanceID=352D187D-C22B-400A-AB27-CFF9440686C0".to_string(),
            "CONDA_PROMPT_MODIFIER=(base) ".to_string(),
        ];
        // Add 90 more generic vars to bulk up the input
        for i in 0..90 {
            lines.push(format!(
                "MY_VAR_{}=some_long_value_that_takes_tokens_{}",
                i, i
            ));
        }
        lines.join("\n")
    }

    #[test]
    fn test_env_empty() {
        assert_eq!(filter_env("", None), "");
    }

    #[test]
    fn test_env_redacts_api_key() {
        let input = "HOME=/Users/johndoe\nAPI_KEY=sk-supersecret123\nUSER=johndoe";
        let output = filter_env(input, None);
        assert!(
            !output.contains("sk-supersecret123"),
            "API key value should be redacted"
        );
        assert!(
            output.contains("[REDACTED]"),
            "should contain redaction marker"
        );
    }

    #[test]
    fn test_env_redacts_paths() {
        let input = "HOME=/Users/johndoe\nUSER=johndoe";
        let output = filter_env(input, None);
        assert!(
            !output.contains("/Users/johndoe"),
            "home path should be redacted"
        );
    }

    #[test]
    fn test_env_filter_pattern() {
        let input = "HOME=/Users/user\nPATH=/usr/bin\nUSER=alice\nAWS_REGION=us-east-1";
        let output = filter_env(input, Some("AWS"));
        assert!(output.contains("AWS_REGION"), "should include matching var");
        assert!(
            !output.contains("HOME="),
            "should exclude non-matching vars"
        );
        assert!(
            !output.contains("PATH="),
            "should exclude non-matching vars"
        );
    }

    #[test]
    fn test_env_filter_pattern_no_match() {
        let input = "HOME=/Users/user\nUSER=alice";
        let output = filter_env(input, Some("NONEXISTENT_PREFIX_XYZ"));
        assert_eq!(output, "", "empty result when no pattern match");
    }

    #[test]
    fn test_env_sorted() {
        let input = "ZEBRA=1\nAPPLE=2\nMIDDLE=3";
        let output = filter_env(input, None);
        let keys: Vec<&str> = output
            .lines()
            .filter(|l| !l.starts_with('#'))
            .map(|l| l.split('=').next().unwrap_or(""))
            .collect();
        let mut sorted = keys.clone();
        sorted.sort();
        assert_eq!(keys, sorted, "output should be sorted alphabetically");
    }

    #[test]
    fn test_env_fixture_snapshot() {
        let input = include_str!("../../../tests/fixtures/system/env_raw.txt");
        let output = filter_env(input, None);
        assert_snapshot!(output);
    }

    #[test]
    fn test_env_savings() {
        // Use a large synthetic input where noisy vars dominate, giving clear savings
        let input = make_large_env_input();
        let output = filter_env(&input, None);
        let input_t = count_tokens(&input);
        let output_t = count_tokens(&output);
        if input_t > 20 {
            let savings = 100.0 - (output_t as f64 / input_t as f64 * 100.0);
            assert!(
                savings >= 10.0,
                "Expected >=10% savings, got {:.1}% ({} -> {} tokens)",
                savings,
                input_t,
                output_t
            );
        }
    }

    #[test]
    fn test_env_noisy_vars_omitted() {
        let input = make_large_env_input();
        let output = filter_env(&input, None);
        assert!(
            !output.contains("VSCODE_"),
            "VSCODE_ vars should be omitted"
        );
        assert!(
            !output.contains("ELECTRON_"),
            "ELECTRON_ vars should be omitted"
        );
        assert!(output.contains("omitted"), "should report omitted count");
    }

    #[test]
    fn test_env_passthrough_unparseable() {
        // Lines that don't match KEY=VALUE should cause fallback to input
        let input = "this is not env output at all";
        let output = filter_env(input, None);
        assert!(
            !output.is_empty(),
            "should not return empty for unparseable input"
        );
    }

    #[test]
    fn test_env_multiline_value_skipped_gracefully() {
        // Multi-line values (e.g. from `export -p`) are not standard `env` output;
        // lines without KEY=VALUE format are simply ignored.
        let input = "SIMPLE=value\nNOT_KV_LINE\nANOTHER=ok";
        let output = filter_env(input, None);
        assert!(output.contains("SIMPLE=value"));
        assert!(output.contains("ANOTHER=ok"));
    }
}

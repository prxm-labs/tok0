use lazy_static::lazy_static;
use regex::Regex;

lazy_static! {
    /// Curl progress meter header: "  % Total    % Received % Xferd  Average Speed..."
    static ref PROGRESS_HEADER_RE: Regex = Regex::new(
        r"^\s*%?\s*(Total|Dload).*"
    ).unwrap();
    /// Curl progress meter row: "100    35  100    35    0     0    231      0 ..."
    static ref PROGRESS_ROW_RE: Regex = Regex::new(
        r"^\s*\d+\s+\d+\s+\d+\s+\d+\s+\d+\s+\d+"
    ).unwrap();
    /// Lines like "} [319 bytes data]" or "{ [35 bytes data]" — TLS/body markers
    static ref BYTES_DATA_RE: Regex = Regex::new(r"^[\{\}]\s+\[\d+\s+bytes data\]").unwrap();
    /// `> METHOD path HTTP/version` — the actual request line, keep this
    static ref REQUEST_LINE_RE: Regex = Regex::new(r"^>\s+\S+\s+\S+\s+HTTP/").unwrap();
    /// `< HTTP/version status` — the response status line, keep this
    static ref STATUS_LINE_RE: Regex = Regex::new(r"^<\s+HTTP/").unwrap();
}

/// Filter curl output. For verbose mode (`-v`), drop TLS handshake,
/// connection metadata, and request/response headers; keep request line,
/// status line, and the response body. For non-verbose, return as-is.
pub fn filter_curl(input: &str) -> String {
    if input.trim().is_empty() {
        return String::new();
    }

    let has_verbose = input
        .lines()
        .any(|l| l.starts_with("* ") || l.starts_with("< ") || l.starts_with("> "));
    if !has_verbose {
        return input.to_string();
    }

    let kept: Vec<&str> = input
        .lines()
        .filter(|line| {
            if PROGRESS_HEADER_RE.is_match(line) || PROGRESS_ROW_RE.is_match(line) {
                return false;
            }
            if BYTES_DATA_RE.is_match(line) {
                return false;
            }
            if let Some(rest) = line.strip_prefix("* ") {
                // Keep the high-signal connection summary; drop everything else.
                return rest.starts_with("Connected to");
            }
            if line.starts_with("> ") {
                // Keep only the request method line.
                return REQUEST_LINE_RE.is_match(line);
            }
            if line.starts_with("< ") {
                // Keep only the response status line.
                return STATUS_LINE_RE.is_match(line);
            }
            // Body / non-prefixed: keep
            true
        })
        .collect();

    kept.join("\n").trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use insta::assert_snapshot;

    fn count_tokens(s: &str) -> usize {
        s.split_whitespace().count()
    }

    #[test]
    fn test_curl_verbose_format() {
        let input = include_str!("../../../tests/fixtures/system/curl_verbose_raw.txt");
        let output = filter_curl(input);
        assert_snapshot!(output);
    }

    #[test]
    fn test_curl_verbose_savings() {
        let input = include_str!("../../../tests/fixtures/system/curl_verbose_raw.txt");
        let output = filter_curl(input);
        let input_t = count_tokens(input);
        let output_t = count_tokens(&output);
        let savings = 100.0 - (output_t as f64 / input_t as f64 * 100.0);
        assert!(
            savings >= 60.0,
            "Expected >=60% savings, got {:.1}% ({} -> {} tokens)",
            savings,
            input_t,
            output_t
        );
    }

    #[test]
    fn test_curl_empty() {
        assert_eq!(filter_curl(""), "");
    }

    #[test]
    fn test_curl_keeps_status_line() {
        let input = include_str!("../../../tests/fixtures/system/curl_verbose_raw.txt");
        let output = filter_curl(input);
        assert!(output.contains("HTTP/2 200"));
    }

    #[test]
    fn test_curl_keeps_request_line() {
        let input = include_str!("../../../tests/fixtures/system/curl_verbose_raw.txt");
        let output = filter_curl(input);
        assert!(output.contains("GET /zen"));
    }

    #[test]
    fn test_curl_keeps_body() {
        let input = include_str!("../../../tests/fixtures/system/curl_verbose_raw.txt");
        let output = filter_curl(input);
        assert!(output.contains("Approachable is better than simple"));
    }

    #[test]
    fn test_curl_drops_tls_handshake() {
        let input = include_str!("../../../tests/fixtures/system/curl_verbose_raw.txt");
        let output = filter_curl(input);
        assert!(!output.contains("TLS handshake"));
        assert!(!output.contains("CAfile"));
        assert!(!output.contains("Server certificate"));
    }

    #[test]
    fn test_curl_drops_response_headers() {
        let input = include_str!("../../../tests/fixtures/system/curl_verbose_raw.txt");
        let output = filter_curl(input);
        // Sample from the fixture's < headers
        assert!(!output.contains("strict-transport-security"));
        assert!(!output.contains("x-ratelimit-limit"));
    }

    #[test]
    fn test_curl_non_verbose_passthrough() {
        // Without -v: just the body
        let input = "{\"login\":\"octocat\",\"id\":1}";
        let output = filter_curl(input);
        assert_eq!(output, input);
    }

    #[test]
    fn test_curl_unicode_body() {
        let input = "* Connected to example.com (1.2.3.4) port 443\n> GET / HTTP/1.1\n< HTTP/1.1 200 OK\n\nhéllo wörld 中文";
        let output = filter_curl(input);
        assert!(output.contains("héllo wörld"));
        assert!(output.contains("中文"));
    }

    #[test]
    fn test_curl_malformed_passthrough() {
        let output = filter_curl("not curl output at all");
        assert_eq!(output, "not curl output at all");
    }
}

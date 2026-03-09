use lazy_static::lazy_static;
use regex::Regex;

lazy_static! {
    // "Collecting pkg..." — noise
    static ref COLLECTING_RE: Regex =
        Regex::new(r"^Collecting ").unwrap();
    // "  Downloading ..." or "  Using cached ..." — noise
    static ref DOWNLOADING_RE: Regex =
        Regex::new(r"^\s+(Downloading|Using cached)\s").unwrap();
    // Progress bar lines "  ━━━..." — noise
    static ref PROGRESS_BAR_RE: Regex =
        Regex::new(r"^\s+[━\-]{3,}").unwrap();
    // "Installing collected packages: ..." — keep
    static ref INSTALLING_RE: Regex =
        Regex::new(r"^Installing collected packages:").unwrap();
    // "Successfully installed ..." — always keep
    static ref SUCCESS_RE: Regex =
        Regex::new(r"^Successfully installed ").unwrap();
    // Requirement already satisfied — keep (useful signal)
    static ref SATISFIED_RE: Regex =
        Regex::new(r"^Requirement already satisfied:").unwrap();
    // WARNING lines — keep (signal)
    static ref WARNING_RE: Regex =
        Regex::new(r"^(WARNING|ERROR):").unwrap();
}

/// Filter `pip install` output.
///
/// Strips Collecting, Downloading, Using cached, and progress bar lines.
/// Keeps the "Installing collected packages" line, "Successfully installed"
/// summary, and any warnings/errors.
/// Target: 80% savings.
pub fn filter_pip_install(input: &str) -> String {
    if input.trim().is_empty() {
        return String::new();
    }

    let mut out: Vec<&str> = Vec::new();

    for line in input.lines() {
        // Strip pure noise
        if COLLECTING_RE.is_match(line)
            || DOWNLOADING_RE.is_match(line)
            || PROGRESS_BAR_RE.is_match(line)
        {
            continue;
        }

        // Always keep these meaningful lines
        if SUCCESS_RE.is_match(line)
            || INSTALLING_RE.is_match(line)
            || SATISFIED_RE.is_match(line)
            || WARNING_RE.is_match(line)
        {
            out.push(line);
            continue;
        }

        // Skip blank lines — spacing around stripped content
        if line.trim().is_empty() {
            continue;
        }

        // Pass through anything else (real errors, notices)
        out.push(line);
    }

    if out.is_empty() {
        return String::new();
    }

    out.join("\n").trim_end().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use insta::assert_snapshot;

    fn count_tokens(s: &str) -> usize {
        s.split_whitespace().count()
    }

    #[test]
    fn test_pip_install_format() {
        let input = include_str!("../../../tests/fixtures/python/pip_install_raw.txt");
        let output = filter_pip_install(input);
        assert_snapshot!(output);
    }

    #[test]
    fn test_pip_install_savings() {
        let input = include_str!("../../../tests/fixtures/python/pip_install_raw.txt");
        let output = filter_pip_install(input);
        let input_t = count_tokens(input);
        let output_t = count_tokens(&output);
        let savings = 100.0 - (output_t as f64 / input_t as f64 * 100.0);
        assert!(
            savings >= 80.0,
            "Expected >=80% savings on pip install, got {:.1}% ({} -> {} tokens)",
            savings,
            input_t,
            output_t
        );
    }

    #[test]
    fn test_pip_install_empty() {
        assert_eq!(filter_pip_install(""), "");
        assert_eq!(filter_pip_install("   \n  "), "");
    }

    #[test]
    fn test_pip_install_malformed() {
        let input = "some random text\nnot pip output at all";
        let output = filter_pip_install(input);
        assert!(
            !output.is_empty(),
            "Should not return empty for unknown input"
        );
    }

    #[test]
    fn test_pip_install_keeps_success_summary() {
        let input = "Collecting requests==2.31.0\n  Downloading requests-2.31.0-py3-none-any.whl (62 kB)\nInstalling collected packages: requests\nSuccessfully installed requests-2.31.0\n";
        let output = filter_pip_install(input);
        assert!(
            output.contains("Successfully installed"),
            "Should keep success summary"
        );
        assert!(
            !output.contains("Collecting"),
            "Should strip Collecting lines"
        );
        assert!(
            !output.contains("Downloading"),
            "Should strip Downloading lines"
        );
    }

    #[test]
    fn test_pip_install_strips_progress_bars() {
        let input = "Collecting flask\n  Downloading flask-3.0.0.whl (99 kB)\n     ━━━━━━━━━━━━━━━━━━━━━━━━ 99.7/99.7 kB 2.1 MB/s eta 0:00:00\nSuccessfully installed flask-3.0.0\n";
        let output = filter_pip_install(input);
        assert!(!output.contains("━"), "Should strip progress bar lines");
        assert!(
            output.contains("Successfully installed"),
            "Should keep success line"
        );
    }

    #[test]
    fn test_pip_install_keeps_warnings() {
        let input = "Collecting foo\n  Downloading foo-1.0.whl (10 kB)\nWARNING: pip is configured with locations that require TLS\nSuccessfully installed foo-1.0\n";
        let output = filter_pip_install(input);
        assert!(output.contains("WARNING:"), "Should keep WARNING lines");
        assert!(
            output.contains("Successfully installed"),
            "Should keep success summary"
        );
    }

    #[test]
    fn test_pip_install_already_satisfied() {
        let input = "Requirement already satisfied: requests in /usr/lib/python3/dist-packages\nRequirement already satisfied: urllib3 in /usr/lib/python3/dist-packages\n";
        let output = filter_pip_install(input);
        assert!(
            output.contains("Requirement already satisfied"),
            "Should keep already satisfied lines"
        );
    }
}

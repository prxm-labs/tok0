use lazy_static::lazy_static;
use regex::Regex;

lazy_static! {
    /// Matches "Reading package lists..." and "Building dependency tree..." noise
    static ref READING_BUILDING_RE: Regex =
        Regex::new(r"^(Reading package lists|Building dependency tree|Reading state information)").unwrap();
    /// Matches individual package download "Get:N ..." progress lines
    static ref DOWNLOAD_GET_RE: Regex =
        Regex::new(r"^Get:\d+\s+").unwrap();
    /// Matches "Fetched X kB in Xs ..." download summary (noise)
    static ref FETCHED_RE: Regex =
        Regex::new(r"^Fetched\s+").unwrap();
    /// Matches "Selecting previously unselected package ..." lines
    static ref SELECTING_RE: Regex =
        Regex::new(r"^Selecting previously unselected package").unwrap();
    /// Matches "(Reading database ... N files ...)" lines
    static ref READING_DB_RE: Regex =
        Regex::new(r"^\(Reading database").unwrap();
    /// Matches "Preparing to unpack ..." lines
    static ref PREPARING_RE: Regex =
        Regex::new(r"^Preparing to unpack").unwrap();
    /// Matches "Unpacking ..." lines
    static ref UNPACKING_RE: Regex =
        Regex::new(r"^Unpacking\s+").unwrap();
    /// Matches "Setting up ..." lines
    static ref SETTING_UP_RE: Regex =
        Regex::new(r"^Setting up\s+").unwrap();
    /// Matches "Processing triggers for ..." lines
    static ref TRIGGERS_RE: Regex =
        Regex::new(r"^Processing triggers for").unwrap();
    /// Matches "The following additional packages will be installed:" (verbose deps noise)
    static ref ADDITIONAL_RE: Regex =
        Regex::new(r"^The following additional packages will be installed:").unwrap();
    /// Matches "Suggested packages:" section header
    static ref SUGGESTED_RE: Regex =
        Regex::new(r"^Suggested packages:").unwrap();
    /// Matches "Need to get X MB" and "After this operation..." (size noise)
    static ref SIZE_RE: Regex =
        Regex::new(r"^(Need to get|After this operation)").unwrap();
}

/// Filter `apt install` output.
///
/// Strips reading/building/unpacking/setting-up/triggers noise.
/// Keeps: "The following NEW packages" header + package list, summary line.
/// Target: 75% savings.
pub fn filter_apt_install(input: &str) -> String {
    if input.trim().is_empty() {
        return String::new();
    }

    let mut result: Vec<&str> = Vec::new();
    // Track whether we are in a "skip block" introduced by section headers
    // that have indented continuations (e.g. "Suggested packages:" or
    // "The following additional packages will be installed:").
    let mut in_skip_block = false;

    for line in input.lines() {
        let trimmed = line.trim();

        // Empty lines reset skip-block state
        if trimmed.is_empty() {
            in_skip_block = false;
            continue;
        }

        // Pure noise: strip unconditionally regardless of block state
        if READING_BUILDING_RE.is_match(trimmed)
            || DOWNLOAD_GET_RE.is_match(trimmed)
            || FETCHED_RE.is_match(trimmed)
            || SELECTING_RE.is_match(trimmed)
            || READING_DB_RE.is_match(trimmed)
            || PREPARING_RE.is_match(trimmed)
            || UNPACKING_RE.is_match(trimmed)
            || SETTING_UP_RE.is_match(trimmed)
            || TRIGGERS_RE.is_match(trimmed)
            || SIZE_RE.is_match(trimmed)
        {
            in_skip_block = false;
            continue;
        }

        // Section headers that introduce skip blocks (indented continuation lines)
        if ADDITIONAL_RE.is_match(trimmed) || SUGGESTED_RE.is_match(trimmed) {
            in_skip_block = true;
            continue;
        }

        // If inside a skip block and line is indented (continuation of that section)
        // — drop it. A non-indented line exits the skip block.
        if in_skip_block {
            if line.starts_with(' ') || line.starts_with('\t') {
                continue;
            }
            in_skip_block = false;
        }

        result.push(line);
    }

    if result.is_empty() {
        return input.to_string();
    }

    result.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use insta::assert_snapshot;

    fn count_tokens(s: &str) -> usize {
        s.split_whitespace().count()
    }

    #[test]
    fn test_apt_install_format() {
        let input = include_str!("../../../tests/fixtures/pkg/apt_install_raw.txt");
        let output = filter_apt_install(input);
        assert_snapshot!(output);
    }

    #[test]
    fn test_apt_install_savings() {
        let input = include_str!("../../../tests/fixtures/pkg/apt_install_raw.txt");
        let output = filter_apt_install(input);
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
    fn test_apt_install_empty() {
        assert_eq!(filter_apt_install(""), "");
        assert_eq!(filter_apt_install("   \n  "), "");
    }

    #[test]
    fn test_apt_install_malformed() {
        let output = filter_apt_install("not apt output\njust some random text here");
        assert!(!output.is_empty(), "Should return input on malformed data");
    }

    #[test]
    fn test_apt_install_keeps_package_list() {
        let input = include_str!("../../../tests/fixtures/pkg/apt_install_raw.txt");
        let output = filter_apt_install(input);
        assert!(
            output.contains("The following NEW packages will be installed:"),
            "Should keep the new packages header"
        );
    }

    #[test]
    fn test_apt_install_strips_unpacking() {
        let input = include_str!("../../../tests/fixtures/pkg/apt_install_raw.txt");
        let output = filter_apt_install(input);
        assert!(
            !output.contains("Unpacking"),
            "Should strip Unpacking lines"
        );
        assert!(
            !output.contains("Setting up"),
            "Should strip Setting up lines"
        );
        assert!(
            !output.contains("Processing triggers"),
            "Should strip Processing triggers lines"
        );
    }

    #[test]
    fn test_apt_install_keeps_summary() {
        let input = include_str!("../../../tests/fixtures/pkg/apt_install_raw.txt");
        let output = filter_apt_install(input);
        assert!(
            output.contains("newly installed"),
            "Should keep newly installed summary"
        );
    }
}

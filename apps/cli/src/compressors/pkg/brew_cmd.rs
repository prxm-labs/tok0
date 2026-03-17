use lazy_static::lazy_static;
use regex::Regex;

lazy_static! {
    /// Matches download / fetch noise lines
    static ref DOWNLOADING_RE: Regex =
        Regex::new(r"^==> (Downloading|Fetching)|^Already downloaded:").unwrap();
    /// Matches pouring bottle lines
    static ref POURING_RE: Regex =
        Regex::new(r"^==> Pouring").unwrap();
    /// Matches progress bar lines (####... or %)
    static ref PROGRESS_RE: Regex =
        Regex::new(r"^#{5,}|^\s*\d+\.\d+%").unwrap();
    /// Matches the beer mug installation confirmation line
    static ref INSTALLED_RE: Regex =
        Regex::new(r"^🍺\s+/").unwrap();
    /// Matches cleanup noise lines (Running cleanup, Removing cached files, env hints)
    static ref CLEANUP_RE: Regex =
        Regex::new(r"^==> Running `brew cleanup|^Removing:|^Disable this behaviour|^Hide these hints").unwrap();
    /// Matches installing dependency lines
    static ref INSTALLING_DEP_RE: Regex =
        Regex::new(r"^==> Installing .* dependency:|^==> Installing dependencies").unwrap();
    /// Matches the top-level "==> Installing <pkg>" line (the one we want to keep)
    /// Note: can't use look-ahead in regex crate, so we match broadly and filter in code
    static ref INSTALLING_TOP_RE: Regex =
        Regex::new(r"^==> Installing (\S+)$").unwrap();
    /// Matches caveats section header
    static ref CAVEATS_HEADER_RE: Regex =
        Regex::new(r"^==> Caveats$").unwrap();
    /// Matches ==> Summary and ==> <pkg-name> duplicate sub-headers
    static ref NOISE_HEADER_RE: Regex =
        Regex::new(r"^==> (Summary|[a-zA-Z][@\w\-./]*)$").unwrap();
}

/// Filter `brew install` / `brew upgrade` output.
///
/// Strips all download/progress/pouring/cleanup noise, keeps only:
/// - The final `🍺 /path` installed confirmation (last one only, for the primary package)
/// - The first `==> Caveats` block content
///
/// Target: 75% savings.
pub fn filter_brew_install(input: &str) -> String {
    if input.trim().is_empty() {
        return String::new();
    }

    // Collect all 🍺 lines — we only want the last one (the top-level package)
    let installed_lines: Vec<&str> = input.lines().filter(|l| INSTALLED_RE.is_match(l)).collect();
    let final_installed = installed_lines.last().copied();

    // Extract caveats block (first occurrence only)
    let mut caveats_lines: Vec<&str> = Vec::new();
    let mut in_caveats = false;
    let mut seen_caveats = false;

    for line in input.lines() {
        if CAVEATS_HEADER_RE.is_match(line) {
            if !seen_caveats {
                seen_caveats = true;
                in_caveats = true;
                caveats_lines.push(line);
            } else {
                // Second caveats block — stop collecting
                in_caveats = false;
            }
            continue;
        }

        // Caveats ends when we hit the next ==> header or a 🍺 line
        if in_caveats && (NOISE_HEADER_RE.is_match(line) || INSTALLED_RE.is_match(line)) {
            in_caveats = false;
        }

        if in_caveats && !line.trim().is_empty() {
            caveats_lines.push(line);
        }
    }

    // Build compact output
    let mut lines_out: Vec<&str> = Vec::new();

    if !caveats_lines.is_empty() {
        lines_out.extend_from_slice(&caveats_lines);
    }

    if let Some(installed) = final_installed {
        lines_out.push(installed);
    }

    let result = lines_out.join("\n");
    if result.trim().is_empty() {
        input.to_string()
    } else {
        result
    }
}

/// Filter `brew list` output.
///
/// Extracts the formula name (first whitespace-delimited token) from each line
/// and produces a compact comma-separated summary with count.  Target: 60% savings.
pub fn filter_brew_list(input: &str) -> String {
    if input.trim().is_empty() {
        return String::new();
    }

    let packages: Vec<&str> = input
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        // Take only the first token (formula name), ignoring version/flags
        .filter_map(|l| l.split_whitespace().next())
        .collect();

    if packages.is_empty() {
        return String::new();
    }

    format!("{} installed: {}", packages.len(), packages.join(", "))
}

#[cfg(test)]
mod tests {
    use super::*;
    use insta::assert_snapshot;

    fn count_tokens(s: &str) -> usize {
        s.split_whitespace().count()
    }

    // --- filter_brew_install ---

    #[test]
    fn test_brew_install_format() {
        let input = include_str!("../../../tests/fixtures/pkg/brew_install_raw.txt");
        let output = filter_brew_install(input);
        assert_snapshot!(output);
    }

    #[test]
    fn test_brew_install_savings() {
        let input = include_str!("../../../tests/fixtures/pkg/brew_install_raw.txt");
        let output = filter_brew_install(input);
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
    fn test_brew_install_empty() {
        assert_eq!(filter_brew_install(""), "");
        assert_eq!(filter_brew_install("   \n  "), "");
    }

    #[test]
    fn test_brew_install_malformed() {
        let output = filter_brew_install("not brew output\njust some text");
        assert!(!output.is_empty(), "Should return input on malformed data");
    }

    #[test]
    fn test_brew_install_keeps_installed_line() {
        let input = "==> Downloading https://example.com\n######### 100.0%\n==> Pouring foo.bottle.tar.gz\n🍺  /opt/homebrew/Cellar/foo/1.0: 5 files";
        let output = filter_brew_install(input);
        assert!(
            output.contains("🍺"),
            "Should keep the installed confirmation"
        );
        assert!(!output.contains("Downloading"), "Should strip downloading");
        assert!(!output.contains("Pouring"), "Should strip pouring");
    }

    #[test]
    fn test_brew_install_keeps_caveats() {
        let input = "==> Pouring foo.bottle.tar.gz\n==> Caveats\nSome important caveat here\n🍺  /opt/homebrew/Cellar/foo/1.0: 5 files";
        let output = filter_brew_install(input);
        assert!(output.contains("Caveats"), "Should keep caveats header");
        assert!(
            output.contains("Some important caveat"),
            "Should keep caveat content"
        );
    }

    // --- filter_brew_list ---

    #[test]
    fn test_brew_list_format() {
        let input = include_str!("../../../tests/fixtures/pkg/brew_list_raw.txt");
        let output = filter_brew_list(input);
        assert_snapshot!(output);
    }

    #[test]
    fn test_brew_list_savings() {
        let input = include_str!("../../../tests/fixtures/pkg/brew_list_raw.txt");
        let output = filter_brew_list(input);
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
    fn test_brew_list_empty() {
        assert_eq!(filter_brew_list(""), "");
        assert_eq!(filter_brew_list("   \n"), "");
    }

    #[test]
    fn test_brew_list_single_package() {
        let output = filter_brew_list("wget\n");
        assert_eq!(output, "1 installed: wget");
    }

    #[test]
    fn test_brew_list_count_included() {
        let input = include_str!("../../../tests/fixtures/pkg/brew_list_raw.txt");
        let output = filter_brew_list(input);
        assert!(
            output.starts_with("31 installed:"),
            "Expected '31 installed:' prefix, got: {}",
            &output[..output.len().min(40)]
        );
    }
}

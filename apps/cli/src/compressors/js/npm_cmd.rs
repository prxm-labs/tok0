use lazy_static::lazy_static;
use regex::Regex;

lazy_static! {
    // npm install: lines to strip
    static ref PROGRESS_RE: Regex = Regex::new(r"^[⠙⠹⠸⠼⠴⠦⠧⠇⠏⠋]").unwrap();
    static ref REIFY_RE: Regex = Regex::new(r"^\s*(reify:|timing\s)").unwrap();
    static ref DEPRECATED_WARN_RE: Regex =
        Regex::new(r"^npm warn deprecated ").unwrap();
    static ref PEER_WARN_RE: Regex =
        Regex::new(r"^npm warn peer ").unwrap();
    static ref FUNDING_RE: Regex =
        Regex::new(r"^(\d+ packages? are looking for funding|  run `npm fund`)").unwrap();
    static ref TIMING_RE: Regex =
        Regex::new(r"^npm timing ").unwrap();
    static ref ADDED_RE: Regex =
        Regex::new(r"^added \d+ packages?").unwrap();
    static ref AUDIT_VULN_RE: Regex =
        Regex::new(r"^\d+ vulnerabilit").unwrap();
    static ref AUDIT_FIX_RE: Regex =
        Regex::new(r"^\s*(To address|Run `npm audit|npm audit fix)").unwrap();
    static ref AUDITED_RE: Regex =
        Regex::new(r"^(\d+ packages?, and audited|and audited)").unwrap();

    // pnpm install: leading-space progress, banner box, package counters.
    // pnpm prefixes its own progress lines with "Progress:" (no spinner)
    // and pads warning levels with two spaces — the original npm regex
    // set ignored both, so a real `pnpm install` flowed through largely
    // unchanged.
    static ref PNPM_PROGRESS_RE: Regex = Regex::new(r"^Progress: ").unwrap();
    static ref PNPM_WARN_RE: Regex = Regex::new(r"^\s*WARN\s").unwrap();
    static ref PNPM_BANNER_RE: Regex =
        Regex::new(r"^\s*(╭─+╮|╰─+╯|│.*│)\s*$").unwrap();
    static ref PNPM_PKG_COUNT_RE: Regex =
        Regex::new(r"^(Packages: [+-]\d+|\++)$").unwrap();

    // yarn classic (v1) framing: numbered step markers and the install banner.
    static ref YARN1_STEP_RE: Regex = Regex::new(r"^\[\d+/\d+\]").unwrap();
    static ref YARN1_BANNER_RE: Regex =
        Regex::new(r"^(yarn install v|info |success Saved lockfile)").unwrap();

    // yarn berry (3.x+) section framing without value:
    // YN0000 boxes the resolution/fetch/link steps, YN0013 announces
    // each cache miss — both are pure noise during a clean install.
    static ref YARN_BERRY_FRAME_RE: Regex =
        Regex::new(r"^➤ YN0000: [┌└]").unwrap();
    static ref YARN_BERRY_FETCH_RE: Regex = Regex::new(r"^➤ YN0013:").unwrap();

    // bun install banner ("bun install vX.Y.Z (hash)"). Stripping the
    // banner is harmless — the version is on the "Done" line.
    static ref BUN_BANNER_RE: Regex = Regex::new(r"^bun install v\d").unwrap();

    // Node deprecation footer that pnpm/yarn forward unchanged from
    // the underlying Node runtime — never user-actionable.
    static ref NODE_DEPRECATION_RE: Regex =
        Regex::new(r"^\(node:\d+\)|^\(Use `node ").unwrap();

    // npm test: lines to keep / strip
    static ref TEST_PASS_LINE_RE: Regex =
        Regex::new(r"^\s+✓ ").unwrap();
    static ref TEST_SUITE_PASS_RE: Regex =
        Regex::new(r"^ PASS  ").unwrap();
    static ref TEST_SUITE_FAIL_RE: Regex =
        Regex::new(r"^ FAIL  ").unwrap();
    static ref TEST_SUMMARY_RE: Regex =
        Regex::new(r"^(Test Suites:|Tests:|Snapshots:|Time:|Ran all)").unwrap();
    static ref TEST_SCRIPT_RE: Regex =
        Regex::new(r"^> .+ test$").unwrap();
    static ref TEST_RUNNER_RE: Regex =
        Regex::new(r"^> jest").unwrap();
}

/// Filter `npm install` / `npm ci` output.
/// Keeps: summary line, audit vuln counts, high-sev guidance.
/// Strips: progress spinners, deprecation warnings, funding blurbs, timing.
pub fn filter_npm_install(input: &str) -> String {
    if input.trim().is_empty() {
        return String::new();
    }

    let mut out: Vec<&str> = Vec::new();

    for line in input.lines() {
        // Always strip pure noise
        if PROGRESS_RE.is_match(line)
            || REIFY_RE.is_match(line)
            || DEPRECATED_WARN_RE.is_match(line)
            || TIMING_RE.is_match(line)
            || FUNDING_RE.is_match(line)
            || PNPM_PROGRESS_RE.is_match(line)
            || PNPM_WARN_RE.is_match(line)
            || PNPM_BANNER_RE.is_match(line)
            || PNPM_PKG_COUNT_RE.is_match(line)
            || YARN1_STEP_RE.is_match(line)
            || YARN1_BANNER_RE.is_match(line)
            || YARN_BERRY_FRAME_RE.is_match(line)
            || YARN_BERRY_FETCH_RE.is_match(line)
            || BUN_BANNER_RE.is_match(line)
            || NODE_DEPRECATION_RE.is_match(line)
        {
            continue;
        }

        // Keep important lines
        if ADDED_RE.is_match(line)
            || AUDIT_VULN_RE.is_match(line)
            || AUDIT_FIX_RE.is_match(line)
            || AUDITED_RE.is_match(line)
            || PEER_WARN_RE.is_match(line)
        {
            out.push(line);
            continue;
        }

        // Skip blank lines — they're just spacing around stripped content
        if line.trim().is_empty() {
            continue;
        }

        // Pass through anything else (e.g. real errors)
        out.push(line);
    }

    if out.is_empty() {
        return String::new();
    }

    out.join("\n").trim_end().to_string()
}

/// Filter `npm test` / `npm run test` output.
/// Keeps: FAIL suite headers, failure details, summary block.
/// Strips: PASS suite headers, individual passing test lines.
pub fn filter_npm_test(input: &str) -> String {
    if input.trim().is_empty() {
        return String::new();
    }

    let mut out: Vec<&str> = Vec::new();
    let mut in_failure_block = false;

    for line in input.lines() {
        // Strip script invocation noise
        if TEST_SCRIPT_RE.is_match(line) || TEST_RUNNER_RE.is_match(line) {
            continue;
        }

        // Strip individual passing test lines
        if TEST_PASS_LINE_RE.is_match(line) {
            continue;
        }

        // FAIL suite lines — keep and enter failure mode
        if TEST_SUITE_FAIL_RE.is_match(line) {
            in_failure_block = true;
            out.push(line);
            continue;
        }

        // PASS suite lines — collapse to nothing (saved by summary)
        if TEST_SUITE_PASS_RE.is_match(line) {
            in_failure_block = false;
            continue;
        }

        // Summary block always kept
        if TEST_SUMMARY_RE.is_match(line) {
            in_failure_block = false;
            out.push(line);
            continue;
        }

        // Inside failure block: keep everything (error messages, stack traces)
        if in_failure_block {
            out.push(line);
            continue;
        }

        // Blank lines between sections: skip
        if line.trim().is_empty() {
            continue;
        }

        // Anything else (runner header, etc.) — pass through
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

    // ── npm install tests ──────────────────────────────────────────────────

    #[test]
    fn test_npm_install_format() {
        let input = include_str!("../../../tests/fixtures/js/npm_install_raw.txt");
        let output = filter_npm_install(input);
        assert_snapshot!(output);
    }

    #[test]
    fn test_npm_install_savings() {
        let input = include_str!("../../../tests/fixtures/js/npm_install_raw.txt");
        let output = filter_npm_install(input);
        let input_t = count_tokens(input);
        let output_t = count_tokens(&output);
        let savings = 100.0 - (output_t as f64 / input_t as f64 * 100.0);
        assert!(
            savings >= 70.0,
            "Expected >=70% savings on npm install, got {:.1}% ({} -> {} tokens)",
            savings,
            input_t,
            output_t
        );
    }

    #[test]
    fn test_npm_install_empty() {
        assert_eq!(filter_npm_install(""), "");
        assert_eq!(filter_npm_install("   \n  "), "");
    }

    #[test]
    fn test_npm_install_malformed() {
        // Non-npm output should pass through unchanged (no crash)
        let input = "some random text\nnot npm output at all";
        let output = filter_npm_install(input);
        assert!(
            !output.is_empty(),
            "Should not return empty for unknown input"
        );
    }

    #[test]
    fn test_npm_install_snapshot() {
        let raw = include_str!("../../../tests/fixtures/js/npm_install_raw.txt");
        let out = filter_npm_install(raw);
        insta::assert_snapshot!(out);
    }

    #[test]
    fn test_npm_install_savings_at_least_60pct() {
        let raw = include_str!("../../../tests/fixtures/js/npm_install_raw.txt");
        let in_t = raw.split_whitespace().count();
        let out_t = filter_npm_install(raw).split_whitespace().count();
        let pct = 100 - (out_t * 100 / in_t.max(1));
        assert!(pct >= 60, "expected >=60%, got {}%", pct);
    }

    #[test]
    fn test_npm_install_keeps_added_summary() {
        let input = "npm warn deprecated foo@1.0.0: deprecated\nadded 42 packages, and audited 43 packages in 5s\n";
        let output = filter_npm_install(input);
        assert!(
            output.contains("added 42 packages"),
            "Should keep summary line"
        );
        assert!(
            !output.contains("deprecated"),
            "Should strip deprecation warning"
        );
    }

    #[test]
    fn test_npm_install_keeps_audit_vulns() {
        let input = "added 10 packages, and audited 11 packages in 1s\n\n8 vulnerabilities (3 low, 2 moderate, 3 high)\n\n  To address issues that do not require attention, run:\n    npm audit fix\n";
        let output = filter_npm_install(input);
        assert!(output.contains("vulnerabilit"), "Should keep vuln count");
    }

    #[test]
    fn test_npm_install_strips_progress_spinners() {
        let input = "⠙ reify:lodash: http fetch GET 200 ...\nadded 5 packages in 1s\n";
        let output = filter_npm_install(input);
        assert!(
            !output.contains("reify:"),
            "Should strip spinner/reify lines"
        );
        assert!(output.contains("added 5 packages"));
    }

    // ── pnpm / yarn / bun install (same dispatcher path) ───────────────────

    /// All four JS package managers route their `install` subcommand
    /// through `filter_npm_install`. Before this expansion, only npm
    /// noise was stripped, so pnpm progress lines, yarn step markers,
    /// and the bun banner all flowed through unchanged.
    #[test]
    fn test_pnpm_install_pkg_mgr_conflict_savings() {
        let raw = include_str!("../../../tests/fixtures/js/pnpm_install_pkg_mgr_conflict_raw.txt");
        let out = filter_npm_install(raw);
        let in_t = raw.split_whitespace().count();
        let out_t = out.split_whitespace().count();
        let pct = 100 - (out_t * 100 / in_t.max(1));
        assert!(
            pct >= 70,
            "expected ≥70% savings on pnpm sample, got {}%",
            pct
        );
        // The signal we keep: every direct dep on its own line.
        assert!(out.contains("react 19.2.6"));
        assert!(out.contains("vite 8.0.11"));
        assert!(out.contains("Done in 9.3s"));
    }

    #[test]
    fn test_pnpm_install_pkg_mgr_conflict_snapshot() {
        let raw = include_str!("../../../tests/fixtures/js/pnpm_install_pkg_mgr_conflict_raw.txt");
        let out = filter_npm_install(raw);
        insta::assert_snapshot!(out);
    }

    #[test]
    fn test_pnpm_install_strips_warn_moving_lines() {
        let input = " WARN  Moving react that was installed by a different package manager to \"node_modules/.ignored\"\n WARN  Moving vite that was installed by a different package manager to \"node_modules/.ignored\"\nProgress: resolved 100, reused 0, downloaded 80, added 0\n+ react 19.2.6\n";
        let out = filter_npm_install(input);
        assert!(
            !out.contains("Moving"),
            "Should strip leading-space WARN lines"
        );
        assert!(
            !out.contains("Progress:"),
            "Should strip pnpm Progress lines"
        );
        assert!(out.contains("react 19.2.6"));
    }

    #[test]
    fn test_pnpm_install_strips_update_banner() {
        let input = "   ╭─────────────────────────────────╮\n   │   Update available! 10 → 11.   │\n   ╰─────────────────────────────────╯\n+ react 19.2.6\n";
        let out = filter_npm_install(input);
        assert!(!out.contains("╭"), "Should strip banner top");
        assert!(
            !out.contains("Update available"),
            "Should strip banner body"
        );
        assert!(!out.contains("╰"), "Should strip banner bottom");
        assert!(out.contains("react 19.2.6"));
    }

    #[test]
    fn test_pnpm_install_strips_packages_counter() {
        let input = "Packages: +196\n++++++++++++++++++++++++++++++++\n+ react 19.2.6\n";
        let out = filter_npm_install(input);
        assert!(
            !out.contains("Packages: +"),
            "Should strip pnpm pkg counter"
        );
        assert!(!out.contains("++++"), "Should strip the +++ progress bar");
        assert!(out.contains("react 19.2.6"));
    }

    #[test]
    fn test_yarn1_install_strips_step_markers() {
        let input = "yarn install v1.22.21\ninfo No lockfile found.\n[1/4] Resolving packages...\n[2/4] Fetching packages...\n[3/4] Linking dependencies...\n[4/4] Building fresh packages...\nsuccess Saved lockfile.\nDone in 0.53s.\n";
        let out = filter_npm_install(input);
        assert!(!out.contains("[1/4]"), "Should strip yarn step markers");
        assert!(!out.contains("yarn install v"), "Should strip yarn banner");
        assert!(!out.contains("info "), "Should strip yarn info lines");
        assert!(
            !out.contains("success Saved"),
            "Should strip lockfile success"
        );
        assert!(out.contains("Done in"));
    }

    #[test]
    fn test_yarn_berry_install_strips_section_frames() {
        let input = "➤ YN0000: ┌ Resolution step\n➤ YN0061: │ left-pad@npm:1.3.0 is deprecated\n➤ YN0000: └ Completed in 0s 324ms\n➤ YN0013: │ react@npm:18 fetched from registry\n➤ YN0000: Done with warnings in 0s 457ms\n";
        let out = filter_npm_install(input);
        assert!(!out.contains("┌"), "Should strip section open frame");
        assert!(!out.contains("└"), "Should strip section close frame");
        assert!(!out.contains("YN0013"), "Should strip cache-miss noise");
        // Genuine warning (YN0061 = deprecation) is kept.
        assert!(out.contains("YN0061"), "Should keep deprecation warning");
        assert!(out.contains("Done with warnings"));
    }

    #[test]
    fn test_bun_install_strips_banner() {
        let input =
            "bun install v1.3.6 (d530ed99)\n+ react@18.3.1\n104 packages installed [1.52s]\n";
        let out = filter_npm_install(input);
        assert!(!out.contains("bun install v"), "Should strip bun banner");
        assert!(out.contains("react@18.3.1"));
        assert!(out.contains("104 packages installed"));
    }

    #[test]
    fn test_install_strips_node_deprecation_footer() {
        let input = "+ react 19.2.6\n(node:13518) [DEP0169] DeprecationWarning: url.parse() is deprecated\n(Use `node --trace-deprecation ...` to show where the warning was created)\nDone in 9.3s\n";
        let out = filter_npm_install(input);
        assert!(
            !out.contains("DeprecationWarning"),
            "Should strip node DEP warning"
        );
        assert!(
            !out.contains("trace-deprecation"),
            "Should strip trace hint"
        );
        assert!(out.contains("react 19.2.6"));
        assert!(out.contains("Done in"));
    }

    /// Real-fixture coverage for the dispatcher path. The dispatcher
    /// routes `yarn install`, `yarn1 install`, `bun install` through
    /// this same function — these tests catch regressions in the
    /// expanded regex set against captured-from-prod output.
    #[test]
    fn test_yarn1_real_fixture_savings() {
        let raw = include_str!("../../../tests/fixtures/js/yarn1_install_raw.txt");
        let out = filter_npm_install(raw);
        let in_t = raw.split_whitespace().count();
        let out_t = out.split_whitespace().count();
        let pct = 100 - (out_t * 100 / in_t.max(1));
        assert!(pct >= 60, "yarn1 fixture: expected ≥60%, got {}%", pct);
        assert!(out.contains("Done in"));
    }

    #[test]
    fn test_yarn1_real_fixture_snapshot() {
        let raw = include_str!("../../../tests/fixtures/js/yarn1_install_raw.txt");
        insta::assert_snapshot!(filter_npm_install(raw));
    }

    #[test]
    fn test_yarn_berry_real_fixture_savings() {
        let raw = include_str!("../../../tests/fixtures/js/yarn_berry_install_raw.txt");
        let out = filter_npm_install(raw);
        let in_t = raw.split_whitespace().count();
        let out_t = out.split_whitespace().count();
        let pct = 100 - (out_t * 100 / in_t.max(1));
        assert!(pct >= 30, "yarn-berry fixture: expected ≥30%, got {}%", pct);
        // Deprecation warning (YN0061) preserved; framing/fetch noise gone.
        assert!(out.contains("YN0061") || out.contains("Done with warnings"));
    }

    #[test]
    fn test_yarn_berry_real_fixture_snapshot() {
        let raw = include_str!("../../../tests/fixtures/js/yarn_berry_install_raw.txt");
        insta::assert_snapshot!(filter_npm_install(raw));
    }

    #[test]
    fn test_bun_real_fixture_snapshot() {
        let raw = include_str!("../../../tests/fixtures/js/bun_install_raw.txt");
        insta::assert_snapshot!(filter_npm_install(raw));
    }

    #[test]
    fn test_install_unicode_pkg_name_no_panic() {
        // Unicode in pkg names (rare but legal) must not break filtering.
        let input = " WARN  Moving パッケージ that was installed elsewhere to .ignored\n+ パッケージ 1.0.0\nDone in 1s\n";
        let out = filter_npm_install(input);
        assert!(out.contains("パッケージ 1.0.0"));
        assert!(!out.contains("Moving"));
    }

    #[test]
    fn test_install_keeps_real_errors() {
        // Genuine errors must always pass through — never swallow signal.
        let input = " WARN  Moving foo to .ignored\nProgress: resolved 1\nERR_PNPM_FETCH_404 GET https://registry.npmjs.org/nonexistent: Not Found\n";
        let out = filter_npm_install(input);
        assert!(
            out.contains("ERR_PNPM_FETCH_404"),
            "Real errors must survive compression"
        );
    }

    // ── npm test tests ─────────────────────────────────────────────────────

    #[test]
    fn test_npm_test_format() {
        let input = include_str!("../../../tests/fixtures/js/npm_test_raw.txt");
        let output = filter_npm_test(input);
        assert_snapshot!(output);
    }

    #[test]
    fn test_npm_test_savings() {
        let input = include_str!("../../../tests/fixtures/js/npm_test_raw.txt");
        let output = filter_npm_test(input);
        let input_t = count_tokens(input);
        let output_t = count_tokens(&output);
        let savings = 100.0 - (output_t as f64 / input_t as f64 * 100.0);
        assert!(
            savings >= 45.0,
            "Expected >=45% savings on npm test, got {:.1}% ({} -> {} tokens)",
            savings,
            input_t,
            output_t
        );
    }

    #[test]
    fn test_npm_test_empty() {
        assert_eq!(filter_npm_test(""), "");
        assert_eq!(filter_npm_test("  \n  "), "");
    }

    #[test]
    fn test_npm_test_malformed() {
        let input = "not jest output\nrandom text here";
        let output = filter_npm_test(input);
        assert!(
            !output.is_empty(),
            "Should not return empty for unknown input"
        );
    }

    #[test]
    fn test_npm_test_keeps_failures() {
        let input = " FAIL  src/foo.test.ts\n  ● foo › fails\n\n    Expected: 1\n    Received: 0\n\nTests: 1 failed, 0 passed, 1 total\n";
        let output = filter_npm_test(input);
        assert!(output.contains("FAIL"), "Should keep FAIL suite header");
        assert!(
            output.contains("Expected: 1"),
            "Should keep failure details"
        );
        assert!(output.contains("1 failed"), "Should keep test summary");
    }

    #[test]
    fn test_npm_test_strips_passing_lines() {
        let input = " PASS  src/foo.test.ts\n  foo\n    ✓ passes (5 ms)\n    ✓ also passes (3 ms)\n\nTests: 0 failed, 2 passed, 2 total\n";
        let output = filter_npm_test(input);
        assert!(!output.contains("✓"), "Should strip passing test lines");
        assert!(!output.contains("PASS "), "Should strip PASS suite header");
        assert!(output.contains("2 passed"), "Should keep summary");
    }

    #[test]
    fn test_npm_test_all_passing() {
        let input = " PASS  src/a.test.ts\n    ✓ test one (2 ms)\n\nTest Suites: 0 failed, 1 passed, 1 total\nTests:       0 failed, 1 passed, 1 total\nTime:        1.234 s\n";
        let output = filter_npm_test(input);
        assert!(output.contains("Test Suites:"), "Should keep summary block");
        assert!(output.contains("Tests:"), "Should keep tests count");
        assert!(output.contains("Time:"), "Should keep timing in summary");
    }
}

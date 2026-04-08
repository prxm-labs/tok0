//! Comprehensive benchmark: runs ALL compressor filters against REAL captured output
//! and reports token savings. Every filter must not panic on real data.
//! Savings are printed for manual review and asserted against minimum thresholds.

mod common;

use tok0::compressors::git::git_cmd;
use tok0::compressors::pkg::brew_cmd;
use tok0::compressors::rust::cargo_cmd;
use tok0::compressors::system::{
    df_cmd, diff_cmd, du_cmd, env_cmd, find_cmd, grep_cmd, ls_cmd, read_cmd, smart_cmd, wc_cmd,
};

fn count_tokens(s: &str) -> usize {
    s.split_whitespace().count()
}

fn report(name: &str, input: &str, output: &str) -> f64 {
    let i = count_tokens(input);
    let o = count_tokens(output);
    if i == 0 {
        return 0.0;
    }
    let pct = 100.0 - (o as f64 / i as f64 * 100.0);
    eprintln!(
        "  {:<25} {:>6} -> {:>6} tokens  ({:>5.1}% savings)",
        name, i, o, pct
    );
    pct
}

// ============================================================
// System compressors vs real output
// ============================================================

#[test]
fn bench_ls_real() {
    let input = include_str!("fixtures/real/ls_la.txt");
    let output = ls_cmd::filter_ls(input, ".");
    let pct = report("ls -la", input, &output);
    assert!(pct >= 20.0, "ls savings {:.1}% below 20%", pct);
    assert!(!output.is_empty());
}

#[test]
fn bench_find_real() {
    let input = include_str!("fixtures/real/find_rs.txt");
    let output = find_cmd::filter_find(input);
    let pct = report("find *.rs", input, &output);
    assert!(pct >= 20.0, "find savings {:.1}% below 20%", pct);
}

#[test]
fn bench_grep_real() {
    let input = include_str!("fixtures/real/grep_pubfn.txt");
    let output = grep_cmd::filter_grep(input, 50, 5);
    let pct = report("grep pub fn", input, &output);
    assert!(pct >= 5.0, "grep savings {:.1}% below 5%", pct);
}

#[test]
fn bench_df_real() {
    let input = include_str!("fixtures/real/df_h.txt");
    let output = df_cmd::filter_df(input);
    let pct = report("df -h", input, &output);
    assert!(pct >= 10.0, "df savings {:.1}% below 10%", pct);
}

#[test]
fn bench_du_real() {
    let input = include_str!("fixtures/real/du_sh.txt");
    let output = du_cmd::filter_du(input, 10);
    report("du -sh", input, &output);
    assert!(!output.is_empty());
}

#[test]
fn bench_ps_real() {
    let input = include_str!("fixtures/real/ps_aux.txt");
    // ps doesn't have a dedicated filter yet; use read as proxy
    let output = read_cmd::filter_read(input, "ps_output");
    let pct = report("ps aux (via read)", input, &output);
    assert!(!output.is_empty());
    let _ = pct;
}

#[test]
fn bench_env_real() {
    let input = include_str!("fixtures/real/env.txt");
    let output = env_cmd::filter_env(input, None);
    let pct = report("env", input, &output);
    assert!(pct >= 5.0, "env savings {:.1}% below 5%", pct);
}

#[test]
fn bench_wc_real() {
    let input = include_str!("fixtures/real/wc_l.txt");
    let output = wc_cmd::filter_wc(input);
    report("wc -l", input, &output);
    assert!(!output.is_empty());
}

#[test]
fn bench_read_large_real() {
    // Use the cargo_test output as a large file read scenario
    let input = include_str!("fixtures/real/cargo_test.txt");
    let output = read_cmd::filter_read(input, "cargo_test_output.txt");
    let pct = report("read (380 lines)", input, &output);
    assert!(pct >= 40.0, "large file read savings {:.1}% below 40%", pct);
}

#[test]
fn bench_smart_real() {
    // Use a real Rust source file as smart input
    let input = include_str!("../src/engine/config.rs");
    let output = smart_cmd::filter_smart(input);
    report("smart (config.rs)", input, &output);
    assert!(!output.is_empty());
}

// ============================================================
// Git compressors vs real output
// ============================================================

#[test]
fn bench_git_status_real() {
    let input = include_str!("fixtures/real/git_status.txt");
    let output = git_cmd::filter_git_status(input);
    let pct = report("git status", input, &output);
    assert!(pct >= 10.0, "git status savings {:.1}% below 10%", pct);
}

// ============================================================
// Cargo compressors vs real output
// ============================================================

#[test]
fn bench_cargo_test_real() {
    let input = include_str!("fixtures/real/cargo_test.txt");
    let output = cargo_cmd::filter_cargo_test(input);
    let pct = report("cargo test", input, &output);
    assert!(pct >= 70.0, "cargo test savings {:.1}% below 70%", pct);
}

#[test]
fn bench_cargo_build_real() {
    let input = include_str!("fixtures/real/cargo_build.txt");
    let output = cargo_cmd::filter_cargo_build(input);
    let pct = report("cargo build", input, &output);
    assert!(pct >= 60.0, "cargo build savings {:.1}% below 60%", pct);
}

#[test]
fn bench_cargo_clippy_real() {
    let input = include_str!("fixtures/real/cargo_clippy.txt");
    let output = cargo_cmd::filter_cargo_clippy(input);
    let pct = report("cargo clippy", input, &output);
    assert!(pct >= 50.0, "cargo clippy savings {:.1}% below 50%", pct);
}

// ============================================================
// Brew compressor vs real output
// ============================================================

#[test]
fn bench_brew_list_real() {
    let input = include_str!("fixtures/real/brew_list.txt");
    let output = brew_cmd::filter_brew_list(input);
    let _pct = report("brew list", input, &output);
    // brew list: value is structural (194 lines -> 1 line), not token savings
    // with short package names, token count is similar
    assert!(
        output.lines().count() <= 3,
        "brew list should be compact (<=3 lines)"
    );
}

// ============================================================
// Cross-compressor: diff filter on real cargo clippy (multiline)
// ============================================================

#[test]
fn bench_diff_on_real_clippy() {
    // clippy output has no diff format, so diff filter should passthrough
    let input = include_str!("fixtures/real/cargo_clippy.txt");
    let output = diff_cmd::filter_diff(input);
    // Should passthrough since it's not diff format
    assert!(!output.is_empty());
}

// ============================================================
// Summary: all compressors must not panic on each other's input
// ============================================================

#[test]
fn cross_compressor_no_panic() {
    let inputs = vec![
        include_str!("fixtures/real/ls_la.txt"),
        include_str!("fixtures/real/find_rs.txt"),
        include_str!("fixtures/real/grep_pubfn.txt"),
        include_str!("fixtures/real/df_h.txt"),
        include_str!("fixtures/real/du_sh.txt"),
        include_str!("fixtures/real/env.txt"),
        include_str!("fixtures/real/cargo_test.txt"),
        include_str!("fixtures/real/cargo_build.txt"),
        include_str!("fixtures/real/brew_list.txt"),
        include_str!("fixtures/real/git_status.txt"),
    ];

    for input in &inputs {
        // Every filter must handle arbitrary input without panicking
        let _ = ls_cmd::filter_ls(input, ".");
        let _ = find_cmd::filter_find(input);
        let _ = grep_cmd::filter_grep(input, 50, 5);
        let _ = diff_cmd::filter_diff(input);
        let _ = read_cmd::filter_read(input, "test.txt");
        let _ = smart_cmd::filter_smart(input);
        let _ = wc_cmd::filter_wc(input);
        let _ = env_cmd::filter_env(input, None);
        let _ = du_cmd::filter_du(input, 10);
        let _ = df_cmd::filter_df(input);
        let _ = git_cmd::filter_git_log(input);
        let _ = git_cmd::filter_git_status(input);
        let _ = git_cmd::filter_git_diff(input);
        let _ = git_cmd::filter_git_simple(input, "push");
        let _ = cargo_cmd::filter_cargo_test(input);
        let _ = cargo_cmd::filter_cargo_build(input);
        let _ = cargo_cmd::filter_cargo_clippy(input);
        let _ = brew_cmd::filter_brew_list(input);
    }
}

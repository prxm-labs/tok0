//! Per-area compression benchmark showcase.
//! Exercises every compressor against its real fixture and reports token savings.
//!
//! Run with: cargo test --test benchmark_showcase -- --nocapture

mod common;

use tok0::compressors::build::make_cmd;
use tok0::compressors::cloud::{docker_cmd, terraform_cmd};
use tok0::compressors::db::db_cmd;
use tok0::compressors::git::git_cmd;
use tok0::compressors::go::go_cmd;
use tok0::compressors::js::{npm_cmd, tsc_cmd as js_tsc_cmd, vitest_cmd};
use tok0::compressors::lint::lint_cmd;
use tok0::compressors::pkg::brew_cmd;
use tok0::compressors::python::{pytest_cmd, ruff_cmd};
use tok0::compressors::rust::cargo_cmd;
use tok0::compressors::system::{diff_cmd, find_cmd, grep_cmd, ls_cmd, read_cmd, smart_cmd};

/// Token count proxy (whitespace-delimited words).
fn tokens(s: &str) -> usize {
    s.split_whitespace().count()
}

/// Print a benchmark row and return (input_tokens, output_tokens).
fn bench(area: &str, label: &str, input: &str, output: &str) -> (usize, usize) {
    let i = tokens(input);
    let o = tokens(output);
    let pct = if i > 0 {
        100.0 - (o as f64 / i as f64 * 100.0)
    } else {
        0.0
    };
    println!(
        "  BENCH  {:<12} {:<24} {:>6} → {:>6} tokens  ({:>5.1}% saved)",
        area, label, i, o, pct
    );
    (i, o)
}

/// Assert minimum savings threshold.
fn assert_min(area: &str, label: &str, input_t: usize, output_t: usize, min_pct: f64) {
    if input_t == 0 {
        return;
    }
    let pct = 100.0 - (output_t as f64 / input_t as f64 * 100.0);
    assert!(
        pct >= min_pct,
        "{}/{}: expected >= {:.0}% savings, got {:.1}% ({} → {} tokens)",
        area,
        label,
        min_pct,
        pct,
        input_t,
        output_t
    );
}

// ── Git ─────────────────────────────────────────────────────────────

#[test]
fn bench_git_log() {
    let input = include_str!("fixtures/git/log_raw.txt");
    let output = git_cmd::filter_git_log(input);
    let (i, o) = bench("git", "log", input, &output);
    assert_min("git", "log", i, o, 60.0);
}

#[test]
fn bench_git_status() {
    let input = include_str!("fixtures/git/status_raw.txt");
    let output = git_cmd::filter_git_status(input);
    let (i, o) = bench("git", "status", input, &output);
    assert_min("git", "status", i, o, 25.0);
}

#[test]
fn bench_git_diff() {
    let input = include_str!("fixtures/git/diff_raw.txt");
    let output = git_cmd::filter_git_diff(input);
    let (i, o) = bench("git", "diff", input, &output);
    assert_min("git", "diff", i, o, 50.0);
}

#[test]
fn bench_git_simple_push() {
    let input = "Enumerating objects: 5, done.\nCounting objects: 100% (5/5), done.\nDelta compression using up to 10 threads\nCompressing objects: 100% (3/3), done.\nWriting objects: 100% (3/3), 1.23 KiB | 1.23 MiB/s, done.\nTotal 3 (delta 2), reused 0 (delta 0)\nTo github.com:user/repo.git\n   abc1234..def5678  main -> main";
    let output = git_cmd::filter_git_simple(input, "push");
    let (i, o) = bench("git", "simple (push)", input, &output);
    assert_min("git", "simple (push)", i, o, 90.0);
}

#[test]
fn bench_gh_pr_list() {
    let input = include_str!("fixtures/git/gh_pr_list_raw.txt");
    let output = git_cmd::filter_git_log(input);
    let (_i, _o) = bench("git", "gh pr list", input, &output);
    // gh pr list uses log filter as fallback; savings vary
}

// ── System ──────────────────────────────────────────────────────────

#[test]
fn bench_ls() {
    let input = include_str!("fixtures/system/ls_raw.txt");
    let output = ls_cmd::filter_ls(input, ".");
    let (i, o) = bench("system", "ls", input, &output);
    assert_min("system", "ls", i, o, 30.0);
}

#[test]
fn bench_find() {
    let input = include_str!("fixtures/system/find_raw.txt");
    let output = find_cmd::filter_find(input);
    let (i, o) = bench("system", "find", input, &output);
    assert_min("system", "find", i, o, 30.0);
}

#[test]
fn bench_grep() {
    let input = include_str!("fixtures/system/grep_raw.txt");
    let output = grep_cmd::filter_grep(input, 50, 5);
    let (i, o) = bench("system", "grep", input, &output);
    assert_min("system", "grep", i, o, 10.0);
}

#[test]
fn bench_diff() {
    let input = include_str!("fixtures/system/find_raw.txt");
    let output = diff_cmd::filter_diff(input);
    let (_i, _o) = bench("system", "diff", input, &output);
}

#[test]
fn bench_read_large() {
    let lines: Vec<String> = (1..=500)
        .map(|i| format!("line {} content here with some data and more words", i))
        .collect();
    let input = lines.join("\n");
    let output = read_cmd::filter_read(&input, "big.rs");
    let (i, o) = bench("system", "read (500 lines)", &input, &output);
    assert_min("system", "read (500 lines)", i, o, 50.0);
}

#[test]
fn bench_smart_large() {
    let mut lines = vec![
        "use std::io;".to_string(),
        "use anyhow::Result;".to_string(),
        "".to_string(),
        "pub fn main() -> Result<()> {".to_string(),
    ];
    for i in 0..60 {
        lines.push(format!("    let x{} = compute_{}();", i, i));
    }
    lines.push("}".to_string());
    lines.push("pub struct Config { pub name: String }".to_string());
    let input = lines.join("\n");
    let output = smart_cmd::filter_smart(&input);
    let (i, o) = bench("system", "smart (65 lines)", &input, &output);
    assert_min("system", "smart (65 lines)", i, o, 40.0);
}

// ── Rust ────────────────────────────────────────────────────────────

#[test]
fn bench_cargo_test() {
    let input = include_str!("fixtures/rust/cargo_test_raw.txt");
    let output = cargo_cmd::filter_cargo_test(input);
    let (i, o) = bench("rust", "cargo test", input, &output);
    assert_min("rust", "cargo test", i, o, 60.0);
}

#[test]
fn bench_cargo_build() {
    let input = include_str!("fixtures/rust/cargo_build_raw.txt");
    let output = cargo_cmd::filter_cargo_build(input);
    let (i, o) = bench("rust", "cargo build", input, &output);
    assert_min("rust", "cargo build", i, o, 60.0);
}

#[test]
fn bench_cargo_clippy() {
    let input = include_str!("fixtures/rust/cargo_clippy_raw.txt");
    let output = cargo_cmd::filter_cargo_clippy(input);
    let (i, o) = bench("rust", "cargo clippy", input, &output);
    assert_min("rust", "cargo clippy", i, o, 40.0);
}

// ── Python ──────────────────────────────────────────────────────────

#[test]
fn bench_pytest() {
    let input = include_str!("fixtures/python/pytest_raw.txt");
    let output = pytest_cmd::filter_pytest(input);
    let (i, o) = bench("python", "pytest", input, &output);
    assert_min("python", "pytest", i, o, 40.0);
}

#[test]
fn bench_pytest_failures() {
    let input = include_str!("fixtures/python/pytest_failures_raw.txt");
    let output = pytest_cmd::filter_pytest(input);
    let (i, o) = bench("python", "pytest (failures)", input, &output);
    assert_min("python", "pytest (failures)", i, o, 30.0);
}

// ── Go ──────────────────────────────────────────────────────────────

#[test]
fn bench_go_test() {
    let input = include_str!("fixtures/go/go_test_raw.txt");
    let output = go_cmd::filter_go_test(input);
    let (i, o) = bench("go", "go test", input, &output);
    assert_min("go", "go test", i, o, 40.0);
}

#[test]
fn bench_go_build() {
    let input = include_str!("fixtures/go/go_build_raw.txt");
    let output = go_cmd::filter_go_build(input);
    let (i, o) = bench("go", "go build", input, &output);
    assert_min("go", "go build", i, o, 40.0);
}

#[test]
fn bench_golangci_lint() {
    let input = include_str!("fixtures/go/golangci_lint_raw.txt");
    let output = go_cmd::filter_golangci_lint(input);
    let (i, o) = bench("go", "golangci-lint", input, &output);
    assert_min("go", "golangci-lint", i, o, 30.0);
}

// ── JavaScript ──────────────────────────────────────────────────────

#[test]
fn bench_npm_install() {
    let input = include_str!("fixtures/js/npm_install_raw.txt");
    let output = npm_cmd::filter_npm_install(input);
    let (i, o) = bench("js", "npm install", input, &output);
    assert_min("js", "npm install", i, o, 50.0);
}

#[test]
fn bench_npm_test() {
    let input = include_str!("fixtures/js/npm_test_raw.txt");
    let output = npm_cmd::filter_npm_test(input);
    let (i, o) = bench("js", "npm test", input, &output);
    assert_min("js", "npm test", i, o, 40.0);
}

#[test]
fn bench_vitest() {
    let input = include_str!("fixtures/js/vitest_raw.txt");
    let output = vitest_cmd::filter_vitest(input);
    let (i, o) = bench("js", "vitest", input, &output);
    assert_min("js", "vitest", i, o, 40.0);
}

#[test]
fn bench_tsc_js() {
    let input = include_str!("fixtures/js/tsc_raw.txt");
    let output = js_tsc_cmd::filter_tsc(input);
    let (i, o) = bench("js", "tsc", input, &output);
    assert_min("js", "tsc", i, o, 10.0);
}

// ── Cloud ───────────────────────────────────────────────────────────

#[test]
fn bench_docker_ps() {
    let input = include_str!("fixtures/cloud/docker_ps_raw.txt");
    let output = docker_cmd::filter_docker_ps(input);
    let (i, o) = bench("cloud", "docker ps", input, &output);
    assert_min("cloud", "docker ps", i, o, 40.0);
}

#[test]
fn bench_docker_build() {
    let input = include_str!("fixtures/cloud/docker_build_raw.txt");
    let output = docker_cmd::filter_docker_build(input);
    let (i, o) = bench("cloud", "docker build", input, &output);
    assert_min("cloud", "docker build", i, o, 40.0);
}

#[test]
fn bench_terraform_plan() {
    let input = include_str!("fixtures/cloud/terraform_plan_raw.txt");
    let output = terraform_cmd::filter_terraform_plan(input);
    let (i, o) = bench("cloud", "terraform plan", input, &output);
    assert_min("cloud", "terraform plan", i, o, 50.0);
}

#[test]
fn bench_terraform_apply() {
    let input = include_str!("fixtures/cloud/terraform_apply_raw.txt");
    let output = terraform_cmd::filter_terraform_apply(input);
    let (i, o) = bench("cloud", "terraform apply", input, &output);
    assert_min("cloud", "terraform apply", i, o, 40.0);
}

// ── Package managers ────────────────────────────────────────────────

#[test]
fn bench_brew_install() {
    let input = include_str!("fixtures/pkg/brew_install_raw.txt");
    let output = brew_cmd::filter_brew_install(input);
    let (i, o) = bench("pkg", "brew install", input, &output);
    assert_min("pkg", "brew install", i, o, 50.0);
}

#[test]
fn bench_brew_list() {
    let input = include_str!("fixtures/pkg/brew_list_raw.txt");
    let output = brew_cmd::filter_brew_list(input);
    let (i, o) = bench("pkg", "brew list", input, &output);
    assert_min("pkg", "brew list", i, o, 30.0);
}

// ── Build ───────────────────────────────────────────────────────────

#[test]
fn bench_make() {
    let input = include_str!("fixtures/build/make_raw.txt");
    let output = make_cmd::filter_make(input);
    let (i, o) = bench("build", "make", input, &output);
    assert_min("build", "make", i, o, 30.0);
}

#[test]
fn bench_make_error() {
    let input = include_str!("fixtures/build/make_error_raw.txt");
    let output = make_cmd::filter_make(input);
    let (i, o) = bench("build", "make (error)", input, &output);
    assert_min("build", "make (error)", i, o, 20.0);
}

// ── Lint ────────────────────────────────────────────────────────────

#[test]
fn bench_eslint() {
    let input = include_str!("fixtures/lint/eslint_raw.txt");
    let output = lint_cmd::filter_eslint(input);
    let (i, o) = bench("lint", "eslint", input, &output);
    assert_min("lint", "eslint", i, o, 30.0);
}

#[test]
fn bench_tsc() {
    let input = include_str!("fixtures/lint/tsc_raw.txt");
    let output = js_tsc_cmd::filter_tsc(input);
    let (i, o) = bench("lint", "tsc", input, &output);
    assert_min("lint", "tsc", i, o, 30.0);
}

#[test]
fn bench_ruff() {
    let input = include_str!("fixtures/lint/ruff_check_raw.txt");
    let output = ruff_cmd::filter_ruff(input);
    let (i, o) = bench("lint", "ruff check", input, &output);
    assert_min("lint", "ruff check", i, o, 30.0);
}

#[test]
fn bench_shellcheck() {
    let input = include_str!("fixtures/lint/shellcheck_raw.txt");
    let output = lint_cmd::filter_shellcheck(input);
    let (i, o) = bench("lint", "shellcheck", input, &output);
    assert_min("lint", "shellcheck", i, o, 20.0);
}

// ── Database ────────────────────────────────────────────────────────

#[test]
fn bench_psql() {
    let input = include_str!("fixtures/db/psql_raw.txt");
    let output = db_cmd::filter_psql(input);
    let (i, o) = bench("db", "psql", input, &output);
    assert_min("db", "psql", i, o, 20.0);
}

#[test]
fn bench_redis_info() {
    let input = include_str!("fixtures/db/redis_info_raw.txt");
    let output = db_cmd::filter_redis_info(input);
    let (i, o) = bench("db", "redis info", input, &output);
    assert_min("db", "redis info", i, o, 40.0);
}

// ── Aggregate summary ───────────────────────────────────────────────

#[test]
fn bench_summary() {
    println!();
    println!("  ---------------------------------------------------------------");
    println!(
        "  BENCH  {:<12} {:<24} {:>6}   {:>6}          {:>6}",
        "AREA", "COMMAND", "INPUT", "OUTPUT", "SAVED"
    );
    println!("  ---------------------------------------------------------------");

    let cases: Vec<(&str, &str, &str, String)> = vec![
        (
            "git",
            "log",
            include_str!("fixtures/git/log_raw.txt"),
            git_cmd::filter_git_log(include_str!("fixtures/git/log_raw.txt")),
        ),
        (
            "git",
            "status",
            include_str!("fixtures/git/status_raw.txt"),
            git_cmd::filter_git_status(include_str!("fixtures/git/status_raw.txt")),
        ),
        (
            "git",
            "diff",
            include_str!("fixtures/git/diff_raw.txt"),
            git_cmd::filter_git_diff(include_str!("fixtures/git/diff_raw.txt")),
        ),
        (
            "system",
            "ls",
            include_str!("fixtures/system/ls_raw.txt"),
            ls_cmd::filter_ls(include_str!("fixtures/system/ls_raw.txt"), "."),
        ),
        (
            "system",
            "find",
            include_str!("fixtures/system/find_raw.txt"),
            find_cmd::filter_find(include_str!("fixtures/system/find_raw.txt")),
        ),
        (
            "system",
            "grep",
            include_str!("fixtures/system/grep_raw.txt"),
            grep_cmd::filter_grep(include_str!("fixtures/system/grep_raw.txt"), 50, 5),
        ),
        (
            "rust",
            "cargo test",
            include_str!("fixtures/rust/cargo_test_raw.txt"),
            cargo_cmd::filter_cargo_test(include_str!("fixtures/rust/cargo_test_raw.txt")),
        ),
        (
            "rust",
            "cargo build",
            include_str!("fixtures/rust/cargo_build_raw.txt"),
            cargo_cmd::filter_cargo_build(include_str!("fixtures/rust/cargo_build_raw.txt")),
        ),
        (
            "rust",
            "cargo clippy",
            include_str!("fixtures/rust/cargo_clippy_raw.txt"),
            cargo_cmd::filter_cargo_clippy(include_str!("fixtures/rust/cargo_clippy_raw.txt")),
        ),
        (
            "python",
            "pytest",
            include_str!("fixtures/python/pytest_raw.txt"),
            pytest_cmd::filter_pytest(include_str!("fixtures/python/pytest_raw.txt")),
        ),
        (
            "go",
            "go test",
            include_str!("fixtures/go/go_test_raw.txt"),
            go_cmd::filter_go_test(include_str!("fixtures/go/go_test_raw.txt")),
        ),
        (
            "go",
            "go build",
            include_str!("fixtures/go/go_build_raw.txt"),
            go_cmd::filter_go_build(include_str!("fixtures/go/go_build_raw.txt")),
        ),
        (
            "js",
            "npm install",
            include_str!("fixtures/js/npm_install_raw.txt"),
            npm_cmd::filter_npm_install(include_str!("fixtures/js/npm_install_raw.txt")),
        ),
        (
            "js",
            "npm test",
            include_str!("fixtures/js/npm_test_raw.txt"),
            npm_cmd::filter_npm_test(include_str!("fixtures/js/npm_test_raw.txt")),
        ),
        (
            "js",
            "vitest",
            include_str!("fixtures/js/vitest_raw.txt"),
            vitest_cmd::filter_vitest(include_str!("fixtures/js/vitest_raw.txt")),
        ),
        (
            "js",
            "tsc",
            include_str!("fixtures/js/tsc_raw.txt"),
            js_tsc_cmd::filter_tsc(include_str!("fixtures/js/tsc_raw.txt")),
        ),
        (
            "cloud",
            "docker ps",
            include_str!("fixtures/cloud/docker_ps_raw.txt"),
            docker_cmd::filter_docker_ps(include_str!("fixtures/cloud/docker_ps_raw.txt")),
        ),
        (
            "cloud",
            "docker build",
            include_str!("fixtures/cloud/docker_build_raw.txt"),
            docker_cmd::filter_docker_build(include_str!("fixtures/cloud/docker_build_raw.txt")),
        ),
        (
            "cloud",
            "terraform plan",
            include_str!("fixtures/cloud/terraform_plan_raw.txt"),
            terraform_cmd::filter_terraform_plan(include_str!(
                "fixtures/cloud/terraform_plan_raw.txt"
            )),
        ),
        (
            "pkg",
            "brew install",
            include_str!("fixtures/pkg/brew_install_raw.txt"),
            brew_cmd::filter_brew_install(include_str!("fixtures/pkg/brew_install_raw.txt")),
        ),
        (
            "pkg",
            "brew list",
            include_str!("fixtures/pkg/brew_list_raw.txt"),
            brew_cmd::filter_brew_list(include_str!("fixtures/pkg/brew_list_raw.txt")),
        ),
        (
            "build",
            "make",
            include_str!("fixtures/build/make_raw.txt"),
            make_cmd::filter_make(include_str!("fixtures/build/make_raw.txt")),
        ),
        (
            "lint",
            "eslint",
            include_str!("fixtures/lint/eslint_raw.txt"),
            lint_cmd::filter_eslint(include_str!("fixtures/lint/eslint_raw.txt")),
        ),
        (
            "lint",
            "tsc",
            include_str!("fixtures/lint/tsc_raw.txt"),
            js_tsc_cmd::filter_tsc(include_str!("fixtures/lint/tsc_raw.txt")),
        ),
        (
            "lint",
            "ruff check",
            include_str!("fixtures/lint/ruff_check_raw.txt"),
            ruff_cmd::filter_ruff(include_str!("fixtures/lint/ruff_check_raw.txt")),
        ),
        (
            "lint",
            "shellcheck",
            include_str!("fixtures/lint/shellcheck_raw.txt"),
            lint_cmd::filter_shellcheck(include_str!("fixtures/lint/shellcheck_raw.txt")),
        ),
        (
            "db",
            "psql",
            include_str!("fixtures/db/psql_raw.txt"),
            db_cmd::filter_psql(include_str!("fixtures/db/psql_raw.txt")),
        ),
        (
            "db",
            "redis info",
            include_str!("fixtures/db/redis_info_raw.txt"),
            db_cmd::filter_redis_info(include_str!("fixtures/db/redis_info_raw.txt")),
        ),
    ];

    let mut total_in: usize = 0;
    let mut total_out: usize = 0;

    for (area, label, input, output) in &cases {
        let (i, o) = bench(area, label, input, output);
        total_in += i;
        total_out += o;
    }

    let total_pct = if total_in > 0 {
        100.0 - (total_out as f64 / total_in as f64 * 100.0)
    } else {
        0.0
    };
    println!("  ---------------------------------------------------------------");
    println!(
        "  BENCH  {:<12} {:<24} {:>6} → {:>6} tokens  ({:>5.1}% saved)",
        "TOTAL",
        format!("{} compressors", cases.len()),
        total_in,
        total_out,
        total_pct
    );
    println!("  ---------------------------------------------------------------");
}

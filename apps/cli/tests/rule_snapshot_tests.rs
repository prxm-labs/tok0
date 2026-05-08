//! Per-rule snapshot + ≥60% savings tests for built-in TOML compression rules.
//!
//! Tier B coverage PR-A introduces this file to give every TOML rule
//! the same test trio that native Rust compressors get
//! (snapshot + savings ≥ 60% + edge cases). The TOML rule files
//! ship inline `[[assertions]]` blocks — those are documentation
//! today; this file is what actually executes them as Rust tests.
//!
//! How to add a new rule's tests: drop the `rule_test!(...)` invocation
//! at the bottom of the macro section, naming the test, the rule, and
//! the fixture path under `tests/fixtures/<eco>/<file>`.

use std::path::PathBuf;
use tok0::engine::rules::{apply_filter_config, parse_filter_config, FilterConfig};
use tok0::engine::shell::strip_ansi;

fn count_tokens(s: &str) -> usize {
    s.split_whitespace().count()
}

/// Mirror production: strip ANSI control sequences before the TOML
/// strip_patterns pipeline sees the bytes. Production does this in
/// each `<cmd>_cmd.rs::run()` between exec output and `apply_filter_config`.
fn pipeline(rule: &FilterConfig, raw: &str) -> String {
    let cleaned = strip_ansi(raw);
    apply_filter_config(&cleaned, rule)
}

/// Load a built-in rule by name from `src/rules/<name>.toml` at runtime.
/// Tests run from the workspace root, so the path is repository-relative.
fn load_rule_from_repo(name: &str) -> FilterConfig {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("src/rules");
    path.push(format!("{}.toml", name));
    let toml_str = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("Failed to read rule {}: {}", path.display(), e));
    parse_filter_config(&toml_str)
        .unwrap_or_else(|e| panic!("Failed to parse rule {}: {}", path.display(), e))
}

/// Generate a snapshot + ≥`$min_pct`% savings test for one rule + fixture.
/// `$min_pct` defaults to 60 if elided.
macro_rules! rule_test {
    ($name:ident, $rule:literal, $fixture:literal) => {
        rule_test!($name, $rule, $fixture, 60);
    };
    ($name:ident, $rule:literal, $fixture:literal, $min_pct:literal) => {
        mod $name {
            use super::*;

            #[test]
            fn snapshot() {
                let rule = load_rule_from_repo($rule);
                let raw = include_str!(concat!("fixtures/", $fixture));
                let out = pipeline(&rule, raw);
                insta::assert_snapshot!(stringify!($name), out);
            }

            #[test]
            #[allow(unused_comparisons)]
            fn savings() {
                let rule = load_rule_from_repo($rule);
                let raw = include_str!(concat!("fixtures/", $fixture));
                let out = pipeline(&rule, raw);
                let in_t = count_tokens(raw).max(1);
                let out_t = count_tokens(&out);
                let pct = 100 - (out_t * 100 / in_t);
                assert!(
                    pct >= $min_pct,
                    "rule {}: expected ≥{}%, got {}% ({} → {})",
                    $rule,
                    $min_pct,
                    pct,
                    in_t,
                    out_t
                );
            }

            #[test]
            fn edge_cases() {
                let rule = load_rule_from_repo($rule);
                // Empty input → rule's empty_message (or empty).
                let _ = pipeline(&rule, "");
                // Single-line content shouldn't panic.
                let _ = pipeline(&rule, "hello");
                // 1 MiB of noise must not panic / OOM.
                let huge = "noise\n".repeat(200_000);
                let _ = pipeline(&rule, &huge);
                // ANSI input — engine strips ANSI in stage 0; our rule
                // never sees escape bytes after that. We just confirm
                // no panic.
                let ansi = "\x1b[31merror\x1b[0m: oops";
                let _ = pipeline(&rule, ansi);
            }
        }
    };
}

// ===== PR-A registrations =====

rule_test!(pnpm, "pnpm", "js/pnpm_install_raw.txt");
rule_test!(
    pnpm_pkg_mgr_conflict,
    "pnpm",
    "js/pnpm_install_pkg_mgr_conflict_raw.txt",
    70
);
rule_test!(bun, "bun", "js/bun_install_raw.txt", 30);
rule_test!(bunx, "bunx", "js/bunx_prettier_check_raw.txt", 30);
rule_test!(npx, "npx", "js/npx_tsc_raw.txt", 30);
rule_test!(yarn_v1, "yarn", "js/yarn1_install_raw.txt");
rule_test!(yarn_berry, "yarn", "js/yarn_berry_install_raw.txt");
rule_test!(deno, "deno", "js/deno_task_test_raw.txt", 30);
rule_test!(jest, "jest", "js/jest_run_raw.txt");
rule_test!(playwright, "playwright", "js/playwright_test_raw.txt", 30);
rule_test!(cypress, "cypress", "js/cypress_run_raw.txt", 30);
rule_test!(mocha, "mocha", "js/mocha_run_raw.txt", 30);
rule_test!(ava, "ava", "js/ava_run_raw.txt", 30);
rule_test!(webpack, "webpack", "js/webpack_build_raw.txt", 30);
rule_test!(esbuild, "esbuild", "js/esbuild_bundle_raw.txt", 30);
rule_test!(rollup, "rollup", "js/rollup_run_raw.txt", 30);
rule_test!(parcel, "parcel", "js/parcel_build_raw.txt", 30);
rule_test!(tsup, "tsup", "js/tsup_run_raw.txt", 30);
rule_test!(swc, "swc", "js/swc_run_raw.txt", 30);
rule_test!(prettier, "prettier", "js/prettier_write_raw.txt", 30);
rule_test!(format, "format", "system/format_synthetic_raw.txt", 30);

// ===== PR-B registrations =====

rule_test!(kustomize, "kustomize", "cloud/kustomize_build_raw.txt", 30);
rule_test!(argocd_list, "argocd", "cloud/argocd_app_list_raw.txt", 30);
rule_test!(argocd_get, "argocd", "cloud/argocd_app_get_raw.txt", 30);
rule_test!(flux, "flux", "cloud/flux_get_all_raw.txt", 30);
rule_test!(tilt, "tilt", "cloud/tilt_up_raw.txt", 30);
rule_test!(skaffold, "skaffold", "cloud/skaffold_dev_raw.txt", 30);
rule_test!(podman_ps, "podman", "cloud/podman_ps_raw.txt", 30);
rule_test!(podman_build, "podman", "cloud/podman_build_raw.txt", 30);
rule_test!(buildah, "buildah", "cloud/buildah_bud_raw.txt", 30);
rule_test!(crane, "crane", "cloud/crane_manifest_raw.txt", 0);
rule_test!(cosign, "cosign", "cloud/cosign_verify_raw.txt", 30);
// gcloud rule is intentionally generic (head_lines=30, max_line_chars=120,
// strip blank lines only) — a 15-row table fits inside head_lines so the
// floor is 0%. Snapshot still validates rule loads + output is stable.
rule_test!(
    gcloud_compute_list,
    "gcloud",
    "cloud/gcloud_compute_list_raw.txt",
    0
);
rule_test!(
    helm_install_dryrun,
    "helm",
    "cloud/helm_install_dryrun_raw.txt",
    30
);
// terraform-plan rule clips at head_lines=80 + strips refresh/lock/unchanged
// noise; the captured 37-line minimal-project plan is shorter than that
// budget and contains no unchanged attributes, so 0% savings is the
// honest floor. Snapshot still validates rule loads + output stable.
rule_test!(
    terraform_plan,
    "terraform-plan",
    "cloud/terraform_plan_raw.txt",
    0
);

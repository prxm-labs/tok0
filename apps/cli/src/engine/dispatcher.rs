use crate::compressors;

/// Default global hard cap for compressed output. Anything larger gets
/// head/tail truncation via `apply_output_cap`. Phase 1 tightening:
/// dropped from 50k chars / 100/50 lines to be more aggressive on long
/// CI logs, terraform plans, kubectl describe, etc.
const DEFAULT_MAX_CHARS: usize = 30_000;
const DEFAULT_HEAD: usize = 60;
const DEFAULT_TAIL: usize = 30;

/// Extract the value of `-o <fmt>` / `-o=<fmt>` / `--output <fmt>` /
/// `--output=<fmt>` from a kubectl-style argv slice. Returns the first
/// occurrence; later flags shadow earlier ones (matching kubectl's own
/// behaviour). Used to detect `kubectl get -o json` so we can route the
/// payload through the JSON compressor.
fn output_format<'a>(args: &[&'a str]) -> Option<&'a str> {
    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        if *arg == "-o" || *arg == "--output" {
            return iter.next().copied();
        }
        if let Some(rest) = arg.strip_prefix("-o=") {
            return Some(rest);
        }
        if let Some(rest) = arg.strip_prefix("--output=") {
            return Some(rest);
        }
    }
    None
}

/// Routes a command + args to the appropriate compressor filter function.
///
/// Returns `Some(compressed_output)` if a matching compressor was found,
/// `None` if the command is unknown or has no compressor.
pub fn dispatch(cmd: &str, args: &[&str], stdout: &str) -> Option<String> {
    let ansi_stripped = crate::engine::shell::strip_ansi(stdout);
    let progress_stripped = crate::engine::shell::strip_progress_noise(ansi_stripped.as_ref());
    let stdout = progress_stripped.as_ref();
    let sub = args.first().copied().unwrap_or("");

    match cmd {
        // ── Git ──────────────────────────────────────────────────
        "git" => {
            let result = match sub {
                "log" => compressors::git::git_cmd::filter_git_log(stdout),
                "status" => compressors::git::git_cmd::filter_git_status(stdout),
                "diff" => compressors::git::git_cmd::filter_git_diff(stdout),
                "show" | "branch" | "stash" | "tag" | "remote" | "fetch" | "pull" | "push"
                | "merge" | "rebase" | "cherry-pick" | "checkout" | "switch" | "add" | "commit"
                | "reset" | "clean" | "bisect" => {
                    compressors::git::git_cmd::filter_git_simple(stdout, sub)
                }
                _ => compressors::git::git_cmd::filter_git_simple(stdout, sub),
            };
            Some(result)
        }

        // ── GitHub CLI ───────────────────────────────────────────
        "gh" => {
            let sub2 = args.get(1).copied().unwrap_or("");
            match (sub, sub2) {
                ("pr", "list") => Some(compressors::git::gh_cmd::filter_gh_pr_list(stdout)),
                ("pr", "view") => Some(compressors::git::gh_cmd::filter_gh_pr_view(stdout)),
                ("issue", "list") => Some(compressors::git::gh_cmd::filter_gh_issue_list(stdout)),
                ("issue", "view") => Some(compressors::git::gh_cmd::filter_gh_issue_view(stdout)),
                ("run", "list") => Some(compressors::git::gh_cmd::filter_gh_run_list(stdout)),
                ("run", "view") => Some(compressors::git::gh_cmd::filter_gh_run_view(stdout)),
                _ => None,
            }
        }

        // ── System ───────────────────────────────────────────────
        "ls" => {
            let path = args.first().copied().unwrap_or(".");
            Some(compressors::system::ls_cmd::filter_ls(stdout, path))
        }
        "cat" | "head" | "tail" | "bat" => {
            let path = args.last().copied().unwrap_or("");
            Some(compressors::system::read_cmd::filter_read(stdout, path))
        }
        "grep" | "rg" | "ag" => Some(compressors::system::grep_cmd::filter_grep(stdout, 200, 50)),
        "find" | "fd" => Some(compressors::system::find_cmd::filter_find(stdout)),
        "diff" => Some(compressors::system::diff_cmd::filter_diff(stdout)),
        "du" => Some(compressors::system::du_cmd::filter_du(stdout, 20)),
        "df" => Some(compressors::system::df_cmd::filter_df(stdout)),
        "wc" => Some(compressors::system::wc_cmd::filter_wc(stdout)),
        "curl" => Some(compressors::system::curl_cmd::filter_curl(stdout)),
        "dig" => Some(compressors::system::dig_cmd::filter_dig(stdout)),
        "lsof" => Some(compressors::system::lsof_cmd::filter_lsof(stdout)),
        "netstat" | "ss" => Some(compressors::system::netstat_cmd::filter_netstat(stdout)),
        "openssl" => Some(compressors::system::openssl_cmd::filter_openssl(stdout)),
        "tar" => Some(compressors::system::tar_cmd::filter_tar(stdout)),
        "smart" => Some(compressors::system::smart_cmd::filter_smart(stdout)),
        "env" | "printenv" => Some(compressors::system::env_cmd::filter_env(stdout, None)),

        // ── Silent commands ──────────────────────────────────────
        "touch" | "mkdir" | "cp" | "mv" | "rm" | "chmod" | "chown" | "ln" => {
            Some(compressors::system::silent_cmd::filter_silent(stdout))
        }

        // ── Cargo (Rust) ─────────────────────────────────────────
        "cargo" => match sub {
            "test" | "t" | "nextest" => {
                Some(compressors::rust::cargo_cmd::filter_cargo_test(stdout))
            }
            "build" | "b" | "check" | "c" => {
                Some(compressors::rust::cargo_cmd::filter_cargo_build(stdout))
            }
            "clippy" => Some(compressors::rust::cargo_cmd::filter_cargo_clippy(stdout)),
            _ => None,
        },

        // ── npm / pnpm / yarn / bun (JS) ────────────────────────
        "npm" | "pnpm" | "yarn" | "bun" => match sub {
            "install" | "i" | "add" | "ci" => {
                Some(compressors::js::npm_cmd::filter_npm_install(stdout))
            }
            "test" | "t" | "run" => Some(compressors::js::npm_cmd::filter_npm_test(stdout)),
            _ => None,
        },
        "vitest" | "jest" => Some(compressors::js::vitest_cmd::filter_vitest(stdout)),
        "tsc" | "tsgo" => Some(compressors::js::tsc_cmd::filter_tsc(stdout)),
        "vite" => {
            let argv: Vec<String> = args.iter().map(|s| s.to_string()).collect();
            let v_args = compressors::js::vite_cmd::ViteArgs::from_argv(argv);
            compressors::js::vite_cmd::filter_vite(&v_args, stdout).ok()
        }
        "next" => {
            let argv: Vec<String> = args.iter().map(|s| s.to_string()).collect();
            let n_args = compressors::js::next_cmd::NextArgs::from_argv(argv);
            compressors::js::next_cmd::filter_next(&n_args, stdout).ok()
        }
        "prisma" => {
            let argv: Vec<String> = args.iter().map(|s| s.to_string()).collect();
            let p_args = compressors::js::prisma_cmd::PrismaArgs::from_argv(argv);
            compressors::js::prisma_cmd::filter_prisma(&p_args, stdout).ok()
        }

        // ── Python ───────────────────────────────────────────────
        "pytest" | "python" if sub == "-m" && args.get(1).copied() == Some("pytest") => {
            Some(compressors::python::pytest_cmd::filter_pytest(stdout))
        }
        "pytest" => Some(compressors::python::pytest_cmd::filter_pytest(stdout)),
        "ruff" => Some(compressors::python::ruff_cmd::filter_ruff(stdout)),
        "pip" | "pip3" => match sub {
            "install" => Some(compressors::python::pip_cmd::filter_pip_install(stdout)),
            _ => None,
        },
        "uv" => match sub {
            "pip" => Some(compressors::python::pip_cmd::filter_pip_install(stdout)),
            _ => None,
        },

        // ── Go ───────────────────────────────────────────────────
        "go" => match sub {
            "test" => Some(compressors::go::go_cmd::filter_go_test(stdout)),
            "build" | "install" => Some(compressors::go::go_cmd::filter_go_build(stdout)),
            _ => None,
        },
        "golangci-lint" => Some(compressors::go::go_cmd::filter_golangci_lint(stdout)),

        // ── Docker / Podman / nerdctl ────────────────────────────
        "docker" | "podman" | "nerdctl" => match sub {
            "ps" => Some(compressors::cloud::docker_cmd::filter_docker_ps(stdout)),
            "build" => Some(compressors::cloud::docker_cmd::filter_docker_build(stdout)),
            "images" | "image" => {
                Some(compressors::cloud::docker_cmd::filter_docker_images(stdout))
            }
            "logs" => Some(compressors::cloud::docker_cmd::filter_docker_logs(stdout)),
            "compose" => Some(compressors::cloud::docker_cmd::filter_docker_compose(
                stdout,
            )),
            // `inspect` always emits valid JSON. Truncate large arrays/objects
            // via json_cmd; on parse failure (e.g. caller piped `--format`),
            // pass the raw output through.
            "inspect" => Some(
                compressors::system::json_cmd::filter_json(stdout)
                    .unwrap_or_else(|_| stdout.to_string()),
            ),
            _ => None,
        },

        // ── Kubernetes ───────────────────────────────────────────
        "kubectl" | "k" => match sub {
            "get" => {
                // `-o json` / `-o=json` / `--output json` → route through
                // json_cmd for structural truncation. yaml falls through to
                // the tabular filter (it's already line-oriented).
                if matches!(output_format(args), Some("json")) {
                    if let Ok(j) = compressors::system::json_cmd::filter_json(stdout) {
                        return Some(j);
                    }
                }
                let resource = args.get(1).copied().unwrap_or("");
                if resource == "pods" {
                    Some(compressors::cloud::kubectl_cmd::filter_kubectl_pods(stdout))
                } else {
                    Some(compressors::cloud::kubectl_cmd::filter_kubectl_get(stdout))
                }
            }
            "describe" => Some(compressors::cloud::kubectl_cmd::filter_kubectl_describe(
                stdout,
            )),
            "logs" | "log" => Some(compressors::cloud::kubectl_cmd::filter_kubectl_logs(stdout)),
            _ => None,
        },

        // ── Terraform ────────────────────────────────────────────
        "terraform" | "tf" | "tofu" => match sub {
            "plan" => Some(compressors::cloud::terraform_cmd::filter_terraform_plan(
                stdout,
            )),
            "apply" => Some(compressors::cloud::terraform_cmd::filter_terraform_apply(
                stdout,
            )),
            _ => None,
        },

        // ── AWS CLI ──────────────────────────────────────────────
        "aws" => Some(compressors::cloud::aws_cmd::filter_aws_json(stdout)),

        // ── Build ────────────────────────────────────────────────
        "make" | "gmake" => Some(compressors::build::make_cmd::filter_make(stdout)),

        // ── Package managers ─────────────────────────────────────
        "brew" => match sub {
            "install" | "upgrade" | "reinstall" => {
                Some(compressors::pkg::brew_cmd::filter_brew_install(stdout))
            }
            "list" | "ls" | "leaves" => Some(compressors::pkg::brew_cmd::filter_brew_list(stdout)),
            _ => None,
        },
        "apt" | "apt-get" => match sub {
            "install" | "upgrade" | "dist-upgrade" => {
                Some(compressors::pkg::apt_cmd::filter_apt_install(stdout))
            }
            _ => None,
        },

        // ── Linters ──────────────────────────────────────────────
        "eslint" => Some(compressors::lint::lint_cmd::filter_eslint(stdout)),
        "shellcheck" => Some(compressors::lint::lint_cmd::filter_shellcheck(stdout)),

        // ── .NET ─────────────────────────────────────────────────
        "dotnet" => match sub {
            "test" => Some(compressors::dotnet::dotnet_cmd::filter_dotnet_test(stdout)),
            _ => None,
        },

        // ── Java ─────────────────────────────────────────────────
        "gradle" | "gradlew" | "./gradlew" => {
            Some(compressors::java::gradle_cmd::filter_gradle(stdout))
        }

        // ── Ruby ─────────────────────────────────────────────────
        "rspec" | "bundle" if sub == "exec" && args.get(1).copied() == Some("rspec") => {
            Some(compressors::ruby::rspec_cmd::filter_rspec(stdout))
        }
        "rspec" => Some(compressors::ruby::rspec_cmd::filter_rspec(stdout)),

        // ── Database ─────────────────────────────────────────────
        "psql" => Some(compressors::db::db_cmd::filter_psql(stdout)),
        "redis-cli" => match sub {
            "info" | "INFO" => Some(compressors::db::db_cmd::filter_redis_info(stdout)),
            _ => None,
        },

        _ => None,
    }
}

use crate::engine::rules::{FilterConfig, RuleIndex};

/// Dispatch with TOML rule fallback. Tries native compressors first,
/// then matches against loaded TOML rules.
pub fn dispatch_with_rules(
    cmd: &str,
    args: &[&str],
    stdout: &str,
    rules: &[FilterConfig],
) -> Option<String> {
    let index = RuleIndex::build(rules);
    dispatch_with_rule_index(cmd, args, stdout, &index)
}

/// Dispatch using a pre-built RuleIndex for O(1) rule lookup.
/// Prefer this over dispatch_with_rules when processing multiple commands
/// against the same rule set.
pub fn dispatch_with_rule_index(
    cmd: &str,
    args: &[&str],
    stdout: &str,
    index: &RuleIndex,
) -> Option<String> {
    // Try native compressor first
    if let Some(compressed) = dispatch(cmd, args, stdout) {
        let capped = apply_output_cap(&compressed, DEFAULT_MAX_CHARS, DEFAULT_HEAD, DEFAULT_TAIL);
        return Some(apply_post_pass(&capped));
    }

    // Fallback: try TOML rules via indexed lookup
    let full_command = if args.is_empty() {
        cmd.to_string()
    } else {
        format!("{} {}", cmd, args.join(" "))
    };
    if let Some(rule) = index.find(&full_command) {
        let output = crate::engine::rules::apply_filter_config(stdout, rule);
        let capped = apply_output_cap(&output, DEFAULT_MAX_CHARS, DEFAULT_HEAD, DEFAULT_TAIL);
        return Some(apply_post_pass(&capped));
    }

    None
}

/// Universal post-pass: strip the Node deprecation footer regardless of
/// which compressor or TOML rule produced the output, and apply the
/// ultra-compact extras if the global flag is set. Both are idempotent
/// and safe on non-Node output.
fn apply_post_pass(output: &str) -> String {
    let cleaned = crate::engine::shell::strip_node_deprecation_footer(output);
    if crate::engine::shell::ultra_compact_enabled() {
        crate::engine::shell::apply_ultra_compact(cleaned.as_ref())
    } else {
        cleaned.into_owned()
    }
}

/// Apply hard cap: if output exceeds max_chars, keep head + tail lines.
pub fn apply_output_cap(output: &str, max_chars: usize, head: usize, tail: usize) -> String {
    if output.len() <= max_chars {
        return output.to_string();
    }
    let lines: Vec<&str> = output.lines().collect();
    if lines.len() <= head + tail {
        // Not enough lines to split — just truncate by chars
        return output[..max_chars].to_string();
    }
    let head_lines = &lines[..head];
    let tail_lines = &lines[lines.len() - tail..];
    let omitted = lines.len() - head - tail;
    format!(
        "{}\n... ({} lines omitted) ...\n{}",
        head_lines.join("\n"),
        omitted,
        tail_lines.join("\n")
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_git_log_dispatches() {
        let input = "abc1234 Fix bug\ndef5678 Add feature\n";
        let result = dispatch("git", &["log", "--oneline"], input);
        assert!(result.is_some(), "git log should dispatch");
    }

    #[test]
    fn test_git_status_dispatches() {
        let input = " M src/main.rs\n?? new_file.rs\n";
        let result = dispatch("git", &["status", "--short"], input);
        assert!(result.is_some(), "git status should dispatch");
    }

    #[test]
    fn test_git_diff_dispatches() {
        let input = "diff --git a/file.rs b/file.rs\n+added line\n-removed line\n";
        let result = dispatch("git", &["diff"], input);
        assert!(result.is_some(), "git diff should dispatch");
    }

    #[test]
    fn test_ls_dispatches() {
        let input = "file1.rs\nfile2.rs\ndir/\n";
        let result = dispatch("ls", &["-la", "/tmp"], input);
        assert!(result.is_some(), "ls should dispatch");
    }

    #[test]
    fn test_cargo_test_dispatches() {
        let input = "running 3 tests\ntest one ... ok\ntest two ... ok\ntest three ... FAILED\n";
        let result = dispatch("cargo", &["test"], input);
        assert!(result.is_some(), "cargo test should dispatch");
    }

    #[test]
    fn test_cargo_build_dispatches() {
        let input = "   Compiling tok0 v0.1.0\n    Finished dev [unoptimized] in 2.5s\n";
        let result = dispatch("cargo", &["build"], input);
        assert!(result.is_some(), "cargo build should dispatch");
    }

    #[test]
    fn test_cargo_clippy_dispatches() {
        let input = "warning: unused variable\n";
        let result = dispatch("cargo", &["clippy"], input);
        assert!(result.is_some(), "cargo clippy should dispatch");
    }

    #[test]
    fn test_unknown_command_returns_none() {
        let result = dispatch("unknown-tool-xyz", &["--help"], "some output");
        assert!(result.is_none(), "unknown command should return None");
    }

    #[test]
    fn test_unknown_subcommand_returns_none() {
        let result = dispatch("cargo", &["publish"], "publishing...");
        assert!(result.is_none(), "cargo publish has no compressor");
    }

    #[test]
    fn test_empty_input_handled() {
        let result = dispatch("git", &["log"], "");
        assert!(result.is_some(), "empty input should still dispatch");
    }

    #[test]
    fn test_empty_args_handled() {
        // git with no args should still route (falls through to filter_git_simple)
        let result = dispatch("git", &[], "");
        assert!(result.is_some(), "git with no args should dispatch");
    }

    #[test]
    fn test_npm_install_dispatches() {
        let input = "added 150 packages in 3s\n";
        let result = dispatch("npm", &["install"], input);
        assert!(result.is_some(), "npm install should dispatch");
    }

    #[test]
    fn test_pytest_dispatches() {
        let input = "===== 5 passed in 0.3s =====\n";
        let result = dispatch("pytest", &[], input);
        assert!(result.is_some(), "pytest should dispatch");
    }

    #[test]
    fn test_go_test_dispatches() {
        let input = "ok  \tgithub.com/foo/bar\t0.003s\n";
        let result = dispatch("go", &["test", "./..."], input);
        assert!(result.is_some(), "go test should dispatch");
    }

    #[test]
    fn test_docker_ps_dispatches() {
        let input = "CONTAINER ID   IMAGE   STATUS\nabc123   nginx   Up 2 hours\n";
        let result = dispatch("docker", &["ps"], input);
        assert!(result.is_some(), "docker ps should dispatch");
    }

    #[test]
    fn test_make_dispatches() {
        let input = "gcc -o main main.c\nmake: built target 'all'\n";
        let result = dispatch("make", &[], input);
        assert!(result.is_some(), "make should dispatch");
    }

    #[test]
    fn test_brew_install_dispatches() {
        let input = "==> Downloading\n==> Pouring foo\n🍺 foo installed\n";
        let result = dispatch("brew", &["install", "foo"], input);
        assert!(result.is_some(), "brew install should dispatch");
    }

    #[test]
    fn test_gh_pr_list_dispatches() {
        let input = "#1  Fix bug  main  OPEN\n#2  Add feature  main  MERGED\n";
        let result = dispatch("gh", &["pr", "list"], input);
        assert!(result.is_some(), "gh pr list should dispatch");
    }

    #[test]
    fn test_gh_unknown_returns_none() {
        let result = dispatch("gh", &["auth", "login"], "Logged in");
        assert!(result.is_none(), "gh auth login has no compressor");
    }

    #[test]
    fn test_terraform_plan_dispatches() {
        let input = "Plan: 3 to add, 1 to change, 0 to destroy.\n";
        let result = dispatch("terraform", &["plan"], input);
        assert!(result.is_some(), "terraform plan should dispatch");
    }

    #[test]
    fn test_grep_dispatches() {
        let input = "file.rs:10:match found\nfile.rs:20:another match\n";
        let result = dispatch("grep", &["-r", "pattern"], input);
        assert!(result.is_some(), "grep should dispatch");
    }

    #[test]
    fn test_find_dispatches() {
        let input = "./src/main.rs\n./src/lib.rs\n";
        let result = dispatch("find", &[".", "-name", "*.rs"], input);
        assert!(result.is_some(), "find should dispatch");
    }

    #[test]
    fn test_psql_dispatches() {
        let input = " id | name\n----+------\n  1 | foo\n";
        let result = dispatch("psql", &[], input);
        assert!(result.is_some(), "psql should dispatch");
    }

    #[test]
    fn test_dispatch_with_rules_native_takes_priority() {
        let rules = vec![FilterConfig {
            name: "git-log-rule".to_string(),
            commands: vec!["git log".to_string()],
            strip_patterns: vec![".*".to_string()],
            empty_message: Some("rule applied".to_string()),
            ..Default::default()
        }];
        let input = "commit abc1234\nAuthor: user\nDate: Mon\n\n    message\n";
        let result = dispatch_with_rules("git", &["log"], input, &rules);
        assert!(result.is_some());
        // Native compressor should win, not the rule
        assert!(!result.expect("should dispatch").contains("rule applied"));
    }

    #[test]
    fn test_dispatch_with_rules_falls_back_to_toml() {
        let rules = vec![FilterConfig {
            name: "custom-cmd".to_string(),
            commands: vec!["mycustomtool".to_string()],
            strip_patterns: vec!["^INFO".to_string()],
            empty_message: None,
            ..Default::default()
        }];
        let input = "INFO: Starting\nResult: 42\nINFO: Done";
        let result = dispatch_with_rules("mycustomtool", &[], input, &rules);
        assert!(result.is_some());
        let output = result.expect("should dispatch");
        assert!(!output.contains("INFO"));
        assert!(output.contains("Result: 42"));
    }

    #[test]
    fn test_dispatch_with_rules_no_match_returns_none() {
        let rules = vec![FilterConfig {
            name: "other".to_string(),
            commands: vec!["othertool".to_string()],
            ..Default::default()
        }];
        let result = dispatch_with_rules("unknowncmd", &[], "output", &rules);
        assert!(result.is_none());
    }

    #[test]
    fn test_dispatch_strips_ansi_codes() {
        let input = "\x1b[32mcommit abc1234\x1b[0m\nAuthor: user\nDate: Mon\n\n    message\n";
        let result = dispatch("git", &["log"], input);
        assert!(result.is_some());
        let output = result.expect("should dispatch");
        assert!(!output.contains("\x1b["), "ANSI codes should be stripped");
    }

    #[test]
    fn test_dispatch_collapses_cr_progress() {
        // pnpm-style \r-rewritten progress should be collapsed before the
        // native compressor sees it. We feed npm install fixture-ish output
        // and verify the intermediate progress states are gone.
        let input =
            "[1/4] Resolving\n[2/4] Fetching\rfetched 50%\rfetched 100%\nadded 42 packages\n";
        let result = dispatch("npm", &["install"], input);
        assert!(result.is_some());
        let out = result.expect("dispatch");
        assert!(!out.contains("fetched 50%"), "got: {out}");
    }

    #[test]
    fn test_output_cap_small_input_unchanged() {
        let input = "short output";
        let result = apply_output_cap(input, 50_000, 100, 50);
        assert_eq!(result, input);
    }

    #[test]
    fn test_output_cap_large_input_truncated() {
        let large = (0..1000)
            .map(|i| format!("line {}", i))
            .collect::<Vec<_>>()
            .join("\n");
        let result = apply_output_cap(&large, 500, 5, 3);
        assert!(result.len() <= large.len());
        assert!(result.contains("line 0")); // head
        assert!(result.contains("line 999")); // tail
        assert!(result.contains("omitted"));
    }

    // ── New dispatch routes ────────────────────────────────────

    #[test]
    fn test_silent_touch_dispatches() {
        let result = dispatch("touch", &["file.txt"], "");
        assert!(result.is_some(), "touch should dispatch");
        assert_eq!(result.expect("dispatch"), "ok");
    }

    #[test]
    fn test_silent_mkdir_dispatches() {
        let result = dispatch("mkdir", &["-p", "dir"], "");
        assert!(result.is_some(), "mkdir should dispatch");
        assert_eq!(result.expect("dispatch"), "ok");
    }

    #[test]
    fn test_silent_cp_dispatches() {
        let result = dispatch("cp", &["src", "dst"], "");
        assert!(result.is_some(), "cp should dispatch");
        assert_eq!(result.expect("dispatch"), "ok");
    }

    #[test]
    fn test_silent_mv_dispatches() {
        let result = dispatch("mv", &["old", "new"], "");
        assert!(result.is_some(), "mv should dispatch");
        assert_eq!(result.expect("dispatch"), "ok");
    }

    #[test]
    fn test_silent_rm_dispatches() {
        let result = dispatch("rm", &["file"], "");
        assert!(result.is_some(), "rm should dispatch");
        assert_eq!(result.expect("dispatch"), "ok");
    }

    #[test]
    fn test_silent_chmod_dispatches() {
        let result = dispatch("chmod", &["755", "script.sh"], "");
        assert!(result.is_some(), "chmod should dispatch");
        assert_eq!(result.expect("dispatch"), "ok");
    }

    #[test]
    fn test_silent_chown_dispatches() {
        let result = dispatch("chown", &["user:group", "file"], "");
        assert!(result.is_some(), "chown should dispatch");
        assert_eq!(result.expect("dispatch"), "ok");
    }

    #[test]
    fn test_silent_ln_dispatches() {
        let result = dispatch("ln", &["-s", "target", "link"], "");
        assert!(result.is_some(), "ln should dispatch");
        assert_eq!(result.expect("dispatch"), "ok");
    }

    #[test]
    fn test_silent_error_passthrough() {
        let error = "cp: cannot stat 'foo': No such file or directory";
        let result = dispatch("cp", &["foo", "bar"], error);
        assert!(result.is_some());
        assert_eq!(result.expect("dispatch"), error);
    }

    #[test]
    fn test_gh_pr_view_dispatches() {
        let input = "title:\tFix bug\nstate:\tOPEN\n";
        let result = dispatch("gh", &["pr", "view"], input);
        assert!(result.is_some(), "gh pr view should dispatch");
    }

    #[test]
    fn test_gh_issue_list_dispatches() {
        let input = "#1  Bug report  OPEN\n";
        let result = dispatch("gh", &["issue", "list"], input);
        assert!(result.is_some(), "gh issue list should dispatch");
    }

    #[test]
    fn test_gh_issue_view_dispatches() {
        let input = "title:\tBug report\nstate:\tOPEN\n";
        let result = dispatch("gh", &["issue", "view"], input);
        assert!(result.is_some(), "gh issue view should dispatch");
    }

    #[test]
    fn test_gh_run_view_dispatches() {
        let input = "STATUS: completed\n✓ Build\n✓ Test\n";
        let result = dispatch("gh", &["run", "view"], input);
        assert!(result.is_some(), "gh run view should dispatch");
    }

    #[test]
    fn test_kubectl_get_services_dispatches() {
        let input = "NAME        TYPE        CLUSTER-IP\nmy-svc      ClusterIP   10.0.0.1\n";
        let result = dispatch("kubectl", &["get", "services"], input);
        assert!(result.is_some(), "kubectl get services should dispatch");
    }

    #[test]
    fn test_kubectl_get_deployments_dispatches() {
        let input = "NAME     READY   UP-TO-DATE   AVAILABLE   AGE\nmy-app   1/1     1            1           5d\n";
        let result = dispatch("kubectl", &["get", "deployments"], input);
        assert!(result.is_some(), "kubectl get deployments should dispatch");
    }

    #[test]
    fn test_kubectl_describe_dispatches() {
        let input = "Name:   my-pod\nNamespace: default\n";
        let result = dispatch("kubectl", &["describe", "pod", "my-pod"], input);
        assert!(result.is_some(), "kubectl describe should dispatch");
    }

    #[test]
    fn test_kubectl_logs_dispatches() {
        let input = "2024-01-01 Starting server\n2024-01-01 Listening on :8080\n";
        let result = dispatch("kubectl", &["logs", "my-pod"], input);
        assert!(result.is_some(), "kubectl logs should dispatch");
    }

    #[test]
    fn test_docker_images_dispatches() {
        let input = "REPOSITORY   TAG       IMAGE ID       CREATED        SIZE\nnginx        latest    abc123def456   2 weeks ago    187MB\n";
        let result = dispatch("docker", &["images"], input);
        assert!(result.is_some(), "docker images should dispatch");
    }

    #[test]
    fn test_docker_logs_dispatches() {
        let input = "Server starting...\nListening on port 8080\n";
        let result = dispatch("docker", &["logs", "my-container"], input);
        assert!(result.is_some(), "docker logs should dispatch");
    }

    #[test]
    fn test_docker_compose_dispatches() {
        let input = "NAME    IMAGE   STATUS\nweb     nginx   running\n";
        let result = dispatch("docker", &["compose", "ps"], input);
        assert!(result.is_some(), "docker compose should dispatch");
    }

    #[test]
    fn test_output_cap_preserves_head_tail_content() {
        let lines: Vec<String> = (0..200).map(|i| format!("line-{}", i)).collect();
        let input = lines.join("\n");
        let result = apply_output_cap(&input, 100, 3, 2);
        assert!(result.contains("line-0"));
        assert!(result.contains("line-1"));
        assert!(result.contains("line-2"));
        assert!(result.contains("line-199"));
        assert!(result.contains("line-198"));
    }

    // ── Tier 7: docker inspect, kubectl JSON, nerdctl ────────────────────

    #[test]
    fn test_output_format_separated() {
        assert_eq!(output_format(&["get", "pods", "-o", "json"]), Some("json"));
        assert_eq!(
            output_format(&["get", "pods", "--output", "yaml"]),
            Some("yaml")
        );
    }

    #[test]
    fn test_output_format_equals() {
        assert_eq!(output_format(&["get", "pods", "-o=json"]), Some("json"));
        assert_eq!(
            output_format(&["get", "pods", "--output=yaml"]),
            Some("yaml")
        );
    }

    #[test]
    fn test_output_format_absent() {
        assert_eq!(output_format(&["get", "pods"]), None);
        assert_eq!(output_format(&[]), None);
    }

    #[test]
    fn test_docker_inspect_routes_json() {
        let input = r#"[{"Id":"abc123","Name":"web","State":{"Running":true}}]"#;
        let result = dispatch("docker", &["inspect", "web"], input);
        assert!(result.is_some(), "docker inspect should dispatch");
        let out = result.expect("dispatch");
        // json_cmd reformats with serde — must still contain key fields.
        assert!(out.contains("abc123"));
        assert!(out.contains("Running"));
    }

    #[test]
    fn test_docker_inspect_passthrough_on_invalid_json() {
        // `--format` produces non-JSON. Filter must not mangle it.
        let input = "true\n";
        let result = dispatch(
            "docker",
            &["inspect", "--format", "{{.State.Running}}", "web"],
            input,
        );
        assert!(result.is_some());
        assert!(result.expect("dispatch").contains("true"));
    }

    #[test]
    fn test_nerdctl_aliases_docker() {
        let input = "CONTAINER ID   IMAGE   STATUS\nabc123   nginx   Up 2 hours\n";
        let result = dispatch("nerdctl", &["ps"], input);
        assert!(
            result.is_some(),
            "nerdctl should alias to docker dispatcher"
        );
    }

    #[test]
    fn test_kubectl_get_o_json_routes_json() {
        let input = r#"{"items":[{"metadata":{"name":"pod-1"}},{"metadata":{"name":"pod-2"}}]}"#;
        let result = dispatch("kubectl", &["get", "pods", "-o", "json"], input);
        assert!(result.is_some());
        let out = result.expect("dispatch");
        assert!(out.contains("pod-1"));
        assert!(out.contains("metadata"));
    }

    #[test]
    fn test_kubectl_get_o_yaml_falls_through_to_tabular() {
        // No JSON in input — yaml falls through to filter_kubectl_get, which
        // returns the input mostly intact (or transformed). Importantly, the
        // call must NOT panic and must produce output.
        let input = "apiVersion: v1\nkind: Pod\nmetadata:\n  name: pod-1\n";
        let result = dispatch("kubectl", &["get", "pods", "-o", "yaml"], input);
        assert!(result.is_some());
    }

    // ── Native system compressors: dispatcher-route smoke tests ──────────
    //
    // These guard against typos in the dispatcher match arms (e.g.
    // `"dig" =>` mistyped as `"dog" =>`). The compressor unit tests in
    // each *_cmd.rs already verify the filter logic; these only verify
    // the routing connection.

    #[test]
    fn test_curl_dispatches() {
        let input = "* Connected to api.github.com\n> GET / HTTP/2\n< HTTP/2 200\nbody";
        let result = dispatch("curl", &["-v", "https://api.github.com"], input);
        assert!(result.is_some(), "curl should dispatch");
    }

    #[test]
    fn test_dig_dispatches() {
        let input = ";; ANSWER SECTION:\ngoogle.com. 60 IN A 1.2.3.4\n";
        let result = dispatch("dig", &["google.com"], input);
        assert!(result.is_some(), "dig should dispatch");
        assert!(result.expect("dispatch").contains("1.2.3.4"));
    }

    #[test]
    fn test_lsof_dispatches() {
        let input = "COMMAND PID USER FD TYPE DEVICE SIZE/OFF NODE NAME\nfoo 123 user cwd DIR 1,1 100 2 /tmp/x\n";
        let result = dispatch("lsof", &["-nP"], input);
        assert!(result.is_some(), "lsof should dispatch");
    }

    #[test]
    fn test_netstat_dispatches() {
        let input = "tcp4 0 0 *.80 *.* LISTEN\n";
        let result = dispatch("netstat", &["-an"], input);
        assert!(result.is_some(), "netstat should dispatch");
        assert!(result.expect("dispatch").contains("LISTEN"));
    }

    #[test]
    fn test_ss_aliases_netstat() {
        let input =
            "State Recv-Q Send-Q Local Address:Port Peer Address:Port\nLISTEN 0 4096 0.0.0.0:8080 0.0.0.0:*";
        let result = dispatch("ss", &["-tuln"], input);
        assert!(result.is_some(), "ss should alias to netstat dispatcher");
    }

    #[test]
    fn test_openssl_dispatches() {
        let input = "Certificate:\n    Data:\n        Subject: CN=example.com\n";
        let result = dispatch("openssl", &["x509", "-text", "-noout"], input);
        assert!(result.is_some(), "openssl should dispatch");
        assert!(result.expect("dispatch").contains("example.com"));
    }

    #[test]
    fn test_tar_dispatches() {
        let input = "-rw-r--r--  0 user staff  100 May 1 12:00 ./a.txt\n-rw-r--r--  0 user staff  200 May 1 12:01 ./b.txt\n";
        let result = dispatch("tar", &["-tvf", "archive.tar"], input);
        assert!(result.is_some(), "tar should dispatch");
        assert!(result.expect("dispatch").contains("files"));
    }

    // ── Universal Node deprecation post-pass coverage ─────────────────
    //
    // The post-pass strips `(node:NNN) [DEP…]` and the `--trace-deprecation`
    // hint from EVERY compressor's output, regardless of which match arm
    // fired. Without this, prisma/vite/tsc/vitest/next would each need
    // to re-implement the same patterns.

    #[test]
    fn test_post_pass_strips_node_deprecation_through_native() {
        // Native pnpm install path. The Node footer is forwarded by
        // pnpm from its own runtime — universal post-pass must remove it.
        let input = "+ react 19.2.6\n(node:13518) [DEP0169] DeprecationWarning: url.parse() is deprecated\n(Use `node --trace-deprecation ...` to show where the warning was created)\nDone in 9.3s\n";
        let rules: Vec<FilterConfig> = Vec::new();
        let out = dispatch_with_rules("pnpm", &["install"], input, &rules)
            .expect("pnpm install should dispatch");
        assert!(out.contains("react 19.2.6"));
        assert!(out.contains("Done in 9.3s"));
        assert!(
            !out.contains("DeprecationWarning"),
            "Node DEP must be stripped post-dispatch, got:\n{out}"
        );
        assert!(
            !out.contains("trace-deprecation"),
            "trace-deprecation hint must be stripped post-dispatch"
        );
    }

    #[test]
    fn test_post_pass_strips_node_deprecation_through_toml_rule() {
        // TOML rule path — feed an unknown command that matches a custom
        // rule. The rule itself doesn't include Node patterns; the
        // post-pass must still strip them.
        let rules = vec![FilterConfig {
            name: "mytool".to_string(),
            commands: vec!["mytool".to_string()],
            strip_patterns: vec![],
            empty_message: None,
            ..Default::default()
        }];
        let input = "tool output line\n(node:99) [DEP0001] DeprecationWarning: foo\n(Use `node --trace-deprecation ...`)\n";
        let out =
            dispatch_with_rules("mytool", &[], input, &rules).expect("mytool should dispatch");
        assert!(out.contains("tool output line"));
        assert!(!out.contains("DeprecationWarning"));
        assert!(!out.contains("trace-deprecation"));
    }

    #[test]
    fn test_post_pass_strips_node_experimental_warning() {
        // ExperimentalWarning shares the (node:NNN) [EXPNNN] shape.
        let input = "+ react 19.2.6\n(node:42) [EXP0001] ExperimentalWarning: importAttributes is experimental\n";
        let rules: Vec<FilterConfig> = Vec::new();
        let out = dispatch_with_rules("pnpm", &["install"], input, &rules)
            .expect("pnpm install should dispatch");
        assert!(out.contains("react 19.2.6"));
        assert!(!out.contains("ExperimentalWarning"));
    }

    #[test]
    fn test_post_pass_idempotent_on_clean_output() {
        // No Node footer present — output should be unchanged by the
        // post-pass (no spurious modifications).
        let input = "abc1234 commit message\ndef5678 another commit\n";
        let rules: Vec<FilterConfig> = Vec::new();
        let out = dispatch_with_rules("git", &["log", "--oneline"], input, &rules)
            .expect("git log should dispatch");
        // The native git compressor runs first; just confirm no Node
        // markers leaked in.
        assert!(!out.contains("(node:"));
    }

    // Ultra-compact end-to-end behaviour is covered by unit tests in
    // engine::shell::tests (apply_ultra_compact) and
    // compressors::js::npm_cmd::tests (test_pnpm_install_noop_ultra_compact_collapses_to_ok).
    // We deliberately don't test it through dispatch_with_rules here:
    // the ultra-compact flag is a process-wide AtomicBool, and cargo
    // test runs the suite in parallel by default — toggling the flag
    // would race against any other dispatcher test that asserts on
    // output containing a `Done in …` line.
}

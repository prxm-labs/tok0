use anyhow::{Context, Result};
use clap::{CommandFactory, Parser, Subcommand};
use std::sync::OnceLock;

mod bridge;
mod compressors;
mod engine;
mod extensions;
mod insights;
mod scanner;

use engine::shell;

static EXTENSION_RULES: OnceLock<Vec<engine::rules::FilterConfig>> = OnceLock::new();

/// Ensures the "skipped N project rules (untrusted)" stderr hint prints
/// at most once per process, even if something re-enters the rule-loading
/// path. Each tok0 invocation is a fresh process, so this is effectively
/// per-invocation.
static UNTRUSTED_HINT_SHOWN: OnceLock<()> = OnceLock::new();

fn get_extension_rules() -> &'static [engine::rules::FilterConfig] {
    EXTENSION_RULES.get_or_init(|| {
        // Priority (highest → lowest):
        //   1. Project-local rules (.tok0/filters/*.toml) — requires trust
        //   2. User global rules (~/.config/tok0/filters/)
        //   3. Installed extension rules
        //   4. Built-in embedded rules
        // Higher-priority rules with the same name shadow lower-priority ones.

        // 1. Project-local rules (trust-gated)
        let project_rules = load_project_local_rules();

        // 2-3. Extension rules (user-installed, always loaded)
        let extension_rules: Vec<engine::rules::FilterConfig> =
            extensions::loader::load_extension_rules()
                .unwrap_or_default()
                .into_iter()
                .map(|r| r.filter)
                .collect();

        // 4. Built-in rules
        let builtin = engine::builtin_rules::builtin_rules();

        // Dedup: higher-priority rules shadow lower-priority by name
        let mut seen = std::collections::HashSet::new();
        let mut merged =
            Vec::with_capacity(project_rules.len() + extension_rules.len() + builtin.len());

        for rule in project_rules {
            seen.insert(rule.name.clone());
            merged.push(rule);
        }
        for rule in extension_rules {
            if !seen.contains(&rule.name) {
                seen.insert(rule.name.clone());
                merged.push(rule);
            }
        }
        for rule in builtin {
            if !seen.contains(&rule.name) {
                seen.insert(rule.name.clone());
                merged.push(rule.clone());
            }
        }

        merged
    })
}

use engine::guards::{is_shell_builtin, is_var_assignment};

/// Detect the user's shell from the $SHELL env value. Pure + testable.
/// Returns None if unset or unrecognized.
fn detect_shell(shell_env: Option<&str>) -> Option<&'static str> {
    let path = shell_env?;
    let name = path.rsplit('/').next().unwrap_or(path);
    match name {
        "bash" => Some("bash"),
        "zsh" => Some("zsh"),
        "fish" => Some("fish"),
        _ => None,
    }
}

/// Completion install path for a given shell, under the user's home.
/// Pure + testable. Returns None for unrecognized shells.
fn completions_install_path(shell: &str, home: &std::path::Path) -> Option<std::path::PathBuf> {
    match shell {
        "bash" => Some(
            home.join(".local")
                .join("share")
                .join("bash-completion")
                .join("completions")
                .join("tok0"),
        ),
        "zsh" => Some(home.join(".zsh").join("completions").join("_tok0")),
        "fish" => Some(
            home.join(".config")
                .join("fish")
                .join("completions")
                .join("tok0.fish"),
        ),
        _ => None,
    }
}

/// Print the "install shell completions" hint for the detected shell.
/// Pure + testable via a Writer injection.
fn print_completions_hint<W: std::io::Write>(
    writer: &mut W,
    shell_env: Option<&str>,
    home: &std::path::Path,
) -> std::io::Result<()> {
    let Some(shell) = detect_shell(shell_env) else {
        return Ok(());
    };
    let Some(target) = completions_install_path(shell, home) else {
        return Ok(());
    };
    writeln!(writer)?;
    writeln!(writer, "Shell completions ({}):", shell)?;
    writeln!(
        writer,
        "  Run: tok0 completions {} > {}",
        shell,
        target.display()
    )?;
    Ok(())
}

/// Load project-local rules from `.tok0/filters/`, gated by trust.
/// Returns empty vec if project is untrusted or no rules exist.
fn load_project_local_rules() -> Vec<engine::rules::FilterConfig> {
    let project_dir = match std::env::current_dir() {
        Ok(d) => d,
        Err(_) => return vec![],
    };

    let filters_dir = project_dir.join(".tok0").join("filters");
    if !filters_dir.is_dir() {
        return vec![];
    }

    // Trust gate: skip untrusted projects
    match bridge::trust::is_trusted(&project_dir) {
        Ok(true) => {}
        Ok(false) => {
            // Count rules that would be loaded
            let count = std::fs::read_dir(&filters_dir)
                .map(|entries| {
                    entries
                        .filter_map(|e| e.ok())
                        .filter(|e| {
                            e.path().extension().and_then(|ext| ext.to_str()) == Some("toml")
                        })
                        .count()
                })
                .unwrap_or(0);
            if count > 0 {
                UNTRUSTED_HINT_SHOWN.get_or_init(|| {
                    eprintln!(
                        "tok0: skipped {} project rule(s) (untrusted \u{2014} run `tok0 trust`)",
                        count
                    );
                });
            }
            return vec![];
        }
        Err(_) => return vec![],
    }

    engine::rules::load_rules_from_dir(&filters_dir, None)
        .unwrap_or_default()
        .into_iter()
        .map(|r| r.filter)
        .collect()
}

#[derive(Parser)]
#[command(
    name = "tok0",
    version,
    about = "Token-optimized CLI proxy for AI tools"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Increase verbosity (-v, -vv, -vvv)
    #[arg(short, long, action = clap::ArgAction::Count, global = true)]
    verbose: u8,

    /// Ultra-compact output mode
    #[arg(short, long, global = true)]
    ultra_compact: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Token savings analytics
    Stats {
        #[arg(long)]
        graph: bool,
        #[arg(long)]
        history: bool,
        #[arg(long)]
        daily: bool,
        #[arg(long)]
        format: Option<String>,
    },
    /// Git operations
    Git {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        command: Vec<String>,
    },
    /// Smart file reading
    Read {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    /// Directory listing
    Ls {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    /// Search files
    Grep {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    /// Find files
    Find {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    /// File diff
    Diff {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    /// Smart code summary
    Smart {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    /// Proxy passthrough (no filtering, tracking only)
    Proxy {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    /// Show tok0 status: version, hooks, savings, trust, extensions
    Status,
    /// Diagnose configuration + hook-health problems
    Doctor,
    /// Inspect and test compression rules
    Rule {
        #[command(subcommand)]
        command: RuleCommands,
    },
    /// Initialize hooks for AI tools
    Init {
        #[arg(short, long)]
        global: bool,
        #[arg(long)]
        uninstall: bool,
        #[arg(long)]
        show: bool,
        /// Run the guided onboarding wizard
        #[arg(long)]
        wizard: bool,
    },
    /// Update tok0 to the latest version (use --check to only check)
    Update {
        /// Only check whether an update is available; do not install it
        #[arg(long)]
        check: bool,
    },
    /// Discover missed savings opportunities
    Discover {
        #[arg(long)]
        all: bool,
        #[arg(long)]
        since: Option<u32>,
    },
    /// Rewrite command (used by hooks)
    Rewrite {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    /// Profile compression pipeline timing
    Profile {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    /// Manage extension rule packs
    Ext {
        #[command(subcommand)]
        command: ExtCommands,
    },
    /// Control anonymous telemetry (on/off/status)
    #[cfg(feature = "cloud")]
    Telemetry {
        #[command(subcommand)]
        command: TelemetryCommands,
    },
    /// Authenticate with tok0 cloud for team analytics
    #[cfg(feature = "cloud")]
    Auth {
        #[command(subcommand)]
        command: AuthCommands,
    },
    /// View team dashboard (requires auth)
    #[cfg(feature = "cloud")]
    Cloud {
        #[command(subcommand)]
        command: CloudCommands,
    },
    /// Generate shell completions
    Completions {
        /// Shell to generate completions for
        #[arg(value_enum)]
        shell: clap_complete::Shell,
    },
    /// Trust a project directory for local filter rules
    Trust,
    /// Remove trust from a project directory
    Untrust,
    /// Verify hook file integrity
    Verify {
        /// Path to the hook file
        path: String,
        /// Expected SHA-256 hash
        hash: String,
    },
    /// Vite (dev / build) — JS/TS bundler & dev server
    Vite {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    /// Next.js CLI (dev / build / start)
    Next {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    /// Prisma CLI (migrate / generate / validate / studio)
    Prisma {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    /// JSON pretty-print + array/object truncation (reads stdin)
    Json,
    /// Catch-all for any subcommand tok0 does not explicitly define
    /// (e.g. `tok0 wc -l file`, `tok0 tr a-z A-Z`). The first element is
    /// the command name; the rest are forwarded as args. The command is
    /// run through `run_proxy`, so the dispatcher gets a chance to
    /// compress its output and the meter records token savings. If no
    /// compressor matches, output passes through unchanged.
    #[command(external_subcommand)]
    External(Vec<String>),
}

#[cfg(feature = "cloud")]
#[derive(Subcommand)]
enum TelemetryCommands {
    /// Enable anonymous telemetry
    On,
    /// Disable anonymous telemetry
    Off,
    /// Show current telemetry status
    Status,
}

#[derive(Subcommand)]
enum RuleCommands {
    /// List all active rules (builtin + extensions + project-local)
    List,
    /// Show full config for a single rule by name
    Show {
        /// Rule name (matches the `name` field in the [filter] section)
        name: String,
    },
    /// Apply a rule to stdin and print the before/after savings
    Test {
        /// Rule name to exercise
        name: String,
    },
}

#[cfg(feature = "cloud")]
#[derive(Subcommand)]
enum AuthCommands {
    /// Login with API token
    Login {
        /// API token (or set TOK0_API_KEY env var)
        #[arg(long, env = "TOK0_API_KEY")]
        token: String,
        /// Cloud API URL
        #[arg(long, default_value = "https://api.tok0.dev")]
        api_url: String,
    },
    /// Show authentication status
    Status,
    /// Remove stored credentials and disable cloud reporting
    Logout,
}

#[cfg(feature = "cloud")]
#[derive(Subcommand)]
enum CloudCommands {
    /// Show team savings summary
    Team,
}

#[derive(Subcommand)]
enum ExtCommands {
    /// Install extension from a git URL
    Install {
        url: String,
        #[arg(long)]
        name: Option<String>,
        /// Pin the install to a specific git commit SHA (7-40 hex chars).
        /// Without this the install tracks the remote default-branch tip
        /// and updates silently on every clone — pinning gives you
        /// reproducible, auditable extension content.
        #[arg(long)]
        commit: Option<String>,
    },
    /// List installed extensions
    List,
    /// Remove an installed extension
    Remove { name: String },
}

fn main() {
    let cli = Cli::parse();
    if let Err(e) = run(cli) {
        eprintln!("tok0: {:#}", e);
        std::process::exit(1);
    }
}

fn run(cli: Cli) -> Result<()> {
    engine::shell::set_ultra_compact(cli.ultra_compact);
    match cli.command {
        Commands::Git { command } => run_proxy("git", &command),
        Commands::Ls { args } => run_proxy("ls", &args),
        Commands::Read { args } => run_proxy("cat", &args),
        Commands::Grep { args } => run_proxy("grep", &args),
        Commands::Find { args } => run_proxy("find", &args),
        Commands::Diff { args } => run_proxy("diff", &args),
        Commands::Smart { args } => run_proxy("smart", &args),
        Commands::Vite { args } => run_proxy("vite", &args),
        Commands::Next { args } => run_proxy("next", &args),
        Commands::Prisma { args } => run_proxy("prisma", &args),
        Commands::Json => {
            // JSON reads stdin, filters, writes stdout — no subprocess.
            use std::io::Read;
            let mut buf = String::new();
            std::io::stdin()
                .read_to_string(&mut buf)
                .context("Failed to read stdin")?;
            let filtered = compressors::system::json_cmd::filter_json(&buf).unwrap_or_else(|e| {
                eprintln!("tok0: json filter warning: {}", e);
                buf.clone()
            });
            print!("{}", filtered);
            Ok(())
        }
        Commands::Proxy { args } => run_raw_proxy(&args),
        Commands::External(parts) => {
            let mut iter = parts.into_iter();
            let cmd = iter
                .next()
                .context("External subcommand had no command name")?;
            if is_shell_builtin(&cmd) {
                eprintln!(
                    "tok0: '{cmd}' is a shell builtin and cannot run through tok0 \u{2014} \
                     it would execute in a child process and not affect your shell. \
                     Drop the 'tok0' prefix and run '{cmd}' directly."
                );
                std::process::exit(1);
            }
            if is_var_assignment(&cmd) {
                eprintln!(
                    "tok0: '{cmd}' looks like a variable assignment, not a command. \
                     tok0 cannot export env vars to a child process this way. \
                     Use `tok0 bash -c '{cmd} <command>'` instead, or set the variable \
                     in your shell first."
                );
                std::process::exit(1);
            }
            let args: Vec<String> = iter.collect();
            run_proxy(&cmd, &args)
        }
        Commands::Stats {
            graph,
            history,
            format,
            ..
        } => {
            let fmt = format.as_deref().unwrap_or("text");
            run_meta(|| insights::stats::run(graph, history, fmt))
        }
        Commands::Status => run_meta(insights::status::run),
        Commands::Doctor => run_meta(insights::doctor::run),
        Commands::Rule { command } => match command {
            RuleCommands::List => run_meta(insights::rules_cli::run_list),
            RuleCommands::Show { name } => run_meta(|| insights::rules_cli::run_show(&name)),
            RuleCommands::Test { name } => run_meta(|| insights::rules_cli::run_test(&name)),
        },
        Commands::Init {
            global,
            uninstall,
            show,
            wizard,
        } => {
            if wizard {
                let _ = bridge::wizard::run()?;
                return Ok(());
            }
            if show {
                let tools = bridge::setup::detect_tools();
                if tools.is_empty() {
                    println!("No supported AI tools detected.");
                } else {
                    println!("Detected AI tools:");
                    for tool in &tools {
                        println!("  {:?}", tool);
                    }
                }
                return Ok(());
            }

            let tools = bridge::setup::detect_tools();
            if tools.is_empty() {
                println!("No supported AI tools detected.");
                return Ok(());
            }

            let home = bridge::setup::home_dir_or_default()
                .context("Could not determine home directory")?;
            let cwd = std::env::current_dir().context("Could not determine current directory")?;

            // Per-tool config dir. Only ClaudeCode supports per-project
            // (non-global) installation today; all other tools always use
            // their home directory.
            let dir_for = |tool: &bridge::setup::ToolTarget| -> std::path::PathBuf {
                if !global && matches!(tool, bridge::setup::ToolTarget::ClaudeCode) {
                    cwd.join(".claude")
                } else {
                    bridge::setup::config_dir_for(tool, &home)
                }
            };

            if uninstall {
                println!("tok0 uninstall:");
                for tool in &tools {
                    let config_dir = dir_for(tool);
                    match bridge::setup::uninstall_hook_at(tool, &config_dir) {
                        Ok(result) => {
                            if result.already_installed {
                                println!(
                                    "  {:?}: hook removed from {}",
                                    tool,
                                    result.path.display()
                                );
                            } else {
                                println!("  {:?}: no hook found (already clean)", tool);
                            }
                        }
                        Err(e) => {
                            eprintln!("  {:?}: skipped ({})", tool, e);
                        }
                    }
                }
            } else {
                println!("tok0 init:");
                for tool in &tools {
                    let config_dir = dir_for(tool);
                    match bridge::setup::install_hook_at(tool, &config_dir) {
                        Ok(result) => {
                            if result.already_installed {
                                println!("  {:?}: already installed", tool);
                            } else {
                                println!(
                                    "  {:?}: hook installed at {}",
                                    tool,
                                    result.path.display()
                                );
                            }
                            if let Some(hint) = bridge::setup::post_install_hint(tool) {
                                println!("    \u{26a0}  {}", hint);
                                println!("       File: {}", result.path.display());
                            }
                        }
                        Err(e) => {
                            eprintln!("  {:?}: skipped ({})", tool, e);
                        }
                    }
                }
                println!();
                println!("Next: run any command through your AI tool \u{2014} tok0 compresses automatically.");
                println!("Run `tok0 stats` to see your savings.");
                // Optional: shell completions hint
                let shell_env = std::env::var("SHELL").ok();
                let _ = print_completions_hint(&mut std::io::stdout(), shell_env.as_deref(), &home);
            }
            Ok(())
        }
        Commands::Update { check } => {
            if check {
                let current = env!("CARGO_PKG_VERSION");
                match engine::updater::check_for_update(current)? {
                    Some(release) => {
                        eprintln!(
                            "tok0: new version {} available. Run `tok0 update` to install.",
                            release.version
                        );
                    }
                    None => {
                        eprintln!("tok0: already up to date (v{}).", current);
                    }
                }
                Ok(())
            } else {
                engine::updater::run_update()
            }
        }
        Commands::Discover { .. } => run_meta(scanner::opportunity::run),
        Commands::Rewrite { args } => {
            let rewritten = bridge::rewriter::rewrite_command(&args);
            if rewritten.is_empty() {
                return Ok(());
            }
            print!("{}", rewritten);
            Ok(())
        }
        Commands::Profile { args } => {
            if args.is_empty() {
                anyhow::bail!("Usage: tok0 profile <command> [args...]");
            }
            let cmd = &args[0];
            let cmd_args: Vec<&str> = args[1..].iter().map(|s| s.as_str()).collect();
            run_meta(|| insights::profiler::run(cmd, &cmd_args))
        }
        Commands::Ext { command } => match command {
            ExtCommands::Install { url, name, commit } => {
                let ext_name = name.unwrap_or_else(|| {
                    url.rsplit('/')
                        .find(|s| !s.is_empty())
                        .unwrap_or("unknown")
                        .trim_end_matches(".git")
                        .to_string()
                });
                extensions::catalog::install_from_url(&url, &ext_name, commit.as_deref())
            }
            ExtCommands::List => {
                let installed = extensions::catalog::list_installed()?;
                if installed.is_empty() {
                    println!("No extensions installed.");
                } else {
                    for name in &installed {
                        println!("  {}", name);
                    }
                }
                Ok(())
            }
            ExtCommands::Remove { name } => extensions::catalog::remove_extension(&name),
        },
        #[cfg(feature = "cloud")]
        Commands::Telemetry { command } => match command {
            TelemetryCommands::On => {
                engine::config::set_telemetry_enabled(&engine::config::config_path(), true)?;
                println!("Telemetry enabled. Anonymous usage stats will be sent once daily.");
                Ok(())
            }
            TelemetryCommands::Off => {
                engine::config::set_telemetry_enabled(&engine::config::config_path(), false)?;
                println!("Telemetry disabled. No data will be sent.");
                Ok(())
            }
            TelemetryCommands::Status => {
                let config = engine::config::load_config()?;
                if config.telemetry.enabled {
                    println!("Telemetry: enabled (anonymous, once daily)");
                } else {
                    println!("Telemetry: disabled");
                }
                Ok(())
            }
        },
        #[cfg(feature = "cloud")]
        Commands::Cloud { command } => match command {
            CloudCommands::Team => {
                let config = engine::config::load_config()?;
                // T1.4: API key now lives in the OS keyring. Fall back
                // to legacy plaintext config.toml for users mid-migration.
                let api_key = engine::config::get_api_key_from_keyring()
                    .ok()
                    .flatten()
                    .or_else(|| config.cloud.api_key.clone());
                if !config.cloud.enabled || api_key.is_none() {
                    anyhow::bail!(
                        "Not authenticated. Run `tok0 auth login --token <TOKEN>` first."
                    );
                }
                let api_url = config
                    .cloud
                    .api_url
                    .as_deref()
                    .unwrap_or("https://api.tok0.dev");
                let stats =
                    engine::cloud::fetch_team_stats(api_url, api_key.as_deref().unwrap_or(""))?;
                println!("{}", engine::cloud::format_team_stats(&stats));
                Ok(())
            }
        },
        Commands::Completions { shell } => {
            let mut cmd = Cli::command();
            let bin_name = cmd.get_name().to_string();
            clap_complete::generate(shell, &mut cmd, bin_name, &mut std::io::stdout());
            Ok(())
        }
        Commands::Trust => {
            let project_dir =
                std::env::current_dir().context("Could not determine current directory")?;
            bridge::trust::trust_project(&project_dir)?;
            println!("Trusted: {}", project_dir.display());
            Ok(())
        }
        Commands::Untrust => {
            let project_dir =
                std::env::current_dir().context("Could not determine current directory")?;
            bridge::trust::untrust_project(&project_dir)?;
            println!("Untrusted: {}", project_dir.display());
            Ok(())
        }
        Commands::Verify { path, hash } => {
            let result = bridge::verify::check_integrity(std::path::Path::new(&path), &hash);
            match result {
                bridge::verify::VerifyResult::Ok => {
                    println!("ok: hook integrity verified");
                }
                bridge::verify::VerifyResult::Tampered => {
                    eprintln!("TAMPERED: hook hash does not match expected value");
                    std::process::exit(1);
                }
                bridge::verify::VerifyResult::Missing => {
                    eprintln!("MISSING: hook file not found at {}", path);
                    std::process::exit(1);
                }
            }
            Ok(())
        }
        #[cfg(feature = "cloud")]
        Commands::Auth { command } => match command {
            AuthCommands::Login { token, api_url } => {
                eprintln!("tok0: validating token...");
                engine::cloud::validate_token(&api_url, &token)?;
                engine::config::set_cloud_credentials(
                    &engine::config::config_path(),
                    &token,
                    &api_url,
                )?;
                println!("Authenticated. Cloud team reporting enabled.");
                Ok(())
            }
            AuthCommands::Status => {
                let config = engine::config::load_config()?;
                // T1.4: token in keyring beats legacy plaintext.
                let has_key = engine::config::get_api_key_from_keyring()
                    .ok()
                    .flatten()
                    .is_some()
                    || config.cloud.api_key.is_some();
                if config.cloud.enabled && has_key {
                    let url = config
                        .cloud
                        .api_url
                        .as_deref()
                        .unwrap_or("https://api.tok0.dev");
                    println!("Cloud: authenticated");
                    println!("  API: {}", url);
                    if let Some(ref team) = config.cloud.team_id {
                        println!("  Team: {}", team);
                    }
                } else {
                    println!("Cloud: not authenticated");
                    println!("  Run `tok0 auth login --token <TOKEN>` to enable team reporting.");
                }
                Ok(())
            }
            AuthCommands::Logout => {
                engine::config::clear_cloud_credentials(&engine::config::config_path())?;
                println!("Logged out. Cloud reporting disabled.");
                Ok(())
            }
        },
    }
}

/// Run a meta command, then check for updates (rate-limited, silent on failure).
fn run_meta<F: FnOnce() -> Result<()>>(f: F) -> Result<()> {
    let result = f();
    engine::updater::maybe_print_update_hint();
    result
}

/// Apply compression + sanitisation when the output mode says so;
/// otherwise return the raw command output verbatim. Extracted from
/// `run_proxy` so the compress/passthrough contract is unit-testable
/// without spawning subprocesses.
fn maybe_compress(
    cmd: &str,
    args: &[&str],
    raw_stdout: &str,
    ext_rules: &[engine::rules::FilterConfig],
    should_compress: bool,
) -> String {
    if !should_compress {
        return raw_stdout.to_string();
    }
    let compressed = engine::dispatcher::dispatch_with_rules(cmd, args, raw_stdout, ext_rules)
        .unwrap_or_else(|| raw_stdout.to_string());
    engine::sanitize::sanitize_output(&compressed)
}

/// Execute command through compressor pipeline with metering.
fn run_proxy(cmd: &str, args: &[String]) -> Result<()> {
    let str_args: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    let config = engine::config::load_config().unwrap_or_default();
    let timeout = std::time::Duration::from_secs(config.limits.command_timeout_secs);
    let start = std::time::Instant::now();
    let output = engine::timeout::execute_with_timeout(cmd, &str_args, timeout)?;
    let duration_ms = start.elapsed().as_millis() as u64;

    let raw_stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    // Compress only when stdout is a TTY (or explicitly forced). When piped,
    // pass through raw — compressors like `find`/`ls`/`grep` are lossy by
    // design (group-and-truncate) which breaks downstream tools that need
    // verbatim data (e.g. `tok0 find … | xargs grep`).
    let ext_rules = get_extension_rules();
    let compressed = maybe_compress(
        cmd,
        &str_args,
        &raw_stdout,
        ext_rules,
        engine::output_mode::should_compress(),
    );

    // Output first — user sees results immediately
    if !compressed.is_empty() {
        print!("{}", compressed);
    }
    if !stderr.is_empty() {
        eprint!("{}", stderr);
    }

    // Record metrics after output (async — returns immediately; worker
    // thread handles SQLite write off the critical path).
    let project_path = std::env::current_dir()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_default();
    let _ = engine::meter::record(cmd, &raw_stdout, &compressed, duration_ms, &project_path);

    // Best-effort telemetry (rate-limited, silent on failure)
    engine::telemetry::maybe_ping();

    // Drain the meter worker before exit so the last event persists.
    engine::meter::flush();

    if !output.status.success() {
        std::process::exit(output.status.code().unwrap_or(1));
    }
    Ok(())
}

/// Raw proxy — execute command with no filtering, tracking only.
fn run_raw_proxy(args: &[String]) -> Result<()> {
    if args.is_empty() {
        anyhow::bail!("No command specified for proxy");
    }
    let cmd = &args[0];
    let cmd_args: Vec<&str> = args[1..].iter().map(|s| s.as_str()).collect();
    let output = shell::execute_command(cmd, &cmd_args)?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    if !stdout.is_empty() {
        print!("{}", stdout);
    }
    if !stderr.is_empty() {
        eprint!("{}", stderr);
    }

    // Defensive: no-op today (raw proxy doesn't record) but guarantees
    // the async meter worker is drained if someone wires metering into
    // this path in the future.
    engine::meter::flush();

    if !output.status.success() {
        std::process::exit(output.status.code().unwrap_or(1));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    // ─────────────────────────────────────────────────────────────────
    // run_proxy / maybe_compress contract
    //
    // Regression guard for the bug where
    //   `tok0 find <dir> -type f -name "*.rs" | xargs grep -l "node:" | head -10`
    // failed because the `find` compressor groups + truncates paths
    // into "+N more" and the path-redactor rewrites `/Users/name/` →
    // `/Users/user/`, both of which destroy data the pipeline needs.
    //
    // The fix gates compression behind `output_mode::should_compress()`.
    // These tests pin the contract so future refactors can't reintroduce
    // the regression.
    // ─────────────────────────────────────────────────────────────────

    /// Long find listing → with compression OFF, every path is preserved
    /// byte-for-byte. This is the property `xargs grep` needs.
    #[test]
    fn maybe_compress_passthrough_preserves_find_paths() {
        let raw = (0..58)
            .map(|i| format!("/Users/theodorevorillas/project/src/file_{i}.rs"))
            .collect::<Vec<_>>()
            .join("\n");
        let out = maybe_compress("find", &[".", "-name", "*.rs"], &raw, &[], false);
        assert_eq!(out, raw, "passthrough must be byte-identical to raw");
        assert!(out.contains("file_57.rs"), "last path must survive");
        assert!(
            !out.contains("more"),
            "no '+N more' summary should leak through"
        );
        assert!(
            out.contains("/Users/theodorevorillas/"),
            "real home path must NOT be redacted in pipe mode (xargs needs it)"
        );
    }

    /// With compression ON, the find compressor's lossy summary takes over —
    /// the explicit, opt-in case for a TTY or `TOK0_FORCE_COMPRESS=1`.
    #[test]
    fn maybe_compress_with_compression_runs_dispatcher() {
        let raw = (0..58)
            .map(|i| format!("/Users/theodorevorillas/project/src/file_{i}.rs"))
            .collect::<Vec<_>>()
            .join("\n");
        let out = maybe_compress("find", &[".", "-name", "*.rs"], &raw, &[], true);
        assert_ne!(out, raw, "compression must transform the output");
        assert!(
            out.contains("58 files found"),
            "expected the find compressor's count header"
        );
        assert!(
            out.contains("/Users/user/"),
            "sanitiser must redact home path in compress mode"
        );
    }

    /// Empty stdout is preserved exactly in passthrough mode (no spurious
    /// headers, no "ok" replacement).
    #[test]
    fn maybe_compress_passthrough_empty_stdout() {
        assert_eq!(maybe_compress("find", &[], "", &[], false), "");
    }

    /// Unknown command in passthrough mode: raw text wins, no fallback
    /// errors.
    #[test]
    fn maybe_compress_passthrough_unknown_command() {
        let raw = "arbitrary tool output\n";
        assert_eq!(maybe_compress("nosuchcmd", &[], raw, &[], false), raw);
    }

    /// Unknown command in compress mode: the dispatcher returns None,
    /// so we fall back to raw — but sanitiser still runs (path redaction).
    #[test]
    fn maybe_compress_compress_mode_unknown_command_sanitises() {
        let raw = "see /Users/theodorevorillas/secret\n";
        let out = maybe_compress("nosuchcmd", &[], raw, &[], true);
        assert!(!out.contains("theodorevorillas"));
        assert!(out.contains("/Users/user/"));
    }

    /// Pipe-mode passthrough must NOT redact secrets either — sanitisation
    /// runs only when compression runs. (The user is on their own machine;
    /// they can see their own secrets. Sanitisation is for LLM/cloud paths.)
    #[test]
    fn maybe_compress_passthrough_does_not_redact_secrets() {
        let raw = "API_KEY=sk-abcdef0123456789\n";
        assert_eq!(maybe_compress("env", &[], raw, &[], false), raw);
    }

    #[test]
    fn test_completion_generation_bash() {
        let mut cmd = Cli::command();
        let mut buf = Vec::new();
        clap_complete::generate(clap_complete::Shell::Bash, &mut cmd, "tok0", &mut buf);
        assert!(!buf.is_empty());
        let content = String::from_utf8(buf).expect("valid utf8");
        assert!(content.contains("tok0"));
    }

    #[test]
    fn test_completion_generation_zsh() {
        let mut cmd = Cli::command();
        let mut buf = Vec::new();
        clap_complete::generate(clap_complete::Shell::Zsh, &mut cmd, "tok0", &mut buf);
        assert!(!buf.is_empty());
    }

    #[test]
    fn test_completion_generation_fish() {
        let mut cmd = Cli::command();
        let mut buf = Vec::new();
        clap_complete::generate(clap_complete::Shell::Fish, &mut cmd, "tok0", &mut buf);
        assert!(!buf.is_empty());
    }

    #[test]
    fn test_detect_shell_recognises_common_shells() {
        assert_eq!(detect_shell(Some("/bin/bash")), Some("bash"));
        assert_eq!(detect_shell(Some("/usr/bin/zsh")), Some("zsh"));
        assert_eq!(detect_shell(Some("/opt/homebrew/bin/fish")), Some("fish"));
        assert_eq!(detect_shell(Some("bash")), Some("bash"));
    }

    #[test]
    fn test_detect_shell_returns_none_for_unknown_and_missing() {
        assert_eq!(detect_shell(None), None);
        assert_eq!(detect_shell(Some("")), None);
        assert_eq!(detect_shell(Some("/bin/tcsh")), None);
        assert_eq!(detect_shell(Some("/bin/nu")), None);
    }

    #[test]
    fn test_completions_install_path_per_shell() {
        let home = std::path::PathBuf::from("/home/test");
        assert_eq!(
            completions_install_path("bash", &home),
            Some(home.join(".local/share/bash-completion/completions/tok0"))
        );
        assert_eq!(
            completions_install_path("zsh", &home),
            Some(home.join(".zsh/completions/_tok0"))
        );
        assert_eq!(
            completions_install_path("fish", &home),
            Some(home.join(".config/fish/completions/tok0.fish"))
        );
        assert_eq!(completions_install_path("tcsh", &home), None);
    }

    #[test]
    fn test_print_completions_hint_includes_command() {
        let mut buf: Vec<u8> = Vec::new();
        let home = std::path::PathBuf::from("/home/test");
        print_completions_hint(&mut buf, Some("/bin/zsh"), &home).expect("write");
        let s = String::from_utf8(buf).expect("utf8");
        assert!(s.contains("Shell completions (zsh)"));
        assert!(s.contains("tok0 completions zsh"));
        // Assert on the target path via Display so the test passes on
        // both Unix (`/`) and Windows (`\`) separators.
        let expected = completions_install_path("zsh", &home)
            .expect("zsh path")
            .display()
            .to_string();
        assert!(
            s.contains(&expected),
            "expected {:?} in output: {}",
            expected,
            s
        );
    }

    #[test]
    fn test_print_completions_hint_silent_for_unknown_shell() {
        let mut buf: Vec<u8> = Vec::new();
        let home = std::path::PathBuf::from("/home/test");
        print_completions_hint(&mut buf, Some("/bin/tcsh"), &home).expect("write");
        assert!(
            buf.is_empty(),
            "should not print anything for unknown shell"
        );
    }

    #[test]
    fn test_print_completions_hint_silent_when_shell_unset() {
        let mut buf: Vec<u8> = Vec::new();
        let home = std::path::PathBuf::from("/home/test");
        print_completions_hint(&mut buf, None, &home).expect("write");
        assert!(
            buf.is_empty(),
            "should not print anything when SHELL is unset"
        );
    }

    // ── Argv parsing: subcommands must accept leading flags ──────────────
    //
    // `tok0 ls -lah` used to fail with "unexpected argument '-l' found"
    // because clap's `trailing_var_arg = true` only treats arguments as
    // raw values *after* the first positional. Pairing it with
    // `allow_hyphen_values = true` makes the entire arg list opaque, so
    // any `-flag` at the start is captured verbatim and forwarded to the
    // underlying tool (ls, grep, find, git, etc.).

    fn parse_argv(argv: &[&str]) -> Cli {
        Cli::try_parse_from(argv).expect("argv must parse")
    }

    #[test]
    fn test_ls_accepts_leading_short_flag() {
        let cli = parse_argv(&["tok0", "ls", "-lah"]);
        match cli.command {
            Commands::Ls { args } => assert_eq!(args, vec!["-lah".to_string()]),
            other => panic!("expected Ls, got {:?}", std::any::type_name_of_val(&other)),
        }
    }

    #[test]
    fn test_ls_accepts_long_flag_then_path() {
        let cli = parse_argv(&["tok0", "ls", "--all", "/tmp"]);
        match cli.command {
            Commands::Ls { args } => {
                assert_eq!(args, vec!["--all".to_string(), "/tmp".to_string()]);
            }
            _ => panic!("expected Ls"),
        }
    }

    #[test]
    fn test_ls_no_args_still_works() {
        let cli = parse_argv(&["tok0", "ls"]);
        assert!(matches!(cli.command, Commands::Ls { args } if args.is_empty()));
    }

    #[test]
    fn test_ls_positional_only_still_works() {
        let cli = parse_argv(&["tok0", "ls", "/tmp"]);
        match cli.command {
            Commands::Ls { args } => assert_eq!(args, vec!["/tmp".to_string()]),
            _ => panic!("expected Ls"),
        }
    }

    #[test]
    fn test_grep_leading_flag_then_pattern() {
        let cli = parse_argv(&["tok0", "grep", "-r", "foo", "."]);
        match cli.command {
            Commands::Grep { args } => assert_eq!(
                args,
                vec!["-r".to_string(), "foo".to_string(), ".".to_string()]
            ),
            _ => panic!("expected Grep"),
        }
    }

    #[test]
    fn test_find_leading_flag() {
        let cli = parse_argv(&["tok0", "find", "-name", "*.rs"]);
        match cli.command {
            Commands::Find { args } => {
                assert_eq!(args, vec!["-name".to_string(), "*.rs".to_string()]);
            }
            _ => panic!("expected Find"),
        }
    }

    #[test]
    fn test_diff_leading_flag() {
        // `-u` would collide with tok0's --ultra-compact short alias.
        // Use `--brief`, which has no global counterpart.
        let cli = parse_argv(&["tok0", "diff", "--brief", "a.txt", "b.txt"]);
        match cli.command {
            Commands::Diff { args } => assert_eq!(
                args,
                vec![
                    "--brief".to_string(),
                    "a.txt".to_string(),
                    "b.txt".to_string()
                ]
            ),
            _ => panic!("expected Diff"),
        }
    }

    #[test]
    fn test_read_leading_flag() {
        let cli = parse_argv(&["tok0", "read", "-n", "10", "/etc/hosts"]);
        match cli.command {
            Commands::Read { args } => assert_eq!(
                args,
                vec!["-n".to_string(), "10".to_string(), "/etc/hosts".to_string()]
            ),
            _ => panic!("expected Read"),
        }
    }

    #[test]
    fn test_git_log_with_flags() {
        let cli = parse_argv(&["tok0", "git", "log", "--oneline", "-3"]);
        match cli.command {
            Commands::Git { command } => assert_eq!(
                command,
                vec!["log".to_string(), "--oneline".to_string(), "-3".to_string()]
            ),
            _ => panic!("expected Git"),
        }
    }

    #[test]
    fn test_vite_dev_with_flag() {
        let cli = parse_argv(&["tok0", "vite", "dev", "--port", "3000"]);
        match cli.command {
            Commands::Vite { args } => assert_eq!(
                args,
                vec!["dev".to_string(), "--port".to_string(), "3000".to_string()]
            ),
            _ => panic!("expected Vite"),
        }
    }

    #[test]
    fn test_smart_leading_flag() {
        // Avoid `-v`/`-u`: those collide with tok0's global --verbose /
        // --ultra-compact flags (see the Cli struct). When a passthrough
        // command needs those, the user has to use `--` or move the
        // global flag before the subcommand.
        let cli = parse_argv(&["tok0", "smart", "-q", "src/lib.rs"]);
        match cli.command {
            Commands::Smart { args } => {
                assert_eq!(args, vec!["-q".to_string(), "src/lib.rs".to_string()]);
            }
            _ => panic!("expected Smart"),
        }
    }

    #[test]
    fn test_proxy_leading_flag_passthrough() {
        let cli = parse_argv(&["tok0", "proxy", "ls", "-lah"]);
        match cli.command {
            Commands::Proxy { args } => {
                assert_eq!(args, vec!["ls".to_string(), "-lah".to_string()])
            }
            _ => panic!("expected Proxy"),
        }
    }

    #[test]
    fn test_double_dash_separator_still_works() {
        // Backwards compat: users who learned `tok0 ls -- -lah` keep working.
        let cli = parse_argv(&["tok0", "ls", "--", "-lah"]);
        match cli.command {
            Commands::Ls { args } => assert_eq!(args, vec!["-lah".to_string()]),
            _ => panic!("expected Ls"),
        }
    }

    #[test]
    fn test_external_subcommand_with_leading_flag() {
        // `tok0 wc -l file` — wc is not a defined subcommand, so the
        // catch-all External should capture it with all flags preserved.
        let cli = parse_argv(&["tok0", "wc", "-l", "/etc/hosts"]);
        match cli.command {
            Commands::External(parts) => assert_eq!(
                parts,
                vec!["wc".to_string(), "-l".to_string(), "/etc/hosts".to_string()]
            ),
            _ => panic!("expected External"),
        }
    }

    #[test]
    fn test_external_unknown_tool_with_flag() {
        let cli = parse_argv(&["tok0", "lsof", "-nP"]);
        match cli.command {
            Commands::External(parts) => {
                assert_eq!(parts, vec!["lsof".to_string(), "-nP".to_string()])
            }
            _ => panic!("expected External"),
        }
    }

    // Shell-builtin predicate tests live next to the implementation
    // in engine::guards. The wiring into the External arm is exercised
    // by test_cd_builtin_fails_fast_with_clear_error and friends in
    // tests/cli_integration.rs.
}

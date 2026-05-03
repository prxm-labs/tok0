# Post-v0.1 Hardening Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Close the gaps identified after v0.1.0 ship so tok0 is installable, secure, and operationally sound for real users.

**Architecture:** Work is split into six priority tiers. **Tier 0** is blocked on external action (repo visibility, backend deploy) — no code changes. **Tiers 1–3** are code-complete specs ready to execute. **Tiers 4–5** are scope-only and need their own sub-plans before execution.

**Tech Stack:** Rust (1.95 stable), mockito for HTTP mocks, proptest for property tests, criterion for benchmarks, keyring crate for credential storage, `gh` for GitHub automation.

---

## Execution order

```
Tier 0 (blocked)  ──────────┐
Tier 1 (security)           ├── Required for v0.2.0
Tier 2 (coverage)           │
Tier 3 (features)           ┘
Tier 4 (UX polish)  ────────── Optional for v0.2.0
Tier 5 (ops / distribution) ── v0.3.0+
```

Do tiers 1–3 in parallel via subagents. Tier 0 unblocks release; tiers 4–5 are follow-ups.

---

# Tier 0 — Release unblockers (external)

No code changes. Track as GitHub issues; no checkboxes because the work isn't in this repo.

## T0.1 — Flip repo to public

**Owner:** user  
**Acceptance:** `curl -sSL https://github.com/prxm-labs/tok0/releases/download/v0.1.0/tok0-aarch64-apple-darwin.sha256` returns a 64-char hex string (not a 404).

## T0.2 — Publish Homebrew tap

**Owner:** user  
**Acceptance:** `brew tap prxm-labs/tok0 && brew install tok0` installs v0.1.0 successfully.

**Steps:**
1. Create `github.com/prxm-labs/homebrew-tok0` repo (public).
2. Add `scripts/sync-formula.sh` to the main repo that pushes `Formula/tok0.rb` to the tap repo after each release.
3. Wire that into `.github/workflows/release.yml` as a post-build job.

## T0.3 — Decide cloud/telemetry strategy

**Owner:** user  
**Pick one:**

- **Deploy `api.tok0.dev`** with endpoints:
  - `POST /telemetry` (anonymous daily ping)
  - `POST /report` (authenticated event batch)
  - `GET /team/stats` (authenticated team summary)
- **Disable until deployed.** Gate `tok0 telemetry`, `tok0 auth`, `tok0 cloud` behind a `#[cfg(feature = "cloud")]` flag and omit from the default build.

Recommended for v0.2.0: disable (option B), ship without cloud. Re-enable when backend exists.

---

# Tier 1 — Security hardening (~1 week)

Ship in v0.2.0.

## T1.1 — Regex complexity budget (ReDoS protection)

**Problem:** Extension-supplied TOML rules can ship catastrophic-backtracking regexes. Current code compiles them directly with no bound.

**Files:**
- Modify: `src/engine/rules.rs:40` (`FilterConfig::compile_patterns`)
- Test: same file tests module

- [ ] **Step 1: Write the failing test**

```rust
// append to tests module in src/engine/rules.rs
#[test]
fn test_compile_patterns_rejects_oversized_regex() {
    let mut cfg = FilterConfig {
        name: "evil".into(),
        commands: vec!["evil".into()],
        strip_patterns: vec!["a".repeat(10_000)],
        ..Default::default()
    };
    cfg.compile_patterns();
    assert!(
        cfg.compiled_patterns.is_empty(),
        "oversized regex should be rejected"
    );
}

#[test]
fn test_compile_patterns_rejects_pathological_backtrack() {
    let mut cfg = FilterConfig {
        name: "evil".into(),
        commands: vec!["evil".into()],
        // Classic catastrophic pattern
        strip_patterns: vec!["(a+)+b".into()],
        ..Default::default()
    };
    cfg.compile_patterns();
    // With `RegexBuilder::size_limit`, this either compiles safely or is rejected.
    // Verify it never causes CPU burn by bounding pattern execution time.
    let start = std::time::Instant::now();
    let _ = apply_filter_config(&"a".repeat(30), &cfg);
    assert!(
        start.elapsed().as_millis() < 500,
        "pattern match must not take >500ms on adversarial input"
    );
}
```

- [ ] **Step 2: Run tests, confirm failure**

```bash
cargo test test_compile_patterns_rejects -- --nocapture
```
Expected: second test times out or fails.

- [ ] **Step 3: Implement size + DFA cap via RegexBuilder**

Replace the body of `compile_patterns` in `src/engine/rules.rs`:

```rust
pub fn compile_patterns(&mut self) {
    self.compiled_patterns = self
        .strip_patterns
        .iter()
        .filter(|p| p.len() <= 4096) // reject oversized patterns
        .filter_map(|p| {
            regex::RegexBuilder::new(p)
                .size_limit(1 << 20)        // 1 MiB compiled regex cap
                .dfa_size_limit(1 << 20)    // 1 MiB DFA cache cap
                .build()
                .ok()
        })
        .collect();
}
```

- [ ] **Step 4: Re-run tests — both pass**

```bash
cargo test test_compile_patterns_rejects
```
Expected: 2 passed.

- [ ] **Step 5: Commit**

```bash
git add src/engine/rules.rs
git commit -m "fix(security): cap regex size + DFA budget to prevent ReDoS"
```

## T1.2 — Zeroize in-flight bearer token

**Problem:** `format!("Bearer {}", api_key)` creates a `String` that outlives the HTTP request and isn't wiped.

**Files:**
- Modify: `src/engine/cloud.rs` (lines around `flush`, `validate_token`, `fetch_team_stats`)

- [ ] **Step 1: Write the failing test**

Add to `cloud.rs` tests module:

```rust
#[test]
fn test_bearer_header_is_wiped_after_use() {
    // This test verifies the pattern; we exercise it via a closure
    // that mimics the flush() path.
    use zeroize::Zeroize;
    let api_key = "tok_bearer_test_zzz".to_string();
    let mut header = format!("Bearer {}", &api_key);
    let ptr = header.as_ptr();
    let len = header.len();
    header.zeroize();
    let slice = unsafe { std::slice::from_raw_parts(ptr, len) };
    let as_str = std::str::from_utf8(slice).unwrap_or("");
    assert!(
        !as_str.contains("tok_bearer_test_zzz"),
        "bearer header should be wiped, found: {:?}",
        as_str
    );
}
```

- [ ] **Step 2: Verify the pattern works (test should pass with existing zeroize crate)**

```bash
cargo test test_bearer_header_is_wiped_after_use
```

- [ ] **Step 3: Apply zeroize in flush() and fetch_team_stats()**

In `src/engine/cloud.rs`, wrap the bearer header construction:

```rust
// In CloudClient::flush()
let api_key = self.api_key.as_deref().unwrap_or("");
let mut header = format!("Bearer {}", api_key);
let result = ureq::post(&format!("{}/report", self.api_url))
    .set("Authorization", &header)
    .set("Content-Type", "application/json")
    .send_string(&payload);
header.zeroize();
result.context("Failed to send events to cloud API")?;
```

Apply the same pattern in `validate_token` and `fetch_team_stats`.

- [ ] **Step 4: Quality gate**

```bash
cargo fmt --all && cargo clippy --all-targets && cargo test engine::cloud
```

- [ ] **Step 5: Commit**

```bash
git add src/engine/cloud.rs
git commit -m "fix(security): zeroize bearer-token headers after HTTP calls"
```

## T1.3 — Extension signature / commit-pin verification

**Problem:** `tok0 ext install <url>` does `git clone --depth=1` with no hash pinning. A malicious extension author can silently push new content.

**Files:**
- Modify: `src/extensions/catalog.rs::install_from_url`
- Add: `--commit <sha>` flag to the `ext install` subcommand in `src/main.rs`

- [ ] **Step 1: Extend install_from_url signature**

Change `install_from_url(url: &str, name: &str)` to `install_from_url(url: &str, name: &str, pin: Option<&str>)`. All callers must pass `None` or a 40-char SHA.

- [ ] **Step 2: Write test for commit pin**

```rust
// In catalog.rs tests
#[test]
fn test_install_with_invalid_commit_pin() {
    let result = install_from_url(
        "https://github.com/prxm-labs/tok0.git",
        "pinned-ext",
        Some("not-a-real-sha"),
    );
    assert!(result.is_err());
}
```

- [ ] **Step 3: Implement pin checkout**

After the `git clone`, if `pin` is `Some(sha)`, run `git -C <dest> checkout <sha>` inside the cloned dir and verify the resolved HEAD matches. Fail loudly if not.

```rust
if let Some(sha) = pin {
    let checkout = std::process::Command::new("git")
        .args(["-C", dest.to_str().unwrap_or(""), "checkout", sha])
        .status()
        .context("Failed to checkout pinned commit")?;
    if !checkout.success() {
        let _ = std::fs::remove_dir_all(&dest);
        anyhow::bail!("Pinned commit {} not found in {}", sha, url);
    }
}
```

- [ ] **Step 4: Add --commit flag to CLI**

In `src/main.rs`, extend the `ExtCommands::Install` variant with `#[arg(long)] commit: Option<String>`. Wire it through to `install_from_url`.

- [ ] **Step 5: Quality gate + commit**

```bash
cargo fmt --all && cargo clippy --all-targets && cargo test extensions
git add src/extensions/catalog.rs src/main.rs
git commit -m "feat(security): support --commit pin for tok0 ext install"
```

## T1.4 — OS keyring for API credentials

**Problem:** API key lives in `~/.config/tok0/config.toml` as plaintext. Zeroize only helps in-process; file persists.

**Files:**
- Add: `keyring = "3"` to `Cargo.toml`
- Modify: `src/engine/config.rs::set_cloud_credentials`, `clear_cloud_credentials`, and `CloudConfig` loading
- Modify: `src/main.rs::Commands::Auth` handling

- [ ] **Step 1: Add keyring dep**

```toml
# Cargo.toml
keyring = { version = "3", default-features = false, features = ["apple-native", "windows-native", "sync-secret-service"] }
```

- [ ] **Step 2: Write test using the `keyring::mock` feature**

```rust
// src/engine/config.rs tests
#[test]
fn test_api_key_roundtrip_via_keyring() {
    keyring::set_default_credential_builder(keyring::mock::default_credential_builder());
    let key = "tok_secret_from_keyring";
    set_api_key_in_keyring(key).expect("set");
    let got = get_api_key_from_keyring().expect("get");
    assert_eq!(got.as_deref(), Some(key));
    clear_api_key_from_keyring().expect("clear");
    assert!(get_api_key_from_keyring().expect("get").is_none());
}
```

- [ ] **Step 3: Implement three helpers**

Add to `src/engine/config.rs`:

```rust
const KEYRING_SERVICE: &str = "com.prxm-labs.tok0";
const KEYRING_USER: &str = "cloud-api-key";

pub fn set_api_key_in_keyring(key: &str) -> Result<()> {
    keyring::Entry::new(KEYRING_SERVICE, KEYRING_USER)
        .context("Failed to open keyring entry")?
        .set_password(key)
        .context("Failed to write key to OS keyring")
}

pub fn get_api_key_from_keyring() -> Result<Option<String>> {
    match keyring::Entry::new(KEYRING_SERVICE, KEYRING_USER)?.get_password() {
        Ok(v) => Ok(Some(v)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(e) => Err(anyhow::anyhow!(e).context("Failed to read from keyring")),
    }
}

pub fn clear_api_key_from_keyring() -> Result<()> {
    match keyring::Entry::new(KEYRING_SERVICE, KEYRING_USER)?.delete_password() {
        Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(e) => Err(anyhow::anyhow!(e).context("Failed to clear keyring entry")),
    }
}
```

- [ ] **Step 4: Change auth login/logout flow**

In `src/main.rs` `Commands::Auth` handler:
- `Login` → call `set_api_key_in_keyring(&token)` instead of `set_cloud_credentials`.
- `Logout` → call `clear_api_key_from_keyring`.
- When loading config for cloud operations, look up the key from the keyring, not the config file.

Remove `api_key` from what `set_cloud_credentials` writes. Leave `api_url` and `team_id` in TOML.

- [ ] **Step 5: Migration path**

On `Commands::Auth::Login`, if an existing `api_key` is present in the config TOML, migrate it to the keyring and strip it from the file.

- [ ] **Step 6: Quality gate + commit**

```bash
cargo fmt --all && cargo clippy --all-targets && cargo test engine::config
git add Cargo.toml Cargo.lock src/engine/config.rs src/main.rs
git commit -m "feat(security): store API credentials in OS keyring, not config.toml"
```

---

# Tier 2 — Test coverage (~3 days)

Ship in v0.2.0. Can parallelize T2.1/T2.2/T2.3 — independent files.

## T2.1 — HTTP mock tests for cloud.rs

**Problem:** `validate_token`, `fetch_team_stats`, `flush` all hit live ureq. No tests for timeouts, 4xx, 5xx, malformed JSON.

**Files:**
- Add: `mockito = "1"` to dev-dependencies
- Add: tests in `src/engine/cloud.rs` tests module

- [ ] **Step 1: Add mockito**

```toml
[dev-dependencies]
mockito = "1"
```

- [ ] **Step 2: Write tests covering every failure mode**

```rust
// src/engine/cloud.rs tests module
use mockito::Server;

#[test]
fn test_flush_success() {
    let mut srv = Server::new();
    let mock = srv
        .mock("POST", "/report")
        .match_header("authorization", "Bearer tok_test")
        .with_status(200)
        .create();
    let mut c = CloudClient::new(&srv.url(), Some("tok_test"));
    c.record("git log", 1000, 100);
    c.flush().expect("flush should succeed");
    mock.assert();
    assert_eq!(c.pending_count(), 0);
}

#[test]
fn test_flush_propagates_server_500() {
    let mut srv = Server::new();
    srv.mock("POST", "/report").with_status(500).create();
    let mut c = CloudClient::new(&srv.url(), Some("tok_test"));
    c.record("git log", 1000, 100);
    assert!(c.flush().is_err(), "500 should surface as error");
}

#[test]
fn test_validate_token_401_is_error() {
    let mut srv = Server::new();
    srv.mock("GET", "/team/stats").with_status(401).create();
    assert!(validate_token(&srv.url(), "bad_token").is_err());
}

#[test]
fn test_fetch_team_stats_malformed_json() {
    let mut srv = Server::new();
    srv.mock("GET", "/team/stats")
        .with_body("not json")
        .with_status(200)
        .create();
    assert!(fetch_team_stats(&srv.url(), "tok").is_err());
}
```

- [ ] **Step 3: Run — they should all pass**

```bash
cargo test engine::cloud
```

- [ ] **Step 4: Commit**

```bash
git add Cargo.toml Cargo.lock src/engine/cloud.rs
git commit -m "test(cloud): cover success, 500, 401, malformed JSON via mockito"
```

## T2.2 — HTTP mock tests for updater.rs

**Files:**
- Modify: `src/engine/updater.rs` — replace hardcoded `GITHUB_API` constant with a function that accepts a URL override for tests.
- Add: tests using mockito

- [ ] **Step 1: Refactor GITHUB_API to take an override**

```rust
// Add near the top of updater.rs
fn api_url() -> String {
    std::env::var("TOK0_UPDATE_API_URL")
        .unwrap_or_else(|_| GITHUB_API.to_string())
}
```

Replace all `GITHUB_API` uses in `fetch_latest_release` with `api_url()`.

- [ ] **Step 2: Write tests**

```rust
#[test]
fn test_fetch_latest_release_timeout() {
    // no mock started → connection refused
    unsafe { std::env::set_var("TOK0_UPDATE_API_URL", "http://127.0.0.1:1"); }
    assert!(fetch_latest_release().is_err());
    unsafe { std::env::remove_var("TOK0_UPDATE_API_URL"); }
}

#[test]
fn test_fetch_latest_release_malformed_json() {
    let mut srv = mockito::Server::new();
    srv.mock("GET", "/").with_body("garbage").create();
    unsafe { std::env::set_var("TOK0_UPDATE_API_URL", srv.url()); }
    assert!(fetch_latest_release().is_err());
    unsafe { std::env::remove_var("TOK0_UPDATE_API_URL"); }
}

#[test]
fn test_fetch_latest_release_missing_asset_for_platform() {
    let mut srv = mockito::Server::new();
    srv.mock("GET", "/")
        .with_body(r#"{"tag_name":"v0.2.0","assets":[]}"#)
        .create();
    unsafe { std::env::set_var("TOK0_UPDATE_API_URL", srv.url()); }
    let result = fetch_latest_release();
    assert!(result.is_err() || result.unwrap().is_none());
    unsafe { std::env::remove_var("TOK0_UPDATE_API_URL"); }
}
```

- [ ] **Step 3: Run + commit**

```bash
cargo test engine::updater
git add src/engine/updater.rs
git commit -m "test(updater): cover timeout, malformed JSON, missing platform asset"
```

## T2.3 — Git clone failure tests for catalog.rs

**Files:**
- Modify: `src/extensions/catalog.rs` tests

- [ ] **Step 1: Write tests that don't require network**

```rust
#[test]
fn test_install_from_url_invalid_url_fails() {
    let tmp = TempDir::new().expect("tmp");
    unsafe { std::env::set_var("TOK0_CONFIG_DIR", tmp.path()); }
    let result = install_from_url(
        "https://github.com/prxm-labs/definitely-does-not-exist-42.git",
        "dne-ext",
        None,
    );
    unsafe { std::env::remove_var("TOK0_CONFIG_DIR"); }
    assert!(result.is_err(), "should fail on non-existent repo");
    // Destination should be cleaned up on failure
    assert!(!tmp.path().join("extensions").join("dne-ext").exists());
}

#[test]
fn test_install_from_url_non_git_url_rejected() {
    let result = install_from_url("ftp://old.example.com/x", "bad", None);
    assert!(result.is_err());
}
```

- [ ] **Step 2: Run + commit**

```bash
cargo test extensions::catalog
git add src/extensions/catalog.rs
git commit -m "test(extensions): cover clone-failure + bad-URL paths"
```

## T2.4 — Benchmarks via criterion

**Files:**
- Add: `criterion` dev-dep, `benches/compression.rs`, `benches/dispatcher.rs`

- [ ] **Step 1: Add criterion**

```toml
[dev-dependencies]
criterion = { version = "0.5", features = ["html_reports"] }

[[bench]]
name = "compression"
harness = false

[[bench]]
name = "dispatcher"
harness = false
```

- [ ] **Step 2: Write `benches/compression.rs`**

```rust
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use tok0::engine::dispatcher::dispatch_with_rules;
use tok0::engine::rules::FilterConfig;

fn bench_git_log(c: &mut Criterion) {
    let input = include_str!("../tests/fixtures/git/git_log.txt");
    let rules: Vec<FilterConfig> = vec![];
    c.bench_function("dispatch git log", |b| {
        b.iter(|| dispatch_with_rules("git", &["log"], black_box(input), &rules))
    });
}

criterion_group!(benches, bench_git_log);
criterion_main!(benches);
```

Note: this requires `src/lib.rs` to re-export `engine` modules. If it doesn't, create a minimal `src/lib.rs`:

```rust
pub mod bridge;
pub mod compressors;
pub mod engine;
pub mod extensions;
pub mod insights;
pub mod scanner;
```

- [ ] **Step 3: Run + commit**

```bash
cargo bench --bench compression
git add Cargo.toml Cargo.lock benches/ src/lib.rs
git commit -m "bench: add criterion benchmarks for compression pipeline"
```

## T2.5 — Fuzzing via cargo-fuzz

**Defer to v0.3.0.** Scope only:
- Add `fuzz/` workspace member
- Fuzz targets: `filter_config_apply` (random TOML rule + random input), `rewriter::rewrite_command` (random argv).
- Runs on CI nightly, not per-PR.

---

# Tier 3 — Feature completeness (~1 week)

## T3.1 — `tok0 doctor` diagnostic command

**Problem:** `tok0 status` is read-only. Users need a command that _diagnoses and fixes_ common problems (stale hooks, missing config dir, unrecognized rules).

**Files:**
- Add: `src/insights/doctor.rs`
- Modify: `src/insights/mod.rs` (register module)
- Modify: `src/main.rs` (add `Commands::Doctor` variant)

- [ ] **Step 1: Write doctor module structure**

```rust
// src/insights/doctor.rs
use anyhow::Result;

pub struct Diagnostic {
    pub name: String,
    pub status: CheckStatus,
    pub detail: String,
    pub fix_hint: Option<String>,
}

pub enum CheckStatus { Ok, Warn, Error }

pub fn run() -> Result<()> {
    let mut diagnostics = Vec::new();
    diagnostics.push(check_config_dir());
    diagnostics.push(check_db_writable());
    diagnostics.push(check_hooks_not_stale());
    diagnostics.push(check_extensions_loadable());
    diagnostics.push(check_trust_marker_exists_where_rules_do());
    print_report(&diagnostics);
    Ok(())
}

// ... implement each check_* returning a Diagnostic.
```

- [ ] **Step 2: Write tests for each check**

```rust
#[test]
fn test_check_config_dir_ok_when_exists() { ... }
#[test]
fn test_check_db_writable_reports_error_if_readonly() { ... }
// ... one test per check
```

- [ ] **Step 3: Wire into main.rs**

```rust
/// Diagnose common configuration problems
Doctor,
// ...
Commands::Doctor => run_meta(insights::doctor::run),
```

- [ ] **Step 4: Integration test**

Add to `tests/cli_integration.rs`:

```rust
#[test]
fn test_doctor_command_runs() {
    let (_tmp, db) = isolated_env();
    let out = run_tok0(&["doctor"], &db);
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("config dir") || stdout.contains("Diagnostic"));
}
```

- [ ] **Step 5: Commit**

```bash
git add src/insights/doctor.rs src/insights/mod.rs src/main.rs tests/cli_integration.rs
git commit -m "feat(doctor): add tok0 doctor diagnostic command"
```

## T3.2 — Rule management CLI

**Files:**
- Add: `src/insights/rules_cli.rs`
- Modify: `src/main.rs` (add `Commands::Rule` with subcommands)

- [ ] **Step 1: Add subcommands enum**

```rust
#[derive(Subcommand)]
enum RuleCommands {
    /// List all active rules (built-in + project + extensions)
    List,
    /// Show a single rule's config by name
    Show { name: String },
    /// Test a rule against sample input from stdin
    Test { name: String },
}
```

- [ ] **Step 2: Implement each subcommand**

- `List`: enumerate builtins + extensions + project rules, print name + command patterns.
- `Show`: find rule by name; print the parsed FilterConfig.
- `Test`: read stdin, apply `apply_filter_config(&input, rule)`, print before/after + savings %.

- [ ] **Step 3: Tests**

Snapshot tests for output format; integration tests spawning the binary.

- [ ] **Step 4: Commit**

```bash
git commit -m "feat(rules): add tok0 rule list/show/test subcommands"
```

## T3.3 — Hook health check (folded into `tok0 doctor`)

As part of T3.1, extend `check_hooks_not_stale` to verify:

- For Claude Code: parse `settings.json`, confirm the tok0 `PreToolUse` hook command is still `tok0 rewrite`.
- For instruction-file tools: confirm the file still contains `HOOK_MARKER`.

Report a `Diagnostic::Warn` with a `fix_hint` pointing at `tok0 init` to reinstall.

## T3.4 — Extend `tok0 discover` beyond Claude Code

**Files:**
- Modify: `src/scanner/session_reader.rs` — add readers for other tools' history files (if they exist).

**Scope:** Currently only `~/.claude/projects/*/jsonl` is parsed. Add:
- Gemini CLI: parse `~/.gemini/sessions/*.json` (format: research first)
- Cursor: check for a parseable history file (none known publicly — may be punt)
- Codex/OpenCode/Amp: AGENTS.md tools — their history format needs research

**This task needs its own research spike before coding.** Blocked on docs review.

## T3.5 — Auto-install shell completions

**Files:**
- Modify: `src/main.rs::Commands::Init` — add a step to install completions for the user's shell.

- [ ] **Step 1: Detect user's shell via `$SHELL`**

```rust
fn detect_shell() -> Option<&'static str> {
    match std::env::var("SHELL").ok()?.as_str() {
        s if s.ends_with("/bash") => Some("bash"),
        s if s.ends_with("/zsh") => Some("zsh"),
        s if s.ends_with("/fish") => Some("fish"),
        _ => None,
    }
}
```

- [ ] **Step 2: On `tok0 init`, also run equivalent of `tok0 completions <shell>` into the standard location**

- bash: `~/.local/share/bash-completion/completions/tok0`
- zsh: `~/.zsh/completions/_tok0`
- fish: `~/.config/fish/completions/tok0.fish`

Ask for confirmation in the wizard; non-wizard `tok0 init` prints a note pointing at `tok0 completions`.

- [ ] **Step 3: Tests + commit**

---

# Tier 4 — UX polish (~2 days)

Small, low-risk items. Ship in v0.2.0.

## T4.1 — Once-per-session untrusted hint

**File:** `src/main.rs::load_project_local_rules`

Use a `std::sync::OnceLock<()>` to ensure the "skipped N project rule(s)" stderr message prints only once per process:

```rust
static UNTRUSTED_HINT_SHOWN: OnceLock<()> = OnceLock::new();
// ...
if count > 0 {
    UNTRUSTED_HINT_SHOWN.get_or_init(|| {
        eprintln!("tok0: skipped {} project rule(s) (untrusted — run `tok0 trust`)", count);
    });
}
```

**Each tok0 invocation IS a new process**, so this only affects cases where `get_extension_rules()` is called multiple times (rare but possible if something iterates). Primary benefit: future-proof.

## T4.2 — Call `meter::flush()` in `run_raw_proxy` too

Currently `tok0 proxy <cmd>` doesn't record metrics at all (by design) — but if we ever do, flush must be called. Add `engine::meter::flush()` at the end of `run_raw_proxy` as a defensive no-op. Documents intent.

## T4.3 — Cline post-install panel-toggle prompt

**File:** `src/bridge/setup.rs` — add a second `is_post_install_manual_step_required(tool)` helper, extend the printed hint in wizard + `tok0 init` for Cline:

> "⚠  After install: open VS Code, click the Cline icon, open the Rules panel, and toggle `tok0.md` ON."

## T4.4 — CHANGELOG + release runbook

**Files:**
- Add: `CHANGELOG.md` (start with v0.1.0 entries from git log)
- Add: `docs/RELEASING.md` — step-by-step release guide:

```markdown
# Releasing tok0

1. `cargo test && cargo clippy --all-targets`
2. Bump version in `Cargo.toml` + commit
3. Update `CHANGELOG.md`
4. `git tag vX.Y.Z && git push origin vX.Y.Z`
5. Wait for release workflow (~7 min)
6. `./scripts/update-formula.sh vX.Y.Z`
7. `git add Formula/tok0.rb && git commit && git push`
8. (If tap exists) `./scripts/sync-formula.sh`
```

## T4.5 — CONTRIBUTING.md expansion

Current `CONTRIBUTING.md` is 976B. Expand with:
- Dev setup (rustup, cargo, insta)
- How to add a compressor (pattern from `rust-dev` skill)
- How to add a TOML rule
- Test expectations
- PR flow

---

# Tier 5 — Operational / CI (~1 week)

Ship in v0.3.0. Needs a separate plan to execute.

## T5.1 — Upgrade deprecated GitHub Actions

**Files:** `.github/workflows/ci.yml`, `.github/workflows/release.yml`

- `actions/checkout@v4` → `@v5` (Node 24)
- `softprops/action-gh-release@v2` → `@v3` (Node 24)
- Test a release candidate in a throwaway branch/tag before merging.

## T5.2 — Windows integration test coverage

CI runs on Windows but `tests/cli_integration.rs` was written for Unix.
- Audit assumptions (paths, `HOME`, shell).
- Add `#[cfg(windows)]` variants where needed.

## T5.3 — Linux glibc build variant

Add matrix entry to `release.yml` for `x86_64-unknown-linux-gnu`. Update `install.sh` to prefer glibc on systems where ldd reveals GNU libc, fall back to musl.

## T5.4 — Auto-update Homebrew formula post-release

In `release.yml`, add a final job that runs `./scripts/update-formula.sh` and pushes the updated formula to the tap repo (once T0.2 is done).

---

# Tier 6 — Distribution expansion (~2 weeks per channel)

v0.3.0+. Each needs its own plan.

## T6.1 — scoop manifest

Windows alternative to Chocolatey. Single JSON manifest in a bucket repo.

## T6.2 — chocolatey package

Windows. Submission to chocolatey.org.

## T6.3 — apt repo / .deb package

Linux. `cargo deb` crate can generate from `Cargo.toml` metadata.

## T6.4 — snap

Linux universal. `snapcraft.yaml` + snap store submission.

## T6.5 — npm wrapper package

Thin JS wrapper that downloads the right binary post-install. Common pattern (esbuild, prettier use it).

## T6.6 — docker image

Multi-stage build with musl binary. Useful for CI/CD pipelines.

---

# Self-review notes

- **Coverage:** Every item from the "missing or half-baked" audit is mapped to a task.
  - Install blockers → T0.1–T0.3
  - Security → T1.1–T1.4
  - Test coverage → T2.1–T2.5
  - Feature completeness → T3.1–T3.5
  - UX polish → T4.1–T4.5
  - CI/operations → T5.1–T5.4
  - Distribution → T6.1–T6.6
- **Type consistency:** `install_from_url` gains a third arg (`pin`) in T1.3; all callers in `src/main.rs::ExtCommands::Install` must be updated in the same commit.
- **Placeholder scan:** T2.5 (fuzzing) and T3.4 (extend discover) and T6.1–T6.6 are explicitly scoped-only. Every tier-0/1/2/3/4 task has real code or exact file/line references.
- **Dependencies:**
  - T1.2 depends on `zeroize` already added ✅
  - T1.3 depends on git being installed (already required)
  - T1.4 adds `keyring` crate — must verify binary-size impact (currently <5MB target)
  - T2.1/T2.2 add `mockito` dev-dep only
  - T2.4 adds `criterion` dev-dep; requires `src/lib.rs` to exist

---

# Recommended execution batches

**Batch 1 (next 3 days):** T1.1, T1.2, T2.1, T2.2, T2.3, T4.1, T4.2. Zero infra changes, all local code + tests. Ship as v0.2.0-rc1.

**Batch 2 (week 2):** T1.3, T1.4, T3.1, T3.2, T4.3, T4.4, T4.5. Adds keyring dep + doctor command. Ship as v0.2.0.

**Batch 3 (month 2):** T0.1, T0.2, T0.3 (user-owned external actions). T5.1, T5.2, T5.3, T5.4. Ship as v0.3.0.

**Batch 4 (later):** T3.3 (rolled into T3.1), T3.4 (needs research), T3.5, T2.4 (after architecture settles), T2.5, T6.x.

# tok0 — guide for AI-assisted contributions

This file is loaded automatically by Claude Code, Cursor, Cline, and other
AI coding tools when present at the repo root. It captures the
non-obvious rules so an AI assistant can produce changes that pass CI on
the first try.

## Per-app guidance

The repository is a monorepo:

- **CLI work** (`apps/cli/`): the rest of this file describes the CLI's Rust conventions. Apply only when editing files under `apps/cli/`.
- **API work** (`apps/api/`): see `apps/api/CLAUDE.md` (added in Phase 3 of the monorepo restructure).
- **Website work** (`apps/website/`): see `apps/website/CLAUDE.md` (added in Phase 5).

The Rust rules below (no async, `.context()?`, `lazy_static!` regex, fallback pattern) apply *only* to `apps/cli/` and crates beneath it. They explicitly do NOT apply to TypeScript code in `apps/api/` or `apps/website/`.

## What tok0 is

A high-performance CLI proxy that compresses verbose shell command
output before it reaches an LLM. Typical savings 60–90% on git, cargo,
npm, docker, kubectl, and ~250 other commands. Pure Rust binary,
single-threaded, ≤8 MB stripped.

## Non-negotiable rules

These override general Rust conventions; CI enforces all of them.

1. **No async runtime.** Zero `tokio`, `async-std`, `futures`. Single-threaded
   by design. Async adds 5–10 ms startup we can't afford.
2. **`.context()?` on every `?`.** anyhow with a static or dynamic context
   string. Bare `?` only inside `lazy_static!` or test code.
3. **All regex via `lazy_static!`.** Never `Regex::new()` in a function
   body — recompiles on every call.
4. **No `unwrap()` in production paths.** `lazy_static!` init is
   acceptable (programming-error trap), `expect("reason")` in tests.
5. **Fallback pattern in compressors.** If the filter errors, pass
   through the raw output rather than blocking the user.
6. **Exit-code propagation.** `std::process::exit(code)` when the
   underlying command fails.

Full conventions: see `.claude/skills/rust-dev/SKILL.md`.

## Quality gate (run before every commit)

```bash
cargo fmt --all && \
cargo clippy --all-targets -- -D warnings && \
cargo test
```

CI also runs the same with `--features cloud`. Both must pass.

Local toolchain: `rustup update stable` to match CI's 1.95+. Older
clippy versions miss lints CI catches.

## Module map

```
src/
  main.rs                 Clap Commands enum + run_proxy / run_meta
  bridge/                 Hook installers, trust gating, wizard, integrity
  engine/                 Shared infra: config, meter, dispatcher, rules,
                          builtin_rules, telemetry (gated), updater,
                          sanitize, shell, timeout
  compressors/            Per-command native filters (git, rust, js, ...)
  insights/               status, doctor, stats, profiler, rules_cli
  scanner/                opportunity, session_reader, command_catalog
  extensions/             User-installable rule packs from git URLs
  rules/                  Built-in TOML compression rules (embedded)
hooks/                    Static per-tool template files (embedded
                          via include_str! in bridge/setup.rs)
packaging/                npm / scoop / chocolatey / snap manifests
scripts/                  test-install-sh.sh, update-formula.sh, …
```

Key entry points:
- [src/main.rs](src/main.rs) — `Commands` enum, `run_proxy`, `run_meta`.
- [src/bridge/setup.rs](src/bridge/setup.rs) — `install_hook_at`, `detect_tools_in`, `config_dir_for`, `post_install_hint`.
- [src/engine/dispatcher.rs](src/engine/dispatcher.rs) — routes a command + args to the right compressor or TOML rule.
- [src/engine/meter.rs](src/engine/meter.rs) — async mpsc metering, `flush()` semantics.
- [src/insights/doctor.rs](src/insights/doctor.rs) — diagnostic checks.

## Feature flags

- `default = []` — minimal build. `tok0 auth`, `tok0 cloud`,
  `tok0 telemetry` are *not* compiled in. Telemetry ping is a no-op.
- `cloud = []` — adds the cloud subcommands and the real telemetry
  ping. Requires the `api.tok0.dev` backend (currently not deployed —
  see [ROADMAP.md](ROADMAP.md) Tier 0).

CI runs both configurations.

## When asked to add a compressor

Prefer a TOML rule first (under `src/rules/<cmd>.toml`) — they're
compiled into the binary by `build.rs` and cost nothing at runtime.
Only write a Rust `*_cmd.rs` module if the logic needs structured
parsing (multi-pass, AST-aware, JSON reshaping).

Recipe for a Rust compressor:
1. Capture a real fixture: `<cmd> 2>&1 > tests/fixtures/<eco>/<cmd>_raw.txt`.
2. Create `src/compressors/<eco>/<cmd>_cmd.rs` with `pub fn run(args: <Args>) -> Result<()>`.
3. All regex inside `lazy_static!`.
4. Add `assert_snapshot!` + token-savings ≥60% test.
5. Wire into [src/main.rs](src/main.rs) `Commands` enum and
   [src/engine/dispatcher.rs](src/engine/dispatcher.rs) match.

Reference: `.claude/skills/rust-dev/SKILL.md` and `.claude/skills/tdd/SKILL.md`.

## When asked to add an AI tool integration

Read the tool's *current* docs to find the canonical config-file path —
do not guess. As of 2026-04 the supported set lives in
[src/bridge/setup.rs](src/bridge/setup.rs) `config_dir_for()` and
`instructions_filename()`. Adding a new tool means:

1. New variant in `ToolTarget`.
2. `config_dir_for()` + `instructions_filename()` + `tool_slug()` cases.
3. `detect_tools_in()` signal (config dir or `which_binary()`).
4. `post_install_hint()` if the tool needs a manual step (paste into
   Settings, toggle in panel, …).
5. Parametric test in `bridge::setup::tests::all_instruction_tools()`.

## Plan-execution protocol

The repo's plan lives in [ROADMAP.md](ROADMAP.md) (post-v0.1
hardening). When asked to "implement Tier N" or "Phase N", follow the
steps in order — they're already TDD-shaped (test, run, implement,
verify, commit). Don't deviate without flagging.

## Commit style

Conventional prefixes: `feat:`, `fix:`, `test:`, `docs:`, `chore:`,
`perf:`, `refactor:`, `style:`. Optional scope: `fix(bridge): …`.
First line ≤72 chars. Body explains *why*. Co-authorship lines at the
end. release-please drives changelog generation off these prefixes.

## What to avoid

- New runtime dependencies without checking binary-size impact (`cargo bloat`).
- Writing files outside the user's documented config + cache dirs.
- Adding network calls without a timeout (`engine::shell::execute_with_timeout`).
- Introducing async — see rule 1.

# Contributing to tok0

Thanks for considering a contribution. tok0 is a performance-critical
CLI proxy, so we're strict about a few things (no async, `.context()`
on every `?`, regexes in `lazy_static!`). This doc gets you set up and
points at the conventions.

## Dev setup

```bash
git clone https://github.com/prxm-labs/tok0
cd tok0
cargo build
cargo test
```

Use the **latest stable** Rust (CI pins 1.95+). Update via
`rustup update stable`. Local `rustc --version` must match what CI
runs, or you'll see clippy warnings only on CI.

## Quality gate

Every PR must pass all three:

```bash
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test --all
```

Zero clippy warnings. Zero failing tests. `fmt --check` passes.

CI runs the same gate on macOS, Linux, and Windows. If it passes
locally on one OS but fails on another (typically path-separator
bugs), fix and re-push.

## Project layout

```
src/
  main.rs                 — Clap Commands enum + dispatch
  bridge/                 — Hook install, trust gating, wizard
  engine/                 — Shared infra: config, meter, dispatcher,
                            rules, telemetry, updater
  compressors/            — Per-command filters (git, cargo, npm, …)
  extensions/             — Extension loading + catalog
  insights/               — stats / status / doctor / rules_cli
  rules/                  — 58+ embedded TOML compression rules
  scanner/                — Session history scanning
tests/
  fixtures/               — Real command output captured for tests
  cli_integration.rs      — End-to-end binary tests
  edge_cases_test.rs      — Cross-compressor edge cases
```

The project also has skills under `.claude/skills/` that codify
conventions and testing patterns. Read the one for your area before
writing code.

## Adding a compressor (Rust)

For non-trivial parsing (grouping, state machines, structured output):

1. Capture a real fixture first:

   ```bash
   <command> 2>&1 > tests/fixtures/<ecosystem>/<cmd>_raw.txt
   ```

2. Create `src/compressors/<ecosystem>/<cmd>_cmd.rs`. The standard
   shape is `run()` → `execute_command()` → `filter_<cmd>()` →
   `meter::record()` → `print!()`. Look at any existing `*_cmd.rs`
   for the pattern — `src/compressors/git/git_cmd.rs` is a good
   reference.

3. Regex rules:
   - **All** regexes must be compiled once via `lazy_static!` — never
     `Regex::new()` inside a function body.
   - Use `\\s` in patterns only if you actually mean Unicode whitespace;
     otherwise prefer explicit character classes.

4. Error handling:
   - `?` must be followed by `.context(...)` wherever the error might
     surface to the user. See `anyhow::Context` usage throughout.
   - No `unwrap()` in production paths. `expect("...")` is acceptable
     in tests with a specific reason string.

5. Add tests:
   - Snapshot of the filter output via `insta::assert_snapshot!(...)`.
   - Token-savings assertion ≥60% (ecosystem-specific targets in
     `.claude/skills/tdd/SKILL.md`).
   - Edge cases: empty input, malformed input, unicode, ANSI.

6. Register the subcommand in `src/main.rs` (`Commands` enum + match
   arm) and in `src/engine/dispatcher.rs` if it's hookable.

## Adding a TOML rule (simpler)

If the compression is just pattern stripping + head/tail truncation,
skip the Rust module and write a TOML rule. They're embedded at
compile time, so they cost nothing at runtime.

1. Create `src/rules/<cmd>.toml`:

   ```toml
   [filter]
   name = "<cmd>"
   commands = ["<cmd>"]
   strip_patterns = [
     "^\\s*$",
     "^progress: ",
   ]
   head_lines = 30
   tail_lines = 10
   empty_message = "<cmd>: ok"
   ```

2. `build.rs` picks it up automatically on the next build.

3. Test end-to-end via `tok0 rule test <name>` against real command
   output piped in.

## Tests

```bash
cargo test                    # all unit + integration + bin tests
cargo test compressors::git   # narrow to a module path
cargo test -- --nocapture     # see stdout/eprintln from tests
cargo test --test cli_integration   # end-to-end binary tests
```

Snapshot tests live under `src/**/snapshots/`. Review and accept with
`cargo insta review`. Never check in an un-reviewed snapshot.

## Adding an AI tool integration

To make tok0 install a hook into a new AI tool:

1. Read the tool's *current* docs to find the actual file path it
   auto-loads. A lot of tools changed paths in 2026 — don't guess.
2. Add a variant to `ToolTarget` in `src/bridge/setup.rs`.
3. Extend `detect_tools_in()`, `config_dir_for()`,
   `instructions_filename()`, and `tool_slug()` with the new tool.
4. If the tool doesn't auto-load a file (GUI-only rules), wire
   `post_install_hint()` to return a paste-into-Settings message.
5. Cover every new branch with a test under `bridge::setup::tests` —
   the `all_instruction_tools()` helper is parametric; add your
   tool there.

## Commit style

- Conventional-style prefixes: `feat:`, `fix:`, `test:`, `chore:`,
  `docs:`, `refactor:`, `perf:`, `style:`. Scope optional but
  welcome: `fix(bridge): …`.
- First line under 72 chars.
- Body explains **why**, not what. The diff shows what.
- Co-authorship lines (e.g. for LLM-assisted work) go at the end.

## Releasing

See [RELEASING.md](RELEASING.md).

## Getting help

Open an issue on GitHub with:

- What you tried (exact command)
- What you expected
- What happened (paste the output)
- Output of `tok0 doctor` (very helpful)

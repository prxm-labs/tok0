# Per-tool installer templates

Each subdirectory holds the exact bytes that `tok0 init` writes to that tool's config location. Editing a file here is the only way to change what that tool sees; no Rust code is involved.

## How it ties to the binary

[`src/bridge/setup.rs`](../src/bridge/setup.rs)'s `generate_instructions_for(tool)` matches each `ToolTarget` to one file under this tree using `include_str!`, so the bytes ship in the compiled binary. There's no runtime filesystem read; templates aren't downloaded or fetched.

`bridge::setup::tests::test_installed_file_equals_external_template` asserts the installed bytes equal the embedded template byte-for-byte. If you change a template here, that test guards against silent drift.

## Layout

```
hooks/
├── claude/      Claude Code (the hook is a JSON entry, not a file)
├── cursor/      tok0.md, GUI-only reference doc
├── gemini/      GEMINI.md, auto-loaded by Gemini CLI
├── windsurf/    global_rules.md, Cascade Memories
├── cline/       tok0.md, Rules folder (user toggles on in panel)
├── amp/         AGENTS.md, auto-loaded by Sourcegraph Amp
├── opencode/    AGENTS.md, auto-loaded by sst/opencode
├── codex/       AGENTS.md, manual install only
└── kimi/        tok0.md, GUI-only reference doc
```

## Editing rules

1. Keep the literal `tok0` string somewhere in every template. Both uninstall and `tok0 doctor` look for that marker.
2. No interpolation. What you write is what gets installed; no `{{version}}` placeholders.
3. Trailing newlines matter. The equality test compares byte-for-byte.

## Adding a new tool

See "When asked to add an AI tool integration" in [`../CLAUDE.md`](../CLAUDE.md). TL;DR:

1. Create `hooks/<new-tool>/<filename>` with the canonical content.
2. Add a `ToolTarget::<NewTool>` variant.
3. Wire `config_dir_for`, `instructions_filename`, `tool_slug`, `detect_tools_in`, optionally `post_install_hint`.
4. Add an `include_str!` arm in `generate_instructions_for`.
5. Extend the parametric tests in `bridge::setup::tests`.

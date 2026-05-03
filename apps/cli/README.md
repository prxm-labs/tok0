# tok0

The CLI crate. The repo is a monorepo; see the [root README](../../README.md) for the API and website. To contribute, read [CONTRIBUTING.md](../../CONTRIBUTING.md).

Token-optimized CLI proxy. 60–90% savings on LLM dev operations.

## Install

```bash
curl -fsSL https://raw.githubusercontent.com/tok0/tok0/main/install.sh | bash
```

Or with Homebrew:

```bash
brew install tok0
```

## Quick start

```bash
tok0 init      # install hooks into detected AI tools
tok0 status    # show hooks, savings, trust, extensions, telemetry
tok0 stats     # view token savings
```

## Supported AI tools

`tok0 init` auto-detects installed tools and writes the right hook or instruction file. Re-run any time; it's idempotent.

| Tool         | Mechanism             | Path                                            |
|--------------|-----------------------|-------------------------------------------------|
| Claude Code  | PreToolUse hook       | `~/.claude/settings.json`                       |
| Gemini CLI   | Auto-loaded GEMINI.md | `~/.gemini/GEMINI.md`                           |
| Windsurf     | Cascade Memories      | `~/.codeium/windsurf/memories/global_rules.md`  |
| Cline        | Rules folder (toggle) | `~/Documents/Cline/Rules/tok0.md`               |
| Amp          | Auto-loaded AGENTS.md | `~/.config/amp/AGENTS.md`                       |
| OpenCode     | Auto-loaded AGENTS.md | `~/.config/opencode/AGENTS.md`                  |
| Codex        | Auto-loaded AGENTS.md | `~/.codex/AGENTS.md` (manual install only)      |
| Cursor       | GUI-only (reference)  | `~/.cursor/tok0.md` (paste into Settings)       |
| Kimi Code    | GUI-only (reference)  | `~/.kimi/tok0.md` (paste into chat system)      |

GUI-only tools don't auto-load instruction files. tok0 drops a reference doc and prints a paste-in hint. Codex isn't auto-detected; install it via the wizard if you use it.

Cline's Rules folder also has to be toggled on inside the Cline panel (VS Code). The extension scans the folder but only applies rules you've enabled.

## Common commands

```bash
tok0 init                  # install hooks
tok0 init --uninstall      # remove hooks (preserves other settings)
tok0 init --show           # list detected tools without installing
tok0 status                # status snapshot
tok0 stats                 # savings summary
tok0 stats --graph         # include daily savings chart
tok0 trust / untrust       # control project-local rule loading
tok0 ext install <url>     # install a rule extension from a git URL
tok0 update                # check for and install updates
tok0 telemetry off         # opt out of anonymous telemetry
```

## Trust model

Project-local rules in `.tok0/filters/*.toml` only load when the project is trusted. Run `tok0 trust` in the project directory to enable, `tok0 untrust` to revoke.

## License

MIT.

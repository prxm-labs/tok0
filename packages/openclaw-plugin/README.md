# tok0 plugin for OpenClaw

Rewrites shell commands executed via OpenClaw's `exec` tool to their tok0 equivalents. 60–90% LLM token savings.

The OpenClaw counterpart to the AI-tool hooks in [`hooks/`](../hooks/) (Claude Code's PreToolUse hook, Gemini's `GEMINI.md`, etc.).

## How it works

The plugin registers a `before_tool_call` hook on `exec`. When the agent runs `git status`, the plugin asks `tok0 rewrite` for the optimized form (`tok0 git status`) and substitutes that. The compressed output is what reaches the model's context window.

All rewrite logic lives in tok0 itself ([`src/bridge/rewriter.rs`](../src/bridge/rewriter.rs)). The plugin is thin glue; new filters added to tok0 work automatically with no changes here.

## Installation

`tok0` must be installed and on `$PATH`:

```bash
brew install prxm-labs/tok0/tok0
# or
curl -fsSL https://raw.githubusercontent.com/prxm-labs/tok0/main/install.sh | sh
```

Install the plugin:

```bash
mkdir -p ~/.openclaw/extensions/tok0-rewrite
cp openclaw/index.ts openclaw/openclaw.plugin.json ~/.openclaw/extensions/tok0-rewrite/
openclaw gateway restart
```

Or via the OpenClaw CLI:

```bash
openclaw plugins install ./openclaw
```

## Configuration

In `openclaw.json`:

```json5
{
  plugins: {
    entries: {
      "tok0-rewrite": {
        enabled: true,
        config: {
          enabled: true,     // toggle rewriting on/off
          verbose: false,    // log rewrites to stderr
          timeout_ms: 2000   // fall back to the original command after N ms
        }
      }
    }
  }
}
```

## What gets rewritten

Anything `tok0 rewrite` recognises: about 250 commands routed through the dispatcher. See the `Commands` enum in [`src/main.rs`](../src/main.rs) and `REWRITE_MAP` in [`src/bridge/rewriter.rs`](../src/bridge/rewriter.rs).

Skipped:

- Commands already prefixed with `tok0` (no double-wrap).
- Heredocs (`<<EOF ...`).
- Empty argv.

Pipes (`|`, `&&`, `;`) pass through with `tok0` prepended once. tok0 relies on the shell to split them, not the plugin.

## Measured savings

| Command          | Token savings |
|------------------|---------------|
| `git log --stat` | ~85%          |
| `git status`     | ~75%          |
| `cargo test`     | ~85%          |
| `npm install`    | ~75%          |
| `ls -la`         | ~70%          |
| `grep -r`        | ~50%          |

Numbers come from the fixtures under [`tests/fixtures/`](../tests/fixtures/) and vary with input volume.

## Troubleshooting

If you see "tok0 binary not found in PATH, plugin disabled", install tok0 per the prerequisites above. The plugin runs `tok0 --version` once at registration and caches the result.

If rewrites don't appear in verbose logs, confirm `verbose: true` in `openclaw.json` and restart the OpenClaw gateway after the config change.

For slow rewrites, bump `timeout_ms`. The 2000 ms default is generous for the in-memory rewrite path; it only matters if `tok0` itself cold-starts from a slow filesystem.

## License

MIT, same as tok0.

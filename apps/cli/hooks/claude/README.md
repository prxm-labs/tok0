# Claude Code

Adds a `PreToolUse` hook to `~/.claude/settings.json` (JSON, not markdown; no template file in this directory):

```json
{
  "hooks": {
    "PreToolUse": [
      {
        "matcher": "",
        "hooks": [
          { "type": "command", "command": "tok0 rewrite" }
        ]
      }
    ]
  }
}
```

Other matchers and top-level keys are preserved. Re-running `tok0 init` is a no-op once "tok0" appears anywhere in the file.

Claude Code calls the hook before every Bash tool invocation.

`tok0 init --uninstall` strips just the tok0 entry from `PreToolUse[]`.

Source: `install_claude_code` / `uninstall_claude_code` in [`src/bridge/setup.rs`](../../src/bridge/setup.rs).

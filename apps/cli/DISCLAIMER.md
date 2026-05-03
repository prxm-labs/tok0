# Disclaimer

## Compression is best-effort

tok0 compresses verbose shell command output before it reaches your
LLM. The compression is heuristic — built from regex patterns, line
selection rules, and per-command native filters. It is **not a
guaranteed-lossless transformation.**

In particular:

- A compressor may strip a line that turns out to be relevant to your
  question.
- A novel error format from a tool you use may not be matched by any
  built-in rule and will fall through to a generic head/tail truncation.
- Token-savings percentages reported by `tok0 stats` are computed
  against a whitespace-token approximation, not the exact tokenizer
  your LLM uses.

If a command's output is critical and you need every byte, run it
without the proxy: `tok0 init --uninstall` for a single tool, or
prefix the command with the actual binary path to bypass the hook
once.

## Telemetry

The default build (`cargo install tok0`) does **not** send any
telemetry — the `cloud` feature is off, and the network ping is a
compile-time no-op. The `tok0 telemetry`, `tok0 auth`, and
`tok0 cloud team` subcommands are not present in the default build's
`--help`.

Builds with `--features cloud` and `tok0 telemetry on` send a single
daily anonymous payload to `api.tok0.dev/telemetry`:

```
{
  "installation_id": "<sha256(username + config_dir)>",
  "version": "x.y.z",
  "os": "macos-aarch64",
  "total_commands": <count>,
  "total_saved_tokens": <count>,
  "avg_savings_pct": <float>
}
```

No command contents, paths, environment variables, or output samples
are sent. The `installation_id` is a one-way hash that can't be
reversed to identify a user, but it's stable across runs on the same
machine.

You can disable telemetry at any time: `tok0 telemetry off`.

## Liability

tok0 is provided "as is" under the MIT License (see
[LICENSE](LICENSE)). The authors and copyright holders are not
liable for:

- Output that is filtered, truncated, or transformed in a way that
  produces incorrect LLM responses.
- Costs incurred from API calls whose outcomes were affected by
  compression artifacts.
- Hook installations that interact unexpectedly with future versions
  of your AI tool's settings format.
- Data loss in the metering database (`~/.local/share/tok0/tracking.db`)
  — this is best-effort analytics, not transactional storage.

If you're using tok0 in a setting where output fidelity matters more
than token cost (regulated environment, audit trail, etc.), test the
compression against your most demanding commands before relying on
it in production.

## Trademark and brand

"tok0" is a project name. The names of AI tools mentioned in this
project (Claude Code, Cursor, Windsurf, Cline, Amp, OpenCode, Gemini
CLI, Kimi Code, Codex) are trademarks of their respective owners.
tok0 is not affiliated with, endorsed by, or sponsored by those
projects.

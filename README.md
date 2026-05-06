<p align="center">
  <a href="https://tok0.dev">
    <img src=".github/assets/logo.svg" alt="tok0" width="96" height="96" />
  </a>
</p>

<h1 align="center">Strip the noise from every shell command<br/><em>before it hits the model.</em></h1>

<p align="center">
  An open-source CLI proxy that compresses verbose shell output
  (<code>git</code>, <code>cargo</code>, <code>npm</code>, <code>docker</code>, <code>kubectl</code>…)
  before it reaches your AI agent.<br/>
  Native Rust · single static binary · ~8 MB · <strong>60–90% savings</strong> on the commands your agent runs most.
</p>

<p align="center">
  <a href="https://tok0.dev"><strong>tok0.dev</strong></a> ·
  <a href="https://tok0.dev/docs/introduction/">Docs</a> ·
  <a href="https://tok0.dev/docs/getting-started/">Quickstart</a>
</p>

---

## Install

```bash
# macOS / Linux
curl -fsSL https://tok0.dev/install.sh | bash

# Homebrew
brew install tok0
```

Then:

```bash
tok0 init      # auto-detects every AI tool you have installed
tok0 status    # confirm hooks, savings, trust, telemetry
tok0 stats     # see how many tokens you saved last week
```

That's it. Every supported shell command compresses from then on. No prompt changes, no workflow changes.

## Why it exists

Coding agents spend tokens on shell output that is mostly noise: progress bars, ANSI codes, repeated banners, dependency trees, vendor warnings. A typical `npm install` is 2–5 KB of useful text wrapped in 30–50 KB of formatting.

`tok0` sits in front of the shell, runs the command, drops the noise, and forwards what the model actually needs. Exit codes pass through. If a filter ever errors, you get the raw output. Nothing breaks.

```
agent → shell command → tok0 → model context
```

## What's in the box

|  | |
|---|---|
| **Deterministic compression pipeline** | Eight pure stages — strip ANSI · pattern replace · output match · line select · truncate · head/tail window · hard cap · empty fallback. Same input, same output, every time. Snapshot-tested with a CI floor on `min_savings_pct`. |
| **120+ declarative TOML rules** | Embedded at build time. Cover git, cargo, npm/pnpm/bun, docker, kubectl, terraform, pytest, brew, apt, and ~250 more. Each rule ships with an asserted savings floor; PRs that drop below it fail to merge. |
| **52 native compressors** | Hand-tuned Rust modules for commands that need real parsing — JSON reshaping, AST-aware filtering, multi-pass analysis. |
| **Drop-in AI tool bridges** | One `tok0 init` installs into Claude Code, Cursor, Gemini CLI, Windsurf, Cline, Amp, OpenCode, Codex, and Kimi Code. |
| **Safe by default** | Single-threaded Rust, zero `tokio`, cold start under 5 ms. Local SQLite meter. No account, no upload, no telemetry until you opt in. |
| **Trust-gated extensions** | Project rules (`.tok0/filters/*.toml`) only load after `tok0 trust`. Bridge hooks are SHA-256 verified on every run; `tok0 verify` catches drift or tampering. |

## Drops into the agent you already use

| Tool         | Mechanism                | Status        |
|--------------|--------------------------|---------------|
| Claude Code  | `PreToolUse` hook        | Auto-install  |
| Gemini CLI   | Auto-loaded `GEMINI.md`  | Auto-install  |
| Windsurf     | Cascade Memories         | Auto-install  |
| Cline        | Rules folder (toggle)    | Auto-install  |
| Amp          | Auto-loaded `AGENTS.md`  | Auto-install  |
| OpenCode     | Auto-loaded `AGENTS.md`  | Auto-install  |
| Codex        | `AGENTS.md` (manual)     | Manual        |
| Cursor       | Settings → Rules paste-in| Paste-in      |
| Kimi Code    | Manual paste-in          | Paste-in      |

`tok0 init` is idempotent — re-run any time you install a new tool. GUI-only tools get a reference doc and a paste-in hint.

## Repo layout

This is a monorepo. Each subtree is independently buildable.

| Path | What's there |
|---|---|
| [`apps/cli/`](apps/cli/) | The Rust CLI binary (the thing you `brew install`). Single crate. |
| [`apps/api/`](apps/api/) | Cloudflare Workers backend at `api.tok0.dev`. Hono + D1 + KV. |
| [`apps/website/`](apps/website/) | Astro site at `tok0.dev`. Cloudflare Pages. |
| [`packages/openclaw-plugin/`](packages/openclaw-plugin/) | OpenClaw IDE plugin (`@prxm-labs/tok0-rewrite` on npm). |

## Working in this repo

- **CLI** — `cd apps/cli && cargo test`. Conventions: [`apps/cli/CLAUDE.md`](apps/cli/CLAUDE.md).
- **API** — `pnpm install && pnpm --filter @tok0/api test`.
- **Website** — `pnpm install && pnpm --filter @tok0/website dev`.

Contributors: start with [`CONTRIBUTING.md`](CONTRIBUTING.md). Security reports: [`SECURITY.md`](SECURITY.md).

## Releases

CLI releases are driven by [release-please](https://github.com/googleapis/release-please) off conventional commits. Each merged release PR cuts a `vX.Y.Z` tag, matrix-builds five binaries plus a `.deb`, updates the Homebrew formula, and pushes a Docker image to GHCR.

API and website ship continuously: every merge to `main` deploys, every PR gets a preview.

## License

MIT — see [LICENSE](LICENSE).

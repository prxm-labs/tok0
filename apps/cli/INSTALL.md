# Installing tok0

tok0 ships as a single static binary. There's no daemon, no service,
no background process to manage — `tok0 init` writes a tiny hook into
your AI tool's config and that's it.

## Quick install

```bash
# macOS / Linux
curl -fsSL https://raw.githubusercontent.com/prxm-labs/tok0/main/install.sh | sh

# Windows (PowerShell)
iwr https://raw.githubusercontent.com/prxm-labs/tok0/main/install.ps1 -UseBasicParsing | iex
```

Then:

```bash
tok0 init       # detect AI tools and install hooks
tok0 status     # verify
tok0 stats      # see your token savings (after a few commands)
```

## Per-channel install

| Channel       | Install command                                    | Status                        |
|---------------|----------------------------------------------------|-------------------------------|
| curl/sh       | see Quick install above                            | Shipping                      |
| Homebrew      | `brew install prxm-labs/tok0/tok0`                 | Pending tap publish (T0.2)    |
| Debian/Ubuntu | Download `.deb` from the GitHub release; `dpkg -i` | Shipping                      |
| Docker        | `docker pull ghcr.io/prxm-labs/tok0:latest`        | Shipping                      |
| npm           | `npm i -g @prxm-labs/tok0`                         | Scaffold (publish pending)    |
| scoop         | `scoop install <raw url>`                          | Scaffold (bucket pending)     |
| chocolatey    | `choco install tok0`                               | Scaffold (submission pending) |
| snap          | `snap install tok0 --classic`                      | Scaffold (snapcraft pending)  |
| Cargo source  | `cargo install --git https://github.com/prxm-labs/tok0` | Always works              |
| OpenClaw      | `openclaw plugins install ./openclaw`              | Plugin manifest in `openclaw/` |

See [packaging/README.md](packaging/README.md) for per-channel publish state.

### Environment variables

`install.sh` reads:

| Var                 | Default            | Purpose                                |
|---------------------|--------------------|----------------------------------------|
| `TOK0_VERSION`      | `latest`           | Pin to a specific release              |
| `TOK0_INSTALL_DIR`  | `/usr/local/bin`   | Where to drop the binary               |

Example: install v0.1.0 into `~/.local/bin`:

```bash
TOK0_VERSION=0.1.0 TOK0_INSTALL_DIR="$HOME/.local/bin" \
  curl -fsSL https://raw.githubusercontent.com/prxm-labs/tok0/main/install.sh | sh
```

The script detects platform (Darwin/Linux × x86_64/aarch64), prefers
glibc on Linux but falls back to musl on Alpine/BusyBox, downloads the
matching binary, verifies its SHA-256, and installs.

### From source

Requires Rust 1.95+ (`rustup update stable`).

```bash
git clone https://github.com/prxm-labs/tok0
cd tok0
cargo install --path .
```

For the cloud subcommands (auth, telemetry ping, team dashboard) — only
useful once the `api.tok0.dev` backend is live:

```bash
cargo install --path . --features cloud
```

### OpenClaw plugin

If you run an OpenClaw gateway, the [`openclaw/`](openclaw/) directory
ships a TypeScript plugin that intercepts every `exec` tool call,
delegates to `tok0 rewrite`, and substitutes the optimized command.
Install:

```bash
openclaw plugins install ./openclaw
# or copy by hand:
mkdir -p ~/.openclaw/extensions/tok0-rewrite
cp openclaw/index.ts openclaw/openclaw.plugin.json \
   ~/.openclaw/extensions/tok0-rewrite/
openclaw gateway restart
```

Configuration lives in `openclaw.json`:

```json5
{
  plugins: {
    entries: {
      "tok0-rewrite": {
        enabled: true,
        config: { enabled: true, verbose: false, timeout_ms: 2000 }
      }
    }
  }
}
```

Full options + troubleshooting in [`openclaw/README.md`](openclaw/README.md).

## Per-AI-tool integration

`tok0 init` detects each tool by its config directory and writes the
appropriate file. Re-runs are idempotent.

| Tool         | What gets written                                       | Auto-loaded? | Manual step                                            |
|--------------|---------------------------------------------------------|--------------|--------------------------------------------------------|
| Claude Code  | `~/.claude/settings.json` (PreToolUse hook)             | Yes          | —                                                      |
| Gemini CLI   | `~/.gemini/GEMINI.md`                                   | Yes          | —                                                      |
| Windsurf     | `~/.codeium/windsurf/memories/global_rules.md`          | Yes          | —                                                      |
| Cline        | `~/Documents/Cline/Rules/tok0.md`                       | No           | Open VS Code → Cline → Rules tab → toggle `tok0.md` ON |
| Amp          | `~/.config/amp/AGENTS.md`                               | Yes          | —                                                      |
| OpenCode     | `~/.config/opencode/AGENTS.md`                          | Yes          | —                                                      |
| Codex        | `~/.codex/AGENTS.md`                                    | Yes          | Run `tok0 init` even though Codex isn't auto-detected  |
| Cursor       | `~/.cursor/tok0.md` (reference doc)                     | No           | Cursor Settings → Rules → paste the file's contents    |
| Kimi Code    | `~/.kimi/tok0.md` (reference doc)                       | No           | Same — paste into Kimi's chat-system prompt            |

`tok0 init` prints the manual step automatically for any tool that
needs one. The wizard (`tok0 init --wizard`) walks you through tool
selection and telemetry opt-in interactively.

### Per-project vs global

```bash
tok0 init              # per-project: writes ./.claude/settings.json (Claude Code only)
tok0 init --global     # all detected tools, written to home dir
tok0 init --show       # list detected tools without installing anything
tok0 init --uninstall  # remove every tok0 hook (preserves your other settings)
```

## Troubleshooting

### `tok0 doctor`

Run this first. It checks config-dir writability, DB path, hook
freshness per detected tool, extension rule validity, and the
project-trust state. Each check returns Ok / Warn / Error with a fix
hint:

```bash
tok0 doctor
```

Common outputs:

- `[ warn] ClaudeCode hook: ... missing the 'tok0' marker` — settings
  file was edited externally; rerun `tok0 init`.
- `[error] extension: foo: failed to load rules: ...` — bad TOML in
  one of your installed extensions; remove with `tok0 ext remove foo`.
- `[ warn] project trust: 3 project rule(s) present but project
  untrusted` — you cloned a repo with `.tok0/filters/`. Run `tok0 trust`
  in that directory if you trust the rules; otherwise they stay disabled.

### Hook fired but compression didn't happen

Order of fallbacks for an unknown command:

1. Native compressor in `src/compressors/`.
2. Built-in TOML rule (any of 70+ files in `src/rules/`).
3. Extension rules from `~/.config/tok0/extensions/`.
4. Project-local rules in `.tok0/filters/` (only if trusted).
5. Raw output passthrough.

Use `tok0 rule list` to see what would match a command name, and
`tok0 rule test <name>` to dry-run a specific rule against piped input.

### `curl | sh` returns 404

Until the GitHub repo is public, the install scripts can't download
release assets anonymously. Either flip the repo to public (see
[RELEASING.md](RELEASING.md) "External setup") or build from source.

### Binary too old / want to upgrade

```bash
tok0 update --check    # see if a newer version is available
tok0 update            # download + replace in place (atomic rename)
```

The update path SHA-256-verifies the new binary before replacing the
running one and rolls back to a `.bak` if the rename fails.

## Uninstall

```bash
tok0 init --uninstall                    # remove hooks from every detected tool
brew uninstall tok0                      # if installed via Homebrew
rm -rf ~/.config/tok0 ~/.local/share/tok0  # remove config + tracking DB
sudo rm /usr/local/bin/tok0              # remove the binary itself
```

The uninstall preserves your AI tools' other settings — only the tok0
hook entries are stripped from `settings.json`, and tok0-only files
(`AGENTS.md`, `tok0.md`, `global_rules.md` if they only contain tok0
content) are removed.

## Reporting install bugs

Include in the issue:

- Output of `tok0 doctor`.
- OS + arch (`uname -a` on Unix, `systeminfo` on Windows).
- The exact `tok0 init` command + its full stdout/stderr.
- Whether the install was via curl, brew, npm, docker, or source.

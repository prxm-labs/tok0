# tok0 — monorepo

Compresses verbose shell command output before it reaches an LLM, plus the cloud backend and the website that document and dashboard both.

## Layout

| Path | Contents |
|---|---|
| [`apps/cli/`](apps/cli/) | The Rust CLI binary (the thing you `brew install`). Single crate. |
| [`apps/api/`](apps/api/) | Cloudflare Workers backend at `api.tok0.dev`. Hono + D1 + KV. |
| [`apps/website/`](apps/website/) | Astro site at `tok0.dev`. Cloudflare Pages. |
| [`packages/openclaw-plugin/`](packages/openclaw-plugin/) | OpenClaw IDE plugin (`@prxm-labs/tok0-rewrite` on npm). |

## Working in this repo

- CLI: `cd apps/cli && cargo test`. Conventions in [`apps/cli/CLAUDE.md`](apps/cli/CLAUDE.md).
- API: `pnpm install && pnpm --filter @tok0/api test`.
- Website: `pnpm install && pnpm --filter @tok0/website dev`.

## Releases

CLI releases are driven by release-please off conventional commits. Each merged release PR cuts a `vX.Y.Z` tag, matrix-builds five binaries plus a `.deb`, updates the Homebrew formula, and pushes a Docker image to GHCR.

API and website ship continuously: every merge to `main` deploys, every PR gets a preview.

## License

MIT. See [LICENSE](LICENSE).

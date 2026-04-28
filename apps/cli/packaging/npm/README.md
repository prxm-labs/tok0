# @prxm-labs/tok0

npm wrapper for [tok0](https://github.com/prxm-labs/tok0), a token-optimized CLI proxy for AI coding tools.

## Install

```bash
npm install -g @prxm-labs/tok0
```

The postinstall script downloads the matching prebuilt binary from GitHub Releases (with SHA-256 verification) and places it in the package's `bin/` directory.

## Usage

Identical to the native binary. See the [main README](https://github.com/prxm-labs/tok0/blob/main/README.md).

```bash
tok0 init          # install hooks into your AI tools
tok0 status        # show what's installed
tok0 stats         # see token savings
```

## Supported platforms

- macOS (Intel and Apple Silicon)
- Linux (x86_64, glibc)
- Windows (x86_64)

## Version sync

The npm version always matches a GitHub Release tag. To upgrade: `npm install -g @prxm-labs/tok0@latest`.

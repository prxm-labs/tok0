# tok0 — Token-Optimized Command Proxy

This project uses tok0 to reduce AI token consumption.

## Usage

Prefix commands with `tok0` to route them through the proxy:

```
tok0 git status
tok0 cargo build
tok0 npm test
```

tok0 compresses verbose command output before it reaches the LLM,
saving 60-90% of tokens on typical development operations.

Run `tok0 stats` to see cumulative token savings.

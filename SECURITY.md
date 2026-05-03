# Security policy

## Supported versions

| Version | Supported          |
| ------- | ------------------ |
| `0.1.x` | :white_check_mark: |
| < `0.1` | :x:                |

Once `0.2.0` ships, only the latest minor will receive security fixes.

## Reporting a vulnerability

**Do not open a public GitHub issue for security problems.**

Use one of:

1. **GitHub Security Advisories** (preferred). Open a private advisory
   at <https://github.com/prxm-labs/tok0/security/advisories/new>.
2. **Email.** Send a description + reproduction to the maintainers at
   `info@vorillaz.com`. Include "tok0 security" in the subject.

We aim to acknowledge within 72 hours and ship a fix or mitigation
within 30 days for confirmed issues. Disclosure is coordinated with
the reporter.

## What counts as a security bug

- Anything that lets an untrusted project, extension, or AI tool
  payload escalate to arbitrary command execution under the user's
  shell (beyond what `tok0 proxy <cmd>` already does intentionally).
- Anything that exfiltrates content of the user's home directory,
  config, or environment variables to a non-local destination.
- Anything that bypasses the project-trust gate
  ([bridge/trust.rs](src/bridge/trust.rs)) so untrusted
  `.tok0/filters/*.toml` rules load silently.
- Sensitive data (API keys, bearer tokens) lingering in process
  memory or written to disk in plaintext beyond the documented
  config locations.
- ReDoS / pathological-regex attacks via untrusted TOML rule patterns.

Bugs in compression accuracy (output truncated unexpectedly) are
correctness issues, not security issues, unless they actively leak data.

## Critical files

Touching any of these in a PR requires an explicit security note in
the description:

| File                                                   | Why it's critical                                  |
| ------------------------------------------------------ | -------------------------------------------------- |
| [src/bridge/integrity.rs](src/bridge/integrity.rs)     | SHA-256 hook verification                          |
| [src/bridge/trust.rs](src/bridge/trust.rs)             | Project-trust enforcement                          |
| [src/bridge/setup.rs](src/bridge/setup.rs)             | Hook installation; writes to user home             |
| [src/extensions/catalog.rs](src/extensions/catalog.rs) | `git clone` of user-supplied URLs; name validation |
| [src/engine/cloud.rs](src/engine/cloud.rs)             | Bearer-token handling, network egress              |
| [src/engine/config.rs](src/engine/config.rs)           | API-credential persistence; zeroize on drop        |
| [src/engine/sanitize.rs](src/engine/sanitize.rs)       | Strips secrets from output                         |
| [install.sh](install.sh) / [install.ps1](install.ps1)  | Privileged install path; checksum verification     |

## Current threat model

- **Trusted:** the user, their shell, the system PATH, the AI tool
  itself, the GitHub Releases the install scripts download from
  (verified by SHA-256).
- **Untrusted:** project directories tok0 runs in, third-party
  extensions installed via `tok0 ext install`, command output piped
  through compressors.

Defenses in place:

- Project rules under `.tok0/filters/` only load after explicit
  `tok0 trust`. Untrusted projects show a one-line stderr hint.
- Extension names are validated against `^[a-zA-Z0-9_-]+$` to block
  `git clone` path traversal.
- Native commands invoke via `std::process::Command` with separate
  argv — no shell interpolation.
- API keys held in `CloudConfig`/`CloudClient` are wiped via `zeroize`
  on drop (in-memory only — file persistence is plaintext until the
  OS-keyring work in [ROADMAP.md](ROADMAP.md) Tier 1 lands).

## Known gaps

Tracked in [ROADMAP.md](ROADMAP.md):

- Bearer-token strings allocated for `format!("Bearer {}", key)` are
  not zeroized after the request returns (Tier 1.2).
- TOML rule regexes have a size cap but no execution-time budget
  (Tier 1.1).
- Extensions clone via `git clone --depth=1` with no signature or
  commit-pin verification by default (Tier 1.3).
- Credentials persist plaintext in `~/.config/tok0/config.toml`;
  OS-keyring migration pending (Tier 1.4).

If you find an issue not on this list, please report it.

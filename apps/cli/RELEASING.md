# Releasing tok0

Two paths: **automated via release-please** (preferred) and **manual
via git tag** (fallback / hotfix). Both paths flow into the same
release.yml build matrix.

## Path A — release-please (default)

`release-please` watches the `main` branch and opens a single
"chore(main): release X.Y.Z" PR that accumulates every conventional
commit since the last release. Cutting a release is just merging that PR.

1. Land your work on `main` using conventional commits (`feat:`,
   `fix:`, `perf:`, etc.). The full commit-prefix → CHANGELOG-section
   mapping lives in [release-please-config.json](release-please-config.json).
2. release-please opens or updates a release PR automatically. Inspect
   the proposed CHANGELOG diff. Edit nothing in the PR — change source
   commit messages on `main` if the changelog is wrong.
3. Merge the release PR. release-please:
   - Bumps `version` in [Cargo.toml](Cargo.toml) and the manifest.
   - Updates `Formula/tok0.rb` (the `version "X.Y.Z"` line — SHA-256s
     are filled in later by the existing `update-formula` job).
   - Pushes the `vX.Y.Z` git tag.
4. The tag push triggers [release.yml](.github/workflows/release.yml).
   Build matrix produces the four-target binaries + `.deb`, uploads to
   the release, and the trailing `update-formula` job patches the real
   Homebrew SHA-256s back into `Formula/tok0.rb`.
5. The same tag push fires [docker.yml](.github/workflows/docker.yml).
   GHCR receives `:vX.Y.Z` and `:latest` images.

That's the entire release. ~7 minutes from merge to a fully-published
GitHub release with all binaries, .deb, Homebrew formula, and Docker
images.

### Known gotcha: tag from release-please doesn't trigger release.yml

GitHub deliberately suppresses workflow runs when a tag (or any ref)
is pushed by the default `GITHUB_TOKEN`. release-please pushes its
tag using exactly that token, so [release.yml](.github/workflows/release.yml)
— which is `on: push: tags: ['v*']` — does **not** fire automatically
after the release PR merges.

Workarounds, in order of preference:

1. **Add a Personal Access Token.** Create a fine-grained PAT with
   `Contents: write` on this repo, store as `RELEASE_PLEASE_TOKEN`,
   and pass it to the action via `with: token: ${{ secrets.RELEASE_PLEASE_TOKEN }}`
   in `.github/workflows/release-please.yml`. Tag pushes from a PAT
   trigger downstream workflows normally.
2. **Manually re-tag from a local checkout** (no PAT needed):
   ```bash
   git fetch --tags origin
   git tag -d vX.Y.Z 2>/dev/null || true
   git push origin :refs/tags/vX.Y.Z 2>/dev/null || true
   git tag vX.Y.Z <release-please-commit-sha>
   git push origin vX.Y.Z
   ```
3. **Trigger release.yml via `workflow_dispatch`** — not currently
   wired; would require adding a `workflow_dispatch` trigger and
   plumbing the version through.

This is tracked in [ROADMAP.md](ROADMAP.md) Tier 5 / Tier 0 follow-ups.

## Path B — manual tag (hotfix / when release-please is unavailable)

Step-by-step runbook for cutting a new release manually. Takes
~10 minutes if everything is green.

## Preconditions

- You are on `main` with a clean working tree.
- The `main` branch is green on CI (all three OS matrix entries).
- You have `gh` authenticated against `prxm-labs/tok0`.
- You have `cargo`, `rustup` with the current stable toolchain.

## 1. Sanity check

```bash
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test
```

All three MUST pass. If any fail, stop and fix first.

## 2. Bump the version

Edit [`Cargo.toml`](Cargo.toml):

```diff
-version = "0.1.0"
+version = "0.2.0"
```

Then refresh the lock file:

```bash
cargo build --release
```

This updates `Cargo.lock` to reflect the new version.

## 3. Update CHANGELOG

In [`CHANGELOG.md`](CHANGELOG.md):

1. Rename the `[Unreleased]` heading to `[X.Y.Z] — YYYY-MM-DD`.
2. Add a fresh empty `## [Unreleased]` section above it.
3. Update the link-reference lines at the bottom:

```diff
-[Unreleased]: https://github.com/prxm-labs/tok0/compare/v0.1.0...HEAD
+[Unreleased]: https://github.com/prxm-labs/tok0/compare/vX.Y.Z...HEAD
+[X.Y.Z]: https://github.com/prxm-labs/tok0/releases/tag/vX.Y.Z
 [0.1.0]: https://github.com/prxm-labs/tok0/releases/tag/v0.1.0
```

## 4. Commit the version bump

```bash
git add Cargo.toml Cargo.lock CHANGELOG.md
git commit -m "chore: release vX.Y.Z"
git push origin main
```

Wait for CI to go green on this commit before tagging.

## 5. Tag and push

```bash
git tag vX.Y.Z
git push origin vX.Y.Z
```

This triggers `.github/workflows/release.yml`. It builds binaries for
four targets, computes `.sha256` companion files, and uploads
everything to a new GitHub Release named `vX.Y.Z`.

## 6. Wait for the release workflow

```bash
gh run watch $(gh run list --workflow=release.yml --limit=1 --json databaseId -q '.[0].databaseId') --exit-status
```

Typical runtime: 5–7 minutes. Fail modes:

- **Cross-compile failure** — usually flaky network; re-run the job.
- **Binary >8 MB** — the size check will fail. Investigate bloat
  with `cargo bloat --release --crates`.

## 7. Verify release assets

```bash
gh release view vX.Y.Z --json assets -q '.assets[].name'
```

You should see exactly 8 entries:

```
tok0-aarch64-apple-darwin
tok0-aarch64-apple-darwin.sha256
tok0-x86_64-apple-darwin
tok0-x86_64-apple-darwin.sha256
tok0-x86_64-pc-windows-msvc.exe
tok0-x86_64-pc-windows-msvc.exe.sha256
tok0-x86_64-unknown-linux-musl
tok0-x86_64-unknown-linux-musl.sha256
```

If any are missing or the names don't have the target triple, something
has regressed in the release workflow — fix and re-tag (delete the
release + tag first with `gh release delete vX.Y.Z --cleanup-tag --yes`).

## 8. Update the Homebrew formula

```bash
./scripts/update-formula.sh vX.Y.Z
```

This downloads the ARM64 + x86_64 macOS `.sha256` files and patches
`Formula/tok0.rb` with the real checksums + version number. Review the
diff:

```bash
git diff Formula/tok0.rb
```

You should see only the `version` string and two `sha256` lines change.

## 9. Commit and push the formula

```bash
git add Formula/tok0.rb
git commit -m "chore(release): update Homebrew formula for vX.Y.Z"
git push origin main
```

## 10. Smoke-test the install

Against a throwaway dir, run a variant of `install.sh` that exercises
the public URLs. The bundled `scripts/test-install-sh.sh` covers this
against a local mock server, but you should also do one real pull:

```bash
TOK0_VERSION=X.Y.Z TOK0_INSTALL_DIR=/tmp/tok0-smoke sh install.sh
/tmp/tok0-smoke/tok0 --version
/tmp/tok0-smoke/tok0 status
rm -rf /tmp/tok0-smoke
```

(Requires the repo to be **public** — private repos return 404 on the
download URLs.)

## External / one-time setup (owner-only)

These are prerequisites that must be done **once** for end-user install
to work. Not needed for every release, but blocking the first public
release.

### Flip the repo to public (T0.1)

Until `github.com/prxm-labs/tok0` is public, `curl install.sh | bash`
and `brew install` both 404 — GitHub's release asset URLs require
authentication on private repos. Settings → General → Danger Zone →
"Change repository visibility".

### Publish a Homebrew tap (T0.2)

Homebrew can't find `Formula/tok0.rb` inside the source repo. Either:

1. **Tap repo (fast path):**
   - Create `github.com/prxm-labs/homebrew-tok0` (public).
   - Copy `Formula/tok0.rb` into its root.
   - Add a GitHub Action to the main repo that pushes the updated
     formula to the tap repo after `update-formula` runs.
   - Consumer install: `brew tap prxm-labs/tok0 && brew install tok0`.

2. **homebrew-core submission (slower):**
   - Wait for project to hit the criteria (non-trivial usage,
     stable for 30+ days, no GitHub issues older than the lifetime
     etc. — see the homebrew-core guidelines).
   - Open a PR against `Homebrew/homebrew-core` adding the formula.
   - Consumer install: `brew install tok0` (no tap needed).

### Distribution channel submissions (Tier 6)

Once visibility is sorted and `Formula/tok0.rb` is in a tap, the other
channels (`packaging/scoop/tok0.json`, `packaging/chocolatey/`,
`packaging/snap/`, `packaging/npm/`) can be published. Each has its own
workflow:

- **scoop** — publish the JSON to a scoop bucket repo.
- **chocolatey** — `choco push` after nuspec validation; takes ~24h
  for choco.org moderation approval.
- **snap** — `snapcraft login && snapcraft upload` after
  `snapcraft pack`.
- **npm** — `cd packaging/npm && npm publish --access public`.

See [ROADMAP.md](ROADMAP.md) Tier 6 for per-channel effort estimates.

## Rollback

If a release is broken:

```bash
# Yank the release (keeps the tag but hides from "latest")
gh release edit vX.Y.Z --draft

# Or fully delete
gh release delete vX.Y.Z --cleanup-tag --yes
```

Then revert the version bump commit, push, and tag a follow-up patch.

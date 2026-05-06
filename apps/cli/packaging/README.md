# Distribution packaging

Each subdirectory holds the manifest and build recipe for one distribution channel. Binaries are built by `.github/workflows/release.yml` and uploaded to GitHub Releases; these files tell per-channel package managers how to consume them.

| Channel    | Path                  | Status   | Consumer install                       |
|------------|-----------------------|----------|----------------------------------------|
| Homebrew   | `../Formula/tok0.rb`  | Shipping | `brew install prxm-labs/tok0/tok0`     |
| Debian     | Cargo.toml metadata   | Shipping | `.deb` attached to each GitHub Release |
| Docker     | `../Dockerfile`       | Shipping | `docker pull ghcr.io/prxm-labs/tok0`   |
| npm        | `npm/`                | Scaffold | `npm i -g @prxm-labs/tok0`             |
| scoop      | `scoop/tok0.json`     | Scaffold | `scoop install <raw-url>`              |
| chocolatey | `chocolatey/`         | Scaffold | (needs choco.org submission)           |
| snap       | `snap/snapcraft.yaml` | Scaffold | (needs snapcraft login + publish)      |

"Scaffold" means the manifest exists in this repo but the channel-side publishing step (tap repo, npm registry, chocolatey submission, etc.) is still external.

## Bumping a version

When cutting a release, the SHA-256 placeholders in several manifests need to be replaced with the real checksums from the new GitHub Release. `scripts/update-formula.sh` handles Homebrew. Add per-channel scripts as each channel goes live.

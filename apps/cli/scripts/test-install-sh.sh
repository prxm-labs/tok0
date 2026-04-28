#!/usr/bin/env bash
# Smoke test for install.sh.
#
# Approach: stand up a local HTTP server mimicking the GitHub release
# download layout, run install.sh against it via BASE_URL override, and
# verify the installed binary works. Repo-visibility-agnostic.
#
# Runs the full install flow: platform detection, checksum verification,
# chmod, move, and final `tok0 --version` sanity check.
#
# Usage: ./scripts/test-install-sh.sh

set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

echo "== Build release binary =="
cargo build --release --quiet

# Detect target triple for this host (install.sh does this via uname)
OS=$(uname -s | tr '[:upper:]' '[:lower:]')
ARCH=$(uname -m)
case "$ARCH" in
  x86_64)        ARCH_NORM="x86_64" ;;
  aarch64|arm64) ARCH_NORM="aarch64" ;;
  *) echo "unsupported arch: $ARCH"; exit 1 ;;
esac
case "$OS" in
  darwin) TARGET="${ARCH_NORM}-apple-darwin" ;;
  linux)
    # Mirror install.sh's glibc-first detection so the staged binary's
    # filename matches the one install.sh will request.
    if ldd --version 2>&1 | grep -qi musl; then
      TARGET="${ARCH_NORM}-unknown-linux-musl"
    else
      TARGET="${ARCH_NORM}-unknown-linux-gnu"
    fi
    ;;
  *) echo "unsupported OS: $OS"; exit 1 ;;
esac

# Stage release layout expected by install.sh:
#   ${BASE_URL}/download/v${VERSION}/tok0-${TARGET}
#   ${BASE_URL}/download/v${VERSION}/tok0-${TARGET}.sha256
VERSION="0.1.0"
STAGING=$(mktemp -d)
trap 'rm -rf "$STAGING"; [ -n "${SERVER_PID:-}" ] && kill "$SERVER_PID" 2>/dev/null || true' EXIT

mkdir -p "$STAGING/releases/download/v${VERSION}"
cp "target/release/tok0" "$STAGING/releases/download/v${VERSION}/tok0-${TARGET}"
(cd "$STAGING/releases/download/v${VERSION}" && {
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum "tok0-${TARGET}" > "tok0-${TARGET}.sha256"
  else
    shasum -a 256 "tok0-${TARGET}" > "tok0-${TARGET}.sha256"
  fi
})

echo "== Start local release server =="
# Pick a random high port so parallel runs / stale servers don't clash.
PORT=$(awk 'BEGIN { srand(); print int(10000 + rand() * 40000) }')
cd "$STAGING"
python3 -m http.server "$PORT" >/dev/null 2>&1 &
SERVER_PID=$!
cd - > /dev/null
# Wait for server to accept connections (up to 3s)
READY=""
for _ in $(seq 1 15); do
  if curl -sSf "http://127.0.0.1:$PORT/" >/dev/null 2>&1; then
    READY=1
    break
  fi
  sleep 0.2
done
if [ -z "$READY" ]; then
  echo "FAIL: local server did not come up on port $PORT"
  kill "$SERVER_PID" 2>/dev/null || true
  exit 1
fi

# Prepare install.sh variant pointed at our local server.
# install.sh hardcodes github.com URLs; rewrite BASE_URL + skip the
# "latest tag" curl by passing an explicit VERSION.
WORK=$(mktemp -d)
cp install.sh "$WORK/install.sh"
# Replace the github.com URL prefix with our local server.
sed -i.bak "s|https://github.com/\${REPO}/releases|http://127.0.0.1:${PORT}/releases|g" "$WORK/install.sh"

INSTALL_DIR="$WORK/bin"
mkdir -p "$INSTALL_DIR"

echo "== Run install.sh (VERSION=$VERSION, TARGET=$TARGET) =="
TOK0_VERSION="$VERSION" TOK0_INSTALL_DIR="$INSTALL_DIR" sh "$WORK/install.sh"

echo "== Verify installed binary runs =="
"$INSTALL_DIR/tok0" --version
"$INSTALL_DIR/tok0" status > /dev/null
echo "OK: install.sh smoke test passed"

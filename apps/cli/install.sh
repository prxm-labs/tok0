#!/bin/sh
set -e

VERSION="${TOK0_VERSION:-latest}"
REPO="prxm-labs/tok0"
BASE_URL="https://github.com/${REPO}/releases"

# Detect platform
OS=$(uname -s | tr '[:upper:]' '[:lower:]')
ARCH=$(uname -m)
case "$ARCH" in
  x86_64)  ARCH="x86_64" ;;
  aarch64|arm64) ARCH="aarch64" ;;
  *) echo "Unsupported architecture: $ARCH"; exit 1 ;;
esac
case "$OS" in
  darwin)  TARGET="${ARCH}-apple-darwin" ;;
  linux)
    # Prefer glibc; fall back to musl on Alpine / other musl-based distros.
    # Detect musl by (a) ldd emitting "musl" in its version output, or
    # (b) `ldd --version` itself failing (BusyBox ldd behaves this way).
    if ldd --version 2>&1 | grep -qi musl; then
      TARGET="${ARCH}-unknown-linux-musl"
    else
      TARGET="${ARCH}-unknown-linux-gnu"
    fi
    ;;
  *) echo "Unsupported OS: $OS"; exit 1 ;;
esac

if [ "$VERSION" = "latest" ]; then
  VERSION=$(curl -sSfL "https://api.github.com/repos/${REPO}/releases/latest" \
    | grep '"tag_name"' | sed 's/.*"v\([^"]*\)".*/\1/')
fi

BINARY_URL="${BASE_URL}/download/v${VERSION}/tok0-${TARGET}"
SHA_URL="${BINARY_URL}.sha256"
INSTALL_DIR="${TOK0_INSTALL_DIR:-/usr/local/bin}"

echo "Installing tok0 v${VERSION} for ${TARGET}..."
TMP=$(mktemp)
curl -sSfL "$BINARY_URL" -o "$TMP"
EXPECTED=$(curl -sSfL "$SHA_URL" | awk '{print $1}')
ACTUAL=$(sha256sum "$TMP" 2>/dev/null | awk '{print $1}' || shasum -a 256 "$TMP" | awk '{print $1}')
if [ "$EXPECTED" != "$ACTUAL" ]; then
  echo "SHA-256 mismatch! Aborting."
  rm -f "$TMP"
  exit 1
fi
chmod +x "$TMP"
mv "$TMP" "${INSTALL_DIR}/tok0"
echo "Installed: ${INSTALL_DIR}/tok0"
echo "Run: tok0 --version"

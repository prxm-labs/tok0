#!/usr/bin/env sh
# tok0 — install script
# Usage: curl -fsSL https://tok0.dev/install.sh | sh
#
# What this does:
#   1. Detects your OS + architecture.
#   2. Downloads the matching pre-built binary from GitHub Releases.
#   3. Verifies its SHA-256 against the published checksums file.
#   4. Drops it into /usr/local/bin (or ~/.local/bin if /usr/local is not writable).
#
# It does NOT run `tok0 init` for you — run that next when you're ready.
#
# Override the install location with TOK0_INSTALL_DIR=/some/path.
# Pin a specific version with TOK0_VERSION=v0.4.2.

set -eu

REPO="prxm-labs/tok0"
BIN="tok0"

GH_API="https://api.github.com/repos/${REPO}"
GH_DL="https://github.com/${REPO}/releases/download"

red()    { printf '\033[31m%s\033[0m\n'  "$1" >&2; }
green()  { printf '\033[32m%s\033[0m\n'  "$1"; }
yellow() { printf '\033[33m%s\033[0m\n'  "$1"; }
dim()    { printf '\033[2m%s\033[0m\n'   "$1"; }

err() { red "tok0: $1"; exit 1; }

require() {
  command -v "$1" >/dev/null 2>&1 || err "missing required tool: $1"
}

require uname
require curl
require tar
require mkdir
# checksum tool: prefer shasum (BSD/macOS), fallback to sha256sum (GNU/Linux)
if command -v shasum >/dev/null 2>&1; then
  SHA256="shasum -a 256"
elif command -v sha256sum >/dev/null 2>&1; then
  SHA256="sha256sum"
else
  err "missing required tool: shasum or sha256sum"
fi

# ── detect target ──────────────────────────────────────────────────────────
KERNEL=$(uname -s)
MACHINE=$(uname -m)

case "$KERNEL" in
  Linux)   OS="unknown-linux-gnu";  EXE="" ;;
  Darwin)  OS="apple-darwin";        EXE="" ;;
  MINGW*|MSYS*|CYGWIN*) err "Windows is not supported by this script — use scoop or chocolatey instead." ;;
  *)       err "unsupported OS: $KERNEL" ;;
esac

case "$MACHINE" in
  x86_64|amd64)    ARCH="x86_64" ;;
  aarch64|arm64)   ARCH="aarch64" ;;
  *)               err "unsupported architecture: $MACHINE" ;;
esac

TARGET="${ARCH}-${OS}"
dim "tok0: detected target ${TARGET}"

# ── resolve version ────────────────────────────────────────────────────────
VERSION="${TOK0_VERSION:-}"
if [ -z "$VERSION" ]; then
  VERSION=$(curl -fsSL "${GH_API}/releases/latest" \
    | grep -E '"tag_name"' \
    | head -n1 \
    | sed -E 's/.*"tag_name": *"([^"]+)".*/\1/') \
    || err "could not resolve latest release. Set TOK0_VERSION=vX.Y.Z to pin one."
fi
case "$VERSION" in v*) ;; *) VERSION="v${VERSION}" ;; esac
dim "tok0: installing ${VERSION}"

# ── download archive + checksums ───────────────────────────────────────────
ARCHIVE="${BIN}-${VERSION}-${TARGET}.tar.gz"
ARCHIVE_URL="${GH_DL}/${VERSION}/${ARCHIVE}"
CHECKSUMS_URL="${GH_DL}/${VERSION}/checksums.txt"

TMP=$(mktemp -d 2>/dev/null || mktemp -d -t tok0)
trap 'rm -rf "$TMP"' EXIT

dim "tok0: downloading ${ARCHIVE_URL}"
curl -fsSL "$ARCHIVE_URL"  -o "$TMP/$ARCHIVE"  || err "failed to download $ARCHIVE_URL"
curl -fsSL "$CHECKSUMS_URL" -o "$TMP/checksums.txt" || err "failed to download checksums"

# ── verify checksum ────────────────────────────────────────────────────────
EXPECTED=$(grep "  ${ARCHIVE}\$" "$TMP/checksums.txt" | awk '{print $1}')
[ -n "$EXPECTED" ] || err "checksum for ${ARCHIVE} not found in checksums.txt"

ACTUAL=$( (cd "$TMP" && $SHA256 "$ARCHIVE") | awk '{print $1}')
[ "$EXPECTED" = "$ACTUAL" ] || err "checksum mismatch for ${ARCHIVE}: expected $EXPECTED, got $ACTUAL"
dim "tok0: checksum verified"

# ── extract ────────────────────────────────────────────────────────────────
( cd "$TMP" && tar -xzf "$ARCHIVE" ) || err "failed to extract $ARCHIVE"

EXTRACTED="$TMP/${BIN}${EXE}"
[ -f "$EXTRACTED" ] || EXTRACTED=$(find "$TMP" -type f -name "${BIN}${EXE}" | head -n1)
[ -f "$EXTRACTED" ] || err "tok0 binary not found inside archive"
chmod +x "$EXTRACTED"

# ── pick install dir ───────────────────────────────────────────────────────
INSTALL_DIR="${TOK0_INSTALL_DIR:-}"
if [ -z "$INSTALL_DIR" ]; then
  if [ -w "/usr/local/bin" ]; then
    INSTALL_DIR="/usr/local/bin"
  elif [ -d "/usr/local/bin" ] && command -v sudo >/dev/null 2>&1; then
    INSTALL_DIR="/usr/local/bin"
    SUDO="sudo"
  else
    INSTALL_DIR="$HOME/.local/bin"
    mkdir -p "$INSTALL_DIR"
  fi
fi
SUDO="${SUDO:-}"

DEST="${INSTALL_DIR}/${BIN}${EXE}"
${SUDO} mv "$EXTRACTED" "$DEST" || err "failed to write ${DEST}"
${SUDO} chmod +x "$DEST"

# ── PATH hint ──────────────────────────────────────────────────────────────
case ":$PATH:" in
  *":$INSTALL_DIR:"*) PATH_OK=1 ;;
  *) PATH_OK=0 ;;
esac

green "tok0 ${VERSION} installed to ${DEST}"

if [ "$PATH_OK" = "0" ]; then
  yellow "tok0: ${INSTALL_DIR} is not on your PATH."
  yellow "      add this to your shell rc:"
  printf '\n        export PATH="%s:$PATH"\n\n' "$INSTALL_DIR"
fi

# ── next steps ─────────────────────────────────────────────────────────────
cat <<EOF

Next:

  ${BIN} init       # auto-detect AI tools and install hooks
  ${BIN} doctor     # diagnose any setup issues
  ${BIN} stats      # see how much you've already saved

Docs: https://tok0.dev/docs/

EOF

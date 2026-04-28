#!/usr/bin/env bash
# Update Homebrew formula SHA-256 checksums from a GitHub Release.
#
# Usage:
#   ./scripts/update-formula.sh v0.1.0
#
# This downloads the .sha256 companion files from the release and patches
# Formula/tok0.rb with the correct hashes and version.

set -euo pipefail

VERSION="${1:?Usage: update-formula.sh <version-tag>}"
VERSION_NUM="${VERSION#v}"
REPO="prxm-labs/tok0"
FORMULA="Formula/tok0.rb"

echo "Updating formula for $VERSION..."

# Download checksums via `gh` so the script works against both public and
# private repos (public install paths use plain curl).
TMP_DIR=$(mktemp -d)
trap "rm -rf $TMP_DIR" EXIT
cd "$TMP_DIR"
gh release download "$VERSION" --repo "$REPO" \
  --pattern "tok0-aarch64-apple-darwin.sha256" \
  --pattern "tok0-x86_64-apple-darwin.sha256"
ARM_SHA=$(awk '{print $1}' tok0-aarch64-apple-darwin.sha256)
X86_SHA=$(awk '{print $1}' tok0-x86_64-apple-darwin.sha256)
cd - > /dev/null

if [ -z "$ARM_SHA" ] || [ -z "$X86_SHA" ]; then
  echo "Error: Failed to download checksums from release $VERSION" >&2
  exit 1
fi

echo "  aarch64: $ARM_SHA"
echo "  x86_64:  $X86_SHA"

# Patch formula
sed -i.bak -E "s/version \"[^\"]+\"/version \"$VERSION_NUM\"/" "$FORMULA"

# Replace the sha256 line after the aarch64 url
awk -v arm="$ARM_SHA" -v x86="$X86_SHA" '
  /aarch64-apple-darwin/ { found_arm=1 }
  /x86_64-apple-darwin/ { found_x86=1 }
  found_arm && /sha256/ { sub(/sha256 "[^"]*"/, "sha256 \"" arm "\""); found_arm=0 }
  found_x86 && /sha256/ { sub(/sha256 "[^"]*"/, "sha256 \"" x86 "\""); found_x86=0 }
  { print }
' "$FORMULA.bak" > "$FORMULA"

rm -f "$FORMULA.bak"

echo "Formula updated: $FORMULA"
echo "Review with: git diff $FORMULA"

#!/usr/bin/env bash
# Capture real command output as test fixtures.
# Usage: ./scripts/capture-fixtures.sh [category]
# Categories: system, git, all
set -euo pipefail

FIXTURE_DIR="tests/fixtures"
mkdir -p "$FIXTURE_DIR"/system "$FIXTURE_DIR"/git

capture() {
    local category="$1"
    local name="$2"
    shift 2
    local outfile="$FIXTURE_DIR/$category/${name}_raw.txt"
    echo "  Capturing: $outfile"
    "$@" > "$outfile" 2>&1 || true
}

do_system() {
    echo "=== System fixtures ==="
    capture system ls       /bin/ls -la
    capture system find     /usr/bin/find . -name "*.rs" -not -path "./target/*"
    capture system grep     /usr/bin/grep -rn "fn " src/ --include="*.rs"
    capture system du       du -sh -- *
    capture system df       df -h
    capture system env      env
    capture system ps       ps aux
}

do_git() {
    echo "=== Git fixtures ==="
    capture git log         git log --oneline -20
    capture git status      git status
    capture git diff        git diff HEAD
    capture git branch      git branch -a
}

case "${1:-all}" in
    system) do_system ;;
    git)    do_git ;;
    all)    do_system; do_git ;;
    *)      echo "Usage: $0 [system|git|all]"; exit 1 ;;
esac

echo ""
echo "Done. Fixtures in $FIXTURE_DIR/"

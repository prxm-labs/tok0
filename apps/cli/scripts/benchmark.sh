#!/usr/bin/env bash
# Performance and compression benchmarks for tok0.
# Validates startup overhead, binary size, memory, and per-area token savings.
#
# Usage:
#   ./scripts/benchmark.sh              # Full benchmark suite
#   ./scripts/benchmark.sh startup      # Startup overhead only
#   ./scripts/benchmark.sh compression  # Compression showcase only
#   ./scripts/benchmark.sh size         # Binary size only
#   ./scripts/benchmark.sh memory       # Memory usage only
#
# Requires: hyperfine (optional, cargo install hyperfine)
set -uo pipefail
# Note: -e intentionally omitted — individual commands may fail (e.g. git log
# on a repo with no commits) and we handle errors per-command.

BIN="target/release/tok0"
FIXTURE_DIR="tests/fixtures"

# ── Helpers ──────────────────────────────────────────────────────────

build_release() {
    echo "=== Build release binary ==="
    cargo build --release 2>&1 | tail -1
    echo ""
}

binary_size() {
    echo "=== Binary size ==="
    ls -lh "$BIN" | awk '{print $5, $NF}'
    echo ""
}

# Time a single command and print the duration.
# Uses bash's `time` builtin, which writes to stderr in the format:
#   real  0m0.123s
# We capture stderr, extract the real line, and print it.
time_cmd() {
    local label="$1"
    shift
    local time_output
    time_output=$( { time "$@" >/dev/null 2>&1; } 2>&1 ) || true
    local real_line
    real_line=$(echo "$time_output" | grep -m1 "^real" || true)
    if [[ -n "$real_line" ]]; then
        printf "  %-16s %s\n" "$label" "$real_line"
    else
        printf "  %-16s %s\n" "$label" "(command failed or no timing data)"
    fi
}

startup_overhead() {
    echo "=== Startup overhead ==="
    if command -v hyperfine &>/dev/null; then
        # --ignore-failure: allow non-zero exit (e.g. git log on empty repo)
        hyperfine --warmup 3 --min-runs 10 --ignore-failure \
            "$BIN ls ." \
            "/bin/ls -la"
        echo ""
        # Only benchmark git if the repo has commits
        if git log --oneline -1 >/dev/null 2>&1; then
            hyperfine --warmup 3 --min-runs 10 \
                "$BIN git log --oneline -5" \
                "git log --oneline -5"
        else
            echo "  (skipping git log benchmark — no commits in repo)"
        fi
    else
        echo "(install hyperfine for detailed benchmarks: cargo install hyperfine)"
        echo ""
        echo "Quick timing — tok0 ls vs raw ls:"
        time_cmd "tok0 ls:" "$BIN" ls .
        time_cmd "raw  ls:" /bin/ls -la
        echo ""
        if git log --oneline -1 >/dev/null 2>&1; then
            echo "Quick timing — tok0 git log vs raw git log:"
            time_cmd "tok0 git:" "$BIN" git log --oneline -5
            time_cmd "raw  git:" git log --oneline -5
        else
            echo "(skipping git log benchmark — no commits in repo)"
        fi
    fi
    echo ""
}

memory_usage() {
    echo "=== Memory usage ==="
    if [[ -x /usr/bin/time ]]; then
        # /usr/bin/time -l writes stats to stderr.
        # Redirect: stderr→stdout (for capture), stdout→/dev/null (hide command output).
        /usr/bin/time -l "$BIN" ls . 2>&1 >/dev/null | grep -i "maximum resident" || \
            echo "  (could not measure memory — command may have failed)"
    else
        echo "  (/usr/bin/time not available on this platform)"
    fi
    echo ""
}

# ── Compression showcase ────────────────────────────────────────────

compression_showcase() {
    echo "=== Compression showcase (per area) ==="
    echo ""
    echo "  Running cargo test benchmark suite..."
    echo ""

    # Run the Rust-side benchmark test which exercises all compressors against fixtures
    if cargo test --test benchmark_showcase -- --nocapture 2>&1 | grep -E "^\s*(BENCH|----)"; then
        :
    else
        echo "  (benchmark_showcase test not found or failed)"
        echo "  Falling back to fixture file size comparison..."
        echo ""
        printf "  %-12s %-24s %10s\n" "AREA" "FIXTURE" "SIZE"
        printf "  %-12s %-24s %10s\n" "----" "-------" "----"
        for f in "$FIXTURE_DIR"/**/*_raw.txt; do
            if [[ -f "$f" ]]; then
                local area
                area=$(echo "$f" | sed "s|$FIXTURE_DIR/||" | cut -d/ -f1)
                local name
                name=$(basename "$f" _raw.txt)
                local size
                size=$(wc -w < "$f" | tr -d ' ')
                printf "  %-12s %-24s %6s tokens\n" "$area" "$name" "$size"
            fi
        done
    fi
    echo ""
}

# ── Test summary ────────────────────────────────────────────────────

test_summary() {
    echo "=== Test summary ==="
    cargo test --all 2>&1 | tail -3
    echo ""
}

# ── Main ────────────────────────────────────────────────────────────

run_all() {
    build_release
    binary_size
    startup_overhead
    memory_usage
    compression_showcase
    test_summary
}

case "${1:-all}" in
    all)         run_all ;;
    startup)     build_release; startup_overhead ;;
    compression) compression_showcase ;;
    size)        build_release; binary_size ;;
    memory)      build_release; memory_usage ;;
    test)        test_summary ;;
    *)           echo "Usage: $0 [all|startup|compression|size|memory|test]"; exit 1 ;;
esac

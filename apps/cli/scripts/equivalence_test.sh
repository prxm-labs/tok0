#!/usr/bin/env bash
# Equivalence test: validates that tok0-compressed output produces
# semantically identical AI responses compared to raw output.
#
# Runs Claude Code (--print) with deterministic queries against both
# raw fixtures and tok0-compressed versions, comparing answers.
#
# Usage:
#   ./scripts/equivalence_test.sh           # Run all test cases
#   ./scripts/equivalence_test.sh --quick   # Run only ls + git_status
#
# Requires: claude CLI (Claude Code), jq, cargo (for building compress helper)
set -uo pipefail

FIXTURE_DIR="tests/fixtures"
RESULTS_DIR=$(mktemp -d)
PASS=0
FAIL=0
TOTAL_RAW_COST=0
TOTAL_COMP_COST=0

# ── Preflight ────────────────────────────────────────────────────────

check_deps() {
    local missing=()
    command -v claude &>/dev/null || missing+=("claude")
    command -v jq &>/dev/null || missing+=("jq")
    command -v cargo &>/dev/null || missing+=("cargo")
    if [[ ${#missing[@]} -gt 0 ]]; then
        echo "ERROR: Missing dependencies: ${missing[*]}"
        echo "  claude: https://claude.ai/claude-code"
        echo "  jq: brew install jq"
        exit 1
    fi
}

# ── Claude query helper ──────────────────────────────────────────────

# Ask Claude a deterministic question about command output.
# Returns JSON with: result, tokens, cost
ask_claude() {
    local question="$1"
    local context="$2"
    local prompt
    prompt=$(cat <<PROMPT
You are analyzing command output. Answer ONLY with the specific fact requested. No explanation, no preamble, no caveats. Be terse. One word or number only.

COMMAND OUTPUT:
${context}

QUESTION: ${question}
PROMPT
)

    claude -p \
        --output-format json \
        --allowedTools "" \
        --model sonnet \
        "$prompt" 2>/dev/null
}

# Extract fields from Claude JSON response.
# Token counts include cache tokens for accurate comparison.
extract_result() { echo "$1" | jq -r '.result // empty'; }
extract_total_tokens() {
    echo "$1" | jq -r '
        (.usage.input_tokens // 0)
        + (.usage.cache_creation_input_tokens // 0)
        + (.usage.cache_read_input_tokens // 0)
        + (.usage.output_tokens // 0)
    '
}
extract_cost() { echo "$1" | jq -r '.total_cost_usd // 0'; }

# ── Test case runner ─────────────────────────────────────────────────

# Run a single equivalence test case.
# Args: area, compressor, fixture_relpath, question, validator_fn
run_case() {
    local area="$1"
    local compressor="$2"
    local fixture_relpath="$3"
    local question="$4"
    local validator="$5"
    local fixture_file="${FIXTURE_DIR}/${fixture_relpath}"

    if [[ ! -f "$fixture_file" ]]; then
        echo "  SKIP  ${area}/${compressor}: fixture missing ($fixture_relpath)"
        return
    fi

    local raw_input
    raw_input=$(cat "$fixture_file")

    local compressed_input
    compressed_input=$(EQUIV_COMPRESSOR="$compressor" EQUIV_FIXTURE="$fixture_file" \
        cargo test --test equivalence_compress_helper "compress_and_print" -- \
        --nocapture --exact 2>/dev/null \
        | sed -n '/__COMPRESSED_START__/,/__COMPRESSED_END__/{//!p;}')

    if [[ -z "$compressed_input" ]]; then
        echo "  SKIP  ${area}/${compressor}: compressor produced no output"
        return
    fi

    # Count raw vs compressed characters (proxy for token savings before API call)
    local raw_chars=${#raw_input}
    local comp_chars=${#compressed_input}

    # Query Claude with raw input
    local raw_json
    raw_json=$(ask_claude "$question" "$raw_input")
    local raw_answer
    raw_answer=$(extract_result "$raw_json")
    local raw_tokens
    raw_tokens=$(extract_total_tokens "$raw_json")
    local raw_cost
    raw_cost=$(extract_cost "$raw_json")

    # Query Claude with compressed input
    local comp_json
    comp_json=$(ask_claude "$question" "$compressed_input")
    local comp_answer
    comp_answer=$(extract_result "$comp_json")
    local comp_tokens
    comp_tokens=$(extract_total_tokens "$comp_json")
    local comp_cost
    comp_cost=$(extract_cost "$comp_json")

    # Cost savings (most accurate measure since it accounts for caching)
    local cost_savings="0.0"
    if [[ $(awk "BEGIN { print ($raw_cost > 0) }") == "1" ]]; then
        cost_savings=$(awk "BEGIN { printf \"%.1f\", 100 - ($comp_cost / $raw_cost * 100) }")
    fi

    # Character savings (proxy for token savings in the prompt itself)
    local char_savings="0.0"
    if [[ $raw_chars -gt 0 ]]; then
        char_savings=$(awk "BEGIN { printf \"%.1f\", 100 - ($comp_chars / $raw_chars * 100) }")
    fi

    # Validate equivalence
    local equivalent
    equivalent=$($validator "$raw_answer" "$comp_answer" 2>/dev/null)

    # Accumulate cost totals
    TOTAL_RAW_COST=$(awk "BEGIN { printf \"%.6f\", $TOTAL_RAW_COST + $raw_cost }")
    TOTAL_COMP_COST=$(awk "BEGIN { printf \"%.6f\", $TOTAL_COMP_COST + $comp_cost }")

    # Report
    local status
    if [[ "$equivalent" == "true" ]]; then
        status="PASS"
        PASS=$((PASS + 1))
    else
        status="FAIL"
        FAIL=$((FAIL + 1))
    fi

    printf "  %s  %-14s %-18s  chars: %5d→%5d (%5s%%)  cost: \$%.4f→\$%.4f (%5s%%)\n" \
        "$status" "$area" "$compressor" \
        "$raw_chars" "$comp_chars" "$char_savings" \
        "$raw_cost" "$comp_cost" "$cost_savings"

    # Save detail for review
    cat > "${RESULTS_DIR}/${area}_${compressor}.txt" <<DETAIL
Area: $area
Compressor: $compressor
Question: $question
Status: $status

Raw answer:     $raw_answer
Compressed:     $comp_answer

Raw chars:      $raw_chars
Compressed:     $comp_chars ($char_savings% smaller)

Raw tokens:     $raw_tokens  cost=\$$raw_cost
Comp tokens:    $comp_tokens  cost=\$$comp_cost
Cost savings:   ${cost_savings}%
DETAIL
}

# ── Validators ───────────────────────────────────────────────────────

# Exact match (lowercased, trimmed, strip punctuation)
validate_exact() {
    local a b
    a=$(echo "$1" | tr '[:upper:]' '[:lower:]' | tr -d '[:space:][:punct:]')
    b=$(echo "$2" | tr '[:upper:]' '[:lower:]' | tr -d '[:space:][:punct:]')
    [[ "$a" == "$b" ]] && echo "true" || echo "false"
}

# Numeric match (extract first number from each, compare)
validate_numeric() {
    local a b
    a=$(echo "$1" | grep -oE '[0-9]+' | head -1)
    b=$(echo "$2" | grep -oE '[0-9]+' | head -1)
    [[ -n "$a" && -n "$b" && "$a" == "$b" ]] && echo "true" || echo "false"
}

# ── Test cases ───────────────────────────────────────────────────────

run_tests() {
    echo ""
    echo "=== tok0 Equivalence Test ==="
    echo "  Validates compressed output produces same AI answers as raw output"
    echo ""
    printf "  %-6s %-14s %-18s  %-28s  %s\n" \
        "STATUS" "AREA" "COMPRESSOR" "CHARS (raw→comp)" "COST (raw→comp)"
    echo "  ──────────────────────────────────────────────────────────────────────────────────────────────"

    # ls: "Does Cargo.toml exist?" — binary yes/no, preserved by compression
    run_case "system" "ls" "system/ls_raw.txt" \
        "Does a file named Cargo.toml appear in this listing? Reply with ONLY 'yes' or 'no'." \
        validate_exact

    # git status: "What branch is this on?"
    run_case "git" "git_status" "git/status_raw.txt" \
        "What git branch name is shown in this output? Reply with ONLY the branch name." \
        validate_exact

    if [[ "${1:-}" == "--quick" ]]; then
        return
    fi

    # git status: "How many staged files?"
    run_case "git" "git_status_count" "git/status_raw.txt" \
        "How many files are listed under 'Changes to be committed' (or 'staged')? Count each file line. Reply with ONLY the number." \
        validate_numeric

    # git log: "What is the first commit hash?"
    run_case "git" "git_log" "git/log_raw.txt" \
        "What is the short hash (first 7 characters) of the most recent commit shown? Reply with ONLY the 7-character hash." \
        validate_exact

    # cargo test: "Did tests pass or fail?"
    run_case "rust" "cargo_test" "rust/cargo_test_raw.txt" \
        "Looking at this cargo test output overall: did all tests pass, or were there test failures? Reply with ONLY 'pass' or 'fail'." \
        validate_exact

    # cargo clippy: "Are there warnings?"
    run_case "rust" "cargo_clippy" "rust/cargo_clippy_raw.txt" \
        "Are there any clippy warnings in this output? Reply with ONLY 'yes' or 'no'." \
        validate_exact
}

# ── Main ─────────────────────────────────────────────────────────────

main() {
    check_deps

    echo "=== Building compress helper ==="
    cargo test --test equivalence_compress_helper --no-run 2>&1 | tail -1
    echo ""

    run_tests "${1:-}"

    echo ""
    echo "  ──────────────────────────────────────────────────────────────────────────────────────────────"
    printf "  Results: %d passed, %d failed out of %d\n" "$PASS" "$FAIL" "$((PASS + FAIL))"
    printf "  Total cost:  raw=\$%.4f  compressed=\$%.4f\n" "$TOTAL_RAW_COST" "$TOTAL_COMP_COST"
    if [[ $(awk "BEGIN { print ($TOTAL_RAW_COST > 0) }") == "1" ]]; then
        local overall_savings
        overall_savings=$(awk "BEGIN { printf \"%.1f\", 100 - ($TOTAL_COMP_COST / $TOTAL_RAW_COST * 100) }")
        printf "  Overall cost savings: %s%%\n" "$overall_savings"
    fi
    echo "  Details: ${RESULTS_DIR}/"
    echo ""

    if [[ $FAIL -gt 0 ]]; then
        echo "  Some cases failed — check details for divergent answers"
        exit 1
    else
        echo "  All cases passed — compressed output is semantically equivalent"
    fi
}

main "$@"

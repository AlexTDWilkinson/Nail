#!/bin/bash

# Always run from the repository root, wherever this was invoked from.
cd "$(dirname "$0")/.."

# Test files that pass lexer/parser but may fail type checking
echo "Testing Type Checker"
echo "===================="

# Every .nail file in the repository, from one shared list, so no directory
# is tested by nothing (see test_nail_files.sh).
source "$(dirname "$0")/test_nail_files.sh"

cargo build --bin nailc 2>/dev/null || { echo "FATAL: nailc failed to build"; exit 1; }
NAILC=./target/debug/nailc

RESULTS_DIR=$(mktemp -d)
trap 'rm -rf "$RESULTS_DIR"' EXIT

run_one() {
    local file="$1"
    local output result
    output=$("$NAILC" "$file" --check-only 2>&1)

    # Skip files that don't pass lexer/parser (covered by test_lexer_parser.sh).
    # Those stages name themselves in the failure summary.
    if echo "$output" | grep -q "found before parsing\|found while parsing\|has no version line"; then
        echo "SKIP" > "$RESULTS_DIR/$(echo "$file" | tr '/' '_').result"
        return
    fi

    # Files marked with this comment are negative tests: they PASS when
    # the type checker rejects them
    local expect_fail=false
    if head -3 "$file" | grep -q "should FAIL type checking"; then
        expect_fail=true
    fi

    # A clean check prints exactly `ok`, a refused one counts its errors.
    if echo "$output" | grep -qx "ok"; then
        if $expect_fail; then result="FAIL (expected type check to reject this file)"; else result="PASS"; fi
    elif echo "$output" | grep -qE "^[0-9]+ errors? found$"; then
        if $expect_fail; then result="PASS (correctly rejected)"; else result="FAIL (Type checker)"; fi
    elif echo "$output" | grep -q "panic\|thread.*panicked"; then
        result="FAIL (Type checker crash)"
    else
        result="FAIL (unrecognized output)"
    fi
    echo "$result" > "$RESULTS_DIR/$(echo "$file" | tr '/' '_').result"
}
export -f run_one
export NAILC RESULTS_DIR

nail_test_files | tr '\n' '\0' | xargs -0 -P "$(nproc)" -I{} bash -c 'run_one "$@"' _ {}

PASSED=0
FAILED=0
FAILED_FILES=""
for file in $(nail_test_files); do
    [[ -f "$file" ]] || continue
    result=$(cat "$RESULTS_DIR/$(echo "$file" | tr '/' '_').result" 2>/dev/null || echo "FAIL (no result)")
    [[ "$result" == "SKIP" ]] && continue
    if [[ "$result" == PASS* ]]; then
        echo "Type checking $file... ✓ $result"
        ((PASSED++))
    else
        echo "Type checking $file... ✗ $result"
        FAILED_FILES="$FAILED_FILES\n$file - $result"
        ((FAILED++))
    fi
done

echo ""
echo "Summary: $PASSED passed, $FAILED failed"
echo ""
if [[ -n "$FAILED_FILES" ]]; then
    echo "Failed files:"
    echo -e "$FAILED_FILES"
fi
[[ $FAILED -eq 0 ]]

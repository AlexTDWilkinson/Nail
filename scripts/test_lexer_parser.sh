#!/bin/bash

# Always run from the repository root, wherever this was invoked from.
cd "$(dirname "$0")/.."

# Test only lexer and parser stages
echo "Testing Lexer and Parser Only"
echo "=============================="

# Build the compiler once; every test then invokes the binary directly
# instead of paying a cargo up-to-date check per file.
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

    # Files marked as negative tests PASS when the stage rejects them
    local expect_fail=false
    if head -3 "$file" | grep -q "should FAIL parsing"; then
        expect_fail=true
    fi

    if echo "$output" | grep -q "Lexer error"; then
        if $expect_fail; then result="PASS (correctly rejected)"; else result="FAIL (Lexer)"; fi
    elif echo "$output" | grep -q "Parse error"; then
        if $expect_fail; then result="PASS (correctly rejected)"; else result="FAIL (Parser)"; fi
    else
        if $expect_fail; then result="FAIL (expected parse to reject this file)"; else result="PASS"; fi
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
    if [[ "$result" == PASS* ]]; then
        echo "Testing $file... ✓ $result"
        ((PASSED++))
    else
        echo "Testing $file... ✗ $result"
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

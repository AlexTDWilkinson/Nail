#!/bin/bash

# Test transpilation for files that pass type checking
echo "Testing Transpiler"
echo "=================="

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

    # Skip files that don't pass up to type checking (covered by earlier stages)
    if ! echo "$output" | grep -q "Type check successful!"; then
        echo "SKIP" > "$RESULTS_DIR/$(echo "$file" | tr '/' '_').result"
        return
    fi

    # Files marked with this comment are negative tests: they PASS when
    # the transpiler rejects them
    local expect_fail=false
    if head -3 "$file" | grep -q "should FAIL transpilation"; then
        expect_fail=true
    fi

    # --stdout keeps the source tree free of generated .rs files
    if "$NAILC" "$file" --stdout > /dev/null 2>&1; then
        if $expect_fail; then result="FAIL (expected transpilation to reject this file)"; else result="PASS"; fi
    else
        if $expect_fail; then result="PASS (correctly rejected)"; else result="FAIL (Transpiler)"; fi
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
        echo "Transpiling $file... ✓ $result"
        ((PASSED++))
    else
        echo "Transpiling $file... ✗ $result"
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

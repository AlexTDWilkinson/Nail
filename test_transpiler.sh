#!/bin/bash

# Test transpilation for files that pass type checking
echo "Testing Transpiler"
echo "=================="

FAILED_FILES=""
PASSED=0
FAILED=0

for file in tests/*.nail examples/*.nail; do
    if [[ -f "$file" ]]; then
        # First check if it passes up to type checking
        output=$(cargo run --bin nailc "$file" --check-only 2>&1)
        
        if ! echo "$output" | grep -q "Type check successful!"; then
            # Skip files that don't pass type checking
            continue
        fi
        
        echo -n "Transpiling $file... "
        
        # Try to transpile
        transpile_output=$(cargo run --bin nailc "$file" --transpile 2>&1)
        
        if [[ $? -eq 0 ]]; then
            echo "✓ PASS"
            ((PASSED++))
        else
            echo "✗ FAIL (Transpiler)"
            FAILED_FILES="$FAILED_FILES\n$file - Transpilation failed"
            ((FAILED++))
        fi
    fi
done

echo ""
echo "Summary: $PASSED passed, $FAILED failed"
echo ""
if [[ -n "$FAILED_FILES" ]]; then
    echo "Failed files:"
    echo -e "$FAILED_FILES"
fi
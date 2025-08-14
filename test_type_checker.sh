#!/bin/bash

# Test files that pass lexer/parser but may fail type checking
echo "Testing Type Checker"
echo "===================="

FAILED_FILES=""
PASSED=0
FAILED=0

for file in tests/*.nail examples/*.nail; do
    if [[ -f "$file" ]]; then
        # First check if it passes lexer/parser
        output=$(cargo run --bin nailc "$file" --check-only 2>&1)
        
        if echo "$output" | grep -q "Lexer error\|Parse error"; then
            # Skip files that don't pass lexer/parser
            continue
        fi
        
        echo -n "Type checking $file... "
        
        if echo "$output" | grep -q "Type check successful!"; then
            echo "✓ PASS"
            ((PASSED++))
        elif echo "$output" | grep -q "Type check errors"; then
            echo "✗ FAIL (Type checker)"
            FAILED_FILES="$FAILED_FILES\n$file - Type check failed"
            ((FAILED++))
        else
            # Check for crashes
            if echo "$output" | grep -q "panic\|thread.*panicked"; then
                echo "✗ FAIL (Type checker crash)"
                FAILED_FILES="$FAILED_FILES\n$file - Type checker crash"
                ((FAILED++))
            else
                echo "? UNKNOWN"
            fi
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
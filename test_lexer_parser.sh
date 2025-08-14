#!/bin/bash

# Test only lexer and parser stages
echo "Testing Lexer and Parser Only"
echo "=============================="

FAILED_FILES=""
PASSED=0
FAILED=0

for file in tests/*.nail examples/*.nail; do
    if [[ -f "$file" ]]; then
        echo -n "Testing $file... "
        
        # Run only lexer and parser check
        output=$(cargo run --bin nailc "$file" --check-only 2>&1)
        
        if echo "$output" | grep -q "Parse successful!"; then
            echo "✓ PASS"
            ((PASSED++))
        elif echo "$output" | grep -q "Lexer error"; then
            echo "✗ FAIL (Lexer)"
            FAILED_FILES="$FAILED_FILES\n$file - Lexer error"
            ((FAILED++))
        elif echo "$output" | grep -q "Parse error"; then
            echo "✗ FAIL (Parser)"
            FAILED_FILES="$FAILED_FILES\n$file - Parser error"
            ((FAILED++))
        else
            echo "✓ PASS (continues to type checker)"
            ((PASSED++))
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
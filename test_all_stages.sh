#!/bin/bash

echo "========================================="
echo "    Comprehensive Nail Language Tests    "
echo "========================================="
echo ""

# Run lexer/parser tests
echo "Stage 1: Lexer & Parser"
echo "-----------------------"
./test_lexer_parser.sh 2>/dev/null | grep -E "Summary:|Failed files:" -A 100
echo ""

# Run type checker tests
echo "Stage 2: Type Checker"
echo "---------------------"
./test_type_checker.sh 2>/dev/null | grep -E "Summary:|Failed files:" -A 100
echo ""

# Run transpiler tests
echo "Stage 3: Transpiler"
echo "-------------------"
./test_transpiler.sh 2>/dev/null | grep -E "Summary:|Failed files:" -A 100
echo ""

# Run Rust compilation tests (optional - only if requested)
if [[ "$1" == "--with-rust" ]]; then
    echo "Stage 4: Rust Compilation (SLOW)"
    echo "--------------------------------"
    ./test_rust_compilation.sh 2>/dev/null | grep -E "Passed:|Failed:|Failed files:" -A 100
    echo ""
fi

# Overall summary
echo "========================================="
echo "          Overall Test Summary           "
echo "========================================="

LEXER_PARSER=$(./test_lexer_parser.sh 2>/dev/null | grep "Summary:" | cut -d: -f2)
TYPE_CHECKER=$(./test_type_checker.sh 2>/dev/null | grep "Summary:" | cut -d: -f2)
TRANSPILER=$(./test_transpiler.sh 2>/dev/null | grep "Summary:" | cut -d: -f2)

echo "Lexer/Parser: $LEXER_PARSER"
echo "Type Checker: $TYPE_CHECKER"
echo "Transpiler:   $TRANSPILER"
echo ""

# Calculate totals
LP_PASS=$(echo $LEXER_PARSER | grep -o "[0-9]* passed" | grep -o "[0-9]*")
LP_FAIL=$(echo $LEXER_PARSER | grep -o "[0-9]* failed" | grep -o "[0-9]*")
TC_PASS=$(echo $TYPE_CHECKER | grep -o "[0-9]* passed" | grep -o "[0-9]*")
TC_FAIL=$(echo $TYPE_CHECKER | grep -o "[0-9]* failed" | grep -o "[0-9]*")
TR_PASS=$(echo $TRANSPILER | grep -o "[0-9]* passed" | grep -o "[0-9]*")
TR_FAIL=$(echo $TRANSPILER | grep -o "[0-9]* failed" | grep -o "[0-9]*")

TOTAL_PASS=$((LP_PASS))
TOTAL_FAIL=$((LP_FAIL))

echo "Total files tested: $((TOTAL_PASS + TOTAL_FAIL))"
echo "Files passing lexer/parser: $TOTAL_PASS/$((TOTAL_PASS + TOTAL_FAIL))"

if [[ -n "$TC_PASS" ]]; then
    echo "Files passing type checker: $TC_PASS"
fi
if [[ -n "$TR_PASS" ]]; then
    echo "Files passing transpiler: $TR_PASS"
fi
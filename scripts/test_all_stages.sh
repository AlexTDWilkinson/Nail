#!/bin/bash

# Always run from the repository root, wherever this was invoked from.
cd "$(dirname "$0")/.."

echo "========================================="
echo "    Comprehensive Nail Language Tests    "
echo "========================================="
echo ""

TMP_DIR=$(mktemp -d)
trap 'rm -rf "$TMP_DIR"' EXIT

# Run each stage exactly once, saving output for both display and the summary
echo "Stage 1: Lexer & Parser"
echo "-----------------------"
"$(dirname "$0")"/test_lexer_parser.sh > "$TMP_DIR/lexer_parser.out" 2>/dev/null
grep -E "Summary:|Failed files:" -A 100 "$TMP_DIR/lexer_parser.out"
echo ""

echo "Stage 2: Type Checker"
echo "---------------------"
"$(dirname "$0")"/test_type_checker.sh > "$TMP_DIR/type_checker.out" 2>/dev/null
grep -E "Summary:|Failed files:" -A 100 "$TMP_DIR/type_checker.out"
echo ""

echo "Stage 3: Transpiler"
echo "-------------------"
"$(dirname "$0")"/test_transpiler.sh > "$TMP_DIR/transpiler.out" 2>/dev/null
grep -E "Summary:|Failed files:" -A 100 "$TMP_DIR/transpiler.out"
echo ""

# Guard: feature-gated stdlib modules (duckdb, ...) must still compile
echo "Stage 3.5: Feature-Gated Code Check"
echo "-----------------------------------"
    "$(dirname "$0")"/check_all_features.sh
echo ""

# Run Rust compilation tests (optional - only if requested)
if [[ "$1" == "--with-rust" ]]; then
    echo "Stage 4: Rust Compilation (SLOW)"
    echo "--------------------------------"
    "$(dirname "$0")"/test_rust_compilation.sh 2>/dev/null | grep -E "Passed:|Failed:|Failed files:" -A 100
    echo ""
fi

# Overall summary
echo "========================================="
echo "          Overall Test Summary           "
echo "========================================="

LEXER_PARSER=$(grep "Summary:" "$TMP_DIR/lexer_parser.out" | cut -d: -f2)
TYPE_CHECKER=$(grep "Summary:" "$TMP_DIR/type_checker.out" | cut -d: -f2)
TRANSPILER=$(grep "Summary:" "$TMP_DIR/transpiler.out" | cut -d: -f2)

echo "Lexer/Parser: $LEXER_PARSER"
echo "Type Checker: $TYPE_CHECKER"
echo "Transpiler:   $TRANSPILER"
echo ""

count() { echo "$1" | grep -o "[0-9]* $2" | grep -o "[0-9]*"; }
LP_PASS=$(count "$LEXER_PARSER" passed); LP_FAIL=$(count "$LEXER_PARSER" failed)
TC_PASS=$(count "$TYPE_CHECKER" passed); TC_FAIL=$(count "$TYPE_CHECKER" failed)
TR_PASS=$(count "$TRANSPILER" passed);   TR_FAIL=$(count "$TRANSPILER" failed)

TOTAL_FAIL=$(( ${LP_FAIL:-0} + ${TC_FAIL:-0} + ${TR_FAIL:-0} ))

echo "Files passing lexer/parser: ${LP_PASS:-0}/$(( ${LP_PASS:-0} + ${LP_FAIL:-0} ))"
echo "Files passing type checker: ${TC_PASS:-0}/$(( ${TC_PASS:-0} + ${TC_FAIL:-0} ))"
echo "Files passing transpiler:   ${TR_PASS:-0}/$(( ${TR_PASS:-0} + ${TR_FAIL:-0} ))"
echo "Total failures across all stages: $TOTAL_FAIL"

[[ $TOTAL_FAIL -eq 0 ]]

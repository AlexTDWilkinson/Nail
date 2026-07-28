#!/bin/bash

# Golden-file tests for Nail compiler error messages.
#
# Friendly, detailed errors are a core feature of Nail. This harness makes
# error quality enforceable: every tests/errors/*.nail file is a program that
# is SUPPOSED to fail compilation, and its sibling .stderr file is the exact
# diagnostic the compiler must produce (byte-for-byte, human-reviewed).
#
# A wording regression, a lost span (line 0), or a dropped help suggestion
# fails this suite just like a logic bug fails the e2e suite.
#
# Usage:
#   ./test_error_messages.sh                     # run all error tests
#   ./test_error_messages.sh tests/errors/foo.nail  # run specific test(s)
#   ./test_error_messages.sh --bless            # regenerate all goldens
#   ./test_error_messages.sh --bless tests/errors/foo.nail
#
# After --bless, ALWAYS read the regenerated .stderr files and check the
# messages are actually friendly before committing them.

set -u

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

BLESS=false
if [ "${1:-}" = "--bless" ]; then
    BLESS=true
    shift
fi

if [ $# -gt 0 ]; then
    FILES=("$@")
else
    mapfile -t FILES < <(find tests/errors -name '*.nail' | sort)
fi

if [ ${#FILES[@]} -eq 0 ]; then
    echo "No test files found in tests/errors/"
    exit 1
fi

echo "Building nailc..."
if ! cargo build --quiet --bin nailc 2>/dev/null; then
    # Rerun without silencing so the build error is visible
    cargo build --bin nailc
    exit 1
fi
NAILC=./target/debug/nailc

pass=0
fail=0
blessed=0

for nail_file in "${FILES[@]}"; do
    golden="${nail_file%.nail}.stderr"
    actual=$("$NAILC" "$nail_file" --check-only 2>&1 >/dev/null)
    exit_code=$?

    if [ $exit_code -eq 0 ]; then
        echo -e "${RED}✗ $nail_file — compiled successfully but should have failed${NC}"
        fail=$((fail + 1))
        continue
    fi

    if $BLESS; then
        printf '%s\n' "$actual" > "$golden"
        echo -e "${YELLOW}✎ $nail_file — golden written${NC}"
        blessed=$((blessed + 1))
        continue
    fi

    if [ ! -f "$golden" ]; then
        echo -e "${RED}✗ $nail_file — missing golden file $golden (run --bless and review it)${NC}"
        fail=$((fail + 1))
        continue
    fi

    if diff -u "$golden" <(printf '%s\n' "$actual") > /tmp/nail_err_diff.$$; then
        echo -e "${GREEN}✓ $nail_file${NC}"
        pass=$((pass + 1))
    else
        echo -e "${RED}✗ $nail_file — error message changed:${NC}"
        sed 's/^/    /' /tmp/nail_err_diff.$$
        fail=$((fail + 1))
    fi
    rm -f /tmp/nail_err_diff.$$
done

echo
if $BLESS; then
    echo "Summary: $blessed goldens written — review them before committing!"
    exit 0
fi
echo "Summary: $pass passed, $fail failed"
[ $fail -eq 0 ]

#!/bin/bash

# The list of Nail files the fast test scripts check, printed one per line.
#
# It used to be the glob `tests/*.nail examples/*.nail`, which is not
# recursive, so everything in a subdirectory was tested by nothing at all.
# Three files in examples/website_examples/ had stopped compiling and one of
# them was on the live website. This is the list, in one place, so all three
# scripts agree on it and a new directory is covered the day it appears.
#
# Two kinds of file are left out:
#
#   tests/errors/  - programs that must fail, with their exact diagnostic
#                    pinned by ./test_error_messages.sh
#   files whose first three lines say "not a standalone program" - modules
#                    that another file imports, checked through their importer
#
# Everything else has to pass every stage, unless it says "should FAIL
# <stage>" in its first three lines, which the scripts read as a negative test.

PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

nail_test_files() {
    local file
    while IFS= read -r file; do
        if head -3 "$file" | grep -q "not a standalone program"; then
            continue
        fi
        echo "$file"
    # `?*.nail` rather than `*.nail`: the editor keeps its settings in a file
    # called plain `.nail`, which is not a program.
    done < <(cd "$PROJECT_ROOT" && find . -type f -name '?*.nail' \
        -not -path './target/*' \
        -not -path './.git/*' \
        -not -path './tests/errors/*' \
        | sed 's|^\./||' | sort)
}

# Running it directly prints the list, which is how you check what changed.
if [ "${BASH_SOURCE[0]}" = "$0" ]; then
    nail_test_files
fi

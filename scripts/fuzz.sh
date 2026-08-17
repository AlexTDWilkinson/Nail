#!/bin/bash

# The compiler's fuzzer: millions of programs, checked against invariants the
# compiler must never break. See src/fuzz/mod.rs for how it works and
# src/fuzz/oracle.rs for the invariants themselves.
#
# Usage:
#   ./scripts/fuzz.sh smoke              a minute on every core, then rustc on
#                                        the programs it kept
#   ./scripts/fuzz.sh soak [minutes]     a long run (default 30 minutes)
#   ./scripts/fuzz.sh run [options]      pass anything through to nail-fuzz run
#   ./scripts/fuzz.sh case <seed>        print the program a seed writes
#   ./scripts/fuzz.sh check <file.nail>  ask every question of one file
#   ./scripts/fuzz.sh build              compile the queued programs with rustc,
#                                        and run the ones that came with the
#                                        answer they owe
#   ./scripts/fuzz.sh predict <seed>     print one program and the output it owes
#   ./scripts/fuzz.sh imports --cases=N  fuzz the import sandbox with two-file cases
#
# Findings land in target/fuzz/findings as a pair of files each: the shrunken
# program, and what it broke. Everything is reproducible from the seed the
# finding names.
#
# The fuzzer is built in release by default, because it runs the compiler
# hundreds of thousands of times and a debug build spends most of its time in
# the compiler's own bounds checks. NAIL_FUZZ_PROFILE=debug uses the debug
# build instead, which is faster to build and slower to run.

# Always run from the repository root, wherever this was invoked from.
cd "$(dirname "$0")/.."

set -u

PROFILE="${NAIL_FUZZ_PROFILE:-release}"
if [ "$PROFILE" = "release" ]; then
    CARGO_FLAGS="--release"
    BINARY="./target/release/nail-fuzz"
else
    CARGO_FLAGS=""
    BINARY="./target/debug/nail-fuzz"
fi

echo "Building the fuzzer ($PROFILE)..."
if ! cargo build $CARGO_FLAGS --bin nail-fuzz --features fuzz --quiet; then
    cargo build $CARGO_FLAGS --bin nail-fuzz --features fuzz
    exit 1
fi

COMMAND="${1:-smoke}"
shift 2>/dev/null || true

case "$COMMAND" in
    smoke)
        # One minute of both engines, then rustc over what got through. This
        # is the version to run after touching the lexer, the parser, the
        # checker or the transpiler.
        "$BINARY" run --seconds=60 --cases=100000000 --engine=both "$@" || exit 1
        echo
        "$BINARY" imports --cases=4000
        echo
        "$BINARY" build
        ;;
    soak)
        MINUTES="${1:-30}"
        shift 2>/dev/null || true
        # A long run starts from a seed nobody has used, so a soak explores
        # programs the smoke runs never reach.
        SEED=$(( $(date +%s) % 100000000 * 1000 ))
        "$BINARY" run --seconds=$(( MINUTES * 60 )) --cases=100000000 --engine=both --seed="$SEED" --queue=2000 "$@" || exit 1
        echo
        "$BINARY" imports --cases=50000 --seed="$SEED"
        echo
        "$BINARY" build
        ;;
    run|case|check|build|format|predict|imports)
        "$BINARY" "$COMMAND" "$@"
        ;;
    *)
        echo "Unknown command: $COMMAND"
        echo "Try: smoke, soak, run, case, check, build, predict, imports"
        exit 1
        ;;
esac

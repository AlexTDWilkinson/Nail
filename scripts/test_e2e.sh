#!/bin/bash

# End-to-end test suite for the Nail language.
#
# For every tests/e2e/**/*.nail file this script:
#   1. Transpiles it to Rust with nailc
#   2. Compiles ALL transpiled programs as bins of ONE shared Cargo project
#      (dependencies compile once, bins build in parallel)
#   3. Runs each compiled executable
#   4. Compares its stdout byte-for-byte against the sibling .stdout file
#
# This is the sanity harness for language changes: if it is green, the whole
# pipeline (lexer -> parser -> checker -> transpiler -> rustc -> runtime
# behavior) still produces the exact same observable output for hundreds of
# real Nail programs.
#
# Usage:
#   ./scripts/test_e2e.sh                        # run every e2e test
#   ./scripts/test_e2e.sh tests/e2e/basics      # run one category
#   ./scripts/test_e2e.sh tests/e2e/basics/hello_world.nail   # run specific test(s)
#
# Test contract:
#   - Each test is <name>.nail with expected stdout in <name>.stdout
#   - Optional <name>.exitcode holds the expected exit code (default 0)
#   - Optional <name>.stderr holds the expected stderr AFTER Rust panic
#     scaffolding (the "thread ... panicked at" location line, the
#     RUST_BACKTRACE note, and blank lines) is stripped — this pins
#     runtime error messages
#   - Tests must be deterministic (no raw time_now/math_random in output)
#   - Each test runs with its cwd set to a fresh empty directory, so fs tests
#     must create whatever files they read

GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m'

PROJECT_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$PROJECT_ROOT"

E2E_DIR="tests/e2e"
WORK_DIR="${NAIL_E2E_WORK_DIR:-/tmp/nail_e2e_$USER}"
PROJ_DIR="$WORK_DIR/project"
RUN_TIMEOUT="${NAIL_E2E_TIMEOUT:-15}"

# ---------------------------------------------------------------------------
# Collect test files
# ---------------------------------------------------------------------------
declare -a NAIL_FILES=()
if [ $# -gt 0 ]; then
    for arg in "$@"; do
        if [ -d "$arg" ]; then
            while IFS= read -r f; do NAIL_FILES+=("$f"); done < <(find "$arg" -name '*.nail' | sort)
        elif [ -f "$arg" ]; then
            NAIL_FILES+=("$arg")
        else
            echo -e "${RED}Not found: $arg${NC}"; exit 1
        fi
    done
else
    while IFS= read -r f; do NAIL_FILES+=("$f"); done < <(find "$E2E_DIR" -name '*.nail' | sort)
fi

if [ ${#NAIL_FILES[@]} -eq 0 ]; then
    echo "No .nail test files found."
    exit 1
fi

echo "========================================="
echo " Nail End-to-End Test Suite"
echo " ${#NAIL_FILES[@]} test program(s)"
echo "========================================="

# ---------------------------------------------------------------------------
# Build nailc
# ---------------------------------------------------------------------------
echo -n "Building nailc... "
if ! cargo build --bin nailc --quiet 2>"$WORK_DIR.nailc_build.log"; then
    # Retry with output visible if the quiet build failed
    if ! cargo build --bin nailc; then
        echo -e "${RED}FAILED to build nailc${NC}"
        exit 1
    fi
fi
echo "done"
NAILC="$PROJECT_ROOT/target/debug/nailc"

mkdir -p "$PROJ_DIR/src/bin"
# Remove stale bins from previous runs so deleted/renamed tests don't linger
rm -f "$PROJ_DIR"/src/bin/*.rs
rm -rf "$WORK_DIR/manifests" "$WORK_DIR/cwd"
mkdir -p "$WORK_DIR/manifests" "$WORK_DIR/cwd"

PASSED=0
FAILED=0
declare -a FAILED_TESTS=()
declare -a BIN_NAMES=()
declare -a BIN_FILES=()

fail() {
    FAILED=$((FAILED + 1))
    FAILED_TESTS+=("$1: $2")
}

# ---------------------------------------------------------------------------
# Stage 1: transpile every test into the shared project
# ---------------------------------------------------------------------------
echo ""
echo "--- Stage 1: transpile (nailc) ---"
for nail_file in "${NAIL_FILES[@]}"; do
    rel="${nail_file#$E2E_DIR/}"
    rel="${rel#tests/e2e/}"
    bin_name="$(echo "${rel%.nail}" | tr '/-' '__')"

    expected_file="${nail_file%.nail}.stdout"
    if [ ! -f "$expected_file" ]; then
        echo -e "  ${RED}✗ $nail_file — missing expected-output file $(basename "$expected_file")${NC}"
        fail "$nail_file" "missing .stdout file"
        continue
    fi

    if ! "$NAILC" "$nail_file" --transpile 2>"$WORK_DIR/manifests/$bin_name.err"; then
        echo -e "  ${RED}✗ $nail_file — transpilation failed${NC}"
        sed 's/^/      /' "$WORK_DIR/manifests/$bin_name.err" | head -8
        fail "$nail_file" "transpilation failed"
        continue
    fi

    rs_file="${nail_file%.nail}.rs"
    if [ ! -f "$rs_file" ]; then
        echo -e "  ${RED}✗ $nail_file — no .rs output produced${NC}"
        fail "$nail_file" "no .rs output"
        continue
    fi
    mv "$rs_file" "$PROJ_DIR/src/bin/$bin_name.rs"

    # Per-file manifest; merged below so dependencies compile once for all bins
    if ! "$NAILC" "$nail_file" --cargo-toml "--nail-path=$PROJECT_ROOT" --package-name=nail_e2e \
            > "$WORK_DIR/manifests/$bin_name.toml" 2>/dev/null; then
        echo -e "  ${RED}✗ $nail_file — Cargo.toml generation failed${NC}"
        rm -f "$PROJ_DIR/src/bin/$bin_name.rs"
        fail "$nail_file" "cargo-toml generation failed"
        continue
    fi

    BIN_NAMES+=("$bin_name")
    BIN_FILES+=("$nail_file")
done
echo "  transpiled ${#BIN_NAMES[@]}/${#NAIL_FILES[@]} programs"

if [ ${#BIN_NAMES[@]} -eq 0 ]; then
    echo -e "${RED}Nothing transpiled successfully; aborting.${NC}"
    exit 1
fi

# ---------------------------------------------------------------------------
# Stage 2: merge manifests and compile everything in one cargo build
# ---------------------------------------------------------------------------
echo ""
echo "--- Stage 2: compile (cargo, single shared project) ---"
python3 - "$WORK_DIR/manifests" "$PROJECT_ROOT" > "$PROJ_DIR/Cargo.toml" <<'PYEOF'
import glob, re, sys
manifest_dir, nail_path = sys.argv[1], sys.argv[2]
deps, features = set(), set()
for path in glob.glob(manifest_dir + "/*.toml"):
    in_deps = False
    for line in open(path):
        line = line.strip()
        if line == "[dependencies]":
            in_deps = True
            continue
        if line.startswith("["):
            in_deps = False
            continue
        if not in_deps or not line:
            continue
        if line.startswith("nail "):
            m = re.search(r'features\s*=\s*\[([^\]]*)\]', line)
            if m:
                features.update(f.strip() for f in m.group(1).split(",") if f.strip())
        else:
            deps.add(line)
print("[package]")
print('name = "nail_e2e"')
print('version = "0.1.0"')
print('edition = "2021"')
print()
print("[dependencies]")
if features:
    print('nail = { path = "%s", features = [%s] }' % (nail_path, ", ".join(sorted(features))))
else:
    print('nail = { path = "%s" }' % nail_path)
for d in sorted(deps):
    print(d)
print()
# No debug info: with hundreds of bins, link time dominates the build
print("[profile.dev]")
print("debug = false")
PYEOF

export CARGO_TARGET_DIR="$WORK_DIR/target"
BUILD_LOG="$WORK_DIR/build.log"
if ! (cd "$PROJ_DIR" && cargo build --bins 2>"$BUILD_LOG"); then
    echo -e "  ${YELLOW}build reported errors — checking which bins compiled${NC}"
fi

# A bin passes stage 2 iff its executable exists and is newer than its source
declare -a RUNNABLE_NAMES=()
declare -a RUNNABLE_FILES=()
for i in "${!BIN_NAMES[@]}"; do
    bin="$WORK_DIR/target/debug/${BIN_NAMES[$i]}"
    if [ -x "$bin" ] && [ "$bin" -nt "$PROJ_DIR/src/bin/${BIN_NAMES[$i]}.rs" ]; then
        RUNNABLE_NAMES+=("${BIN_NAMES[$i]}")
        RUNNABLE_FILES+=("${BIN_FILES[$i]}")
    else
        echo -e "  ${RED}✗ ${BIN_FILES[$i]} — Rust compilation failed${NC}"
        grep -A 6 "${BIN_NAMES[$i]}.rs" "$BUILD_LOG" 2>/dev/null | head -10 | sed 's/^/      /'
        fail "${BIN_FILES[$i]}" "rust compilation failed"
    fi
done
echo "  compiled ${#RUNNABLE_NAMES[@]}/${#BIN_NAMES[@]} programs"

# ---------------------------------------------------------------------------
# Stage 3: run each executable and compare stdout
# ---------------------------------------------------------------------------
echo ""
echo "--- Stage 3: run & compare output ---"
for i in "${!RUNNABLE_NAMES[@]}"; do
    bin_name="${RUNNABLE_NAMES[$i]}"
    nail_file="${RUNNABLE_FILES[$i]}"
    expected_file="${nail_file%.nail}.stdout"
    exitcode_file="${nail_file%.nail}.exitcode"
    expected_exit=0
    [ -f "$exitcode_file" ] && expected_exit="$(cat "$exitcode_file")"

    run_dir="$WORK_DIR/cwd/$bin_name"
    mkdir -p "$run_dir"
    actual_out="$WORK_DIR/cwd/$bin_name.stdout"
    actual_err="$WORK_DIR/cwd/$bin_name.stderr"

    # LC_ALL=C keeps OS error text (strerror) stable for .stderr/.stdout goldens
    (cd "$run_dir" && LC_ALL=C timeout "$RUN_TIMEOUT" "$WORK_DIR/target/debug/$bin_name" \
        >"$actual_out" 2>"$actual_err")
    actual_exit=$?

    if [ "$actual_exit" -eq 124 ]; then
        echo -e "  ${RED}✗ $nail_file — TIMEOUT after ${RUN_TIMEOUT}s${NC}"
        fail "$nail_file" "timeout"
        continue
    fi

    if [ "$actual_exit" -ne "$expected_exit" ]; then
        echo -e "  ${RED}✗ $nail_file — exit code $actual_exit (expected $expected_exit)${NC}"
        head -5 "$actual_err" | sed 's/^/      stderr: /'
        fail "$nail_file" "exit code $actual_exit != $expected_exit"
        continue
    fi

    if ! diff -q "$expected_file" "$actual_out" >/dev/null 2>&1; then
        echo -e "  ${RED}✗ $nail_file — output mismatch${NC}"
        diff "$expected_file" "$actual_out" | head -12 | sed 's/^/      /'
        fail "$nail_file" "output mismatch"
        continue
    fi

    # Runtime error message golden: compare stderr with Rust panic scaffolding
    # stripped (the location line and backtrace note change with the
    # transpiler's output layout; the message itself must not)
    stderr_golden="${nail_file%.nail}.stderr"
    if [ -f "$stderr_golden" ]; then
        filtered_err="$WORK_DIR/cwd/$bin_name.stderr.filtered"
        sed -E "/^$/d; /^thread '[^']*'( \([0-9]+\))? panicked at /d; /^note: run with \`RUST_BACKTRACE=1\`/d" \
            "$actual_err" > "$filtered_err"
        if ! diff -q "$stderr_golden" "$filtered_err" >/dev/null 2>&1; then
            echo -e "  ${RED}✗ $nail_file — stderr mismatch${NC}"
            diff "$stderr_golden" "$filtered_err" | head -12 | sed 's/^/      /'
            fail "$nail_file" "stderr mismatch"
            continue
        fi
    fi

    PASSED=$((PASSED + 1))
done
echo "  ran ${#RUNNABLE_NAMES[@]} programs"

# ---------------------------------------------------------------------------
# Summary
# ---------------------------------------------------------------------------
echo ""
echo "========================================="
echo "           E2E Test Summary"
echo "========================================="
echo -e "${GREEN}Passed: $PASSED${NC}"
echo -e "${RED}Failed: $FAILED${NC}"

if [ ${#FAILED_TESTS[@]} -gt 0 ]; then
    echo ""
    echo "Failed tests:"
    for t in "${FAILED_TESTS[@]}"; do
        echo "  - $t"
    done
fi

echo ""
if [ $FAILED -eq 0 ]; then
    echo -e "${GREEN}All e2e tests passed!${NC}"
    exit 0
else
    echo -e "${RED}$FAILED e2e test(s) failed${NC}"
    exit 1
fi

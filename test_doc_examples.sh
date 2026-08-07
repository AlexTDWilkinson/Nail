#!/bin/bash

# Compile every documentation example in the standard library registry.
#
# An example is not an illustration. `nail docs` prints it, the IDE's F1 panel
# inserts it into the file being edited, and the website shows it. Someone
# copies it and expects it to build. The Rust unit tests in
# `src/stdlib_registry/mod.rs` already prove every example parses and type
# checks, but the checker reads the registry's declaration of a function while
# the generated code calls the real Rust behind it. Only rustc compares the
# two. This script is that comparison, run over every example at once:
#
#   1. `nailc --dump-examples` writes every example out as a whole program
#   2. Each is transpiled to Rust
#   3. All of them compile as bins of ONE shared Cargo project, so the
#      dependencies build once and the bins build in parallel
#   4. Each is run in a working directory of its own, unless it is named in
#      tests/doc_examples_needing_the_world.txt
#
# What it catches that nothing else does: a registry signature that no longer
# matches its Rust function (wrong arity, wrong types, sync declared async), a
# missing import in generated code, a feature-gated crate the manifest forgets,
# and an example that compiles but panics on the values it was written with.
# Two examples were shipping uncompilable Rust when this was written, and four
# more panicked the moment they ran.
#
# Running them also matters for a reason beyond the examples: every example
# calls the function it documents, so running them all is the only thing that
# executes most of the standard library.
#
# Usage:
#   ./test_doc_examples.sh              # every example
#   ./test_doc_examples.sh array_       # only examples whose name starts with array_
#   ./test_doc_examples.sh array_chunk hashmap_new   # named examples
#
# First run is slow: it builds the whole stdlib dependency set once. Later runs
# reuse the same target directory and only rebuild what changed.

GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m'

PROJECT_ROOT="$(cd "$(dirname "$0")" && pwd)"
cd "$PROJECT_ROOT" || exit 1

WORK_DIR="${NAIL_DOCEX_WORK_DIR:-/tmp/nail_doc_examples_$USER}"
EX_DIR="$WORK_DIR/examples"
PROJ_DIR="$WORK_DIR/project"

echo "========================================="
echo " Nail Documentation Example Suite"
echo "========================================="

# ---------------------------------------------------------------------------
# Build nailc
# ---------------------------------------------------------------------------
echo -n "Building nailc... "
if ! cargo build --bin nailc --quiet 2>"$WORK_DIR.nailc_build.log"; then
    if ! cargo build --bin nailc; then
        echo -e "${RED}FAILED to build nailc${NC}"
        exit 1
    fi
fi
echo "done"
NAILC="$PROJECT_ROOT/target/debug/nailc"

# ---------------------------------------------------------------------------
# Stage 1: write every example out, then transpile the ones asked for
# ---------------------------------------------------------------------------
rm -rf "$EX_DIR" "$WORK_DIR/manifests"
mkdir -p "$EX_DIR" "$WORK_DIR/manifests" "$PROJ_DIR/src/bin"
rm -f "$PROJ_DIR"/src/bin/*.rs

DUMPED=$("$NAILC" "--dump-examples=$EX_DIR")
if [ -z "$DUMPED" ]; then
    echo -e "${RED}nailc wrote no examples${NC}"
    exit 1
fi
echo "$DUMPED examples in the registry"

declare -a NAIL_FILES=()
if [ $# -gt 0 ]; then
    for arg in "$@"; do
        while IFS= read -r f; do NAIL_FILES+=("$f"); done < <(find "$EX_DIR" -name "$arg*.nail" | sort)
    done
    if [ ${#NAIL_FILES[@]} -eq 0 ]; then
        echo -e "${RED}No example matches: $*${NC}"
        exit 1
    fi
else
    while IFS= read -r f; do NAIL_FILES+=("$f"); done < <(find "$EX_DIR" -name '*.nail' | sort)
fi

echo ""
echo "--- Stage 1: transpile (nailc) ---"
declare -a BIN_NAMES=()
declare -a TRANSPILE_FAILURES=()
for nail_file in "${NAIL_FILES[@]}"; do
    bin_name="$(basename "$nail_file" .nail)"

    if ! "$NAILC" "$nail_file" --transpile -o "$PROJ_DIR/src/bin/$bin_name.rs" 2>"$WORK_DIR/manifests/$bin_name.err"; then
        TRANSPILE_FAILURES+=("$bin_name: $(grep -m1 'error' "$WORK_DIR/manifests/$bin_name.err" | sed 's/^ *//')")
        rm -f "$PROJ_DIR/src/bin/$bin_name.rs"
        continue
    fi

    if ! "$NAILC" "$nail_file" --cargo-toml "--nail-path=$PROJECT_ROOT" --package-name=nail_doc_examples \
            > "$WORK_DIR/manifests/$bin_name.toml" 2>/dev/null; then
        TRANSPILE_FAILURES+=("$bin_name: Cargo.toml generation failed")
        rm -f "$PROJ_DIR/src/bin/$bin_name.rs"
        continue
    fi

    BIN_NAMES+=("$bin_name")
done
echo "  transpiled ${#BIN_NAMES[@]}/${#NAIL_FILES[@]} examples"

for failure in "${TRANSPILE_FAILURES[@]}"; do
    echo -e "  ${RED}✗ $failure${NC}"
done

if [ ${#BIN_NAMES[@]} -eq 0 ]; then
    echo -e "${RED}Nothing transpiled; aborting.${NC}"
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
print('name = "nail_doc_examples"')
print('version = "0.1.0"')
print('edition = "2021"')
print()
print("[dependencies]")
if features:
    print('nail = { path = "%s", features = [%s] }' % (nail_path, ", ".join(sorted(features))))
else:
    print('nail = { path = "%s" }' % nail_path)
for line in sorted(deps):
    print(line)
print()
# A thousand bins of debug info is tens of gigabytes for information nobody
# reads: the question here is whether the code compiles at all.
print("[profile.dev]")
print("debug = 0")
print("incremental = false")
PYEOF

# Start from the versions this repository is known to build with. Left to
# resolve on its own, a fresh project picks the newest release of every
# transitive crate, and one of them failing to compile says nothing about the
# examples: zune-jpeg 0.5 does not build on this toolchain while the 0.4 the
# lockfile pins does.
if [ -f "$PROJECT_ROOT/Cargo.lock" ]; then
    cp "$PROJECT_ROOT/Cargo.lock" "$PROJ_DIR/Cargo.lock"
fi

BUILD_LOG="$WORK_DIR/build.log"
# --keep-going matters: without it cargo stops at the first bad bin and every
# other broken example stays hidden behind it.
(cd "$PROJ_DIR" && cargo build --keep-going --message-format=short) >"$BUILD_LOG" 2>&1
BUILD_STATUS=$?

# A bin that failed to compile is named in cargo's summary line for it.
declare -a COMPILE_FAILURES=()
while IFS= read -r name; do
    COMPILE_FAILURES+=("$name")
done < <(grep -oE 'could not compile `nail_doc_examples` \(bin "[^"]+"\)' "$BUILD_LOG" | sed -E 's/.*bin "([^"]+)".*/\1/' | sort -u)

# ---------------------------------------------------------------------------
# Stage 3: run what can be run
# ---------------------------------------------------------------------------
# Every example calls the function it documents (a test in the registry proves
# it), so running the examples runs almost the whole standard library. That is
# the only thing that ever executes most of it.
#
# An example that reads a file, talks to a database, opens a window or waits on
# a keystroke cannot run unattended. Those are named in
# tests/doc_examples_needing_the_world.txt with the reason, and the file is
# checked both ways: a listed example that starts working is a line to delete,
# which is how the list shrinks instead of rotting.
NEEDS_THE_WORLD="$PROJECT_ROOT/tests/doc_examples_needing_the_world.txt"
RUN_TIMEOUT="${NAIL_DOCEX_TIMEOUT:-10}"

declare -a RUN_FAILURES=()
declare -a UNEXPECTEDLY_FINE=()
RAN=0
if [ ${#COMPILE_FAILURES[@]} -eq 0 ] && [ -f "$NEEDS_THE_WORLD" ]; then
    echo ""
    echo "--- Stage 3: run (fresh working directory each, ${RUN_TIMEOUT}s limit) ---"
    RUN_ROOT="$WORK_DIR/run"
    rm -rf "$RUN_ROOT"
    mkdir -p "$RUN_ROOT"
    for bin_name in "${BIN_NAMES[@]}"; do
        binary="$PROJ_DIR/target/debug/$bin_name"
        [ -x "$binary" ] || continue
        expected_to_fail=false
        if grep -q "^$bin_name\b" "$NEEDS_THE_WORLD"; then
            expected_to_fail=true
        fi

        run_dir="$RUN_ROOT/$bin_name"
        mkdir -p "$run_dir"
        # No display: a test suite must never put a window on somebody's screen.
        # The game and 3D examples open one, and the only reason they do not
        # here is that they are listed as needing the world. Taking the display
        # away means that stays true for an example nobody has thought about yet.
        output=$( (cd "$run_dir" && env -u DISPLAY -u WAYLAND_DISPLAY timeout "$RUN_TIMEOUT" "$binary" 2>&1 </dev/null) )
        status=$?
        rm -rf "$run_dir"
        RAN=$((RAN + 1))

        if [ $status -eq 0 ]; then
            if $expected_to_fail; then
                UNEXPECTEDLY_FINE+=("$bin_name")
            fi
        elif ! $expected_to_fail; then
            reason=$(echo "$output" | grep -m1 'Nail Error' | sed 's/^ *//')
            [ -z "$reason" ] && reason="exit $status"
            RUN_FAILURES+=("$bin_name: $reason")
        fi
    done
    echo "  ran $RAN examples, ${#RUN_FAILURES[@]} failed"
fi

echo ""
echo "========================================="
echo " Results"
echo "========================================="
echo "  examples checked:   ${#NAIL_FILES[@]}"
echo -e "  transpile failures: ${#TRANSPILE_FAILURES[@]}"
echo -e "  compile failures:   ${#COMPILE_FAILURES[@]}"
if [ $RAN -gt 0 ]; then
    echo -e "  examples run:       $RAN"
    echo -e "  run failures:       ${#RUN_FAILURES[@]}"
fi

for failure in "${RUN_FAILURES[@]}"; do
    echo -e "${RED}✗ ${failure}${NC}"
    echo "    example: $EX_DIR/${failure%%:*}.nail"
done

if [ ${#UNEXPECTEDLY_FINE[@]} -gt 0 ]; then
    echo ""
    echo -e "${YELLOW}These run fine now, so delete their lines from tests/doc_examples_needing_the_world.txt:${NC}"
    for name in "${UNEXPECTEDLY_FINE[@]}"; do
        echo "    $name"
    done
fi

for name in "${COMPILE_FAILURES[@]}"; do
    echo ""
    echo -e "${RED}✗ $name${NC}"
    sed -n "/src\/bin\/$name\.rs.*error/p" "$BUILD_LOG" | head -4 | sed 's/^/    /'
    echo "    example: $EX_DIR/$name.nail"
done

if [ ${#TRANSPILE_FAILURES[@]} -eq 0 ] && [ ${#COMPILE_FAILURES[@]} -eq 0 ] && [ ${#RUN_FAILURES[@]} -eq 0 ] && [ ${#UNEXPECTEDLY_FINE[@]} -eq 0 ]; then
    if [ $BUILD_STATUS -ne 0 ]; then
        # No example failed, so the build broke somewhere else: a dependency
        # that does not compile on this toolchain, a missing system library.
        echo -e "${YELLOW}No example failed, but cargo exited $BUILD_STATUS:${NC}"
        grep -E '^error|error(\[|:)' "$BUILD_LOG" | head -5 | sed 's/^/    /'
        echo "Full log: $BUILD_LOG"
        exit 1
    fi
    if [ $RAN -gt 0 ]; then
        echo -e "${GREEN}Every documentation example compiles, and every one that can run does.${NC}"
    else
        echo -e "${GREEN}Every documentation example compiles.${NC}"
    fi
    exit 0
fi

echo ""
echo "Full build log: $BUILD_LOG"
exit 1

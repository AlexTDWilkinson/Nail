#!/usr/bin/env bash
# The release gate: proves an installed bundle delivers the product promise -
# compile and run a Nail program with NO network, NO system Rust, and NO
# system C toolchain, using only what is inside the bundle. Run after
# build_bundle.sh (or on a fresh machine after install.sh). Release is
# blocked if this fails.
set -euo pipefail

# Versions live side by side, so with no NAIL_HOME the gate runs against the
# newest one installed.
ROOT="${NAIL_HOME:-$(ls -d /opt/nail/versions/*/ 2>/dev/null | sort -V | tail -1)}"
ROOT="${ROOT%/}"
TARGET=x86_64-unknown-linux-musl
WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT

[ -n "$ROOT" ] && [ -x "$ROOT/bin/nailc" ] || { echo "FAIL: no bundle at ${ROOT:-/opt/nail/versions}" >&2; exit 1; }

# Same steps the IDE build thread performs, with the same scrubbed env.
mkdir -p "$WORK/src"
cp "$(dirname "$0")/hello.nail" "$WORK/hello.nail"
"$ROOT/bin/nailc" "$WORK/hello.nail" --cargo-toml --nail-path="$ROOT/nail" --package-name=nail_transpilation > "$WORK/Cargo.toml"
"$ROOT/bin/nailc" "$WORK/hello.nail" --transpile -o "$WORK/src/main.rs"

# Deny network when the kernel allows unprivileged namespaces; the offline
# cargo config protects the build either way.
DENY_NET=()
if unshare -rn true 2>/dev/null; then
    DENY_NET=(unshare -rn)
fi

START=$(date +%s)
"${DENY_NET[@]}" env -i \
    PATH="$ROOT/toolchain/bin:/usr/bin:/bin" \
    HOME="$WORK" \
    CARGO_HOME="$ROOT/cargo-home" \
    CARGO_TARGET_DIR="$ROOT/cache/target" \
    "$ROOT/toolchain/bin/cargo" build --release --manifest-path "$WORK/Cargo.toml"
ELAPSED=$(( $(date +%s) - START ))

BINARY="$ROOT/cache/target/$TARGET/release/nail_transpilation"
OUTPUT="$("$BINARY")"
echo "build took ${ELAPSED}s, program printed: $OUTPUT"

if [ "$OUTPUT" != "hello from nail" ]; then
    echo "FAIL: unexpected program output" >&2
    exit 1
fi
if ! file "$BINARY" | grep -q "static"; then
    echo "FAIL: binary is not statically linked" >&2
    exit 1
fi

# The other half of the promise: a file that pins this version must reach this
# version's compiler through the launcher, with no network involved.
if [ -x /opt/nail/bin/nail ]; then
    VERSION="$(basename "$ROOT")"
    printf 'nail %s\n' "$VERSION" > "$WORK/pinned.nail"
    cat "$(dirname "$0")/hello.nail" >> "$WORK/pinned.nail"
    RESOLVED="$(/opt/nail/bin/nail which "$WORK/pinned.nail")"
    case "$RESOLVED" in
        *"pins $VERSION"*) echo "nail resolves the version line to $VERSION" ;;
        *) echo "FAIL: nail did not resolve the version line: $RESOLVED" >&2; exit 1 ;;
    esac
else
    echo "note: no launcher installed, skipped the resolution check"
fi

echo "PASS: offline build + run OK"

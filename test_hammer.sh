#!/usr/bin/env bash
# Exercises every hammer command against a throwaway store.
#
# Hammer is the only part of the toolchain whose commands nothing else runs:
# the compiler suites test the compiler, and the e2e suite tests programs, but
# a broken subcommand reaches a user untouched. `hammer run` shipped passing an
# argument nailc had never heard of, and nothing noticed. This is what notices.
#
# No network: the store is populated by hand from binaries already built, so
# this tests resolution, dispatch and the file-walking commands rather than
# downloading.
set -uo pipefail

cd "$(dirname "$0")"

PASS=0
FAIL=0
check() {
	local what="$1" expected="$2" got="$3"
	if [[ "$got" == *"$expected"* ]]; then
		PASS=$((PASS + 1))
	else
		FAIL=$((FAIL + 1))
		echo "FAIL: $what"
		echo "  expected to contain: $expected"
		echo "  got: $got"
	fi
}

echo "== building =="
cargo build --release --bin hammer --bin nailc --bin nail 2>&1 | grep -E "^error" && exit 1

WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT
export NAIL_STORE="$WORK/store"
export XDG_CONFIG_HOME="$WORK/config"
# Nothing here should ever reach the network. A wrong address makes that loud
# instead of silently slow.
export NAIL_ORIGIN="http://127.0.0.1:9"

VERSION="$(grep -m1 '^version' Cargo.toml | cut -d'"' -f2)"
STORE_VERSION="$NAIL_STORE/versions/$VERSION"
mkdir -p "$STORE_VERSION/bin" "$STORE_VERSION/toolchain/bin" "$STORE_VERSION/nail" "$STORE_VERSION/cache"
cp target/release/nail target/release/nailc "$STORE_VERSION/bin/"
touch "$STORE_VERSION/toolchain/bin/cargo" && chmod +x "$STORE_VERSION/toolchain/bin/cargo"
touch "$STORE_VERSION/nail/Cargo.toml"

HAMMER="$PWD/target/release/hammer"
cd "$WORK"

echo "== resolution =="
"$HAMMER" new demo.nail >/dev/null
check "new stamps a concrete version" "nail $VERSION" "$(head -1 demo.nail)"
check "new writes a runnable program" "print" "$(tail -1 demo.nail)"

"$HAMMER" new tracker.nail --latest >/dev/null
check "new --latest writes the sentinel" "nail latest" "$(head -1 tracker.nail)"

check "which explains an exact pin" "pins $VERSION" "$("$HAMMER" which demo.nail 2>&1)"
check "which explains an unpinned file" "pins no version" "$(printf 'print(\`x\`);\n' > bare.nail && "$HAMMER" which bare.nail 2>&1)"
check "new refuses to clobber" "already exists" "$("$HAMMER" new demo.nail 2>&1)"

echo "== dispatch =="
# The bug this suite exists for: `run` reached nailc as `--run <file>`, and
# nailc takes the file first, so it tried to open "--run" as a file. A real
# compile needs a real toolchain, which this store does not have, but argument
# order is exactly what broke and it is assertable here.
RUN_OUTPUT="$("$HAMMER" run demo.nail 2>&1)"
if [[ "$RUN_OUTPUT" == *"Error reading file '--"* ]]; then
	FAIL=$((FAIL + 1))
	echo "FAIL: run passes its mode flag where nailc expects the file"
	echo "  got: $RUN_OUTPUT"
else
	PASS=$((PASS + 1))
fi

check "forwarded subcommands reach nailc" "no version line" "$(printf 'print(\`x\`);\n' > naked.nail && "$HAMMER" -- naked.nail --check-only 2>&1)"
check "argv[0] dispatch works as nailc" "Type check successful" "$(ln -sf "$HAMMER" nailc && ./nailc demo.nail --check-only 2>&1)"

echo "== the store =="
check "list shows the version" "$VERSION" "$("$HAMMER" list 2>&1)"
check "doctor passes on a good store" "nothing wrong" "$("$HAMMER" doctor 2>&1 | tail -1)"
check "gc has nothing to take" "nothing to reclaim" "$("$HAMMER" gc 2>&1)"
check "config round-trips" "warn = 2GB" "$("$HAMMER" config warn 2GB 2>&1)"
check "config rejects nonsense" "not a size" "$("$HAMMER" config warn banana 2>&1)"

echo "== walking a tree =="
mkdir -p tree/vendor
cp demo.nail tree/kept.nail
printf 'nail 9.9.9\nprint(\`old\`);\n' > tree/vendor/theirs.nail
# The editor's settings file is called `.nail` and is not source. Restamping it
# would corrupt it.
printf 'theme=dark' > tree/.nail
# kept.nail is the only source here: vendor/ belongs to whoever wrote it and
# `.nail` is the editor's settings file.
check "fetch skips vendor and the settings file" "1 Nail files" "$("$HAMMER" fetch tree 2>&1 | head -1)"

# A file at another version is what update has to offer to move.
printf 'nail 0.0.1\nprint(\`old\`);\n' > tree/old.nail
check "update offers to move an older file" "1 file(s) would move" "$("$HAMMER" update tree --to="$VERSION" 2>&1 | head -1)"
check "update is a dry run by default" "nothing changed" "$("$HAMMER" update tree --to="$VERSION" 2>&1 | tail -1)"
check "update left the older file alone" "nail 0.0.1" "$(head -1 tree/old.nail)"
check "the settings file was left alone" "theme=dark" "$(cat tree/.nail)"

echo
if [[ $FAIL -eq 0 ]]; then
	echo "PASS: $PASS checks"
	exit 0
fi
echo "FAILED: $FAIL of $((PASS + FAIL)) checks"
exit 1

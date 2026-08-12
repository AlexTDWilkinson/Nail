#!/usr/bin/env bash
# Exercises every nail command against a throwaway store.
#
# The launcher is the only part of the toolchain whose commands nothing else runs:
# the compiler suites test the compiler, and the e2e suite tests programs, but
# a broken subcommand reaches a user untouched. `nail run` shipped passing an
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
cargo build --release --bin nail-launcher --bin nailc --bin nail 2>&1 | grep -E "^error" && exit 1

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

LAUNCHER="$PWD/target/release/nail-launcher"
cd "$WORK"

echo "== resolution =="
"$LAUNCHER" new demo.nail >/dev/null
check "new stamps a concrete version" "nail $VERSION" "$(head -1 demo.nail)"
check "new writes a runnable program" "print" "$(tail -1 demo.nail)"

"$LAUNCHER" new tracker.nail --latest >/dev/null
check "new --latest writes the sentinel" "nail latest" "$(head -1 tracker.nail)"

check "which explains an exact pin" "pins $VERSION" "$("$LAUNCHER" which demo.nail 2>&1)"
check "which explains an unpinned file" "pins no version" "$(printf 'print(\`x\`);\n' > bare.nail && "$LAUNCHER" which bare.nail 2>&1)"
check "new refuses to clobber" "already exists" "$("$LAUNCHER" new demo.nail 2>&1)"

echo "== dispatch =="
# The bug this suite exists for: `run` reached nailc as `--run <file>`, and
# nailc takes the file first, so it tried to open "--run" as a file. A real
# compile needs a real toolchain, which this store does not have, but argument
# order is exactly what broke and it is assertable here.
RUN_OUTPUT="$("$LAUNCHER" run demo.nail 2>&1)"
if [[ "$RUN_OUTPUT" == *"Error reading file '--"* ]]; then
	FAIL=$((FAIL + 1))
	echo "FAIL: run passes its mode flag where nailc expects the file"
	echo "  got: $RUN_OUTPUT"
else
	PASS=$((PASS + 1))
fi

check "forwarded subcommands reach nailc" "no version line" "$(printf 'print(\`x\`);\n' > naked.nail && "$LAUNCHER" -- naked.nail --check-only 2>&1)"
# The extension is how the desktop recognises a file, not something a person
# should have to type.
check "a bare name resolves to the .nail file" "pins $VERSION" "$("$LAUNCHER" which demo 2>&1)"
check "the extension still works" "pins $VERSION" "$("$LAUNCHER" which demo.nail 2>&1)"
"$LAUNCHER" new fresh >/dev/null
check "new adds the extension" "nail $VERSION" "$(head -1 fresh.nail)"
check "new accepts it spelled out" "nail $VERSION" "$("$LAUNCHER" new spelled.nail >/dev/null && head -1 spelled.nail)"

# `nail hi` has to reach the editor the same as `nail hi.nail`. It did not:
# the bare name fell through to the compiler, which tried to open a file
# called "hi".
BARE_OUTPUT="$("$LAUNCHER" demo 2>&1)"
if [ "${BARE_OUTPUT#*Error reading file}" != "$BARE_OUTPUT" ]; then
	FAIL=$((FAIL + 1))
	echo "FAIL: a bare file name did not resolve to the .nail file"
	echo "  got: $BARE_OUTPUT"
else
	PASS=$((PASS + 1))
fi
check "an unknown subcommand still forwards" "notacommand" "$("$LAUNCHER" notacommand 2>&1)"

# Running a command with no arguments prints its help. Bare `nail` used to
# open an empty editor, which nobody expects from a command line tool.
check "bare nail prints the help" "Writing code:" "$("$LAUNCHER" 2>&1)"
check "the help prefixes every command" "nail install <version>" "$("$LAUNCHER" help 2>&1)"
check "the help leads with the explicit open" "nail open <file>" "$("$LAUNCHER" help 2>&1)"

echo "== the other commands =="
check "check type checks without building" "Type check successful" "$("$LAUNCHER" check demo 2>&1)"
check "check reports a broken file" "no version line" "$("$LAUNCHER" check naked 2>&1)"
check "bare docs lists the whole library" "functions in" "$("$LAUNCHER" docs 2>&1)"
check "bare docs is not the website" "archive" "$("$LAUNCHER" docs 2>&1 | head -1)"
check "docs answers about the language, not just the library" "Error Handling" "$("$LAUNCHER" docs errors 2>&1 | head -1)"
check "bare docs offers the language topics too" "The language itself" "$("$LAUNCHER" docs 2>&1)"
check "docs finds a function exactly" "string_split(input:s, delimiter:s):a:s" "$("$LAUNCHER" docs string_split 2>&1)"
check "docs searches when there is no exact match" "stats_percentile" "$("$LAUNCHER" docs percentile 2>&1)"
check "docs says so when nothing matches" "Nothing in the standard library" "$("$LAUNCHER" docs zzzznotathing 2>&1)"
# `website` and `github` hand a URL to the desktop only when a person is
# watching. Output here is captured, so these check the printed address and no
# browser opens. That guard is in open_url, and it is the reason running this
# suite no longer leaves two tabs behind.
check "website prints the address" "$NAIL_ORIGIN" "$("$LAUNCHER" website 2>&1)"
check "github prints the repository" "github.com" "$("$LAUNCHER" github 2>&1)"
check "test needs a tests directory" "no tests/ directory" "$("$LAUNCHER" test 2>&1)"
mkdir -p tests && cp demo.nail tests/test_demo.nail
check "test finds files in tests/" "1 test(s)" "$("$LAUNCHER" test 2>&1 | head -1)"
check "test filters by pattern" "no test in tests/ matches" "$("$LAUNCHER" test nosuchname 2>&1)"

echo "== the store =="
check "list shows the version" "$VERSION" "$("$LAUNCHER" list 2>&1)"
check "doctor passes on a good store" "nothing wrong" "$("$LAUNCHER" doctor 2>&1 | tail -1)"
check "doctor names the store it is talking about" "$NAIL_STORE" "$("$LAUNCHER" doctor 2>&1 | head -1)"

# Nail installs into a directory of the user's own, so a release built on the
# release machine lands at a path it was not built at. Two settings inside it
# hold that path and import rewrites both, which is also what makes a release
# exported from one machine work on another.
FOREIGN="$WORK/foreign/0.9.9"
mkdir -p "$FOREIGN/bin" "$FOREIGN/cargo-home" "$FOREIGN/toolchain/bin" "$FOREIGN/nail"
BUILT_AT=/opt/nail/versions/0.9.9
cat >"$FOREIGN/cargo-home/config.toml" <<EOF
[source.vendored-sources]
directory = "$BUILT_AT/cargo-home/vendor"

[target.x86_64-unknown-linux-musl]
linker = "$BUILT_AT/toolchain/lib/rustlib/x86_64-unknown-linux-gnu/bin/rust-lld"
EOF
# Enough of a release to count as installed. bin/nailc is not executable, so
# the warm-up build that follows a relocation fails at once instead of trying
# to compile anything, which is all this needs it to do.
touch "$FOREIGN/bin/nail" "$FOREIGN/bin/nailc" "$FOREIGN/toolchain/bin/cargo" "$FOREIGN/nail/Cargo.toml"
tar -cf "$WORK/foreign.tar" -C "$WORK/foreign" 0.9.9
"$LAUNCHER" import "$WORK/foreign.tar" >/dev/null 2>&1
FOREIGN_CONFIG="$NAIL_STORE/versions/0.9.9/cargo-home/config.toml"
check "import moves the vendored sources to where they landed" "directory = \"$NAIL_STORE/versions/0.9.9/cargo-home/vendor\"" "$(grep directory "$FOREIGN_CONFIG")"
check "import moves the linker too" "linker = \"$NAIL_STORE/versions/0.9.9/toolchain" "$(grep linker "$FOREIGN_CONFIG")"
check "nothing is left pointing at the machine that built it" "left=0" "left=$(grep -c "$BUILT_AT" "$FOREIGN_CONFIG" || true)"
check "the imported release is installed" "0.9.9" "$("$LAUNCHER" list 2>&1)"
"$LAUNCHER" remove 0.9.9 >/dev/null 2>&1

# Which store nail uses is decided by where nail itself is, so a machine with
# both a home install and a machine-wide one has each using its own. Without
# this the two would race for one directory and whoever did not own it would
# be unable to install anything.
OWN="$WORK/own-store"
mkdir -p "$OWN/bin" "$OWN/versions"
cp "$LAUNCHER" "$OWN/bin/nail"
check "nail uses the store it was installed into" "store $OWN" "$(env -u NAIL_STORE "$OWN/bin/nail" doctor 2>&1 | head -1)"
check "gc has nothing to take" "nothing to reclaim" "$("$LAUNCHER" gc 2>&1)"
check "config round-trips" "warn = 2GB" "$("$LAUNCHER" config warn 2GB 2>&1)"
check "config rejects nonsense" "not a size" "$("$LAUNCHER" config warn banana 2>&1)"

echo "== walking a tree =="
mkdir -p tree/vendor
cp demo.nail tree/kept.nail
printf 'nail 9.9.9\nprint(\`old\`);\n' > tree/vendor/theirs.nail
# The editor's settings file is called `.nail` and is not source. Restamping it
# would corrupt it.
printf 'theme=dark' > tree/.nail
# kept.nail is the only source here: vendor/ belongs to whoever wrote it and
# `.nail` is the editor's settings file.
check "fetch skips vendor and the settings file" "1 Nail files" "$("$LAUNCHER" fetch tree 2>&1 | head -1)"

# A file at another version is what update has to offer to move.
printf 'nail 0.0.1\nprint(\`old\`);\n' > tree/old.nail
check "update offers to move an older file" "1 file(s) would move" "$("$LAUNCHER" update tree --to="$VERSION" 2>&1 | head -1)"
check "update is a dry run by default" "nothing changed" "$("$LAUNCHER" update tree --to="$VERSION" 2>&1 | tail -1)"
check "update left the older file alone" "nail 0.0.1" "$(head -1 tree/old.nail)"
check "the settings file was left alone" "theme=dark" "$(cat tree/.nail)"

echo
if [[ $FAIL -eq 0 ]]; then
	echo "PASS: $PASS checks"
	exit 0
fi
echo "FAILED: $FAIL of $((PASS + FAIL)) checks"
exit 1

#!/usr/bin/env bash
# Put the working tree's IDE into the local install, so `nail open <file>`
# runs the code you just changed.
#
#   ./scripts/install_ide_locally.sh          # the IDE
#   ./scripts/install_ide_locally.sh nailc    # the compiler as well
#   ./scripts/install_ide_locally.sh all      # IDE, compiler and launcher
#
# The nail on your PATH is only the launcher: it reads a file's version line
# and execs the IDE out of <store>/versions/<version>/bin. That copy is what
# the editor actually runs, which is why rebuilding alone changes nothing.
#
# This is the developer's shortcut, not a release. A release is built by
# bundle/build_bundle.sh, which pins a toolchain, links against musl and ships
# a warm build cache. Nothing here is that: it is the ordinary cargo build of
# the moment, dropped into place.
set -euo pipefail

cd "$(dirname "${BASH_SOURCE[0]}")/.."

WHAT="${1:-ide}"
case "$WHAT" in
	ide) BINARIES=(nail) ;;
	nailc) BINARIES=(nail nailc) ;;
	all) BINARIES=(nail nailc nail-launcher) ;;
	*) echo "usage: $0 [ide|nailc|all]" >&2; exit 1 ;;
esac

VERSION="$(grep -m1 '^version' Cargo.toml | cut -d'"' -f2)"

# The same order nail itself uses: whatever was asked for, then a home install,
# then a machine-wide one. A home install is the default now, so looking only
# in /opt would miss most of them.
USER_STORE="${XDG_DATA_HOME:-$HOME/.local/share}/nail"
if [ -n "${NAIL_ROOT:-}" ]; then
	ROOT="$NAIL_ROOT"
elif [ -d "$USER_STORE/versions/$VERSION" ]; then
	ROOT="$USER_STORE"
else
	ROOT=/opt/nail
fi
VERSION_DIR="$ROOT/versions/$VERSION"

if [ ! -d "$VERSION_DIR/bin" ]; then
	echo "error: $VERSION_DIR/bin does not exist." >&2
	echo "       Install a release first (bundle/install.sh), or set NAIL_ROOT." >&2
	exit 1
fi
if [ ! -w "$VERSION_DIR/bin" ]; then
	echo "error: $VERSION_DIR/bin is not writable by $USER." >&2
	exit 1
fi

for binary in "${BINARIES[@]}"; do
	echo "building $binary"
	cargo build --release --bin "$binary"
done

for binary in "${BINARIES[@]}"; do
	# The launcher is the one thing that goes on PATH, and it lives a level up
	# from the versions it starts.
	if [ "$binary" = "nail-launcher" ]; then
		destination="$ROOT/bin/nail"
	else
		destination="$VERSION_DIR/bin/$binary"
	fi

	# Copy beside the target and rename over it. Writing straight onto a
	# running binary fails with "text file busy", and a rename leaves any
	# editor that is already open running the old file undisturbed.
	install -m 755 "target/release/$binary" "$destination.incoming"
	mv -f "$destination.incoming" "$destination"
	echo "installed $destination"
done

echo
echo "Quit any open editor and start it again to pick this up."

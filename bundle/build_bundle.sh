#!/usr/bin/env bash
# Builds the Nail release bundle: one immutable directory holding the IDE, a
# pinned Rust toolchain, vendored crate sources, the nail crate source, and a
# pre-warmed build cache - then tars it for distribution.
#
# Cargo fingerprints embed absolute paths, so the warm cache shipped in here is
# only valid at the path it was warmed at. A machine that installs to that same
# path gets it for free. Every other machine has the launcher rewrite the two
# settings that hold the path and spend one build re-warming, which is why this
# no longer has to be built at the place it will be installed. Many versions
# live side by side either way.
#
# Build-machine requirements (users need none of this):
#   - network access (crates.io + static.rust-lang.org)
#   - a musl C compiler for the plain-C crates (sqlite, ring, compression):
#     musl-gcc from the musl-tools package is enough. No C++ anywhere.
set -euo pipefail

RUST_VERSION="${RUST_VERSION:-1.92.0}"
HOST=x86_64-unknown-linux-gnu
TARGET=x86_64-unknown-linux-musl
REPO="$(cd "$(dirname "$0")/.." && pwd)"
DIST=https://static.rust-lang.org/dist
DOWNLOADS="$REPO/bundle/downloads"
NAIL_VERSION="$(grep -m1 '^version' "$REPO/Cargo.toml" | cut -d'"' -f2)"
ROOT="${NAIL_HOME:-/opt/nail/versions/$NAIL_VERSION}"

if ! mkdir -p "$ROOT" 2>/dev/null || [ ! -w "$ROOT" ]; then
    echo "error: $ROOT is not writable. Run: sudo mkdir -p $ROOT && sudo chown $USER $ROOT" >&2
    exit 1
fi
mkdir -p "$DOWNLOADS" "$ROOT/bin" "$ROOT/cache" "$ROOT/cargo-home"

# --- musl C compiler (needed only while building the bundle) --------------
MUSL_CC="${CC_x86_64_unknown_linux_musl:-$(command -v x86_64-linux-musl-gcc || command -v musl-gcc || true)}"
if [ -z "$MUSL_CC" ]; then
    echo "error: no musl C compiler found (install musl-tools or set CC_x86_64_unknown_linux_musl)" >&2
    exit 1
fi
export CC_x86_64_unknown_linux_musl="$MUSL_CC"

# --- 1. Pinned Rust toolchain --------------------------------------------
fetch() {
    local file="$1"
    if [ ! -f "$DOWNLOADS/$file" ]; then
        echo "downloading $file"
        curl -fL --proto '=https' "$DIST/$file" -o "$DOWNLOADS/$file"
    fi
}

if [ ! -x "$ROOT/toolchain/bin/cargo" ]; then
    fetch "rust-$RUST_VERSION-$HOST.tar.xz"
    fetch "rust-std-$RUST_VERSION-$TARGET.tar.xz"
    for archive in "rust-$RUST_VERSION-$HOST" "rust-std-$RUST_VERSION-$TARGET"; do
        rm -rf "$DOWNLOADS/$archive"
        tar -xJf "$DOWNLOADS/$archive.tar.xz" -C "$DOWNLOADS"
        # Only the components a build actually uses. The combined installer
        # otherwise brings rust-docs, rust-analyzer, clippy and rustfmt, which
        # is hundreds of megabytes nobody unpacks.
        WANTED="$(grep -E '^(rustc|cargo|rust-std)' "$DOWNLOADS/$archive/components" | paste -sd,)"
        "$DOWNLOADS/$archive/install.sh" --prefix="$ROOT/toolchain" --disable-ldconfig --components="$WANTED" >/dev/null
        rm -rf "$DOWNLOADS/$archive"
    done
fi
echo "toolchain: $("$ROOT/toolchain/bin/rustc" --version)"

# --- 2. IDE + nailc binaries (host target, bundled toolchain) -------------
# The launcher is built here too but does NOT go in the bundle: it is version
# independent, one copy serves every release, and it is published on its own.
# --locked: build exactly what Cargo.lock says or fail. A release must be one
# set of bytes, so a bundle build is never allowed to quietly resolve a newer
# dependency than the one that was tested.
(cd "$REPO" && PATH="$ROOT/toolchain/bin:$PATH" "$ROOT/toolchain/bin/cargo" build --locked --release --bin nail --bin nailc --bin nail-launcher)
cp "$REPO/target/release/nail" "$REPO/target/release/nailc" "$ROOT/bin/"

# --- 3. nail crate source (generated programs depend on it by path) -------
rm -rf "$ROOT/nail"
mkdir -p "$ROOT/nail"
cp "$REPO/Cargo.toml" "$REPO/Cargo.lock" "$ROOT/nail/"
cp -r "$REPO/src" "$ROOT/nail/src"
# assets/ is crate source too: game.rs embeds the font from it with
# include_bytes!, so a nail/ without it does not compile. The specification is
# there for the same reason: docs.rs embeds it so `nail docs` can answer
# questions about the language, and the answer belongs to this version.
cp -r "$REPO/assets" "$ROOT/nail/assets"
cp "$REPO/nail_language_spec.md" "$ROOT/nail/"

# --- 4. Warmup projects ---------------------------------------------------
# superset: every crate the registry can emit, all nail features.
# minimal: exactly what a plain hello-world program generates, so both the
# all-features and no-features builds of the nail crate are cached.
rm -rf "$ROOT/warmup"
mkdir -p "$ROOT/warmup/superset/src" "$ROOT/warmup/minimal/src"

"$ROOT/bin/nailc" --cargo-toml-superset --nail-path="$ROOT/nail" --package-name=nail_transpilation > "$ROOT/warmup/superset/Cargo.toml"
echo 'fn main() {}' > "$ROOT/warmup/superset/src/main.rs"

"$ROOT/bin/nailc" "$REPO/bundle/hello.nail" --cargo-toml --nail-path="$ROOT/nail" --package-name=nail_transpilation > "$ROOT/warmup/minimal/Cargo.toml"
"$ROOT/bin/nailc" "$REPO/bundle/hello.nail" --transpile -o "$ROOT/warmup/minimal/src/main.rs"

# --- 5. Vendor all crate sources and write the build configuration --------
(cd "$ROOT/warmup/superset" && "$ROOT/toolchain/bin/cargo" vendor --sync "$ROOT/warmup/minimal/Cargo.toml" "$ROOT/cargo-home/vendor" >/dev/null)

cat > "$ROOT/cargo-home/config.toml" <<EOF
# All build configuration for Nail programs lives here. The IDE invokes the
# bundled cargo with CARGO_HOME pointing at this directory and a scrubbed
# environment, so this file is the single source of truth.

[source.crates-io]
replace-with = "vendored-sources"

[source.vendored-sources]
directory = "$ROOT/cargo-home/vendor"

[net]
offline = true

[build]
target = "$TARGET"

[target.$TARGET]
# rust-lld + self-contained CRT: linking needs no system compiler or libc
linker = "$ROOT/toolchain/lib/rustlib/$HOST/bin/rust-lld"
rustflags = ["-C", "link-self-contained=yes"]
EOF

# --- 6. Warm the shared cache at its final path ---------------------------
warm_build() {
    env -i \
        PATH="$ROOT/toolchain/bin:/usr/bin:/bin" \
        HOME="$HOME" \
        CARGO_HOME="$ROOT/cargo-home" \
        CARGO_TARGET_DIR="$ROOT/cache/target" \
        CC_x86_64_unknown_linux_musl="$MUSL_CC" \
        "$ROOT/toolchain/bin/cargo" build --release --manifest-path "$1/Cargo.toml"
}
warm_build "$ROOT/warmup/superset"
warm_build "$ROOT/warmup/minimal"

# --- 7. Package -----------------------------------------------------------
# The warmup projects were scaffolding for building the cache. The cache itself
# is what ships, so they do not need to.
rm -rf "$ROOT/warmup"

# The archive holds one directory named for the version, which is exactly what
# the launcher unpacks into /opt/nail/versions.
#
# -9e is the strongest xz preset and -T0 spreads it over every core, so the
# extra ratio costs build-machine time rather than wall clock. Users pay the
# download once and unpack with the fast path either way.
OUT="$REPO/bundle/nail-$NAIL_VERSION-linux-x86_64.tar.xz"
XZ_OPT="-9e -T0" tar -cJf "$OUT" -C "$(dirname "$ROOT")" "$(basename "$ROOT")"
echo "bundle: $OUT ($(du -h "$OUT" | cut -f1))"

# Which commit this was built from, beside the tarball rather than inside it,
# so publishing can check it without decompressing 800MB. A bundle takes half
# an hour to build and the repository moves in minutes, so without this the two
# drift apart quietly and the site hands out a toolchain missing the commands
# its own installer advertises. Which is exactly what happened.
BUILT_FROM="$(cd "$REPO" && git rev-parse HEAD 2>/dev/null || echo unknown)"
if [ -n "$(cd "$REPO" && git status --porcelain 2>/dev/null)" ]; then
    BUILT_FROM="$BUILT_FROM-dirty"
fi
printf '%s\n' "$BUILT_FROM" > "$OUT.built-from"

cat <<EOF

Built. To publish it:
  ./deploy/releases.sh $OUT
EOF

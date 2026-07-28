#!/usr/bin/env bash
# Builds the Nail release bundle: one immutable directory at /opt/nail holding
# the IDE, a pinned Rust toolchain, vendored crate sources, the nail crate
# source, and a pre-warmed build cache - then tars it for distribution.
#
# Must be built AT the final install path (/opt/nail): cargo fingerprints
# embed absolute paths, and building at the final path is what makes the
# shipped warm cache valid on user machines.
#
# Build-machine requirements (users need none of this):
#   - network access (crates.io + static.rust-lang.org)
#   - musl C/C++ cross compilers for the C-containing crates
#     (sqlite, ring, duckdb): musl-gcc from musl-tools plus a musl g++
#     such as x86_64-linux-musl-g++ from https://musl.cc
set -euo pipefail

RUST_VERSION="${RUST_VERSION:-1.92.0}"
HOST=x86_64-unknown-linux-gnu
TARGET=x86_64-unknown-linux-musl
ROOT="${NAIL_HOME:-/opt/nail}"
REPO="$(cd "$(dirname "$0")/.." && pwd)"
DIST=https://static.rust-lang.org/dist
DOWNLOADS="$REPO/bundle/downloads"
NAIL_VERSION="$(grep -m1 '^version' "$REPO/Cargo.toml" | cut -d'"' -f2)"

if ! mkdir -p "$ROOT" 2>/dev/null || [ ! -w "$ROOT" ]; then
    echo "error: $ROOT is not writable. Run: sudo mkdir -p $ROOT && sudo chown $USER $ROOT" >&2
    exit 1
fi
mkdir -p "$DOWNLOADS" "$ROOT/bin" "$ROOT/cache" "$ROOT/cargo-home"

# --- musl C/C++ compilers (needed only while building the bundle) ---------
MUSL_CC="${CC_x86_64_unknown_linux_musl:-$(command -v x86_64-linux-musl-gcc || command -v musl-gcc || true)}"
MUSL_CXX="${CXX_x86_64_unknown_linux_musl:-$(command -v x86_64-linux-musl-g++ || true)}"
if [ -z "$MUSL_CC" ]; then
    echo "error: no musl C compiler found (install musl-tools or set CC_x86_64_unknown_linux_musl)" >&2
    exit 1
fi
if [ -z "$MUSL_CXX" ]; then
    echo "error: no musl C++ compiler found - needed for the duckdb package." >&2
    echo "       Install x86_64-linux-musl-cross from https://musl.cc or set CXX_x86_64_unknown_linux_musl" >&2
    exit 1
fi
export CC_x86_64_unknown_linux_musl="$MUSL_CC"
export CXX_x86_64_unknown_linux_musl="$MUSL_CXX"

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
        "$DOWNLOADS/$archive/install.sh" --prefix="$ROOT/toolchain" --disable-ldconfig >/dev/null
        rm -rf "$DOWNLOADS/$archive"
    done
fi
echo "toolchain: $("$ROOT/toolchain/bin/rustc" --version)"

# --- 2. IDE + nailc binaries (host target, bundled toolchain) -------------
(cd "$REPO" && PATH="$ROOT/toolchain/bin:$PATH" "$ROOT/toolchain/bin/cargo" build --release --bin nail --bin nailc)
cp "$REPO/target/release/nail" "$REPO/target/release/nailc" "$ROOT/bin/"

# --- 3. nail crate source (generated programs depend on it by path) -------
rm -rf "$ROOT/nail"
mkdir -p "$ROOT/nail"
cp "$REPO/Cargo.toml" "$ROOT/nail/"
cp -r "$REPO/src" "$ROOT/nail/src"

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
        CXX_x86_64_unknown_linux_musl="$MUSL_CXX" \
        "$ROOT/toolchain/bin/cargo" build --release --manifest-path "$1/Cargo.toml"
}
warm_build "$ROOT/warmup/superset"
warm_build "$ROOT/warmup/minimal"

# --- 7. Package -----------------------------------------------------------
OUT="$REPO/bundle/nail-$NAIL_VERSION-linux-x86_64.tar.xz"
tar -cJf "$OUT" -C "$(dirname "$ROOT")" "$(basename "$ROOT")"
echo "bundle: $OUT ($(du -h "$OUT" | cut -f1))"

#!/usr/bin/env bash
# Build the browser demos the website serves: each one is a Nail program
# compiled to WebAssembly with `nailc --target=wasm`.
#
#   ./scripts/build_wasm_demos.sh
#
# Output lands in wasm_demos/ (gitignored, regenerated at will):
#   wasm_demos/viewer/pkg/     the 3D model viewer compiled for the browser
#   wasm_demos/monument.glb    the model the viewer fetches, at the path the
#                              program names, resolved against <base href="/wasm/">
#
# The wasm-bindgen CLI version must match the version each demo's lockfile
# resolves. If they drift, wasm-bindgen says so and names the fix.
set -euo pipefail

cd "$(dirname "${BASH_SOURCE[0]}")/.."

NAIL_ROOT="$(pwd)"
BUILD_DIR="$NAIL_ROOT/target/wasm_demo_builds"
OUT_DIR="$NAIL_ROOT/wasm_demos"

build_demo() {
	local name="$1"
	local source="$2"
	local crate="${name}_wasm"
	local project="$BUILD_DIR/$crate"

	echo "Building $name from $source..."
	mkdir -p "$project/src"
	cargo run --quiet --bin nailc "$source" --target=wasm --stdout > "$project/src/lib.rs"
	cargo run --quiet --bin nailc "$source" --cargo-toml --target=wasm \
		"--package-name=$crate" "--nail-path=$NAIL_ROOT" > "$project/Cargo.toml"
	(cd "$project" && cargo build --quiet --release --target wasm32-unknown-unknown)
	mkdir -p "$OUT_DIR/$name"
	wasm-bindgen "$project/target/wasm32-unknown-unknown/release/$crate.wasm" \
		--target web --out-dir "$OUT_DIR/$name/pkg"
	echo "  -> wasm_demos/$name/pkg ($(du -h "$OUT_DIR/$name/pkg/${crate}_bg.wasm" | cut -f1))"
}

build_demo platformer examples/platformer.nail
build_demo viewer examples/model_viewer.nail
build_demo monolith examples/monolith_field.nail

# The viewer program loads monument.glb from beside its source file. In the
# browser the same path is fetched as a URL against the embed page's
# <base href="/wasm/">, so the file sits at the top of the mounted directory.
cp examples/monument.glb "$OUT_DIR/monument.glb"

echo "Browser demos built into wasm_demos/"

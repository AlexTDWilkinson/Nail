#!/usr/bin/env bash
# Build the browser demos the website serves: each one is a Nail program
# compiled to WebAssembly with `nailc --target=wasm`.
#
#   ./scripts/build_wasm_demos.sh
#
# Output lands in wasm_demos/ (gitignored, regenerated at will):
#   wasm_demos/viewer/pkg/     the 3D model viewer compiled for the browser
#   wasm_demos/examples/monument.glb   the model the viewer fetches, at the path
#                                  the program names, resolved from /games/
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

# The viewer program fetches examples/monument.glb relative to its page under
# /games/, so the file sits at that path inside the mounted directory.
mkdir -p "$OUT_DIR/examples"
cp examples/monument.glb "$OUT_DIR/examples/monument.glb"

echo "Browser demos built into wasm_demos/"

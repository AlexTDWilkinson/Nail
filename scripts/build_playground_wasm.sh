#!/usr/bin/env bash
# Build the website playground: the Nail compiler itself compiled to
# WebAssembly, exported to the page as one function (playground_wasm/).
#
#   ./scripts/build_playground_wasm.sh
#
# Output lands in wasm_demos/playground/pkg (gitignored, regenerated at
# will), served by the website's /wasm static mount like the game demos.
set -euo pipefail

cd "$(dirname "${BASH_SOURCE[0]}")/.."

(cd playground_wasm && cargo build --quiet --release --target wasm32-unknown-unknown)
mkdir -p wasm_demos/playground
wasm-bindgen playground_wasm/target/wasm32-unknown-unknown/release/playground_wasm.wasm \
	--target web --out-dir wasm_demos/playground/pkg
echo "Playground built into wasm_demos/playground/pkg ($(du -h wasm_demos/playground/pkg/playground_wasm_bg.wasm | cut -f1))"

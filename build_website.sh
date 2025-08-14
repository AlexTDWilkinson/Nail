#!/bin/bash
set -e

echo "Building Nail website for Render deployment..."

# Build the Nail compiler first
echo "Step 1: Building Nail compiler..."
cargo build --release --bin nailc

# Transpile the nail_website.nail to Rust
echo "Step 2: Transpiling nail_website.nail to Rust..."
./target/release/nailc examples/nail_website.nail --transpile

# Use our existing test compilation infrastructure
echo "Step 3: Compiling website with Rust..."
cd examples
rustc nail_website.rs \
    -o ../target/release/nail_website \
    -L ../target/release/deps \
    --extern nail=../target/release/libnail.rlib \
    --extern tokio=$(find ../target/release/deps -name "libtokio-*.rlib" | head -1) \
    --extern rayon=$(find ../target/release/deps -name "librayon-*.rlib" | head -1) \
    --extern dashmap=$(find ../target/release/deps -name "libdashmap-*.rlib" | head -1) \
    --extern futures=$(find ../target/release/deps -name "libfutures-*.rlib" | head -1) \
    --edition 2021 \
    -C opt-level=3

echo "Build complete! Website binary is at ./target/release/nail_website"
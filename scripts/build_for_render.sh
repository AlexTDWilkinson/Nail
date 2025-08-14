#!/bin/bash
set -e

echo "Building Nail website for Render deployment (with IntoParallelIterator fix)..."

# Step 1: Build the Nail compiler
echo "Step 1: Building Nail compiler..."
cargo build --release --bin nailc

# Step 2: Transpile nail_website.nail to Rust and place in correct location
echo "Step 2: Transpiling nail_website.nail to Rust..."
rm -f examples/nail_website.rs
./target/release/nailc examples/nail_website.nail --transpile
cp examples/nail_website.rs src/bin/nail-website.rs

# Step 3: Build the nail-website binary
echo "Step 3: Building nail-website binary..."
cargo build --release --bin nail-website

# Step 4: Move binary to root for Render
echo "Step 4: Moving binary to root directory..."
mv ./target/release/nail-website ./nail-website
chmod +x ./nail-website

echo "Build complete! Website binary is at ./nail-website"
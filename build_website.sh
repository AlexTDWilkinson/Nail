#!/bin/bash
set -e

echo "Building Nail website for Render deployment (with IntoParallelIterator fix)..."

# Step 1: Build the Nail compiler
echo "Step 1: Building Nail compiler..."
cargo build --release --bin nailc

# Step 2: Transpile nail_website.nail to Rust (force fresh build)
echo "Step 2: Transpiling nail_website.nail to Rust..."
rm -f examples/nail_website.rs
./target/release/nailc examples/nail_website.nail --transpile

# Step 3: Replace the main.rs with transpiled code for Render default build
echo "Step 3: Updating main.rs for Render..."
cp examples/nail_website.rs src/main.rs

# Step 4: Build the website using the main project
echo "Step 4: Building website..."
cargo build --release

echo "Build complete! Website binary is at ./target/release/nail-website"
#!/bin/bash
set -e

echo "Building Nail website for Render deployment..."

# Build the Nail compiler first
echo "Step 1: Building Nail compiler..."
cargo build --release --bin nailc

# Transpile the nail_website.nail to Rust
echo "Step 2: Transpiling nail_website.nail to Rust..."
./target/release/nailc examples/nail_website.nail --transpile

# Create the nail_website_build directory and move the transpiled file
echo "Step 3: Setting up website build directory..."
mkdir -p nail_website_build/src
cp examples/nail_website.rs nail_website_build/src/main.rs || echo "Warning: nail_website.rs not found, will be created by transpilation"

# Create Cargo.toml for the website build
cat > nail_website_build/Cargo.toml << 'EOF'
[package]
name = "nail-website"
version = "0.1.0"
edition = "2021"

[[bin]]
name = "nail-website"
path = "src/main.rs"

[dependencies]
tokio = { version = "1", features = ["full"] }
nail = { path = ".." }
rayon = "1.7"
dashmap = "6.1"
futures = "0.3"
EOF

# Build the website binary
echo "Step 4: Building nail_website binary..."
cd nail_website_build
cargo build --release

# Copy the binary to the expected location for Render
echo "Step 5: Copying binary to target directory..."
cp target/release/nail-website ../target/release/nail_website

echo "Build complete! Website binary is at ./target/release/nail_website"
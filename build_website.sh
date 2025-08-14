#!/bin/bash
set -e

echo "Building Nail website for Render deployment..."

# Build the Nail compiler first
echo "Step 1: Building Nail compiler..."
cargo build --release --bin nailc

# Transpile the nail_website.nail to Rust
echo "Step 2: Transpiling nail_website.nail to Rust..."
./target/release/nailc examples/nail_website.nail --transpile

# Create the nail_website_server directory and move the transpiled file
echo "Step 3: Setting up website server directory..."
mkdir -p nail_website_server/src
mv examples/nail_website.rs nail_website_server/src/main.rs

# Create Cargo.toml for the website server
cat > nail_website_server/Cargo.toml << 'EOF'
[package]
name = "nail_website_server"
version = "0.1.0"
edition = "2021"

[[bin]]
name = "nail_website"
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
cd nail_website_server
cargo build --release

# Copy the binary to the expected location for Render
echo "Step 5: Copying binary to target directory..."
cp target/release/nail_website ../target/release/

echo "Build complete! Website binary is at ./target/release/nail_website"
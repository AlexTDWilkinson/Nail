#!/bin/bash
set -e

echo "Building Nail website for Render deployment..."

# Step 1: Build the Nail compiler
echo "Step 1: Building Nail compiler..."
cargo build --release --bin nailc

# Step 2: Transpile nail_website.nail to Rust
echo "Step 2: Transpiling nail_website.nail to Rust..."
./target/release/nailc examples/nail_website.nail --transpile

# Step 3: Create a proper cargo project for the website
echo "Step 3: Creating website cargo project..."
mkdir -p nail_website_build/src
cd nail_website_build

# Create Cargo.toml with all required dependencies
cat > Cargo.toml << 'EOF'
[package]
name = "nail-website"
version = "0.1.0"
edition = "2021"

[dependencies]
nail = { path = ".." }
rayon = "1.10.0"
tokio = { version = "1", features = ["full"] }
axum = "0.7"
futures = "0.3"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
reqwest = { version = "0.11", features = ["json"] }
dashmap = "6.1.0"
EOF

# Copy the transpiled code as main.rs
cp ../examples/nail_website.rs src/main.rs

# Build the website
echo "Step 4: Building website..."
cargo build --release

echo "Build complete! Website binary is at ./target/release/nail-website"
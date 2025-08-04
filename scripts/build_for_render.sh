#!/bin/bash
set -e

echo "=== Building Nail Website for Render ==="

# Setup Rust in user directory for Render
export RUSTUP_HOME=$HOME/.rustup
export CARGO_HOME=$HOME/.cargo
export PATH=$CARGO_HOME/bin:$PATH

# Install Rust if needed
if ! command -v rustup &> /dev/null; then
    echo "Installing Rust in user directory..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --no-modify-path
fi

# Source cargo env
source $CARGO_HOME/env

# Update to latest nightly Rust (needed for async_closure feature)
echo "Installing Rust nightly..."
$CARGO_HOME/bin/rustup install nightly
$CARGO_HOME/bin/rustup default nightly

# Step 1: Build the Nail compiler
echo "Building Nail compiler..."
cargo +nightly build --release --bin nailc

# Step 2: Transpile the website from Nail to Rust
echo "Transpiling nail_website.nail to Rust..."
./target/release/nailc examples/nail_website.nail --transpile

# Check if transpilation succeeded
if [ ! -f "examples/nail_website.rs" ]; then
    echo "Error: Transpilation failed - no .rs file generated"
    exit 1
fi

# Step 3: Create a temporary Cargo project for the website
echo "Creating temporary Cargo project..."
mkdir -p nail_website_build/src
cd nail_website_build

# Create Cargo.toml with necessary dependencies
cat > Cargo.toml << 'EOF'
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
axum = "0.7"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
rand = "0.8"
futures = "0.3"
pulldown-cmark = "0.9"
dashmap = "6.1.0"

[profile.release]
opt-level = 3
lto = true
codegen-units = 1
strip = true
EOF

# Copy the transpiled Rust code
cp ../examples/nail_website.rs src/main.rs

# Step 4: Build the website binary
echo "Building website binary..."
cargo +nightly build --release

# Step 5: Copy the binary to the root directory for Render
echo "Copying binary to root..."
cp target/release/nail-website ../nail-website

# Step 6: Make it executable
chmod +x ../nail-website

# Step 7: Clean up
cd ..
rm -rf nail_website_build
rm -f examples/nail_website.rs

echo "=== Build complete! Binary: ./nail-website ==="
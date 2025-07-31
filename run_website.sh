#!/bin/bash

# Kill any existing process on port 8080
lsof -ti:8080 | xargs kill -9 2>/dev/null

# Ensure we're in the Nail directory
cd "$(dirname "$0")"

echo "Transpiling Nail website to Rust..."

# Create the nail_website_server directory if it doesn't exist
mkdir -p nail_website_server/src

# Transpile the Nail code to Rust
cargo run --bin nailc examples/nail_website.nail --transpile 2>&1 | awk '/^use nail::std_lib;/,/^Rust code saved to:/' | head -n -1 > nail_website_server/src/main.rs

if [ ${PIPESTATUS[0]} -eq 0 ] && [ -s nail_website_server/src/main.rs ]; then
    echo "Transpilation successful!"
    
    # Create Cargo.toml if it doesn't exist
    if [ ! -f nail_website_server/Cargo.toml ]; then
        cat > nail_website_server/Cargo.toml << 'EOF'
[package]
name = "nail_website_server"
version = "0.1.0"
edition = "2021"

[dependencies]
tokio = { version = "1", features = ["full"] }
nail = { path = ".." }
dashmap = "6"
EOF
    fi
    
    echo "Building Nail website server..."
    
    # Build the server
    cd nail_website_server && cargo build --release
    
    if [ $? -eq 0 ]; then
        echo ""
        echo "Starting Nail website server..."
        echo "Visit http://localhost:8080 to see the Nail programming language website"
        echo "This version has working interactive features with HTMX!"
        echo ""
        
        # Run the server from the Nail directory so it can find the example files
        cd ..
        ./nail_website_server/target/release/nail_website_server
    else
        echo "Build failed!"
    fi
else
    echo "Transpilation failed!"
fi
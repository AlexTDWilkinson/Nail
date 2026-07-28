#!/bin/bash

# Kill any existing process on port 8080
lsof -ti:8080 | xargs kill -9 2>/dev/null

# Ensure we're in the Nail directory
cd "$(dirname "$0")"

echo "Transpiling Nail website to Rust..."

# Create the nail_website_server directory if it doesn't exist
mkdir -p nail_website_server/src

# Transpile the Nail code to Rust (writes examples/nail_website.rs)
if cargo run --bin nailc examples/nail_website.nail --transpile && [ -s examples/nail_website.rs ]; then
    mv examples/nail_website.rs nail_website_server/src/main.rs
    echo "Transpilation successful!"

    # Regenerate Cargo.toml with usage-driven dependencies from the stdlib registry
    cargo run --bin nailc examples/nail_website.nail --cargo-toml "--nail-path=.." --package-name=nail_website_server > nail_website_server/Cargo.toml


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
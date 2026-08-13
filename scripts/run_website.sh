#!/bin/bash

# Kill any existing process on port 8080
lsof -ti:8080 | xargs kill -9 2>/dev/null

# Ensure we're in the Nail directory
cd "$(dirname "$0")/.."

echo "Transpiling Nail website to Rust..."

# The generated server project lives under target/ with the other build
# output. Everything in it is regenerated here, nothing is tracked.
SERVER_DIR=target/nail_website_server
mkdir -p "$SERVER_DIR/src"

# Transpile the Nail code to Rust (writes examples/website/main.rs)
if cargo run --bin nailc examples/website/main.nail --transpile && [ -s examples/website/main.rs ]; then
    mv examples/website/main.rs "$SERVER_DIR/src/main.rs"
    echo "Transpilation successful!"

    # Regenerate Cargo.toml with usage-driven dependencies from the stdlib registry
    cargo run --bin nailc examples/website/main.nail --cargo-toml "--nail-path=../.." --package-name=nail_website_server > "$SERVER_DIR/Cargo.toml"


    echo "Building Nail website server..."

    # Build the server for this machine's CPU: local runs never ship the binary
    cd "$SERVER_DIR" && RUSTFLAGS="-C target-cpu=native" cargo build --release

    if [ $? -eq 0 ]; then
        echo ""
        echo "Starting Nail website server..."
        echo "Visit http://localhost:8080 to see the Nail programming language website"
        echo "This version has working interactive features with HTMX!"
        echo ""

        # The server runs in the website's own directory, the same rule
        # `nail run` applies, so its file-relative reads resolve.
        cd ../../examples/website
        exec ../../target/nail_website_server/target/release/nail_website_server
    else
        echo "Build failed!"
    fi
else
    echo "Transpilation failed!"
fi

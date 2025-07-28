#!/bin/bash

# Kill any existing process on port 8082
lsof -ti:8082 | xargs kill -9 2>/dev/null

echo "Compiling blog with routes example..."
cargo run -- examples/blog_with_routes.nail

if [ $? -eq 0 ]; then
    echo ""
    echo "Running generated Rust code directly..."
    echo ""
    echo "Starting blog server..."
    echo "Visit http://localhost:8082 to see the blog with routing"
    
    # Create a temporary Rust project to run the generated code
    temp_dir=$(mktemp -d)
    cd "$temp_dir"
    
    # Create Cargo.toml with proper dependencies
    cat > Cargo.toml << 'EOF'
[package]
name = "blog_routes"
version = "0.1.0"
edition = "2021"

[[bin]]
name = "blog_routes"
path = "main.rs"

[dependencies]
tokio = { version = "1", features = ["full"] }
axum = "0.7"
dashmap = "5.5"
pulldown-cmark = "0.12.2"

[lib]
name = "Nail"
path = "lib.rs"
EOF

    # Copy the generated Rust file
    cp ~/Nail/examples/blog_with_routes.rs main.rs
    
    # Create a lib.rs that includes all the std_lib modules
    cat > lib.rs << 'EOF'
pub mod std_lib {
    pub mod string;
    pub mod int;
    pub mod float;
    pub mod array;
    pub mod array_functional;
    pub mod print;
    pub mod math;
    pub mod time;
    pub mod env;
    pub mod process;
    pub mod http;
    pub mod error;
    pub mod io;
    pub mod path;
    pub mod fs;
    pub mod hashmap;
    pub mod markdown;
}
EOF

    # Copy all the std_lib files
    mkdir -p std_lib
    cp ~/Nail/src/parser/std_lib/*.rs std_lib/
    
    # Change to examples directory to access blog_posts
    cd ~/Nail/examples
    
    # Run the generated code
    cargo run --manifest-path "$temp_dir/Cargo.toml" --bin blog_routes
    
    # Clean up
    rm -rf "$temp_dir"
else
    echo "Compilation failed!"
fi
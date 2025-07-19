#!/bin/bash
echo "Compiling blog example..."
cargo run --bin nailc examples/blog_with_files.nail --skip-check

echo -e "\nCompiling Rust code..."
cd examples
rustc blog_with_files.rs -o blog_with_files --edition 2021 -L ../target/debug/deps -L ../target/debug --extern tokio=../target/debug/deps/libtokio*.rlib --extern Nail=../target/debug/libNail.rlib 2>&1 || {
    echo "Direct compilation failed, trying with cargo..."
    cd ..
    cargo build --bin blog_with_files 2>&1 || {
        echo "Cargo build failed, running the Rust file directly..."
        cd examples
        cargo init --name blog_with_files . 2>/dev/null || true
        echo '[dependencies]' >> Cargo.toml
        echo 'tokio = { version = "1", features = ["full"] }' >> Cargo.toml
        echo 'Nail = { path = ".." }' >> Cargo.toml
        cargo run --bin blog_with_files
    }
}

echo -e "\nRunning blog server..."
./blog_with_files
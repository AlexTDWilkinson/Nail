#!/bin/bash
cd /home/alex/Nail

echo "Compiling blog example..."
cargo run --bin nailc examples/blog_with_files.nail --skip-check

echo -e "\nRunning generated Rust code directly..."
cd examples
rustc blog_with_files.rs \
  --edition 2021 \
  -L ../target/debug/deps \
  --extern tokio=$(ls ../target/debug/deps/libtokio-*.rlib | head -1) \
  --extern axum=$(ls ../target/debug/deps/libaxum-*.rlib | head -1) \
  --extern pulldown_cmark=$(ls ../target/debug/deps/libpulldown_cmark-*.rlib | head -1) \
  --extern Nail=../target/debug/libNail.rlib \
  -o blog_server

if [ $? -eq 0 ]; then
  echo -e "\nStarting blog server..."
  echo "Visit http://localhost:8080 to see the blog"
  ./blog_server
else
  echo "Compilation failed"
fi
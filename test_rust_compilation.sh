#!/bin/bash

# Test Rust compilation of successfully transpiled Nail files
# This is slow but ensures the generated Rust code actually compiles

set -e

GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo "========================================="
echo "Testing Rust Compilation of Transpiled Files"
echo "========================================="
echo ""

# Create a temporary directory for test projects
TEMP_DIR="/tmp/nail_rust_tests_$$"
mkdir -p "$TEMP_DIR"

# Track results
PASSED=0
FAILED=0
FAILED_FILES=()

# Function to test a single file's Rust compilation
test_rust_compilation() {
    local nail_file="$1"
    local basename=$(basename "$nail_file" .nail)
    
    echo -n "Testing $nail_file... "
    
    # First transpile the file
    if ! ./target/debug/nailc "$nail_file" --transpile 2>/dev/null; then
        echo -e "${RED}✗ Transpilation failed${NC}"
        FAILED=$((FAILED + 1))
        FAILED_FILES+=("$nail_file (transpilation)")
        return 1
    fi
    
    # Check if .rs file was generated
    local rs_file="${nail_file%.nail}.rs"
    if [ ! -f "$rs_file" ]; then
        echo -e "${RED}✗ No .rs file generated${NC}"
        FAILED=$((FAILED + 1))
        FAILED_FILES+=("$nail_file (no output)")
        return 1
    fi
    
    # Create a test project
    local test_dir="$TEMP_DIR/${basename}_test"
    mkdir -p "$test_dir/src"
    
    # Move the generated .rs file
    mv "$rs_file" "$test_dir/src/main.rs"
    
    # Create Cargo.toml with proper dependencies
    cat > "$test_dir/Cargo.toml" << 'EOF'
[package]
name = "nail_test"
edition = "2021"

[dependencies]
tokio = { version = "1", features = ["rt-multi-thread", "macros", "time"] }
rayon = "1.5"
futures = "0.3"
nail = { path = "PROJECT_PATH" }
dashmap = "6.1.0"
EOF
    
    # Replace PROJECT_PATH with actual path
    sed -i "s|PROJECT_PATH|$(pwd)|" "$test_dir/Cargo.toml"
    
    # Try to build the Rust project
    if cd "$test_dir" && cargo build --quiet 2>/dev/null; then
        echo -e "${GREEN}✓ PASS${NC}"
        PASSED=$((PASSED + 1))
        cd - > /dev/null
        return 0
    else
        echo -e "${RED}✗ Rust compilation failed${NC}"
        FAILED=$((FAILED + 1))
        FAILED_FILES+=("$nail_file")
        cd - > /dev/null
        return 1
    fi
}

# Check if specific files were provided as arguments
if [ $# -gt 0 ]; then
    # Test only the specified files
    for file in "$@"; do
        if [ -f "$file" ]; then
            test_rust_compilation "$file"
        else
            echo -e "${RED}File not found: $file${NC}"
            FAILED=$((FAILED + 1))
            FAILED_FILES+=("$file (not found)")
        fi
    done
else
    # Test all .nail files in tests/ and examples/
    echo "Testing files in tests/ directory..."
    for file in tests/*.nail; do
        if [ -f "$file" ]; then
            test_rust_compilation "$file"
        fi
    done

    echo ""
    echo "Testing files in examples/ directory..."
    for file in examples/*.nail examples/*/*.nail; do
        if [ -f "$file" ]; then
            test_rust_compilation "$file"
        fi
    done
fi

# Clean up
rm -rf "$TEMP_DIR"

# Summary
echo ""
echo "========================================="
echo "           Test Summary"
echo "========================================="
echo -e "${GREEN}Passed: $PASSED${NC}"
echo -e "${RED}Failed: $FAILED${NC}"

if [ ${#FAILED_FILES[@]} -gt 0 ]; then
    echo ""
    echo "Failed files:"
    for file in "${FAILED_FILES[@]}"; do
        echo "  - $file"
    done
fi

echo ""
if [ $FAILED -eq 0 ]; then
    echo -e "${GREEN}All Rust compilation tests passed!${NC}"
    exit 0
else
    echo -e "${RED}$FAILED tests failed${NC}"
    exit 1
fi
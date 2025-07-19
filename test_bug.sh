#!/bin/bash
echo "Testing error handling transpilation bug..."

# Create test file
cat > test_error_bug.nail << 'EOF'
f divide(a:i, b:i):i!e {
    if {
        b == 0 => { r e(`Division by zero`); },
        else => { r a / b; }
    }
}

result:i = dangerous(divide(10, 2));
print(string_from(result));
EOF

# Transpile
./target/debug/nailc test_error_bug.nail test_error_bug.rs

echo -e "\n=== Checking transpiled Rust code ==="
grep -n "return.*/" test_error_bug.rs

echo -e "\n=== Looking for Ok() wrapping ==="
if grep -q "return Ok(a / b)" test_error_bug.rs; then
    echo "✓ CORRECT: Return is wrapped in Ok()"
else
    echo "✗ BUG: Return is NOT wrapped in Ok()"
    echo "Found instead:"
    grep "return a / b" test_error_bug.rs
fi

# Clean up
rm -f test_error_bug.nail test_error_bug.rs
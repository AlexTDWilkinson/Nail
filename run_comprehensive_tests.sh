#!/bin/bash

# Comprehensive Nail Language Test Runner
# Runs all tests for both /tests and /examples directories
# Performs lexer, parser, type checker, transpiler, and Rust compilation checks
# Cleans up generated .rs files automatically

set -e  # Exit on any error

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Statistics
TOTAL_TESTS=0
PASSED_TESTS=0
FAILED_TESTS=0
SKIPPED_TESTS=0
COMPILATION_FAILURES=()
COMPILATION_ERROR_DIR=""
RESULTS_FILE="test_results.txt"

# Temporary directory for generated files
TEMP_DIR=$(mktemp -d)
trap "rm -rf $TEMP_DIR" EXIT

# Function to print colored output
print_status() {
    local status=$1
    local message=$2
    case $status in
        "PASS")
            echo -e "${GREEN}*${NC} $message"
            PASSED_TESTS=$((PASSED_TESTS + 1))
            # Don't log passes to results file
            ;;
        "FAIL")
            echo -e "${RED}✗${NC} $message"
            FAILED_TESTS=$((FAILED_TESTS + 1))
            if [[ -n "$RESULTS_FILE" ]]; then
                echo "[FAIL] $message" >> "$RESULTS_FILE"
            fi
            ;;
        "SKIP")
            echo -e "${YELLOW}⚠${NC} $message"
            ((SKIPPED_TESTS++))
            # Don't log skips to results file
            ;;
        "INFO")
            echo -e "${BLUE}i${NC} $message"
            ;;
    esac
}

# Function to run comprehensive test on a single file
test_nail_file() {
    local file_path=$1
    local file_name=$(basename "$file_path")
    local is_fail_test=false
    
    ((TOTAL_TESTS++))
    
    # Check if this is a "fail_" test (should fail compilation)
    if [[ $file_name == fail_* ]]; then
        is_fail_test=true
    fi
    
    echo
    print_status "INFO" "Testing: $file_path"
    
    # Step 1: Lexer check
    echo -n "  Lexer: "
    set +e
    lexer_output=$(./target/debug/nailc "$file_path" --lex-only 2>&1)
    lexer_exit_code=$?
    set -e
    if [[ $lexer_exit_code -eq 0 ]]; then
        if [[ $lexer_output == *"LexerError"* ]]; then
            if $is_fail_test; then
                echo -e "${GREEN}PASS (correctly failed)${NC}"
            else
                echo -e "${RED}FAIL (lexer error)${NC}"
                echo "    $lexer_output" | head -10
                print_status "FAIL" "$file_name - Lexer errors"
                return 1
            fi
        else
            echo -e "${GREEN}PASS${NC}"
        fi
    else
        echo -e "${RED}FAIL (lexer crash)${NC}"
        print_status "FAIL" "$file_name - Lexer crash"
        return 1
    fi
    
    # Step 2: Parser check
    echo -n "  Parser: "
    set +e
    parser_output=$(./target/debug/nailc "$file_path" --parse-only 2>&1)
    parser_exit_code=$?
    set -e
    if [[ $parser_exit_code -eq 0 ]]; then
        if [[ $parser_output == *"Parse error"* ]]; then
            if $is_fail_test; then
                echo -e "${GREEN}PASS (correctly failed)${NC}"
                print_status "PASS" "$file_name - Correctly failed at parse stage"
                return 0
            else
                echo -e "${RED}FAIL (parse error)${NC}"
                echo "    $parser_output" | head -10
                print_status "FAIL" "$file_name - Parse errors"
                return 1
            fi
        else
            echo -e "${GREEN}PASS${NC}"
        fi
    else
        echo -e "${RED}FAIL (parser crash)${NC}"
        print_status "FAIL" "$file_name - Parser crash"
        return 1
    fi
    
    # Step 3: Type checker check
    echo -n "  Type Checker: "
    set +e  # Temporarily disable exit on error for this command
    checker_output=$(./target/debug/nailc "$file_path" --check-only 2>&1)
    checker_exit_code=$?
    set -e  # Re-enable exit on error
    if [[ $checker_exit_code -eq 0 ]]; then
        if [[ $checker_output == *"Type check errors"* ]]; then
            if $is_fail_test; then
                echo -e "${GREEN}PASS (correctly failed)${NC}"
                print_status "PASS" "$file_name - Correctly failed at type check stage"
                return 0
            else
                echo -e "${RED}FAIL (type errors)${NC}"
                echo "    $checker_output" | head -15
                print_status "FAIL" "$file_name - Type check errors"
                return 1
            fi
        else
            echo -e "${GREEN}PASS${NC}"
        fi
    else
        echo -e "${RED}FAIL (type checker crash)${NC}"
        echo "    $checker_output" | head -15
        print_status "FAIL" "$file_name - Type checker crash"
        return 1
    fi
    
    # Step 4: Transpiler check
    echo -n "  Transpiler: "
    local rust_file="$TEMP_DIR/$(basename "$file_path" .nail).rs"
    set +e
    transpiler_output=$(./target/debug/nailc "$file_path" --transpile 2>&1)
    transpiler_exit_code=$?
    set -e
    if [[ $transpiler_exit_code -eq 0 ]]; then
        if [[ -f "${file_path%.nail}.rs" ]]; then
            # Move generated file to temp directory
            mv "${file_path%.nail}.rs" "$rust_file"
            echo -e "${GREEN}PASS${NC}"
        else
            echo -e "${RED}FAIL (no output file)${NC}"
            print_status "FAIL" "$file_name - Transpiler produced no output"
            return 1
        fi
    else
        echo -e "${RED}FAIL (transpiler error)${NC}"
        echo "    $transpiler_output" | head -15
        print_status "FAIL" "$file_name - Transpiler errors"
        return 1
    fi
    
    # Step 5: Rust compilation check using the same approach as IDE
    echo -n "  Rust Compilation: "
    
    # Create temporary cargo project in tests/ directory
    local temp_project="tests/$(basename "$file_path" .nail)_transpilation"
    mkdir -p "$temp_project/src"
    
    # Create Cargo.toml using programmatically determined dependencies
    echo "Determining required dependencies..."
    set +e
    dependencies_output=$(./target/debug/nailc "$file_path" --deps-only 2>/dev/null | grep "=" | grep -v "^=")
    dependencies_exit_code=$?
    set -e
    
    cat > "$temp_project/Cargo.toml" << EOF
[package]
name = "nail_transpilation_test"
edition = "2021"

[dependencies]
tokio = { version = "1", features = ["rt-multi-thread", "macros"] }
nail = { path = "../.." }
EOF
    
    # Add dynamically determined dependencies (excluding tokio which we already include)
    if [[ $dependencies_exit_code -eq 0 && -n "$dependencies_output" ]]; then
        echo "$dependencies_output" | grep -v "^tokio" >> "$temp_project/Cargo.toml"
    fi
    
    # Copy the generated Rust file to the project
    cp "$rust_file" "$temp_project/src/main.rs"
    
    # Try to compile with cargo build (not run, just build)
    set +e
    cargo_output=$(cd "$temp_project" && cargo build --quiet 2>&1)
    cargo_exit_code=$?
    set -e
    if [[ $cargo_exit_code -eq 0 ]]; then
        echo -e "${GREEN}PASS${NC}"
        print_status "PASS" "$file_name - All checks passed"
        # Clean up on success
        rm -rf "$temp_project"
    else
        echo -e "${RED}FAIL (cargo build error)${NC}"
        if [[ -n "$COMPILATION_ERROR_DIR" ]]; then
            local error_file="$COMPILATION_ERROR_DIR/$(basename "$file_path" .nail).error"
            echo "File: $file_path" > "$error_file"
            echo "Compilation Error:" >> "$error_file"
            echo "$cargo_output" >> "$error_file"
            echo "    Error written to: $error_file"
        fi
        if $VERBOSE; then
            echo "    $cargo_output" | head -20
        fi
        print_status "FAIL" "$file_name - Rust compilation failed"
        COMPILATION_FAILURES+=("$file_path")
        # Leave failed compilation directory for debugging
        return 1
    fi
    
    return 0
}

# Function to show help
show_help() {
    cat << EOF
Comprehensive Nail Language Test Runner

USAGE:
    $0 [OPTIONS] [FILES...]

OPTIONS:
    -h, --help              Show this help message
    -v, --verbose           Show verbose output
    --tests-only            Run only files in tests/ directory
    --examples-only         Run only files in examples/ directory
    --include-fails         Include fail_* test files (default: yes)
    --exclude-fails         Exclude fail_* test files
    --parallel              Run tests in parallel (experimental)
    --randomize             Run tests in random order
    --write-errors DIR      Write compilation errors to files in DIR
    --continue-on-error     Continue testing even after compilation failures

ARGUMENTS:
    FILES...                Specific .nail files to test (optional)
                           If not provided, tests all files in tests/ and examples/

EXAMPLES:
    $0                                          # Test all files
    $0 --tests-only                             # Test only tests/ directory
    $0 tests/test_specific.nail                 # Test specific file
    $0 --exclude-fails --examples-only          # Test examples, skip fail tests
    $0 -v tests/test_*.nail                     # Test with verbose output

The runner performs these checks on each file:
1. Lexer analysis (--lex-only)
2. Parser analysis (--parse-only) 
3. Type checker analysis (--check-only)
4. Transpiler to Rust (--transpile)
5. Rust compilation check (rustc)

Generated .rs files are automatically cleaned up after testing.
EOF
}

# Parse command line arguments
VERBOSE=false
TESTS_ONLY=false
EXAMPLES_ONLY=false
INCLUDE_FAILS=true
PARALLEL=false
RANDOMIZE=false
CONTINUE_ON_ERROR=false
SPECIFIC_FILES=()

while [[ $# -gt 0 ]]; do
    case $1 in
        -h|--help)
            show_help
            exit 0
            ;;
        -v|--verbose)
            VERBOSE=true
            shift
            ;;
        --tests-only)
            TESTS_ONLY=true
            shift
            ;;
        --examples-only)
            EXAMPLES_ONLY=true
            shift
            ;;
        --include-fails)
            INCLUDE_FAILS=true
            shift
            ;;
        --exclude-fails)
            INCLUDE_FAILS=false
            shift
            ;;
        --parallel)
            PARALLEL=true
            shift
            ;;
        --randomize)
            RANDOMIZE=true
            shift
            ;;
        --write-errors)
            if [[ -z "$2" || "$2" == --* ]]; then
                echo "Error: --write-errors requires a directory argument"
                exit 1
            fi
            COMPILATION_ERROR_DIR="$2"
            shift 2
            ;;
        --continue-on-error)
            CONTINUE_ON_ERROR=true
            shift
            ;;
        *.nail)
            SPECIFIC_FILES+=("$1")
            shift
            ;;
        *)
            # Handle any other files (including those from wildcard expansion)
            if [[ -f "$1" && "$1" == *.nail ]]; then
                SPECIFIC_FILES+=("$1")
            else
                echo "Unknown option or file: $1"
                show_help
                exit 1
            fi
            shift
            ;;
    esac
done

# Header
echo -e "${BLUE}========================================${NC}"
echo -e "${BLUE}  Comprehensive Nail Language Tests${NC}"
echo -e "${BLUE}========================================${NC}"
echo

# Initialize results file
cat > "$RESULTS_FILE" << EOF
Nail Test Failures
==================
Only recording failures for debugging

Format: [FAIL] test_name.nail - reason

EOF

# Build the nailc compiler first
print_status "INFO" "Building nailc compiler..."
if ! cargo build --bin nailc --quiet; then
    print_status "FAIL" "Failed to build nailc compiler"
    exit 1
fi
echo -e "${GREEN}✓${NC} nailc compiler built successfully"

# Create error directory if specified
if [[ -n "$COMPILATION_ERROR_DIR" ]]; then
    mkdir -p "$COMPILATION_ERROR_DIR"
    print_status "INFO" "Compilation errors will be written to: $COMPILATION_ERROR_DIR"
fi

# Determine which files to test
TEST_FILES=()

if [[ ${#SPECIFIC_FILES[@]} -gt 0 ]]; then
    # Test specific files provided
    TEST_FILES=("${SPECIFIC_FILES[@]}")
    print_status "INFO" "Testing ${#TEST_FILES[@]} specific files"
else
    # Find all .nail files in tests/ and examples/
    if [[ $TESTS_ONLY == false ]]; then
        if [[ -d "examples" ]]; then
            while IFS= read -r -d '' file; do
                TEST_FILES+=("$file")
            done < <(find examples -name "*.nail" -type f -print0 2>/dev/null || true)
        fi
    fi
    
    if [[ $EXAMPLES_ONLY == false ]]; then
        if [[ -d "tests" ]]; then
            while IFS= read -r -d '' file; do
                TEST_FILES+=("$file")
            done < <(find tests -name "*.nail" -type f -print0 2>/dev/null || true)
        fi
    fi
    
    # Filter out fail tests if requested
    if [[ $INCLUDE_FAILS == false ]]; then
        FILTERED_FILES=()
        for file in "${TEST_FILES[@]}"; do
            if [[ $(basename "$file") != fail_* ]]; then
                FILTERED_FILES+=("$file")
            fi
        done
        TEST_FILES=("${FILTERED_FILES[@]}")
    fi
    
    print_status "INFO" "Found ${#TEST_FILES[@]} .nail files to test"

fi

# Debug: show which files we found
echo "Files to test: ${#TEST_FILES[@]} files found"

if [[ ${#TEST_FILES[@]} -eq 0 ]]; then
    print_status "FAIL" "No .nail files found to test"
    exit 1
fi

# Sort or randomize files
if [[ ${#TEST_FILES[@]} -gt 1 ]]; then
    if [[ $RANDOMIZE == true ]]; then
        # Shuffle the array
        readarray -t TEST_FILES < <(printf '%s\n' "${TEST_FILES[@]}" | shuf)
        print_status "INFO" "Tests will run in random order"
    else
        # Sort for consistent output
        readarray -t TEST_FILES < <(printf '%s\n' "${TEST_FILES[@]}" | sort)
    fi
fi

# Run tests
START_TIME=$(date +%s)

for file in "${TEST_FILES[@]}"; do
    if [[ -f "$file" ]]; then
        if test_nail_file "$file"; then
            # Test passed
            true
        else
            # Test failed
            if [[ $CONTINUE_ON_ERROR == false ]]; then
                # Check if it's a compilation failure
                if [[ " ${COMPILATION_FAILURES[@]} " =~ " ${file} " ]]; then
                    print_status "INFO" "Stopping due to compilation failure (use --continue-on-error to continue)"
                    break
                fi
            fi
        fi
    else
        print_status "SKIP" "$file - File not found"
    fi
done

END_TIME=$(date +%s)
DURATION=$((END_TIME - START_TIME))

# Summary
echo
echo -e "${BLUE}========================================${NC}"
echo -e "${BLUE}           Test Summary${NC}"
echo -e "${BLUE}========================================${NC}"
echo -e "Total Tests:     ${TOTAL_TESTS}"
echo -e "${GREEN}Passed:          ${PASSED_TESTS}${NC}"
echo -e "${RED}Failed:          ${FAILED_TESTS}${NC}"
echo -e "${YELLOW}Skipped:         ${SKIPPED_TESTS}${NC}"
echo -e "Duration:        ${DURATION}s"
echo

# Print compilation failures summary if any
if [[ ${#COMPILATION_FAILURES[@]} -gt 0 ]]; then
    echo -e "${RED}========================================${NC}"
    echo -e "${RED}       Compilation Failures${NC}"
    echo -e "${RED}========================================${NC}"
    echo -e "${RED}The following files failed Rust compilation:${NC}"
    for fail_file in "${COMPILATION_FAILURES[@]}"; do
        echo -e "  ${RED}✗${NC} $fail_file"
        if [[ -n "$COMPILATION_ERROR_DIR" ]]; then
            local error_file="$COMPILATION_ERROR_DIR/$(basename "$fail_file" .nail).error"
            if [[ -f "$error_file" ]]; then
                echo -e "    → Error details: $error_file"
            fi
        fi
    done
    echo
fi

# Clean up generated .rs files (but keep transpilation dirs for debugging if tests failed)
print_status "INFO" "Cleaning up generated .rs files..."
find tests/ examples/ -name "*.rs" -type f -delete 2>/dev/null || true

if [[ $FAILED_TESTS -eq 0 ]]; then
    print_status "INFO" "Cleaning up transpilation directories..."
    find tests/ examples/ -name "*_transpilation" -type d -exec rm -rf {} + 2>/dev/null || true
else
    print_status "INFO" "Keeping transpilation directories for debugging failed tests"
fi

# Write summary to results file
{
    echo ""
    echo "========================================="
    echo "Summary: $PASSED_TESTS passed, $FAILED_TESTS failed, $SKIPPED_TESTS skipped out of $TOTAL_TESTS tests"
    echo "Duration: ${DURATION}s"
    echo "Generated: $(date)"
} >> "$RESULTS_FILE"

if [[ $FAILED_TESTS -eq 0 ]]; then
    print_status "PASS" "All tests completed successfully!"
    print_status "INFO" "Results written to: $RESULTS_FILE"
    exit 0
else
    print_status "FAIL" "$FAILED_TESTS tests failed"
    print_status "INFO" "Results written to: $RESULTS_FILE"
    exit 1
fi
# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## CRITICAL: Clean Up Generated Files

**ALWAYS clean up after yourself**:

1. Delete any `.rs` files generated in `examples/` or `tests/` directories after transpilation
2. Remove any temporary test files you create (unless they should be permanent tests)
3. Clean up test compilation directories (`*_transpilation/` folders)
4. Never commit generated/transpiled `.rs` files to the repository
5. When running tests, the test script automatically cleans up - let it do its job

Generated files to watch for and delete:
- `examples/*.rs` (transpiler output)
- `tests/*.rs` (transpiler output)
- `tests/*_transpilation/` (test compilation directories)
- Any temporary test files you create for experimentation

## CRITICAL: Test After Every Change

**IMPORTANT**: After making ANY changes to the Nail language implementation (lexer, parser, checker, transpiler), you MUST:

1. Run `./run_comprehensive_tests.sh` immediately after your changes
2. Verify all previously passing tests still pass
3. If any tests fail that previously passed, investigate and fix the regression
4. Only proceed with additional changes after all tests pass

This is non-negotiable to maintain language stability and prevent regressions.

## CRITICAL: Never Use Workarounds

**NEVER implement workarounds for bugs in the Nail language implementation**. If you encounter a bug:

1. Fix the bug at its source in the compiler/checker/lexer/parser
2. Do NOT modify test files to work around the bug
3. Do NOT suggest temporary solutions
4. Always implement the proper fix in the codebase

This ensures the language remains consistent and bugs are actually fixed, not hidden.

## CRITICAL: Maintain Clean Architecture

**NEVER hard-code special cases for individual functions in core compiler components**. This is terrible architecture:

1. Do NOT add function-specific logic in the type checker (e.g., special handling for "reduce", "map", "filter", etc.)
2. Do NOT hard-code function names in the parser, lexer, checker, or transpiler
3. Do NOT hard-code imports, dependencies, or library calls in the transpiler - generate them based on what's actually used
4. The ONLY exception is `print()` which may need special handling for formatting
5. Instead, design proper abstractions:
   - Use type system features that can express generic relationships
   - Create extensible mechanisms for type inference
   - Keep ALL function-specific logic in configuration files or registries (like stdlib_registry.rs)
   - The core compiler should be completely agnostic to what functions exist
   - Generate imports based on actual usage, not hardcoded assumptions
6. If you must add special handling, it belongs in the registry, not in core compiler logic

## CRITICAL: No TODOs in Code

**NEVER leave TODO comments in code**. This is unprofessional:

1. Do NOT write "TODO: fix this later" or similar comments
2. Do NOT commit half-finished implementations with TODOs
3. Either implement it properly or don't implement it all
4. If something needs future work, track it properly in documentation or issues, not in code comments

## File Management Guidelines

- **DO NOT MAKE MULTIPLE VERSIONS OF .NAIL FILES, LIKE NAIL_WEBSITE.nail and NAIL_WEBSITE_V2.NAIL**

## Testing Guidelines

**ABSOLUTELY MOST IMPORTANT THING Testing Principle**:
- Use the fast test scripts (`test_lexer_parser.sh`, `test_type_checker.sh`, `test_transpiler.sh`) for development
- These scripts test specific compiler stages quickly without slow Rust compilation
- Only test Rust compilation manually when absolutely necessary

## Development Commands

- **Run development mode**: `./start.sh` - Runs `cargo watch -x run` with debug flags enabled
- **Build**: `cargo build` or `cargo build --release`
- **Build compiler**: `cargo build --bin nailc` - Builds the Nail compiler binary

## Testing Commands

### Running Tests

**Fast Test Scripts** (use these for rapid development):
- **`./test_lexer_parser.sh`** - Tests lexer and parser only (very fast)
- **`./test_type_checker.sh`** - Tests type checking for files that pass lexer/parser (fast)
- **`./test_transpiler.sh`** - Tests transpilation for files that pass type checking (fast)
- **`./test_rust_compilation.sh`** - Tests Rust compilation of transpiled files (VERY SLOW - only use when specifically needed)
- **`./test_all_stages.sh`** - DO NOT USE THIS UNLESS ASKED TO TEST ALL (it's slow) - Runs all three fast test scripts in sequence
- **`./test_all_stages.sh --with-rust`** - DO NOT USE UNLESS EXPLICITLY ASKED - Also runs Rust compilation tests (EXTREMELY SLOW)

**Usage:**
```bash
# Test all files
./test_lexer_parser.sh   # Test lexing/parsing
./test_type_checker.sh   # Test type checking
./test_transpiler.sh     # Test transpilation
./test_rust_compilation.sh  # Test Rust compilation (slow)

# Test individual files
./test_rust_compilation.sh tests/test_arrays.nail  # Test single file
./test_rust_compilation.sh tests/*.nail  # Test multiple files

# Run all stages
./test_all_stages.sh     # Run all tests (no Rust compilation)
./test_all_stages.sh --with-rust  # Include Rust compilation (very slow)
```

**Important Notes:**
- These scripts automatically test both `tests/` and `examples/` directories
- They show progress and summarize results
- Much faster than testing Rust compilation (seconds vs minutes)
- Fix issues as you encounter them during testing

**Rust Unit Tests** (for core compiler testing):
- **`cargo test`** - Runs all Rust unit/integration tests
  - Note: May have warnings/errors in examples that don't affect core functionality
- **`cargo test test_name -- --nocapture`** - Run single test with output
- **`cargo test --lib parser`** - Run tests for a specific module

### Test File Organization

- **Language tests**: All Nail language test files (`.nail` files) must be placed in the `tests/` directory, not as temporary files
- **Naming conventions**: Use descriptive names like `test_single_letter_validation.nail` for language feature tests
- **Never use temporary files**: Do not create test files in `/tmp/` or other temporary locations for language testing - they belong in `tests/`
- **Examples vs Tests**: Use `examples/` for demonstration files, `tests/` for validation and regression testing

### Testing Individual Nail Files

```bash
# Check syntax and types only
cargo run --bin nailc tests/example.nail --check-only

# Full transpilation
cargo run --bin nailc tests/example.nail --transpile

# Skip type checking (for debugging)
cargo run --bin nailc tests/example.nail --skip-check
```

## Transpilation Guidelines

- **Return Keyword**: Transpilations should always use the return keyword, even though it's optional in Rust, we always use it because it's easier

## Nail Website

The Nail website is a demonstration of the language written in Nail itself:

- **Source**: `examples/nail_website.nail` - The website code written in Nail
- **Build Script**: `./run_website.sh` - Transpiles and runs the website
- **How it works**:
  1. The script transpiles `nail_website.nail` to Rust
  2. Creates a separate Cargo project in `nail_website_server/`
  3. Builds and runs the server on port 8080
  4. The website showcases Nail examples and features using HTMX for interactivity

**Important**: The `nail-website` binary in Cargo.toml is NOT the actual website - it's just a build helper. The real website runs from the transpiled `nail_website.nail` file.
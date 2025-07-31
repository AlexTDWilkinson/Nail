# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

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
- The only tool that should EVER be used for testing is run_comprehensive_tests.sh with no flags (except to specify the specific test to run). Flags can only be used for specific diagnosis, but generally no flags should ever be used.
- **CRITICAL: Never continue-on-error when running ./run_comprehensive_tests.sh**

## Development Commands

- **Run development mode**: `./start.sh` - Runs `cargo watch -x run` with debug flags enabled
- **Build**: `cargo build` or `cargo build --release`
- **Build compiler**: `cargo build --bin nailc` - Builds the Nail compiler binary

## Testing Commands

### Running Tests

**Primary Test Runner** (use this for all testing):
- **`./run_comprehensive_tests.sh`** - Complete validation of all Nail files and Rust tests
  - Runs lexer → parser → type checker → transpiler → Rust compilation on all files
  - Tests both `tests/` and `examples/` directories
  - Automatically cleans up generated `.rs` files
  - **IMPORTANT**: Takes 5-10 minutes to run full suite due to Rust compilation - use massive timeout (600000ms+)
  - **Options:**
    - `--tests-only` - Run only files in tests/ directory
    - `--examples-only` - Run only files in examples/ directory  
    - `--exclude-fails` - Skip fail_* test files
    - `-v, --verbose` - Show detailed error output
    - `--help` - Show full help
  - **Examples:**
    ```bash
    ./run_comprehensive_tests.sh                    # Test everything
    ./run_comprehensive_tests.sh --tests-only       # Only test files
    ./run_comprehensive_tests.sh tests/specific.nail # Test one file
    ./run_comprehensive_tests.sh -v                 # Verbose output
    ```

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
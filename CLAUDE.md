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

**IMPORTANT**: After making ANY changes to the Nail language implementation (lexer, parser, checker, transpiler, stdlib), you MUST:

1. While iterating, run the fast script for the stage you touched -
   `./test_lexer_parser.sh`, `./test_type_checker.sh`, or `./test_transpiler.sh`
2. Before committing, run `./test_all_stages.sh` - all three stages in sequence
3. Verify all previously passing tests still pass
4. If any tests fail that previously passed, investigate and fix the regression
5. Only proceed with additional changes after all tests pass

This is non-negotiable to maintain language stability and prevent regressions.

A clean run currently reports 570/570 lexer/parser, 569/569 type checker and
550/550 transpiler, with zero failures. Those counts are every `.nail` file in
the repository, from the one shared list in `test_nail_files.sh`, rather than
the two non-recursive globs the scripts used to carry. `cargo test --lib` reports 1322 passing
(1340 with `--features "game audio"`), `cargo test --bin nail` reports 1328
(the library's tests plus the editor's own), and `./test_e2e.sh` reports 376
programs passing. Treat any number below that as a
regression to investigate, not a new baseline.

The server tests in `parser::std_lib::net` and the watch tests in
`parser::std_lib::fs` bind fixed ports and fixed `/tmp` paths, so two test runs
at once will fail each other. Re-run those alone before believing a failure.

`./test_launcher.sh` reports 41 checks passing, and
`./test_error_messages.sh` reports 25 passed, 0 failed.

`./test_doc_examples.sh` compiles all 1166 registry examples and runs the 1033
that can run unattended. The rest are named, with the reason, in
`tests/doc_examples_needing_the_world.txt`, which is checked both ways: an
example listed there that starts passing is reported so the line can be
deleted. Shrinking that list is real work, since an example that writes the
file it then reads is one a reader can paste and run.

## Nail in the documentation

Every fenced block of Nail in `README.md` and `nail_language_spec.md` is
compiled by `cargo test --lib docs`, and what the fence calls itself decides
how hard:

- ` ```nail ` - a whole program: it lexes, parses and type checks
- ` ```nail-fragment ` - a piece of one: it lexes and parses, its context is
  the prose around it
- ` ```nail-refused ` - code the compiler must reject, shown to explain why

Any other fence (`js`, `ebnf`, `bash`) is not Nail and is not checked. Blocks
that import a file the reader is expected to have resolve it against
`tests/docs_imports/`.

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

## CRITICAL: No Semicolons or Em-Dashes in Prose

**NEVER use semicolons (;) or em-dashes (—) in any prose anywhere in this project.**
Write separate sentences, or use a colon, comma, or parentheses instead.

This applies to ALL human-readable text:

1. Website copy (`examples/nail_website.nail`)
2. Stdlib registry descriptions (`src/stdlib_registry/`) - these render on the website and in IDE F1 docs
3. Documentation (README.md, nail_language_spec.md, deploy/README.md)
4. Error messages, code comments, and commit messages

Semicolons that are code syntax (Rust, Nail, JS, CSS, macro separators) and
HTML entities like `&lt;` are unaffected - this rule is about prose punctuation only.

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
- **`./test_all_stages.sh`** - Runs all three fast test scripts in sequence. Too slow for tight iteration, but required before committing a change to the language implementation
- **`./test_all_stages.sh --with-rust`** - DO NOT USE UNLESS EXPLICITLY ASKED - Also runs Rust compilation tests (EXTREMELY SLOW)

**Other suites** (not part of the standard pre-commit run):
- **`./test_launcher.sh`** - Exercises every `nail` subcommand against a
  throwaway store. Nothing else runs them, so a broken subcommand otherwise
  reaches users untouched (`nail run` once shipped passing a flag nailc had
  never heard of). Run it after touching `src/bin/nail_launcher.rs` or
  `src/version_line.rs`
- **`./test_e2e.sh`** - End-to-end runs of compiled Nail programs
- **`./test_doc_examples.sh`** - Transpiles, compiles and runs every
  documentation example in the registry. The Rust tests prove the examples
  parse and type check, which compares them against the registry's own
  declaration of each function. Only rustc compares that declaration to the
  Rust behind it, and only running them proves the example works. Two examples
  were shipping uncompilable Rust and four more panicked when run before this
  existed. Slow (it builds a thousand binaries), so it is not part of the
  pre-commit run, but it is required after touching the registry or the
  transpiler. `./test_doc_examples.sh array_` checks one module
- **`./test_error_messages.sh`** - Checks runtime error message wording against goldens
- **`./check_all_features.sh`** - Verifies every feature-gated combination still compiles

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

# Stop after a single stage, to isolate where something breaks
cargo run --bin nailc tests/example.nail --lex-only
cargo run --bin nailc tests/example.nail --parse-only

# Write transpiler output to stdout instead of a file
cargo run --bin nailc tests/example.nail --transpile --stdout

# Generate the Cargo.toml a transpiled program needs, from its actual usage
cargo run --bin nailc tests/example.nail --cargo-toml --package-name=my_app
```

## Transpilation Guidelines

- **Return Keyword**: Transpilations should always use the return keyword, even though it's optional in Rust, we always use it because it's easier

## Nail Website

The Nail website is a demonstration of the language written in Nail itself:

- **Source**: `examples/nail_website.nail` - The website code written in Nail. This is the ONLY file to edit
- **Local run**: `./run_website.sh` - Transpiles and runs the website on port 8080
- **Deploy**: `./scripts/deploy.sh` - Transpiles, builds, and ships to the droplet
- **How it works**:
  1. The script transpiles `nail_website.nail` to Rust
  2. Writes it into the separate Cargo project in `nail_website_server/`
  3. Builds and runs the server on port 8080
  4. The website showcases Nail examples and features using HTMX for interactivity

**Important**: The `nail-website` binary in Cargo.toml is NOT the actual website - it's just a build helper. The real website runs from the transpiled `nail_website.nail` file.

**`nail_website_server/src/main.rs` is transpiler output and is gitignored.**
It used to be tracked, drifted out of sync with the compiler, and eventually
stopped compiling entirely. Never edit it and never commit it - it is
regenerated on every run and every deploy. Note that
`nail_website_server/Cargo.toml` is also generated (by `nailc --cargo-toml`)
but is still tracked.

The server reads several files at runtime relative to its working directory
(`examples/website_examples/`, `tests/`, `nail_language_spec.md`, `README.md`,
`examples/nail_website.nail`). `scripts/deploy.sh` ships those alongside the
binary - if you add a new `read_file` call to the website, add its path to
`DATA_PATHS` in that script or the deployed site will panic on startup.

## Deployment

The website runs on a DigitalOcean droplet shared with other services. See
`deploy/README.md` for the full runbook. In short:

- `deploy/provision-base.sh` - box-level setup (Caddy, ufw, fail2ban, swap), run once per droplet
- `deploy/add-app.sh` - registers one app: its own user, `/srv/<app>` at 0750, a sandboxed systemd unit, its own Caddy fragment
- `scripts/deploy.sh` - everyday deploy; builds locally and ships a finished binary. Nothing is compiled on the droplet

Apps bind `127.0.0.1` via `BIND_ADDR`, so the reverse proxy is the only public
entrance. Credentials live in `.env` (gitignored).
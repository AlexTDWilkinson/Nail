# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Development Commands

- **Run development mode**: `./start.sh` - Runs `cargo watch -x run` with debug flags enabled
- **Run tests**: `./test.sh` or `cargo test` - Runs all tests with Rust warnings suppressed
- **Build**: `cargo build` or `cargo build --release`

## High-Level Architecture

Nail is a programming language that transpiles to async, parallel Rust code. The architecture follows a traditional compiler pipeline:

### Core Components

1. **Terminal IDE** (src/main.rs):
   - Multi-threaded architecture with separate threads for UI, input, build, and syntax checking
   - Uses `ratatui` for terminal UI
   - Threads communicate via channels and shared state using `Arc<Mutex<>>` patterns

2. **Language Processing Pipeline**:
   - **Lexer** (src/lexer.rs): Tokenizes source code into `Token` structs
   - **Parser** (src/parser.rs): Builds AST from tokens using recursive descent parsing
   - **Type Checker** (src/checker.rs): Performs semantic analysis with scope management
   - **Transpiler** (src/transpilier.rs): Converts Nail AST to async Rust with Tokio runtime
   - **Colorizer** (src/colorizer.rs): Provides syntax highlighting for the IDE

3. **Thread Communication**:
   - `UIState` struct holds editor state, build results, and error messages
   - Channels used for sending commands between threads (key events, resize events, etc.)
   - Build thread runs transpilation pipeline and updates shared state

### Key Design Patterns

- **Error Handling**: Uses `Result<T, E>` pattern throughout with custom error types
- **Code Spans**: `CodeSpan` struct tracks source locations for error reporting
- **AST Nodes**: Comprehensive node types for all language constructs (expressions, statements, types)
- **Standard Library**: Located in `src/parser/std_lib/` with modules like `http.rs`

### Language Features to Remember

- **No traditional loops** - only functional iteration (map, filter, reduce)
- **No if statements** - uses match-like `if { condition => { } }` syntax
- **Immutability by default** - `c` for const, `v` for mutable variables
- **Automatic async/parallel** - all code transpiles to concurrent Rust
- **Error propagation** - errors bubble up with context automatically
- **Linux-only** - explicitly designed for Linux environments only

### Testing

Tests are located within source files using `#[cfg(test)]` modules:
- Parser tests in `src/parser.rs`
- Lexer tests in `src/lexer.rs`

Run a single test with: `cargo test test_name`
Run tests for a module: `cargo test --lib parser`

### Important Notes

- The IDE uses terminal escape codes and expects a Linux terminal environment
- All mutable values are placed in DashMaps for concurrency
- Rust blocks (`R{ }`) allow direct Rust code injection
- Nail escapes (`^[ ]^`) allow Nail expressions within Rust blocks
- The language spec is in `nail_language_spec.md` for detailed syntax reference
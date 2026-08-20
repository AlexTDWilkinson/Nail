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
   `./scripts/test_lexer_parser.sh`, `./scripts/test_type_checker.sh`, or `./scripts/test_transpiler.sh`
2. Before committing, run `./scripts/test_all_stages.sh` - all three stages in sequence
3. Verify all previously passing tests still pass
4. If any tests fail that previously passed, investigate and fix the regression
5. Only proceed with additional changes after all tests pass

This is non-negotiable to maintain language stability and prevent regressions.

A clean run currently reports 587/587 lexer/parser, 586/586 type checker and
567/567 transpiler, with zero failures. (These counts move whenever a test is
added: what matters is that every number is a full pass.) Those counts are every `.nail` file in
the repository, from the one shared list in `scripts/test_nail_files.sh`, rather than
the two non-recursive globs the scripts used to carry. `cargo test --lib` reports 1359 passing
(1374 with `--features fuzz`, which adds the fuzzer's own tests), `cargo test --bin nail`
reports 1382 (the library's tests plus the editor's own), and `./scripts/test_e2e.sh`
reports 398 programs passing. Treat any number below that as a
regression to investigate, not a new baseline.

The server tests in `parser::std_lib::net` and the watch tests in
`parser::std_lib::fs` bind fixed ports and fixed `/tmp` paths, so two test runs
at once will fail each other. Re-run those alone before believing a failure.

`./scripts/test_launcher.sh` reports 57 checks passing, and
`./scripts/test_error_messages.sh` reports 61 passed, 0 failed.

`./scripts/test_doc_examples.sh` compiles all 1189 registry examples and runs the ones
that can run unattended. The rest are named, with the reason, in
`tests/doc_examples_needing_the_world.txt`, which is checked both ways: an
example listed there that starts passing is reported so the line can be
deleted. Shrinking that list is real work, since an example that writes the
file it then reads is one a reader can paste and run.

## Nail in the documentation

Every fenced block of Nail in every markdown file in the repository (the
README, the spec, the blog example's posts, the test READMEs, the agent
definitions) is compiled by `cargo test --lib docs`, and what the fence calls
itself decides how hard:

- ` ```nail ` - a whole program: it lexes, parses and type checks
- ` ```nail-fragment ` - a piece of one: it lexes and parses, its context is
  the prose around it
- ` ```nail-refused ` - code the compiler must reject, shown to explain why

Any other fence (`js`, `ebnf`, `bash`) is not Nail and is not checked. Blocks
that import a file the reader is expected to have resolve it against
`tests/docs_imports/`. A new markdown file is swept automatically, there is no
list to add it to.

The same test run checks the documentation's references: every `.sh` path
named in any markdown file must exist where it says (bare names may live in
`scripts/`, `bundle/` or `deploy/`), and every `DATA_PATHS` entry in
`scripts/deploy.sh` must exist in the repository and be mentioned in
`deploy/README.md`. Both `scripts/deploy.sh` and `deploy/releases.sh` run
`cargo test --quiet --lib docs` before shipping anything, so stale
documentation fails a deploy on the machine that runs it.

## The fuzzer

`./scripts/fuzz.sh smoke` writes a minute's worth of Nail programs, runs the
compiler over every one of them, and reports anything that breaks an invariant
the compiler must never break. Run it after touching the lexer, the parser, the
checker, the transpiler or the formatter. It found five bugs in its first hour,
including two the whole test suite had never covered.

Three engines feed it, and they find different things:

- **generate** writes well typed programs from a type environment, so about
  95% of them reach the transpiler. This is what finds the codegen bugs.
  Everything it knows how to write comes from the language primer, and its
  library calls come from the registry, filtered by the registry's own
  `is_sandbox_safe`, so a function added to the library is fuzzed the same day.
- **mutate** bends the repository's own `.nail` files out of shape a few edits
  at a time. About a quarter of what it produces still compiles. This is what
  finds the crashes and the confused errors.
- **imports** (`./scripts/fuzz.sh imports --cases=N`) writes two-file cases and
  knows the answer each one is owed before it compiles: a helper that only
  computes must be accepted, and one that reaches the world, in any of the ways
  the registry says a function can, must be refused. Both pools come from the
  registry at runtime, so a function added to the library is covered the day it
  lands. It found four functions the policy allowed into a sandbox that should
  never have been there.

A fourth engine predicts. `./scripts/fuzz.sh predict <seed>` writes a program
whose every value the generator worked out as it wrote it, so it knows the
exact output the program owes. Those land in the queue with a `.stdout` beside
them, and `build` runs them and compares byte for byte. That is the tier that
can catch a wrong answer rather than a failure to build.

The invariants live in `src/fuzz/oracle.rs`, one enum with a sentence each:
no stage may panic, a program the checker accepts must transpile and must build
as Rust, an error must point at a place that exists, formatting must be stable
and must not change what a program means, the browser refusal must name exactly
the calls a browser does not have, and highlighting must never crash, never
change a character of the line, and agree with the lexer about what is a string
and what is a number. Being refused is not a finding: most fuzzed programs are
nonsense and refusing them is correct.

The colorizer is held to those last three because it is a second reader of the
language, written as a state machine per line so that half-typed code still
colors sensibly. Where it and the compiler read the same text they have to
agree, and the disagreements were real: numbers written against an operator
(`80&&81`, `count%7`, `5!= 6`) were painted as plain text, and a whole number
too large for an i was painted as a float. The comparison only runs on files
that parse, because on anything less the colorizer is allowed its own reading.

Everything is reproducible from one number. A finding names its seed, and
`./scripts/fuzz.sh case <seed>` prints the exact program again. Findings land
in `target/fuzz/findings` as a pair of files: the program, shrunk to the few
lines that still break, and what it broke.

The rustc tier is separate because it is a thousand times slower: a run keeps
the programs that got all the way through, and `./scripts/fuzz.sh build`
compiles them as bins of one shared cargo project. A program that type checks
and then fails to build is the worst failure the compiler has, because what
reaches the user is a wall of Rust they never wrote.

Hard crashes are why workers are processes. A stack overflow aborts the process
it happens in and no `catch_unwind` sees it, so the parent watches a progress
file per worker: when a worker dies, the seed it was on is the program that
killed it, and the parent rebuilds that program from the seed and shrinks it.

Two limits keep the compiler off the stack's edge, and both are policy rather
than accident: `MAX_NESTING_DEPTH` and `MAX_AST_DEPTH` in `src/parser.rs`, plus
`MAX_TYPE_DEPTH` in `src/lexer.rs`. Every entry into the compiler runs on
`common::with_compiler_stack`, so how much stack there is never depends on who
called: `cargo test` runs the compiler on a two megabyte test thread, and the
editor used to run it on one too.

`cargo test --lib --features fuzz` runs the fuzzer's own tests, which include a
hundred generated and mutated programs put through every invariant. That is the
version that runs unattended.

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

1. Website copy (`examples/website/main.nail`)
2. Stdlib registry descriptions (`src/stdlib_registry/`) - these render on the website and in IDE F1 docs
3. Documentation (README.md, nail_language_spec.md, deploy/README.md)
4. Error messages, code comments, and commit messages

Semicolons that are code syntax (Rust, Nail, JS, CSS, macro separators) and
HTML entities like `&lt;` are unaffected - this rule is about prose punctuation only.

## File Management Guidelines

- **DO NOT MAKE MULTIPLE VERSIONS OF .NAIL FILES, LIKE NAIL_WEBSITE.nail and NAIL_WEBSITE_V2.NAIL**

## Testing Guidelines

**ABSOLUTELY MOST IMPORTANT THING Testing Principle**:
- Use the fast test scripts (`scripts/test_lexer_parser.sh`, `scripts/test_type_checker.sh`, `scripts/test_transpiler.sh`) for development
- These scripts test specific compiler stages quickly without slow Rust compilation
- Only test Rust compilation manually when absolutely necessary

## Development Commands

- **Run development mode**: `./scripts/start.sh` - Runs `cargo watch -x run` with debug flags enabled
- **Build**: `cargo build` or `cargo build --release`
- **Build compiler**: `cargo build --bin nailc` - Builds the Nail compiler binary

## Testing Commands

### Running Tests

**Fast Test Scripts** (use these for rapid development):
- **`./scripts/test_lexer_parser.sh`** - Tests lexer and parser only (very fast)
- **`./scripts/test_type_checker.sh`** - Tests type checking for files that pass lexer/parser (fast)
- **`./scripts/test_transpiler.sh`** - Tests transpilation for files that pass type checking (fast)
- **`./scripts/test_rust_compilation.sh`** - Tests Rust compilation of transpiled files (VERY SLOW - only use when specifically needed)
- **`./scripts/test_all_stages.sh`** - Runs all three fast test scripts in sequence. Too slow for tight iteration, but required before committing a change to the language implementation
- **`./scripts/test_all_stages.sh --with-rust`** - DO NOT USE UNLESS EXPLICITLY ASKED - Also runs Rust compilation tests (EXTREMELY SLOW)

**Other suites** (not part of the standard pre-commit run):
- **`./scripts/test_launcher.sh`** - Exercises every `nail` subcommand against a
  throwaway store. Nothing else runs them, so a broken subcommand otherwise
  reaches users untouched (`nail run` once shipped passing a flag nailc had
  never heard of). Run it after touching `src/bin/nail_launcher.rs` or
  `src/version_line.rs`
- **`./scripts/test_e2e.sh`** - End-to-end runs of compiled Nail programs
- **`./scripts/test_doc_examples.sh`** - Transpiles, compiles and runs every
  documentation example in the registry. The Rust tests prove the examples
  parse and type check, which compares them against the registry's own
  declaration of each function. Only rustc compares that declaration to the
  Rust behind it, and only running them proves the example works. Two examples
  were shipping uncompilable Rust and four more panicked when run before this
  existed. Slow (it builds a thousand binaries), so it is not part of the
  pre-commit run, but it is required after touching the registry or the
  transpiler. `./scripts/test_doc_examples.sh array_` checks one module
- **`./scripts/test_error_messages.sh`** - Checks runtime error message wording against goldens
- **`./scripts/check_all_features.sh`** - Verifies every feature-gated combination still compiles
- **`./scripts/fuzz.sh smoke`** - A minute of generated and mutated programs
  through every compiler stage, then rustc over the ones that got through.
  Run it after touching the lexer, parser, checker, transpiler or formatter.
  See "The fuzzer" above

**Usage:**
```bash
# Test all files
./scripts/test_lexer_parser.sh   # Test lexing/parsing
./scripts/test_type_checker.sh   # Test type checking
./scripts/test_transpiler.sh     # Test transpilation
./scripts/test_rust_compilation.sh  # Test Rust compilation (slow)

# Test individual files
./scripts/test_rust_compilation.sh tests/test_arrays.nail  # Test single file
./scripts/test_rust_compilation.sh tests/*.nail  # Test multiple files

# Run all stages
./scripts/test_all_stages.sh     # Run all tests (no Rust compilation)
./scripts/test_all_stages.sh --with-rust  # Include Rust compilation (very slow)
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

The Nail website is a demonstration of the language written in Nail itself.
Everything that belongs to the site lives in one directory, `examples/website/`:

- `main.nail` - the entry point, and the file `scripts/run_website.sh` and
  `scripts/deploy.sh` transpile
- `startup_data.nail`, `styles.nail`, `page_sections.nail`,
  `server_state.nail`, `routes.nail` and `safe/` - modules that `main.nail`
  imports
- `snippets/` - the Nail programs the site displays and the playground loads
- `screenshots/` - the HTML captures of the IDE shown on the site
- `assets/` - binary data the pages embed

To run it locally: `./scripts/run_website.sh` transpiles `examples/website/main.nail`,
writes the generated Rust into the Cargo project in
`target/nail_website_server/` (entirely generated, nothing in it is tracked),
builds it, and serves on port 8080. `./scripts/deploy.sh` does the same build
and ships the binary to the droplet. The site uses HTMX for interactivity.

The server runs in `examples/website/`, the same run-in-its-own-directory
rule every Nail program follows, so `nail run examples/website/main.nail`
works. Its runtime reads are relative to that directory: files inside the
site by bare name (`snippets/...`, `main.nail`), repo files above it by
`../../` (`../../README.md`, `../../tests/...`). `scripts/run_website.sh`
starts the binary there, and on the droplet a systemd drop-in written by
`scripts/deploy.sh` sets `WorkingDirectory=/srv/nail/examples/website`.

`scripts/deploy.sh` ships the files the server reads alongside the binary,
keeping repo-relative layout - if you add a new `read_file` call to the
website, add its path to `DATA_PATHS` in that script or the deployed site
will panic on startup.

## Deployment

The website runs on a DigitalOcean droplet shared with other services. See
`deploy/README.md` for the full runbook. In short:

- `deploy/provision-base.sh` - box-level setup (Caddy, ufw, fail2ban, swap), run once per droplet
- `deploy/add-app.sh` - registers one app: its own user, `/srv/<app>` at 0750, a sandboxed systemd unit, its own Caddy fragment
- `scripts/deploy.sh` - everyday deploy; builds locally and ships a finished binary. Nothing is compiled on the droplet

Apps bind `127.0.0.1` via `BIND_ADDR`, so the reverse proxy is the only public
entrance. Credentials live in `.env` (gitignored).
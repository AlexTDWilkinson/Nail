# 🔨 Nail: The Programming Language for Grug Brain Devs

Welcome to Nail - the programming language so simple, it's genius. Inspired by the wisdom of [grugbrain.dev](https://www.grugbrain.dev).

🌐 **[Visit the Nail Website](https://nail-idtq.onrender.com/)** - Interactive examples, documentation, and philosophy (hosted on Render free tier - expect 30s cold start delay)

More detailed info in the [Nail Language Spec](nail_language_spec.md).

## 🚀 Why Nail? Because Complexity Bad. Very Bad.

- **Linux Only:** Nail no like cross-platform headache. Linux only.
- **Transpiles to Rust:** Write simple Nail, get fast Rust. Best of both worlds.
- **Async by Default:** All functions async. Concurrency just works.
- **Built-in IDE:** Terminal-based, vim-like editor. Press F7 to run.
- **No Package Manager:** Standard library built-in. No dependency hell.
- **No Imports:** Just use functions. `print()`, `math_sqrt()`, done.

## 🎭 Features That Work Today:

### Language Features
- **Everything Immutable:** Variables declared once. No surprises.
- **Pattern Matching:** Nail's `if` is actually pattern matching in disguise.
- **Functional Iteration:** `map`, `filter`, `reduce`, `scan`. No loops allowed.
- **Simple Lambdas:** Anonymous functions supported, but no nested lambdas allowed.
- **Result Types:** `s!e` for string-or-error. Handle with `danger()` or `safe()`.
- **Structs & Enums:** Simple data types. No methods, no complexity.
- **Parallel Blocks:** `p ... /p` runs code in parallel. Magic!
- **Type Inference:** Nail figures out types. Less typing for you.

### Standard Library (All Working!)

Nail has no package manager, on purpose. That means the standard library has to
ship the things people actually need, audited and pinned, instead of leaving
you to pull a stranger's crate to generate a UUID.

- **Strings & Arrays:** `string_trim()`, `string_replace()`, `array_length()`, `array_join()`, `array_take()`, etc.
- **Math & Numbers:** `math_sqrt()`, `math_atan2()`, `math_modulo()`, `bits_count_ones()`, `stats_median()`, `stats_percentile()`
- **Files & Paths:** `fs_read()`, `fs_write()`, `fs_append()`, `fs_walk()`, `path_join()`, `path_exists()`
- **Time & Dates:** `time_now()`, `time_format()`, `time_add_months()`, `time_weekday()` - real calendar arithmetic, all in UTC
- **HTTP:** `http_request()` for clients, `http_server()` for servers - async networking built-in
- **Data:** `json_serialize()`, `toml_deserialize()`, `csv_open()`, `db_sqlite_query()`
- **Security:** `crypto_hash_password()` (Argon2id), `crypto_hmac_sha256()`, `crypto_random_hex()`, `crypto_secure_equal()`
- **Command line:** `args_parse()` - describe your flags once and get the `--help` page from the same description
- **Terminal:** `term_paint()`, `term_table()`, and `tui_run()` for full-screen programs
- **Machine learning:** `ml_boost_fit()` (gradient boosting), `ml_tree_explain()`, `ml_kmeans()`, `ml_score()`
- **Drawing:** `draw_svg()` and friends - charts and diagrams with no window and no graphics card
- **Testing & logging:** `test_assert_equal_int()`, `log_info()`, `log_with_fields()`

Two of these are worth calling out because nothing else ships them in a
standard library:

**`tui_run()` - full-screen terminal programs, described rather than drawn.**
Every other TUI library in every other language makes you build widget objects
and mutate them. Nail has nothing mutable, so instead you write two ordinary
functions - `view(state)` says what the screen looks like, `update(state, event)`
says what the state becomes - and the runtime owns raw mode, input, redrawing,
resizing, and putting the terminal back even if your program panics. See
`examples/tui_counter.nail`.

**`ml_boost_*` - gradient boosted trees, hand-written, no dependencies.** The
method that actually wins on tabular data, with quantile binning, missing-value
routing learned per split, early stopping against a held-out set, and feature
importance. `ml_tree_explain()` prints the rules a decision tree learned,
because a model you can read beats a slightly better one you cannot.

### What Actually Works
- **Full Compiler Pipeline:** Lexer → Parser → Type Checker → Transpiler → Rust
- **Comprehensive Type System:** Strong typing with inference
- **Error Handling:** Detailed error messages with line/column info
- **IDE Features:** Syntax highlighting, auto-save, real-time error checking
- **Test Suite:** Extensive tests ensure everything works

## 🏗️ Current State: Nail Works!

Nail not baby anymore. Nail teenager with attitude:

- ✅ **Core Language:** All features implemented and tested
- ✅ **Parser:** Handles full language syntax
- ✅ **Type Checker:** Catches errors before runtime
- ✅ **Transpiler:** Generates clean, async Rust code
- ✅ **IDE:** Full terminal IDE with syntax highlighting
- ✅ **Standard Library:** 500 functions across 50 modules, listed in full at [the website's library section](https://nail-idtq.onrender.com/#stdlib) or by calling `stdlib_functions()`
- ✅ **Compiler Binary:** `nailc` for standalone compilation

## 📝 Example Nail Code

```nail
// Structs for data
struct User {
    name:s,
    age:i,
    score:f
}

// Create user - struct literals use = for their fields
user:User = User {
    name = `Grug`,
    age = 42,
    score = 99.5
};

// Pattern matching
message:s = if {
    user.age > 40 -> { `Senior Grug`; },
    user.age > 20 -> { `Adult Grug`; },
    else -> { `Baby Grug`; }
};

// Functional programming
numbers:a:i = [1, 2, 3, 4, 5];
doubled:a:i = map num in numbers {
    y num * 2;
};
sum:i = reduce acc num in doubled from 0 {
    y acc + num;
};

// Parallel execution
p
    print(`Computing...`);
    expensive_result:f = math_sqrt(16.0);
    time_sleep(1.0);
/p

// Error handling - a result must be handled where it occurs
content:s = danger(fs_read(`data.txt`));
print(content);
```

## 🤔 Who Nail For?

- Grug brain devs who think modern programming too complex
- Rust lovers who want simpler syntax
- Python fans who want compiled speed
- Go developers who want better error handling
- Anyone tired of `npm install` taking 5 minutes
- Teams who value maintainability over cleverness

## 🛠️ Getting Started

1. **Clone the repo:**
   ```bash
   git clone https://github.com/AlexTDWilkinson/Nail.git
   cd Nail
   ```

2. **Run the IDE:**
   ```bash
   ./start.sh
   ```
   Or use development mode:
   ```bash
   cargo watch -x run
   ```

3. **Compile Nail files:**
   ```bash
   cargo run --bin nailc examples/simple.nail --transpile
   ```

4. **Run tests:**
   ```bash
   ./test_all_stages.sh          # lexer/parser, type checker, transpiler
   ./test_e2e.sh                 # compile and run every example, compare output
   ./test_rust_compilation.sh    # compile the generated Rust (slow)
   ```

## 🎮 IDE Controls

- **F7**: Compile and run current file
- **F6**: Toggle theme (dark/light)
- **Ctrl+S**: Save file
- **Ctrl+C**: Exit
- **Standard vim movements**: Navigate like a pro

## 📚 Project Structure

```
nail/
├── src/
│   ├── main.rs          # IDE entry point
│   ├── lexer.rs         # Tokenization
│   ├── parser.rs        # AST generation
│   ├── checker.rs       # Type checking
│   ├── transpiler.rs    # Rust code generation
│   └── parser/std_lib/  # Standard library modules
├── examples/            # Example Nail programs
├── tests/              # Test suite
└── nail_language_spec.md # Full language specification
```

## 🤝 Contributing

Nail welcomes grug contributions! Whether you're fixing bugs, adding features, or improving docs, we appreciate simple, clear code.

**Note:** The website at https://nail-idtq.onrender.com/ automatically updates when changes are pushed to the main branch. Your contributions will be live within minutes!

See [CLAUDE.md](CLAUDE.md) for AI-assisted development guidelines.

## 🎉 Ready for Grug Code?

Nail is ready for real use! Join us in the fight against complexity. Together, we make programming simple again.

Remember: **Complexity bad. Nail good. You code now!** 🔨💪
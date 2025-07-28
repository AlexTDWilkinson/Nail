# 🔨 Nail: The Programming Language for Grug Brain Devs

Welcome to Nail - the programming language so simple, it's genius. Inspired by the wisdom of [grugbrain.dev](https://www.grugbrain.dev).

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
- **Functional Iteration:** `map`, `filter`, `reduce`. No loops allowed.
- **Simple Lambdas:** Anonymous functions supported, but no nested lambdas allowed.
- **Result Types:** `s!e` for string-or-error. Handle with `danger()` or `safe()`.
- **Structs & Enums:** Simple data types. No methods, no complexity.
- **Parallel Blocks:** `p ... /p` runs code in parallel. Magic!
- **Type Inference:** Nail figures out types. Less typing for you.

### Standard Library (All Working!)
- **String Operations:** `string_trim()`, `string_to_uppercase()`, `string_replace()`, etc.
- **Math Functions:** `math_sqrt()`, `math_abs()`, `math_pow()`, `math_random()`, etc.
- **Array Functions:** `array_len()`, `array_join()`, `array_take()`, `array_skip()`, etc.
- **I/O Operations:** `file_read()`, `file_write()`, `path_exists()`, etc.
- **Time Functions:** `time_now()`, `time_format()`, `time_sleep()`, etc.
- **HTTP Client:** `http_get()`, `http_post()` - async networking built-in!
- **Environment:** `env_args()`, `env_var()` for system interaction

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
- ✅ **Standard Library:** 50+ functions across 15 modules
- ✅ **Compiler Binary:** `nailc` for standalone compilation

## 📝 Example Nail Code

```nail
// Structs for data
struct User {
    name: s,
    age: i,
    score: f
}

// Create user
user: User = User { 
    name: `Grug`, 
    age: 42, 
    score: 99.5 
};

// Pattern matching
message: s = if {
    user.age > 40 => { `Senior Grug`; },
    user.age > 20 => { `Adult Grug`; },
    else => { `Baby Grug`; }
};

// Functional programming
numbers: a:i = [1, 2, 3, 4, 5];
doubled: a:i = map num in numbers {
    y num * 2;
};
sum: i = reduce acc num in doubled from 0 {
    y acc + num;
};

// Parallel execution
p
    print(`Computing...`);
    expensive_result: f = math_sqrt(16.0);
    time_sleep(1000);
/p

// Error handling
content: s = danger(file_read(`data.txt`));
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
   git clone https://github.com/yourusername/nail.git
   cd nail
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
   ./run_comprehensive_tests.sh
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

See [CLAUDE.md](CLAUDE.md) for AI-assisted development guidelines.

## 🎉 Ready for Grug Code?

Nail is ready for real use! Join us in the fight against complexity. Together, we make programming simple again.

Remember: **Complexity bad. Nail good. You code now!** 🔨💪
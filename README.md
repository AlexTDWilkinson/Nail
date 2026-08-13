# 🔨 Nail

A programming language for grug brain devs, inspired by [grugbrain.dev](https://grugbrain.dev). Complexity bad. Nail good.

## 🌐 [nail.alex-wilkinson.ca](https://nail.alex-wilkinson.ca/)

Everything is on the website: install, interactive examples, the full standard library, the playground. The site is itself a Nail program.

```bash
curl -fsSL https://nail.alex-wilkinson.ca/install | sh
```

Linux only, on purpose.

## Core features

- **Transpiles to Rust.** Write simple Nail, get fast native binaries.
- **Async by default.** Every function is async, and `p ... /p` blocks run in parallel.
- **Everything immutable.** No mutation, no exceptions, no nulls. Iteration is `map`, `filter`, `reduce`, and a `for` that yields values.
- **Errors cannot be ignored.** Result types like `s!e` must be handled where they occur, or it does not compile.
- **No package manager.** 1180 standard library functions built in: HTTP, SQLite, JSON, crypto, ML, TUI, drawing.
- **Built-in terminal IDE.** Syntax highlighting, live errors, profiling, themes. F7 runs your program.
- **Programs compile forever.** Every file pins its compiler version, and `nail` fetches and runs exactly that one.

Full language reference: [nail_language_spec.md](nail_language_spec.md).

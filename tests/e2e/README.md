# Nail End-to-End Test Suite

This directory is the **sanity harness** for the Nail language. Every test here
is a complete Nail program that is transpiled, compiled to a native executable,
executed, and whose stdout is compared **byte-for-byte** against a checked-in
expectation. If `./test_e2e.sh` is green, the entire pipeline — lexer, parser,
type checker, transpiler, generated Rust, runtime stdlib behavior — still
produces the exact same observable results.

**Run it after every change to the language implementation.** The fast stage
scripts (`test_lexer_parser.sh`, `test_type_checker.sh`, `test_transpiler.sh`)
only prove that code compiles; this suite proves programs still *behave* the
same. It exists specifically so AI-driven changes to the compiler cannot
silently alter program semantics.

```bash
./test_e2e.sh                                  # everything
./test_e2e.sh tests/e2e/collections            # one category
./test_e2e.sh tests/e2e/basics/hello_world.nail  # one test
```

The harness transpiles every test, builds them all as bins of ONE shared Cargo
project (dependencies compile once; the build is incremental across runs), then
runs each executable in a fresh empty working directory with a timeout.

## Test contract

- `tests/e2e/<category>/<name>.nail` — a small, focused, self-contained program
- `tests/e2e/<category>/<name>.stdout` — its exact expected stdout (bytes)
- `tests/e2e/<category>/<name>.exitcode` — optional expected exit code (default 0)
- Tests must be **deterministic**: same output on every run, on any machine
- Each test runs with cwd set to a fresh empty directory: filesystem tests must
  first create anything they read, and may write freely
- Keep tests small: one behavior, a handful of `print` calls. Hundreds of tiny
  programs beat dozens of huge ones — a failure should point at one feature

## Determinism rules (non-negotiable)

- Never print raw values of `time_now`, `time_now_millis`, `math_random`,
  `crypto_uuid_v4`, `array_shuffle`. Assert derived facts instead, e.g.
  `print(rand_val >= 0.0);`
- Hashmap iteration order is not deterministic: never print `hashmap_keys` /
  `hashmap_values` results directly — sort them first with `array_sort`, or
  assert via `hashmap_get` / `hashmap_len` / `hashmap_contains_key`
- Don't print from inside `c`/`p`/`spawn` concurrency constructs when ordering
  could race; compute inside, print after the block
- Avoid float operations with inexact decimal results (`0.1 + 0.2`); stick to
  halves and quarters, which print exactly

## Output formatting facts (verified against the runtime)

`print` uses Rust `Debug` formatting and joins multiple arguments with a single
space, then appends one newline per call:

| Nail                          | stdout            |
|-------------------------------|-------------------|
| `print(`hi`);`                | `hi`              |
| `print(`n is`, 42);`          | `n is 42`         |
| `print(3.0);`                 | `3.0`             |
| `print(7.0 / 2.0);`           | `3.5`             |
| `print(10 / 3);`              | `3` (truncating)  |
| `print(string_from(3.0));`    | `3.0`             |
| `print([1, 2, 3]);`           | `[1, 2, 3]`       |
| `print([`a`, `b`]);`          | `["a", "b"]` (strings in arrays keep quotes) |
| `print(my_point);`            | `Pt { x_val: 1, y_val: 2 }` (struct Debug) |
| `print(Color::Red);`          | `Red`             |
| `print(empty_int_array);`     | `[]`              |

- `\n` inside a backtick string becomes a real newline when printed
- `\t` does **not** — it prints literally as `\t`. Avoid tabs
- `${...}` string interpolation is NOT implemented: it prints literally.
  Never use it; pass multiple arguments to `print` instead

## Nail syntax cheatsheet (for test authors)

Statements end with `;`. Comments are `//`. Strings use backticks. Variables
are immutable, declared `name:type = expr;`. No single-letter identifiers.
Types: `i` int, `f` float, `s` string, `b` bool, `a:T` array, `h<K,V>` hashmap,
`StructName`, `EnumName`, `T!e` result.

```nail
// functions ('f', return with 'r'; result types with !e; errors via e(...))
f divide(dividend:i, divisor:i):i!e {
    if {
        divisor == 0 => { r e(`Division by zero`); },
        else => { r dividend / divisor; }
    }
}
val_ok:i = danger(divide(10, 2));          // unwrap or crash
f on_err(error_msg:e):i { r -1; }
val_safe:i = safe(divide(1, 0), on_err);   // unwrap or fallback

// if is an expression; branch bodies use 'r' for their value
grade:s = if { score >= 90 => { r `A`; }, score >= 80 => { r `B`; }, else => { r `C`; } };

// collection expressions; 'y' yields; optional index param after iterator
nums:a:i = [1, 2, 3, 4, 5];
doubled:a:i  = map num in nums { y num * 2; };
evens:a:i    = filter num in nums { y num % 2 == 0; };
total:i      = reduce acc num in nums from 0 { y acc + num; };
first_big:i  = danger(find num in nums { y num > 3; });
all_pos:b    = all num in nums { y num > 0; };
any_big:b    = any num in nums { y num > 4; };
with_idx:a:i = map num idx in nums { y num + idx; };

// loops
for num in nums { print(num); }
for num in nums when num > 2 { print(num); }
loop idx { if { idx >= 3 => { break; }, else => { print(idx); } } }

// structs and enums
struct Point { x_coord:i, y_coord:i }
pt_one:Point = Point { x_coord = 1, y_coord = 2 };
enum Direction { North, South }
heading:Direction = Direction::North;

// hashmaps
ages:h<s,i> = hashmap_new();
hashmap_set(ages, `alice`, 30);
alice_age:i = danger(hashmap_get(ages, `alice`));

// concurrency (compute inside, print after)
c
    left_val:i = 1;
    right_val:i = 2;
/c
print(left_val + right_val);
```

Common pitfalls:

- `array_get`, `array_first`, `array_last`, `array_slice`, `hashmap_get`,
  `hashmap_remove`, `int_from`, `float_from`, `string_slice`, `math_factorial`,
  `math_divide`, and all `regex_*` return results — wrap in `danger(...)` or
  `safe(...)`
- Function names come from the stdlib registry (`src/stdlib_registry/`), which
  is the source of truth for names, parameter types, and return types
- Don't name variables after keywords (`map`, `filter`, `from`, `when`, `max`,
  `in`, `loop`, `step`, `insert`, ...)

## When a test fails

1. **The change broke the language** — most likely. Fix the compiler, not the
   test. (CLAUDE.md: never work around compiler bugs.)
2. **The change intentionally altered behavior** — update the `.stdout` file in
   the same commit, and say why in the commit message.
3. Never delete or weaken a test to make the suite pass.

## Adding tests

Every new language feature or stdlib function should land with e2e coverage.
Write the `.nail` program, reason out the exact expected stdout by hand, save
it as `.stdout`, then run `./test_e2e.sh tests/e2e/<category>/<name>.nail` to
confirm reality matches your reasoning. If it doesn't, understand why before
touching either file — a surprise here is usually a compiler bug worth keeping.


# Nail Programming Language Overview

- Nail takes inspiration from this blog post: https://grugbrain.dev/
- Nail is spiritually similar to HTMX, except for the obvious difference that it is an entire programming language and paradigm.

## Introduction

Nail is a programming language designed with a focus on simplicity, safety, and productivity. Its primary goal is to eliminate common sources of bugs and reduce cognitive load on developers by enforcing strict rules, a strict enviroment and by providing a consistent, straightforward syntax.

Nail can ONLY be written and transpiled in the Nail IDE, which only runs on Linux.

Nail programs are transpiled to async, parallellized (when specified) Rust and then compiled to native executables.

Nail programs often exhibit superior performance compared to typical Rust implementations, as Nail easily incorporates asynchronous, concurrent, and parallel paradigms — optimizations that many developers might not take the time to implement in typical Rust programs. However, it's important to note that a meticulously optimized Rust program can likely exceed Nail's performance, given that Nail is ultimately transpiled to Rust.

## Core Design Principles

Nail adheres to the following core principles:

- Simplicity: The language includes only essential features, avoiding complexity.
- Safety: Strong typing and strict rules prevent common programming errors.
- Productivity: Consistent syntax and built-in best practices enhance developer efficiency.
- Explicitness: The language favors explicit declarations over implicit behavior.

## Language Restrictions

To achieve its goals, Nail imposes the following restrictions:

- Limited data types: integer, float, string, boolean, array, struct, and enum.
- The simple parallell block keyword transforms into paralellized Rust.
- No package manager or external dependencies (The standard library is updated with every new version of Nail)
- No uninitialized constants (constants must be defined with a value)
- No null references.
- No mutability - all variables are immutable.
- No classes, inheritance, or traditional OOP constructs.
- No manual memory allocation or management.
- Immutable loop constructs (for, while) that return values.
- No traditional if statements (replaced by a psuedo match/switch expressions).
- No function or operator overloading.
- No implicit returns.
- No default values.
- No compiler warnings (only errors).
- No direct array indexing (only safe functional operations).
- No optional syntax (consistent, deterministic structure).
- No tuples (named structs only).
- No method attachment to structs or enums.
- No generics.
- No macros or metaprogramming.
- No single letter variable names (must be descriptive)
- No lambda functions or closures
- Explicit collection operation keywords (map, filter, reduce, scan, each, find, all, any) instead of generic functional methods
- Collection operations use 'y' (yield) to produce values, while 'r' (return) exits functions

## Lexical Structure

### 4.1 Keywords

Reserved keywords in Nail:

```
Meh, there's a bunch, see the EBNF file in this repo for specifics.
```

### 4.2 Identifiers

Identifiers follow snake_case convention:

```js
my_constant
calculate_total
```

### 4.3 Comments

Single-line comments only, preceded by `//`:

```js
// This is a comment
some_number:i = 5; // This is an inline comment
```

### 4.4 Literals

- Integer literals: `42`, `-7`
- Floating-point literals: `3.14`, `-0.001`
- String literals: `hello`, `nail is awesome`
- Boolean literals: `true`, `false`

#### 4.4.1 Tagged String Literals

A string literal may carry a language tag written flush against its opening
backtick:

```nail
page:s = html`<section class="hero"><h1>Nail</h1></section>`;
rules:s = css`.hero { color: #86efac; }`;
query:s = sql`SELECT name FROM users WHERE active = 1;`;
```

The tag has no meaning to the compiler. A tagged string is an ordinary `s` in
every respect - it has the same type, the same escape rules, and transpiles to
exactly the same Rust string as an untagged one. Its only purpose is to tell an
editor or a highlighter which language the contents are written in, so a page
built out of long HTML strings reads as HTML rather than as one flat colour.

The compiler keeps no list of languages: any identifier-shaped tag lexes, and
each highlighter recognizes the tags it knows and ignores the rest. A tag must
touch the backtick - `html \`x\`` with a space between them is an identifier
followed by a plain string, not a tagged one.

The highlighters that ship with Nail - the editor's colorizer and
`code_highlight_html` - tokenize these tags:

| Language | Tags |
| --- | --- |
| Markup | `html`, `htm`, `xhtml`, `svg`, `xml`, `rss`, `atom` |
| Stylesheets | `css`, `scss`, `sass`, `less` |
| JavaScript family | `js`, `javascript`, `mjs`, `cjs`, `jsx`, `ts`, `typescript`, `tsx`, `json`, `jsonc` |
| SQL | `sql`, `postgres`, `postgresql`, `mysql`, `sqlite` |
| Shell | `sh`, `bash`, `zsh`, `shell`, `dockerfile` |
| Other languages | `py`, `rb`, `rs`, `go`, `java`, `cs`, `c`, `cpp`, `php`, `swift`, `kt`, `lua`, `graphql` |
| Configuration | `yaml`, `yml`, `toml`, `ini`, `cfg`, `conf`, `properties` |
| Markdown | `md`, `markdown` |

A tag outside this list is still legal, and its string keeps one plain string
colour rather than being run through the wrong tokenizer.

`code_highlight_html` renders a tagged string as
`<span class="tok-str tok-str-html">`, so a page can style tagged strings apart
from plain ones without losing the base string styling. Inside it, each piece of
the embedded language gets a `tok-md-*` class - `tok-md-el` for an element name
or a CSS selector, `tok-md-kw` for a keyword, `tok-md-val` for a string, and so
on - so one set of rules styles every language.

### 4.5 Operators

#### 4.5.1 Arithmetic Operators

Arithmetic is for numbers only; text is joined with `string_concat`.

- `+` Addition
- `-` Subtraction  
- `*` Multiplication
- `/` Division
- `%` Modulo

#### 4.5.2 Comparison Operators
- `==` Equal
- `!=` Not equal
- `<` Less than
- `<=` Less than or equal
- `>` Greater than
- `>=` Greater than or equal

#### 4.5.3 Logical Operators
- `&&` Logical AND
- `||` Logical OR
- `!` Logical NOT

#### 4.5.4 Range Functions
Nail provides range functions for creating sequences in for loops:

```js
// Range function creates arrays for iteration
numbers:a:i = array_range(1, 5);  // Creates [1, 2, 3, 4] (end not included)

// Use in for loops
for idx in array_range(0, 5) {
    print(string_from(idx));  // Prints 0, 1, 2, 3, 4
}

// Common patterns
for idx in array_range(0, array_length(my_array)) {
    item:T = danger(array_get(my_array, idx));
    print(item);
}
```

## Data Types and constants

### 5.1 Type System

Nail uses a prefix-based type system:

- `i`: Integer
- `f`: Float  
- `s`: String
- `b`: Boolean
- `a`: Array
- `e`: Error
- `v`: Void (no return value)
- `h`: HashMap
- `struct`: Struct
- `enum`: Enum

### 5.2 Const Declaration

Constants must include type and initialization:

```js
// Everything in nail is const.
age:i = 30;
name:s = `Grug`;
is_developer:b = true;
```

### 5.3 Type Checking and Conversion

Strict type checking is enforced:

```js
count:i = 5;  // Valid
count:i = 6.0;  // Error: Can't assign float to integer
count:f = 6.0;  // Valid
count:f!e = to_float(5);  // Invalid, all result type errors cannot be assigned to a variable. They must be handled explicitly.
count:f = danger(to_float(5));  // Valid, removes the error type.
count:f = expect(to_float(5));  // Valid, removes the error type (same as danger but different semantic meaning).
// Handler function must be defined separately
f handle_float_error(e:s):f { r 0.0; }
count:f = safe(to_float(5), handle_float_error);  // Valid, handles error safely.
```

### 5.4 Composite Types

#### 5.4.1 Arrays

Homogeneous, non-nested collections:

```js
names:a:s = [`Alice`, `Bob`, `Charlie`];
```

#### 5.4.2 Structs

Custom data types with named fields:

```js
struct Point {
    x_pos:i,
    y_pos:i
}
```

#### 5.4.3 HashMaps

Key-value collections with type-safe keys and values. Both keys and values must be concrete types (cannot be void or error types):

```js
// Create a new hashmap with string keys and integer values
user_scores:h<s,i> = hashmap_new();

// Hashmaps with different valid type combinations
config_map:h<s,s> = hashmap_new();      // String keys, string values
id_to_struct:h<i,Point> = hashmap_new(); // Integer keys, struct values
name_to_active:h<s,b> = hashmap_new();   // String keys, boolean values

// Hashmap operations
hashmap_set(user_scores, `alice`, 100);
hashmap_set(user_scores, `bob`, 85);

score:i = danger(hashmap_get(user_scores, `alice`));
has_charlie:b = hashmap_contains_key(user_scores, `charlie`);
map_size:i = hashmap_len(user_scores);

// Safe access with error handling
f handle_missing_key(err:e):i { r 0; }
alice_score:i = safe(hashmap_get(user_scores, `alice`), handle_missing_key);

// Example with struct values
struct Point { x_pos:i, y_pos:i }
origin:Point = Point { x_pos = 0, y_pos = 0 };
hashmap_set(id_to_struct, 1, origin);
```

#### 5.4.3 Enums

Fixed set of possible values (no associated data):

```js
enum TrafficLight {
    Red,
    Yellow,
    Green
}

current_light:TrafficLight = TrafficLight::Red;
```

## Control Structures

### 6.1 If Statements (Match-like Syntax)

Nail uses a unique match-like syntax for if statements. Traditional if-else syntax is NOT supported.

```js
// Basic if statement syntax
status:i = get_http_status_code(response);

if {
    status == 200 -> { print(`OK`); },
    status == 404 -> { print(`Not Found`); },
    else -> { print(`Unknown Status`); }
}

// If as an expression (returns a value)
result:s = if {
    status == 200 -> { r `Success`; },
    else -> { r `Error`; }
};
```

**Important**: All branches use `->` followed by blocks. When used as an expression, use `r` (return) to produce the value.

### 6.2 Collection Operations

Nail provides explicit collection operation keywords that are more readable and maintainable than generic loops:

#### Map Operation

Map transforms each element in a collection into a new element:

```js
numbers:a:i = [1, 2, 3, 4, 5];

// Basic map - transform each element
doubled:a:i = map num in numbers {
    y num * 2;
};

// Map with index access (no comma between iterators)
indexed_values:a:s = map num idx in numbers {
    y array_join([`Index `, danger(string_from(idx)), `: `, danger(string_from(num))], ``);
};

// Note: To map over characters in a string, first convert to array
// let chars:a:s = string_to_chars(`hello`);
// uppercase_chars:a:s = map char in chars { ... };
```

#### Filter Operation

Filter selects elements from a collection based on a condition:

```js
// Filter even numbers
evens:a:i = filter num in numbers {
    y num % 2 == 0;
};

// Filter with index (no comma between iterators)
first_three:a:i = filter num idx in numbers {
    y idx < 3;
};
```

#### Reduce Operation

Reduce accumulates values from a collection into a single result:

```js
// Sum all numbers
sum:i = reduce acc num in numbers from 0 {
    y acc + num;
};

// Find maximum (with index access)
max_val:i = reduce acc num idx in numbers from danger(array_get(numbers, 0)) {
    y if { num > acc -> { num }, else -> { acc } };
};

// Build string ('+' adds numbers only, so text is joined with string_concat)
concatenated:s = reduce acc word in [`hello`, ` `, `world`] from `` {
    y string_concat([acc, word]);
};
```

#### Scan Operation

Scan is a reduce that keeps its work: the accumulator's value after every
element, so the result is an array as long as the one it scanned.

```js
// Running total: [1, 3, 6, 10, 15]
running:a:i = scan acc num in numbers from 0 {
    y acc + num;
};

// The last value of a scan is what the same reduce would have returned
total:i = reduce acc num in numbers from 0 {
    y acc + num;
};
```

Each value a scan produces depends on every element before it, so a scan runs
in order, one element at a time. Reach for it when the intermediate values are
the point - a running balance, a cumulative chart line, the offset each chunk
starts at - and for `map` when each element stands alone.

#### Each Operation

Each performs side effects without collecting values:

```js
// Print each element (statement form, no assignment)
each num in numbers {
    print(array_join([`Number: `, danger(string_from(num))], ``));
}

// With index (no comma between iterators)
each num idx in numbers {
    print(array_join([`[`, danger(string_from(idx)), `]: `, danger(string_from(num))], ``));
}

// Each can also be assigned to a variable (expression form)
each_result:v = each num in numbers {
    print(array_join([`Number: `, danger(string_from(num))], ``));
};
```

#### Find Operation

Find returns the first element matching a condition:

```js
// Find first even number
first_even:i = danger(find num in numbers {
    y num % 2 == 0;
});

// Find with index (no comma between iterators)
third_element:i = danger(find num idx in numbers {
    y idx == 2;
});
```

#### All/Any Operations

Check if all or any elements match a condition:

```js
// Check if all positive
all_positive:b = all num in numbers {
    y num > 0;
};

// Check if any negative (with index access)
has_negative:b = any num idx in numbers {
    y num < 0;
};
```

### 6.3 Array Function Operations

Standard library provides array functions for common operations:

```js
numbers:a:i = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10];

// Take and skip operations
first_three:a:i = array_take(numbers, 3);  // [1, 2, 3]
skip_three:a:i = array_skip(numbers, 3);   // [4, 5, 6, 7, 8, 9, 10]

// Take/skip while operations with predicates
f less_than_five(num:i):b { r num < 5; }
small_nums:a:i = array_take_while(numbers, less_than_five);  // [1, 2, 3, 4]

// Array utilities
unique_nums:a:i = array_unique([1, 2, 2, 3, 3, 3]);  // [1, 2, 3]
nested:a:a:i = [[1, 2], [3, 4]];
flat_array:a:i = array_flatten(nested);    // [1, 2, 3, 4]

// Finding elements
index:i = danger(array_find(numbers, 5));  // Returns 4 (0-based index)

// Functional operations as library functions
f double(num:i):i { r num * 2; }
doubled:a:i = array_map(numbers, double);

f is_even(num:i):b { r num % 2 == 0; }
evens:a:i = array_filter(numbers, is_even);
```

### 6.4 For Loops

For loops iterate over arrays or function-generated ranges:

```js
// Range iteration - iterator can be any valid name
for index in array_range(0, 5) {
    print(string_from(index)); // Prints 0, 1, 2, 3, 4
}

// Iterator names are flexible
for counter in array_range(1, 4) {
    print(string_from(counter * counter)); // Prints 1, 4, 9
}

// Iterate over array elements directly
numbers:a:i = [10, 20, 30];
for value in numbers {
    print(string_from(value));
}

// Common pattern: iterate by index with descriptive names
for position in array_range(0, array_length(numbers)) {
    current_num:i = danger(array_get(numbers, position));
    print(`Index `, position, `: `, current_num);
}
```

#### While Loops

While loops with safety features to prevent infinite loops:

```js
// While loop with max iterations (required for safety)
factorial:i = while n > 1 from (acc = 1, n = 5) max 10 {
    r (acc * n, n - 1);
};

// The 'from' clause provides initial state
// The 'max' clause prevents infinite loops
// Returns the final accumulator value
```

#### Loop (Infinite Loops)

Loop construct for explicit infinite loops with `break` and `continue` support:

```js
// Basic infinite loop with break
loop {
    print(`Looping...`);
    break; // Must break to avoid infinite loop (immutable variables)
}

// Indexed loop - provides automatic counter (still infinite until break)
loop index {
    print(string_from(index)); // index starts at 0, auto-increments each iteration
    if {
        index >= 10 -> { break; },     // Exits the loop
        index == 5 -> { continue; },   // Skips to next iteration (index becomes 6)
        else -> { /* keep looping */ }
    }
}

// Key points about loop index:
// - Still infinite by default (no built-in termination)
// - index automatically increments each iteration (0, 1, 2, 3...)
// - break and continue work as expected
// - Provides counter without needing mutable variables
```

#### Spawn Blocks (Background Tasks)

Spawn blocks run asynchronously in the background:

```js
// Spawn a background task
spawn {
    print(`Background task started`);
    time_sleep(1.0);
    print(`Background task completed`);
}

// Main thread continues immediately
print(`Main thread continues`);

// Spawn with loop for continuous background processing
spawn {
    loop {
        // Perform periodic task
        health_check();
        time_sleep(60.0); // Sleep 60 seconds
    }
}
```

### 6.4 Collection Operation Transpilation

All collection operations transpile to simple for loops with enumerate() in Rust:

```js
// Nail
doubled:a:i = map num in numbers {
    y num * 2;
};

// Transpiles to Rust
let doubled = {
    let mut __result = Vec::new();
    for (idx, num) in numbers.iter().enumerate() {
        __result.push(num * 2);
    }
    __result
};

// Nail reduce operation
sum:i = reduce acc num in numbers from 0 {
    y acc + num;
};

// Transpiles to Rust (reduce operation)
let sum = {
    let mut __accumulator = 0;
    for (_idx, num) in numbers.iter().enumerate() {
        __accumulator = __accumulator + num;
    }
    __accumulator
};

// Filter operation (block with yield statement)
evens:a:i = filter num in numbers {
    y num % 2 == 0;
};

// Transpiles to Rust (filter operation)
let evens = {
    let mut __result = Vec::new();
    for (_idx, num) in numbers.iter().enumerate() {
        let condition_result = num % 2 == 0;
        if condition_result {
            __result.push(num.clone());
        }
    }
    __result
};
```

#### EBNF

```ebnf
// Collection operation expressions
map_expression :=
    "map" identifier [identifier] "in" expression block

filter_expression :=
    "filter" identifier [identifier] "in" expression block

reduce_expression :=
    "reduce" identifier identifier [identifier] "in" expression "from" expression block

scan_expression :=
    "scan" identifier identifier [identifier] "in" expression "from" expression block

each_statement :=
    "each" identifier [identifier] "in" expression block

find_expression :=
    "find" identifier [identifier] "in" expression block

all_expression :=
    "all" identifier [identifier] "in" expression block

any_expression :=
    "any" identifier [identifier] "in" expression block

// Note: Collection operations with optional index parameter:
// - First identifier is the element iterator
// - Optional second identifier is the index iterator (no comma separator)
// - ALL collection operations use yield (y) statements, not return (r)
// - This maintains Nail's principle of explicit yields in iterations

for_loop :=
    "for" identifier "in" expression block

while_loop :=
    "while" expression ["from" expression] ["max" expression] block

loop :=
    "loop" [identifier] block

spawn_block :=
    "spawn" block

parallel_block :=
    "p" statement* "/p"

concurrent_block :=
    "c" statement* "/c"

block :=
    "{" statement* "}"

return_statement :=
    "r" expression ";"

yield_statement :=
    "y" expression ";"

break_statement :=
    "break" ";"

continue_statement :=
    "continue" ";"

statement :=
    const_decl | struct_decl | enum_decl | function_decl |
    if_expression | for_loop | while_loop | loop |
    spawn_block | parallel_block | concurrent_block |
    return_statement | yield_statement | break_statement | continue_statement |
    expression_statement

expression_statement :=
    expression ";"
```

## Return vs Yield Statements

Nail uses two different keywords for different contexts - this is a critical distinction:

### Return Statements (`r`)
- **Purpose**: Exit a function and return a value to the caller
- **Scope**: Function body only
- **Behavior**: Immediately exits the entire function
- **Required**: All non-void functions must have explicit return statements

```js
f add(num_a:i, num_b:i):i {
    r num_a + num_b;  // Exits function, returns result to caller
}

f process_data(data:s):s!e {
    if {
        string_length(data) == 0 -> { r e(`Empty data`); },
        else -> { r data; }
    }
}
```

### Yield Statements (`y`)
- **Purpose**: Produce a value for the current iteration in collection operations
- **Scope**: Collection operation blocks only (map, filter, reduce, etc.)
- **Behavior**: Provides value for current iteration, continues to next iteration
- **Required**: All collection operation blocks must yield a value

```js
// Map: yield transforms each element
doubled:a:i = map num in numbers {
    y num * 2;  // Yields doubled value for THIS iteration, continues to next
};

// Filter: yield determines if element is included
evens:a:i = filter num in numbers {
    y num % 2 == 0;  // Yields true/false for THIS iteration
};

// Reduce: yield provides the new accumulator value
sum:i = reduce acc num in numbers from 0 {
    y acc + num;  // Yields new accumulator for THIS iteration
};
```

### Critical Rules:
1. **Never mix contexts**: Using `r` in collection operations is a compile error
2. **Never mix contexts**: Using `y` in function bodies is a compile error
3. **Collection operations are NOT functions**: They're language constructs that use yield
4. **Functions always use return**: Even when called inside collection operations

```js
// CORRECT: Function uses r, collection operation uses y
f double_value(num:i):i {
    r num * 2;  // Function returns value
}

doubled:a:i = map num in numbers {
    y double_value(num);  // Collection yields result of function call
};

// WRONG: This would be a compile error
bad_example:a:i = map num in numbers {
    r num * 2;  // ERROR: Cannot use 'r' in collection operation
};
```

## Functions

Functions must use `r` for return statements. Functions that can fail must return a result type (using the `!e` syntax).

```js
f calculate_monthly_payment(principal:i, annual_rate:i, years:i):f!e {
    if {
        annual_rate == 0 -> { 
            r e(`Annual rate cannot be zero`); 
        },
        years <= 0 -> { 
            r e(`Loan term must be positive`); 
        },
        else -> {
            monthly_rate:f = expect(float_from(annual_rate)) / 12.0 / 100.0;
            payments:i = years * 12;
            
            // Division by zero check
            denominator:f = 1.0 - pow(1.0 + monthly_rate, -payments);
            if {
                denominator == 0.0 -> { 
                    r e(`Cannot calculate payment: invalid parameters`);
                },
                else -> {
                    payment:f = expect(float_from(principal)) * monthly_rate / denominator;
                    r payment;
                }
            }
        }
    }
}
```

## Error Handling

Errors must be explicitly handled:

```js
user_input:s!e = lib_io_readline();
user_input:s = danger(lib_io_readline());

// OR safely handle the error
f handle_input_error(e:e):s { r `default value`; }
user_input:s!e = lib_io_readline();
user_input:s = safe(lib_io_readline(), handle_input_error);

```

## File Inclusion

Nail supports compile-time file inclusion through the `insert()` keyword. This allows you to include the contents of one Nail file directly into another, as if the code was typed in place.

### Syntax

```nail
insert(`filename.nail`)
```

### Behavior

- The `insert()` statement must appear at the beginning of a line (no indentation)
- The file path is resolved relative to the current file's directory
- The entire contents of the specified file are inserted at the location of the `insert()` statement
- This happens at compile-time during lexical analysis
- Circular includes are detected and prevented

### Example

```nail
// math_helpers.nail
f add(a:i, b:i):i {
    r a + b;
}

f multiply(a:i, b:i):i {
    r a * b;
}
```

```nail
// main.nail
insert(`math_helpers.nail`)

result:i = add(5, 3);
product:i = multiply(result, 2);
print(product); // Outputs: 16
```

### Use Cases

- Sharing common functions across multiple files
- Organizing large programs into separate files
- Building libraries of reusable code

### Restrictions

- Cannot include files from outside the project directory
- File paths must be string literals (not variables)
- Included files must contain valid Nail code
- No conditional includes (includes always happen)

### Sealed Inclusion: insert_safe

`insert_safe()` includes a file exactly like `insert()`, with the same path
resolution and the same circular include detection, but everything from that
file is sealed: it may only compute, never touch the world. The compiler
proves this at compile time. There is no runtime cost and the generated Rust
is byte for byte identical to what plain `insert()` produces.

This is Nail's supply chain answer. Downloaded code is pasted in with
`insert_safe`, and the compiler guarantees it cannot phone home, read your
disk or environment, spy on global state, or seize resources like stdout.

```nail
insert_safe(`downloaded_library.nail`)

result:s = library_function(`input`);
print(result);
```

#### Rules for sealed files

1. **Only declarations at the top level.** A safe-inserted file may declare
   functions, structs, enums, and constants. Any other top-level statement is
   a compile error: your program decides when sealed code runs, the sealed
   file never runs anything by itself.
2. **Sealed code may only call computation.** Inside sealed functions and
   sealed constant initializers, standard library calls are checked against a
   deny list. Denied: anything that touches the machine (files, network,
   databases, processes, email), anything that reads machine or invocation
   state (environment, system facts, arguments, stdin), anything holding
   process-global state (cache, i18n), and anything that seizes a resource
   (stdout and print, the terminal, the scheduler, sleeping). Allowed: all
   pure computation (math, strings, arrays, parsing, crypto, regex,
   compression, and the rest), plus `log_*` and `print_error`, because stderr
   cannot exfiltrate anything to the code's author and keeps sealed code
   debuggable.
3. **Enforcement is transitive.** Any function reachable from sealed code is
   checked by the same rules, so sealed code cannot launder an effect through
   an unsealed helper. If a sealed function calls your `fetch_report()`
   helper and that helper performs a network call, the program is rejected:

   ```nail
   // downloaded.nail (included with insert_safe)
   f sealed_summary():s {
       r fetch_report(); // fetch_report is now reachable from sealed code
   }
   ```

   ```nail
   // main.nail
   insert_safe(`downloaded.nail`)

   f fetch_report():s {
       // Rejected: reachable from sealed code, and http touches the machine
       r danger(http_download_file(`https://example.com/r`, `r.bin`));
   }
   ```
4. **Nesting stays sealed.** A plain `insert()` inside a safe-inserted file
   is sealed too, the whole subtree is. An `insert_safe()` nested deeper
   simply stays sealed.

If you trust a file and want it to touch the world, include it with plain
`insert()`. The two forms differ only in what the compiler will prove about
the code.

## Error Message Style Guide

Friendly, detailed errors are a core feature of Nail, and their quality is
enforced by golden-file tests (`tests/errors/`, run via
`./test_error_messages.sh`). Every diagnostic the compiler emits must answer
four things:

1. **What is wrong** — stated in plain language, never internal jargon.
   Write "Expected '}' but the file ended first", not "Expected BlockClose".
2. **Where** — the failing line of the user's own code is shown with a
   caret underline pointing at the problem (rendered by `CodeError::render`).
   A diagnostic with a missing span (line 0) is a bug.
3. **Why, with the actual values involved** — name the real types, variables,
   and functions, e.g. "'count' is declared as an integer (i) but its value
   is a string (s)". Use `NailDataTypeDescriptor::describe()` for type names,
   never `{:?}` Debug formatting.
4. **How to fix it** — a concrete `help:` suggestion (the `help` field on
   `CodeError`) whenever a fix is knowable: a "did you mean 'x'?" for typos,
   a corrected code snippet, or the idiomatic alternative.

Standard library runtime errors follow the same rule: always name the
function and echo the offending input, e.g.
"int_from: could not parse 'abc' as an integer".

Example of the required shape:

```
error: 'count' is declared as an integer (i) but its value is a string (s)
  --> tests/errors/type_mismatch_declaration.nail:2:18
   |
 2 | count:i = `hello`;
   |                  ^
help: either change the declaration to 'count:s' or make the value an integer (i)
```

## Versioning and Toolchain Pinning (Planned)

**Status: design commitment — not yet implemented.** This section records the
long-term plan so syntax and file formats evolve toward it.

### End goal

A Nail file records which compiler version it was written for, and the
toolchain automatically obtains and runs that **exact** version to compile it.
A Nail program that compiled once compiles forever — there are no dependency
mismatches, no "works on my machine", and no bit rot from language changes.
Old code never needs migration to keep working; migration is a choice, not a
requirement.

### Version pragma

Files will declare their language version with an optional header pragma on
the first line:

```nail
nail 0.1
```

- Files without a pragma are compiled with the current toolchain as today.
- The pragma syntax is reserved now so that files carrying it remain
  parseable by every future compiler version.

### Rollout phases

1. **Tag releases** — semantic version tags (`v0.1.0`) and a changelog, so
   there is something concrete to pin to.
2. **Parse and warn** — the compiler parses the pragma and warns when the
   file's declared version does not match the running compiler, but compiles
   anyway.
3. **Exact-version execution** — a toolchain manager (in the spirit of
   rustup) resolves the pragma, downloads the archived compiler binary for
   that exact release, and delegates compilation to it. Deferred until there
   are real users spread across multiple releases.

### Distribution: the bundle

Nail ships as **one immutable bundle per release**, installed at `/opt/nail`.
The promise: download, install, open — it works. Offline. No Rust
installation, no C compiler, no crates.io, nothing else on the machine.

The bundle contains everything a build touches:

- `bin/` — the IDE (`nail`) and compiler (`nailc`)
- `toolchain/` — a pinned Rust toolchain (rustc, cargo, rust-lld, std for
  the host and for `x86_64-unknown-linux-musl`)
- `cargo-home/` — `config.toml` (the single source of build configuration)
  plus vendored sources for every crate the stdlib registry can emit
- `nail/` — the nail crate source that generated programs depend on
- `cache/` — a pre-warmed shared build cache, so the first build on a fresh
  machine compiles only the user's program (seconds, not minutes)

Design decisions and why:

- **Fixed install path.** Cargo's build fingerprints embed absolute paths.
  Building the bundle's warm cache at `/opt/nail` and installing to
  `/opt/nail` is what lets the shipped cache stay valid on every machine.
- **Static musl output.** User programs target `x86_64-unknown-linux-musl`,
  linked by the bundled `rust-lld` with `link-self-contained=yes`. Linking
  needs zero system files, and the produced binaries are fully static — they
  run on any Linux distribution, including inside empty containers.
- **Scrubbed build environment.** The IDE invokes the bundled cargo by
  absolute path with a clean environment (`RUSTFLAGS`, `CARGO_*`, rustup
  installs, and the user's `PATH` cannot leak in). All configuration lives in
  the bundle's `cargo-home/config.toml`.
- **Closed dependency set.** Nail programs can only ever require crates the
  stdlib registry declares (`nailc --cargo-toml-superset` emits the full
  set), which is why complete vendoring and cache pre-warming are possible
  at all. Registry crates must be pure Rust or bundle their C source; crates
  that require system libraries at build time are not accepted.
- **Tools require a glibc distribution.** The bundled rustc is the official
  glibc build; the IDE and toolchain therefore run on mainstream distros
  (Ubuntu, Debian, Fedora, Arch, ...) but not musl-based ones like Alpine.
  Output binaries, being static, run anywhere.
- **Development checkouts are unaffected.** When no bundle is installed
  (`NAIL_HOME` overrides the default location), the IDE falls back to the
  system cargo — the workflow in this repository.

Tooling in `bundle/`: `build_bundle.sh` assembles and warms the bundle (the
only step that needs network and a musl C compiler — a build-machine
concern), `install.sh` installs it, and `test_bundle.sh` is the release
gate: on a machine with no Rust, no cc, and no network, compile and run a
Nail program using only the bundle. A release that fails the gate does not
ship.

## Standard Library

Nail includes a comprehensive standard library with functions organized by category:

### Namespaces

A Nail program has one flat name space and no import list, so every standard
library name carries the namespace of the library it belongs to. Functions wear
it in lower case and types in upper case:

```nail
reader:CSV_Reader = danger(csv_open(`big.csv`, csv_default_options()));
config:HTTP_Config = HTTP_Config { static_mounts = [], max_body_bytes = 0, timeout_seconds = 0, state = state };
stamp:s = danger(time_format(time_now(), TIME_Format::ISO8601));
```

`csv_*` and `CSV_*` belong to the CSV library, `http_*` and `HTTP_*` to the
HTTP one, `db_*` and `DB_*` to the databases, and so on - the prefix is what
says where a name comes from, since nothing else does. The rule covers
functions, structs, and enums alike, and holds for every module: a name
without a namespace reads as a word of the language itself.

The language's own words are the only names with no namespace: `print`,
`danger`, `safe`, `expect`, `panic`, `todo` and `spawn`. Two registry tests
enforce the rule, so a new stdlib name cannot skip it.

### The Library Describes Itself

There is no package manager, so what ships with the compiler is the whole set
of what a program can call - which makes the question "does this library go far
enough for what I am building?" worth answering exactly. `stdlib_functions()`
answers it in the program itself:

```nail
functions:a:STDLIB_Function = stdlib_functions();
modules:a:s = stdlib_modules();

string_functions:a:STDLIB_Function = filter function in functions {
    y function.module == `Strings`;
};
```

Each `STDLIB_Function` carries `name`, `module`, `signature`, `description` and
`example`, sorted by module and then by name. The list is read from the registry
the type checker itself consults, so it is exactly what the compiler running the
program can call, never a list someone has to remember to update. The library
section of the Nail website is that list, printed by a Nail program.

The lists below are kept by hand for reading; `stdlib_functions()` is the
authority.

### Core Operations
- `print(value)` - Print any value to stdout
- `assert(condition:b)` - Assert a condition is true, panic if false
- `panic(message:s)` - Panic with a message
- `todo(message:s)` - Mark unimplemented code

### String Operations
- `string_from(value):s!e` - Convert any value to string
- `string_to_uppercase(s:s):s` - Convert to uppercase
- `string_to_lowercase(s:s):s` - Convert to lowercase
- `string_to_title_case(s:s):s` - Convert to title case (capitalize each word)
- `string_to_sentence_case(s:s):s` - Convert to sentence case (capitalize first letter)
- `string_to_snake_case(s:s):s` - Convert to snake_case
- `string_to_kebab_case(s:s):s` - Convert to kebab-case
- `string_contains(s:s, substring:s):b` - Check if string contains substring
- `string_replace(s:s, from:s, to:s):s` - Replace all occurrences of substring
- `string_replace_first(s:s, from:s, to:s):s` - Replace first occurrence of substring
- `string_replace_all(s:s, from:s, to:s):s` - Replace all occurrences (alias for string_replace)
- `string_split(s:s, delimiter:s):a:s` - Split string by delimiter
- `string_split_whitespace(s:s):a:s` - Split string by whitespace
- `string_split_lines(s:s):a:s` - Split string by line breaks
- `string_trim(s:s):s` - Remove leading/trailing whitespace
- `string_trim_start(s:s):s` - Remove leading whitespace
- `string_trim_end(s:s):s` - Remove trailing whitespace
- `string_pad_start(s:s, length:i, pad:s):s` - Pad string on the left to specified length
- `string_pad_end(s:s, length:i, pad:s):s` - Pad string on the right to specified length
- `string_length(s:s):i` - Get string length
- `string_chars(s:s):a:s` - Convert string to array of single-character strings
- `string_starts_with(s:s, prefix:s):b` - Check if string starts with prefix
- `string_ends_with(s:s, suffix:s):b` - Check if string ends with suffix
- `string_index_of(s:s, substring:s):i!e` - Find index of first occurrence (can fail)
- `string_last_index_of(s:s, substring:s):i!e` - Find index of last occurrence (can fail)
- `string_substring(s:s, start:i, end:i):s!e` - Extract substring (can fail)
- `string_repeat(s:s, count:i):s` - Repeat string count times
- `string_reverse(s:s):s` - Reverse string characters
- `string_minify(s:s):s` - Remove all whitespace outside of quoted strings (useful for minifying JSON)
- `string_join(arr:a:s, separator:s):s` - Join array of strings with separator
- `string_is_alphabetic(s:s):b` - Check if string contains only alphabetic characters
- `string_is_digits_only(s:s):b` - Check if string contains only digit characters (0-9)
- `string_is_numeric(s:s):b` - Check if string can be parsed as a number (includes decimals, signs)
- `string_is_alphanumeric(s:s):b` - Check if string contains only alphanumeric characters

### Array Operations
- `array_length(arr:a:T):i` - Get array length
- `array_get(arr:a:T, index:i):T!e` - Get element at index (can fail)
- `array_push(arr:a:T, item:T):v` - Add element to array
- `array_join(arr:a:s, separator:s):s` - Join string array with separator
- `array_contains(arr:a:T, item:T):b` - Check if array contains item
- `array_concat(arr1:a:T, arr2:a:T):a:T` - Concatenate two arrays
- `array_reverse(arr:a:T):a:T` - Reverse array elements
- `array_slice(arr:a:T, start:i, end:i):a:T!e` - Get subarray (can fail)
- `array_sort(arr:a:T):a:T` - Sort array elements
- `array_range(start:i, end:i):a:i` - Generate array of integers from start (inclusive) to end (exclusive)
- `array_range_inclusive(start:i, end:i):a:i` - Generate array of integers from start to end (both inclusive)
- `array_repeat(value:T, count:i):a:T` - Create array with value repeated count times
- `array_take(arr:a:T, count:i):a:T` - Take first count elements from array
- `array_skip(arr:a:T, count:i):a:T` - Skip first count elements from array
- `array_take_while(arr:a:T, predicate:f(T):b):a:T` - Take elements while predicate is true
- `array_skip_while(arr:a:T, predicate:f(T):b):a:T` - Skip elements while predicate is true
- `array_zip(arr1:a:T, arr2:a:U):a:Pair<T,U>` - Combine two arrays element-wise into pairs
- `array_flatten(arr:a:a:T):a:T` - Flatten nested array by one level
- `array_unique(arr:a:T):a:T` - Remove duplicate elements (alias for deduplicate)
- `array_deduplicate(arr:a:T):a:T` - Remove duplicate elements
- `array_find(arr:a:T, value:T):i!e` - Find index of first occurrence (can fail)
- `array_find_last(arr:a:T, value:T):i!e` - Find index of last occurrence (can fail)
- `array_filter(arr:a:T, predicate:f(T):b):a:T` - Filter elements using predicate function
- `array_map(arr:a:T, mapper:f(T):U):a:U` - Transform elements using mapper function
- `array_intersect(arr1:a:T, arr2:a:T):a:T` - Get intersection of two arrays
- `array_difference(arr1:a:T, arr2:a:T):a:T` - Get elements in arr1 but not in arr2
- `array_union(arr1:a:T, arr2:a:T):a:T` - Get union of two arrays (unique elements from both)
- `array_rotate_left(arr:a:T, positions:i):a:T` - Rotate array elements left by n positions
- `array_rotate_right(arr:a:T, positions:i):a:T` - Rotate array elements right by n positions

Editing an array means building the new one, since arrays are immutable:
- `array_insert(arr:a:T, index:i, item:T):a:T!e` - With the item put in, moving the rest along
- `array_remove_at(arr:a:T, index:i):a:T!e` - Without the element at the index
- `array_replace_at(arr:a:T, index:i, item:T):a:T!e` - With that one element changed
- `array_swap(arr:a:T, first:i, second:i):a:T!e` - With two elements exchanged
- `array_index_of(arr:a:T, item:T):i!e` - Where it first appears; absent is an error, not `-1`
- `array_count_of(arr:a:T, item:T):i` - How many times it appears
- `array_is_empty(arr:a:T):b`, `array_all_equal(arr:a:T):b`
- `array_sort_descending(arr:a:T):a:T` - Largest first, for a leaderboard

Where the interesting part of an element is one field of it, the key comes from a
function the program has already named - which is not a closure, and needs none:

```nail
struct Book { title:s, author:s, year:i }
f book_year(book:Book):i    { r book.year; }
f book_author(book:Book):s  { r book.author; }

by_year:a:Book         = array_sort_by(books, book_year);
newest:Book            = danger(array_max_by(books, book_year));
year_total:i           = array_sum_by(books, book_year);
by_author:h<s,a:Book>  = array_group_by(books, book_author);
per_author:h<s,i>      = array_count_by(books, book_author);
```

- `array_sort_by(arr:a:T, key:f(T):K):a:T` / `array_sort_by_descending(...)`
- `array_min_by(arr:a:T, key:f(T):K):T!e` / `array_max_by(...)` - an empty array is an error
- `array_sum_by(arr:a:T, key:f(T):K):K` - K is `i` or `f`
- `array_group_by(arr:a:T, key:f(T):K):h<K,a:T>` - buckets, in the order they appeared
- `array_count_by(arr:a:T, key:f(T):K):h<K,i>` - the bucket sizes alone

Nothing calls the key function while it works: the keys are worked out over the
array first, and only then is anything sorted, bucketed or totalled. So a key
function may read a file or make a request -
`array_sort_by(reports, report_size)` where `report_size` calls `fs_size` is fine,
and costs one read per element rather than one per comparison.

These are the bucketing and ordering that fit in an array. Counting inside groups,
ordering groups by their totals, joining two sets of rows - those are what
`db_sqlite_*` and `db_datafusion_*` are for, and SQL says them better than any
number of array functions would.

### HashMap Operations
**Note**: HashMap keys and values must be concrete types (i, f, s, b, arrays, structs, enums). Void type cannot be used as a value.

- `hashmap_new():h<K,V>` - Create new hashmap (K,V must be concrete types)
- `hashmap_set(map:h<K,V>, key:K, value:V):v` - Insert key-value pair
- `hashmap_get(map:h<K,V>, key:K):V!e` - Get value by key (can fail)
- `hashmap_remove(map:h<K,V>, key:K):V!e` - Remove and return value
- `hashmap_contains_key(map:h<K,V>, key:K):b` - Check if key exists
- `hashmap_len(map:h<K,V>):i` - Get number of entries
- `hashmap_clear(map:h<K,V>):v` - Remove all entries
- `hashmap_keys(map:h<K,V>):a:K` - Get all keys as array
- `hashmap_values(map:h<K,V>):a:V` - Get all values as array
- `hashmap_is_empty(map:h<K,V>):b` - Check if map is empty

### Type Conversion
- `int_from(value):i!e` - Convert to integer
- `float_from(value):f!e` - Convert to float
- `bool_from(value):b!e` - Convert to boolean

### JSON Serialization/Deserialization
- `json_serialize(value:T):s!e` - Serialize any value to pretty-formatted JSON string (with indentation)
- `json_deserialize(json:s):T!e` - Deserialize JSON string to a value (type inferred from variable declaration)

### Database Operations (`db_sqlite_*`)
- `db_sqlite_memory():DB_SQLite!e` - Create an in-memory SQLite database
- `db_sqlite_open(path:s):DB_SQLite!e` - Open a SQLite database file
- `db_sqlite_execute(db:DB_SQLite, sql:s):DB_Result!e` - Execute SQL that doesn't return rows (CREATE, INSERT, UPDATE, DELETE)
- `db_sqlite_query(db:DB_SQLite, sql:s):a:T!e` - Execute SQL query and return results as array of structs (type T inferred from variable declaration)
- `db_sqlite_query_single(db:DB_SQLite, sql:s):T!e` - Execute SQL query and return single result as struct (type T inferred from variable declaration)
- `db_sqlite_execute_params(db:DB_SQLite, sql:s, params:a:s):DB_Result!e` - Execute SQL with `?` placeholders bound to values
- `db_sqlite_query_params(db:DB_SQLite, sql:s, params:a:s):a:T!e` - Query with `?` placeholders bound to values
- `db_sqlite_query_single_params(db:DB_SQLite, sql:s, params:a:s):T!e` - Query with `?` placeholders, returning the first row
- `db_sqlite_close(db:DB_SQLite):v!e` - Close database connection
- `db_sqlite_execute_batch(db:DB_SQLite, statements:a:s):DB_Result!e` - Execute multiple SQL statements atomically

Any value going into a statement belongs in the `_params` list, not in the SQL
text. Quoting values by hand works until one is spliced somewhere a quote was
not expected, and then the value is running as query. SQLite binds every
parameter as text and applies the column's affinity, so a number held in a
string still stores and compares as a number.

### Math Operations (`math_*`)

Rounding returns a float, so the shape of a calculation does not change halfway
through; `math_round_to_int` is the one that leaves the number line of floats.

- `math_abs(value:f):f` - Absolute value
- `math_sign(value:f):i` - -1, 0 or 1 according to the direction of the value
- `math_min(a:f, b:f):f` / `math_max(a:f, b:f):f` - Smaller and larger of two values
- `math_clamp(value:f, min:f, max:f):f` - Restrict a value to a range
- `math_pow(base:f, exponent:f):f` - Raise to a power
- `math_sqrt(value:f):f` - Square root
- `math_cbrt(value:f):f` - Cube root, defined for negative numbers too
- `math_hypot(x:f, y:f):f` - Distance from the origin to (x, y), exact where squaring would overflow
- `math_ceil(value:f):f` / `math_floor(value:f):f` / `math_round(value:f):f` - Rounding, as a float
- `math_round_to_int(value:f):i` - Rounding, as a whole number
- `math_trunc(value:f):f` - Drop the fraction towards zero, so -2.7 gives -2.0 where `math_floor` gives -3.0
- `math_fract(value:f):f` - Just the fractional part, keeping the sign
- `math_divide(a:f, b:f):f!e` - Division that errors instead of producing infinity
- `math_modulo(value:f, divisor:f):f!e` - Remainder with the sign of the divisor, so -1 modulo 12 is 11
- `math_gcd(a:i, b:i):i` / `math_lcm(a:i, b:i):i` - Greatest common divisor, least common multiple
- `math_factorial(n:i):i!e` - Factorial; errors below zero and above 20, where it overflows
- `math_is_prime(n:i):b` - Whether a number is prime
- `math_sin(angle:f):f` / `math_cos` / `math_tan` - Trigonometry, in radians
- `math_asin(value:f):f!e` / `math_acos(value:f):f!e` - Inverses; error outside -1 to 1
- `math_atan(value:f):f` - Arc tangent of a ratio
- `math_atan2(y:f, x:f):f` - The angle to the point (x, y), from -pi to pi. **Use this, not `math_atan`, for angles** - a ratio alone cannot tell the second quadrant from the fourth
- `math_sinh(value:f):f` / `math_cosh` / `math_tanh` - Hyperbolic functions
- `math_to_degrees(radians:f):f` / `math_to_radians(degrees:f):f` - Angle conversion
- `math_log(value:f):f!e` / `math_log2` / `math_log10` - Logarithms; error at or below zero
- `math_log_base(value:f, base:f):f!e` - Logarithm in a base of your choosing
- `math_exp(value:f):f` - e raised to a power
- `math_sigmoid(value:f):f` - 1 / (1 + e^-x)
- `math_lerp(start:f, end:f, t:f):f` - Linear interpolation
- `math_is_nan(value:f):b` / `math_is_infinite(value:f):b` / `math_is_finite(value:f):b` - What kind of number this is. Not-a-number is the one value not equal to itself, so `==` cannot be used to ask
- `math_random():f` - A fraction from 0.0 up to 1.0. **Not for secrets** - see `crypto_random_hex`
- `math_pi():f` / `math_e():f` - Constants

### Statistics (`stats_*`)

Every one of these is undefined on an empty array, so every one returns a
result rather than a number invented out of nothing.

- `stats_mean(values:a:f):f!e` - The average
- `stats_median(values:a:f):f!e` - The middle value, which one outlier cannot move
- `stats_variance(values:a:f):f!e` / `stats_stddev(values:a:f):f!e` - Sample spread; need at least two values
- `stats_percentile(values:a:f, share:f):f!e` - The value below which that share falls, `0.95` for the ninety-fifth percentile
- `stats_range(values:a:f):f!e` - Distance from smallest to largest
- `stats_correlation(first:a:f, second:a:f):f!e` - How closely two columns move together, -1.0 to 1.0

### Bits (`bits_*`)

Whole numbers are 64-bit and signed; every function works on that pattern of 64
bits. A shift or index outside 0 to 63 is an error rather than a silently
different answer.

- `bits_and(left:i, right:i):i` / `bits_or` / `bits_xor` / `bits_not(value:i):i`
- `bits_shift_left(value:i, places:i):i!e` / `bits_shift_right` - Fill with zeros
- `bits_rotate_left(value:i, places:i):i!e` / `bits_rotate_right` - Bits that fall off return at the other end
- `bits_count_ones(value:i):i` / `bits_count_zeros` - Population count, the size of a set held as a bitmask
- `bits_leading_zeros(value:i):i` / `bits_trailing_zeros` - 64 for zero itself
- `bits_get(value:i, index:i):b!e` / `bits_set(value:i, index:i, on:b):i!e` - One bit at a time
- `bits_to_binary(value:i):s` / `bits_from_binary(text:s):i!e` - Ones and zeros; underscores allowed as separators
- `bits_to_hex(value:i):s`

### Drawing (`draw_*`)

Every function returns a string. A shape is a string, a group of shapes is a
string, and a drawing is a string that happens to be an SVG document - which
browsers, editors, README files and print all understand, and which `fs_write`
saves like any other text. There is no window, no canvas to mutate and no
drawing context to hold on to, so a chart is a map and a join like anything
else. Coordinates start at the top left and y grows downward.

- `draw_svg(width:f, height:f, background:s, shapes:a:s):s!e` - Wrap shapes in a document; an empty background leaves it transparent
- `draw_rect(x:f, y:f, width:f, height:f, fill:s, corner_radius:f):s!e`
- `draw_circle(center_x:f, center_y:f, radius:f, fill:s):s!e`
- `draw_ellipse(center_x:f, center_y:f, radius_x:f, radius_y:f, fill:s):s!e`
- `draw_line(x1:f, y1:f, x2:f, y2:f, stroke:s, stroke_width:f):s!e`
- `draw_polyline(points:a:f, stroke:s, stroke_width:f):s!e` - Connected segments from a flat array of x and y values; the shape a line chart is made of
- `draw_polygon(points:a:f, fill:s):s!e` - The same, closed and filled
- `draw_text(x:f, y:f, content:s, size:f, fill:s, anchor:s):s!e` - Anchor is `start`, `middle` or `end`
- `draw_path(commands:s, stroke:s, stroke_width:f, fill:s):s!e` - SVG path notation, for a shape none of the others can make
- `draw_group(offset_x:f, offset_y:f, shapes:a:s):s` - Move several shapes together
- `draw_scale(value:f, from_low:f, from_high:f, to_low:f, to_high:f):f!e` - Move a value between ranges. To plot upward on a screen whose y grows downward, pass the height as `to_low` and `0.0` as `to_high`

Text is XML-escaped, so a label containing `&` or `<` cannot produce a document
nothing will open.

### Audio (`audio_*`)

Two things a program wants from sound: play this file, and beep when something
finishes. Playing is synchronous - the call returns when the sound has finished
- so a notification is one line; put it in a `spawn` block to carry on while it
plays.

- `audio_play_file(path:s):v!e` - Play a WAV, MP3, FLAC or Ogg Vorbis file
- `audio_play_tone(hertz:f, seconds:f, volume:f):v!e` - 440.0 hertz is a concert A; 0.2 is a better volume for a notification than 1.0
- `audio_is_available():b` - Whether this machine has a sound device. Ask before playing anything on a server, where the answer is usually no

This module is behind the `audio` feature. A sound device is not something
every machine has, and on Linux building against one needs ALSA's development
headers - a server that will never make a sound should not have to install
them. `nailc --cargo-toml` turns the feature on only for a program that
actually calls one of these.

### Machine Learning (`ml_*`)

A fitted model is data - a plain struct you can print, store as JSON and
predict with later - not a handle to something living inside the library.
Everything involving randomness takes the seed as an argument, because a model
that trains differently every run cannot be debugged. Features are an array of
rows, each row an array of numbers of the same length.

- `ml_split_train_test(features:a:a:f, labels:a:i, train_share:f, seed:i):ML_Split!e` - Cut a dataset into a part to learn from and a part to be judged on, shuffled first
- `ml_normalize(values:a:f):a:f!e` - Rescale to 0.0 through 1.0, so a column in millions does not drown out one in single digits
- `ml_standardize(values:a:f):a:f!e` - Rescale to sit around zero with a spread of one; the one to use when outliers matter
- `ml_tree_fit(features:a:a:f, labels:a:i, max_depth:i):ML_Tree!e` - A decision tree
- `ml_tree_predict(tree:ML_Tree, row:a:f):i!e`
- `ml_tree_explain(tree:ML_Tree, feature_names:a:s):s!e` - The rules the tree actually applies, written out. The reason to reach for a tree over something more accurate
- `ml_linear_fit(features:a:a:f, targets:a:f):ML_Linear!e` - The straight line closest to the data, exactly rather than iteratively
- `ml_linear_predict(model:ML_Linear, row:a:f):f!e`
- `ml_knn_predict(features:a:a:f, labels:a:i, query:a:f, k:i):i!e` - Ask the k nearest rows what they are. No fitting happens, so this is what to reach for when there is very little data
- `ml_kmeans(points:a:a:f, k:i, seed:i, iterations:i):ML_Clusters!e` - Group points by nearness
- `ml_forest_fit(features:a:a:f, labels:a:i, trees:i, max_depth:i, seed:i):ML_Forest!e` / `ml_forest_predict(forest:ML_Forest, row:a:f):i!e` - Trees that vote. Far less sensitive to settings than boosting, so reach for it when there is no time to tune anything
- `ml_boost_default_config():ML_BoostConfig` / `ml_boost_fit(features:a:a:f, targets:a:f, config:ML_BoostConfig):ML_Boost!e` - Gradient boosting
- `ml_boost_fit_validated(features:a:a:f, targets:a:f, validation_features:a:a:f, validation_targets:a:f, config:ML_BoostConfig):ML_Boost!e` - The same, stopping once a held-out set stops improving
- `ml_boost_predict(model:ML_Boost, row:a:f):f!e`
- `ml_boost_predict_probability(model:ML_Boost, row:a:f):f!e` - For a model fitted with `ML_Objective::Logistic`
- `ml_boost_importance(model:ML_Boost):a:f!e` - How much each column contributed, as a share of the total gain
- `ml_cross_validate_boost(features:a:a:f, targets:a:f, folds:i, config:ML_BoostConfig, seed:i):ML_Regression!e` - Every row takes a turn being held out
- `ml_one_hot(values:a:s):ML_OneHot!e` / `ml_one_hot_with(values:a:s, categories:a:s):a:a:f!e` - A column of words as one column of 0s and 1s per word
- `ml_target_encode(values:a:s, targets:a:f, smoothing:f):h<s,f>!e` / `ml_encode_with(values:a:s, encoding:h<s,f>, fallback:f):a:f` - For columns where one-hot would add a thousand columns
- `ml_score(predicted:a:i, actual:a:i):ML_Scores!e` - Classification, counted four ways
- `ml_regression_scores(predicted:a:f, actual:a:f):ML_Regression!e` - Regression, measured six ways

**Choosing an objective.** `ML_Objective::Squared` fits a number.
`ML_Objective::Logistic` fits a yes-or-no answer, and is not the same as
fitting `Squared` against 0 and 1: it optimises the odds rather than the
distance, so a confident wrong answer is punished far more than a hesitant one.
Read a logistic model with `ml_boost_predict_probability`.

**How many trees is the only hard question**, and `ml_boost_fit_validated`
answers it. Too few and the model has not finished learning; too many and it
starts memorising - and the *training* score improves the whole time, so it
cannot tell you which side you are on. Watching data the model is not learning
from can. Trees grown after the best one are thrown away, and `trees_used` says
how many were kept.

**Missing values are learned, not guessed at.** A `NaN` in a column is routed
by whichever side of each split it helps more, so a model can learn that an
absent value means something - an absent income and an absent postcode do not
mean the same thing. `ml_tree_fit` refuses rows with gaps instead of deciding
for you; `ml_boost_fit` handles them.

**Fit encodings on training rows only.** `ml_target_encode`'s smoothing is what
stops a category with one row being encoded as that row's own answer, which
hands the model what it is supposed to predict and produces something that
scores brilliantly in training and fails completely in use. And keep the
vocabulary `ml_one_hot` returns: encoding new data against a different one
silently shifts every column along.

**ML_Objective values:** `Squared`, `Logistic`.

**Gradient boosting** (`ml_boost_*`) is the method that wins on ordinary
tabular data - the kind that comes out of a database with a few dozen columns -
and it is what LightGBM and XGBoost implement. It grows many small trees, each
one fitted to what the model still gets wrong. Two ideas make it fast enough to
be worth having, and both are here: split points are chosen once from quantiles
rather than searched over every value, and each tree is fitted to the gradient
of the loss rather than to the target, so the arithmetic per node is a pair of
sums. It predicts a number; for yes-or-no questions, fit against 0 and 1 and
treat anything above 0.5 as yes.

**Judge a model on data it never saw.** A tree deep enough will predict its
training rows perfectly and predict nothing else at all, and only the test half
will tell you. That is what `ml_split_train_test` is for, and why `max_depth`
is a required argument rather than an optional one.

**Never trust accuracy alone.** On data where one case in a thousand is
positive, a model answering "no" every time scores 99.9%. That is why
`ml_score` returns precision, recall and f1 beside it, and why
`ml_regression_scores` returns six numbers rather than one - the absolute
measures and the percentage ones disagree, and the disagreement is information.

**ML_BoostConfig fields:** `trees`, `learning_rate`, `max_depth`,
`min_samples_leaf`, `bins`, `lambda_l2`. Start from
`ml_boost_default_config()` and change what you need.

### Randomness (`rand_*`)

Two halves. The plain functions draw from a generator seeded by the operating
system - fine for a dice roll or a jitter delay, never for a session id. The
`rand_seeded_*` functions take the seed as an argument, so the same seed always
gives the same answer: that is what makes a test that samples data reproducible.

- `rand_int(min:i, max:i):i!e` - A whole number, both ends included
- `rand_float():f` / `rand_float_range(min:f, max:f):f!e` - A fraction
- `rand_bool():b` / `rand_chance(probability:f):b!e` - True with the given odds
- `rand_pick(items:a:T):T!e` - One element, chosen evenly
- `rand_sample(items:a:T, count:i):a:T!e` - Several elements without repeats
- `rand_seeded_int(seed:i, min:i, max:i):i!e` / `rand_seeded_float(seed:i):f` / `rand_seeded_shuffle(seed:i, items:a:T):a:T`

### I/O Operations (`io_*`)
- `io_read_line():s!e` - Read a line from standard input
- `io_read_line_prompt(prompt:s):s!e` - Print a prompt, then read a line
- `io_read_int():i!e` / `io_read_int_prompt(prompt:s):i!e` - Read a whole number
- `io_read_float():f!e` / `io_read_float_prompt(prompt:s):f!e` - Read a fraction

Asking a person something, for a command-line tool or a setup script:
- `io_confirm(question:s, default_answer:b):s!e` - Asks until it gets `yes` or `no`; an empty line means the default
- `io_select(question:s, options:a:s):i!e` - A numbered list, answered with the index chosen
- `io_read_secret(prompt:s):s!e` - A line with nothing shown as it is typed
- `io_read_line_or(prompt:s, default_answer:s):s!e` - The default when nothing is typed

Files are read and written with `fs_*` below, not here.

### File System (`fs_*`)
- `fs_read(path:s):s!e` - Read a whole file into a string
- `fs_read_lines(path:s):a:s!e` - Read a file as lines, without the line endings
- `fs_write(path:s, content:s):v!e` - Write a file, creating or truncating it
- `fs_append(path:s, content:s):v!e` - Add to the end of a file, creating it if missing
- `fs_copy(from:s, to:s):v!e` / `fs_move(from:s, to:s):v!e`
- `fs_remove_file(path:s):v!e` - Delete a file
- `fs_create_dir(path:s):v!e` - Create a directory and any missing parents
- `fs_read_dir(path:s):a:s!e` - The sorted paths directly inside a directory
- `fs_walk(path:s):a:s!e` - The sorted paths of every file underneath, however deep. Links are not followed
- `fs_remove_dir(path:s):v!e` - Remove an empty directory; anything inside it is an error
- `fs_remove_dir_all(path:s):v!e` - Remove a directory and everything in it. There is no undoing this
- `fs_size(path:s):i!e` - How many bytes a file holds
- `fs_modified(path:s):i!e` - When it last changed, as a Unix timestamp
- `fs_is_dir(path:s):b` / `fs_is_file(path:s):b` - False for a path that is not there
- `fs_temp_dir():s` - Where this machine keeps temporary files
- `fs_glob(directory:s, pattern:s):a:s!e` - Every file underneath whose path matches a shell pattern

#### Reading a file too large to hold

`fs_read` and `fs_read_lines` load the whole file, which is right until the file is
larger than the machine's memory. Three ways to read one without holding it:

- `fs_reduce_lines(path:s, initial:A, step:f(A, s):A):A!e` - a fold over the lines,
  the way `reduce` folds an array. Nothing to open, nothing to close, and the step
  function may read files or make requests itself
- `fs_open(path:s):FS_Reader!e` / `fs_next_lines(reader:FS_Reader, count:i):a:s!e` /
  `fs_close(reader:FS_Reader):v!e` - the general form, for a loop that stops early
  or does something a fold cannot say. An empty answer from `fs_next_lines` means
  the file is finished, and the reader has closed itself by then
- `fs_append_file(from_path:s, to_path:s):v!e` - one file onto the end of another,
  copied in blocks. How the pieces of a resumable upload are reassembled

```nail
f count_errors(total:i, line:s):i {
    r if { string_contains(line, `ERROR`) -> { r total + 1; }, else -> { r total; } };
}
errors:i = danger(fs_reduce_lines(`app.log`, 0, count_errors));
```

A file that is not text needs none of this: copying, moving, hashing, archiving and
serving it are all path operations already. To look inside one - a header, a footer,
the bytes that say what format it is:

- `fs_read_range_base64(path:s, offset:i, length:i):s!e`
- `fs_read_range_hex(path:s, offset:i, length:i):s!e` - a PNG starts `89504e47`, a zip `504b0304`

Both read exactly the slice asked for and nothing before or after it. For a large
CSV or Parquet file, `db_datafusion_*` is better than any of this: register the file
and write SQL, and the query engine streams it.

`path_matches_glob(pattern:s, path:s):b` matches a path against a pattern
without touching the disk: `*` stays inside one segment, `**` crosses segments,
`?` is one character, and `[abc]` is one of those listed.

Whether a path exists at all is `path_exists`, with the rest of the path
functions.

### HTTP Operations (`http_*`)
- `http_request(method:HTTP_Method, url:s, headers:h<s,s>, body:s):HTTP_Response!e` - Make an outbound HTTP request. `HTTP_Method` is `Get`, `Post`, `Put`, `Delete` or `Patch`
- `http_server(port:i, config:HTTP_Config):v` - Serve HTTP on a port. Blocks forever
- `http_default_config():HTTP_Config` - The default server configuration
- `http_path_matches(pattern:s, path:s):b` - Whether a path matches a route pattern
- `http_path_params(pattern:s, path:s):h<s,s>` - The named segments a pattern binds
- `http_default_cookie(name:s, value:s):HTTP_Cookie` - A cookie with the safe defaults: `/` path, session lifetime, HttpOnly, Secure, SameSite=Lax
- `http_build_cookie(cookie:HTTP_Cookie):s!e` - The `Set-Cookie` header value for a cookie
- `http_parse_cookies(header:s):h<s,s>` - Parse the browser's `Cookie` header, which holds every cookie at once
- `http_request_multipart(method:HTTP_Method, url:s, headers:h<s,s>, parts:a:HTTP_Part):HTTP_Response!e` - Send a `multipart/form-data` request, the encoding a file upload uses
- `http_part_text(name:s, value:s):HTTP_Part` - One text field of a multipart form
- `http_part_file(name:s, file_path:s):HTTP_Part` - One file field, read from disk when the request is sent
- `http_request_retry(method:HTTP_Method, url:s, headers:h<s,s>, body:s, retry:HTTP_Retry):HTTP_Response!e` - Make a request, sending it again while it keeps failing temporarily
- `http_default_retry():HTTP_Retry` - Retry settings worth having: three attempts, 250ms doubling to 5s, 30s per attempt

An upload is a `multipart/form-data` request, and the file's bytes never pass
through the program - a part names a path, and the request reads it on its way
out. That is how a PNG is uploaded from a language with no byte type. The
boundary belongs to the body, so `http_request_multipart` sets `Content-Type`
itself and rejects one passed in `headers` rather than sending a body no server
can parse.

```nail
parts:a:HTTP_Part = [
    http_part_text(`purpose`, `avatar`),
    http_part_file(`file`, `portrait.png`),
];
response:HTTP_Response = danger(http_request_multipart(HTTP_Method::Post, url, headers, parts));
```

`http_request_retry` sends the request again only when the failure might not
happen next time: no answer at all, or a 408, 429, 500, 502, 503 or 504. A 4xx
the server understood and refused comes straight back, because asking again
would be refused again. Each wait doubles up to the ceiling with half of it
randomised, a `Retry-After` header is honoured over that, and the last response
is returned whatever its status - a program still sees the server's own 500
rather than an error from the library.

The request is sent again exactly as it was, so an API that must not act twice
on the same call - a payment, an order - wants an idempotency key in the
headers, which is the same thing its own documentation asks for.

```nail
settings:HTTP_Retry = HTTP_Retry {
    attempts = 4,
    initial_delay_ms = 200,
    max_delay_ms = 3000,
    timeout_ms = 10000
};
response:HTTP_Response = danger(http_request_retry(HTTP_Method::Post, url, headers, body, settings));
```

`http_server` hands **every** request, whatever its method or path, to a
function the program must define:

```nail
f handle_request(request:HTTP_Request, state:h<s,s>):HTTP_Response {
    headers:h<s,s> = hashmap_new();
    if {
        http_path_matches(`/dictionary/:word`, request.path) -> {
            params:h<s,s> = http_path_params(`/dictionary/:word`, request.path);
            word:s = danger(hashmap_get(params, `word`));
            r HTTP_Response { status = 200, body = word, content_type = `text/html`, headers = headers };
        },
        request.method == `POST` -> {
            // A form body uses the same encoding as a query string
            form:h<s,s> = url_parse_query(request.body);
            r HTTP_Response { status = 200, body = danger(hashmap_get(form, `message`)), content_type = `text/plain`, headers = headers };
        },
        else -> {
            r HTTP_Response { status = 404, body = `Not found`, content_type = `text/html`, headers = headers };
        }
    }
}

config:HTTP_Config = HTTP_Config {
    static_mounts = [
        HTTP_Static { prefix = `/static`, directory = `static` },
        HTTP_Static { prefix = `/images`, directory = `static/images` }
    ],
    max_body_bytes = 0,
    timeout_seconds = 0,
    state = hashmap_new()
};
http_server(8080, config);
```

Routing is ordinary Nail code rather than a table the server owns, so a path
and what it serves stay together. Since a function can only see its own
parameters, anything the handler needs - page content, file paths, settings -
travels in through `config`.

`HTTP_Request` has `method:s`, `path:s`, `query:h<s,s>`, `headers:h<s,s>` and
`body:s`. `HTTP_Response` has `status:i`, `body:s`, `content_type:s` and
`headers:h<s,s>`; set `location` in its headers with status 301 to redirect.

`HTTP_Config` fields: `static_mounts` (directories served as static files,
including binary ones; an empty array serves none), `max_body_bytes` (0 means 8 MiB; larger bodies get 413), `timeout_seconds`
(0 means 30; a handler that overruns gives the client 504), and `state`.

Each `HTTP_Static` pairs a URL `prefix` with the `directory` on disk behind
it. It is a list because a real site serves several trees - `/images`,
`/fonts`, `/js` - from different directories, and files are matched against
the mounts before the handler runs.

Settings are typed fields, not string keys. `state` is the one deliberate
hashmap: it carries application data - page content, file paths - straight
through to `handle_request`, and only the program knows its shape. Since a
Nail function can only see its own parameters, that map is how anything
computed at startup reaches the handler.

A pattern segment beginning with `:` matches any one segment, and a trailing
`*` matches the rest of the path. Set `BIND_ADDR=127.0.0.1` to serve only to a
local reverse proxy.

#### Receiving an upload

`HTTP_Request` carries `body_path` alongside `body`. Exactly one of them is ever
set: a text body arrives in `body` as usual, and a body that is not valid UTF-8 -
a photo, a PDF, a zip - is written to a file by the server before any of it is
read as text, with the path in `body_path`.

```nail
f handle_request(request:HTTP_Request, state:h<s,s>):HTTP_Response {
    kind:s = danger(image_format(request.body_path));   // what it really is
    danger(image_resize_within(request.body_path, `public/avatars/small.png`, 200, 200));
    danger(fs_remove_file(request.body_path));
    r ok_response(`saved`);
}
```

The decision is made on the bytes rather than on `Content-Type`, because a client
that mislabels a PNG is not a reason to corrupt it. A body over a mebibyte goes to
a file whatever it is, written as it arrives, so what a request costs in memory
does not depend on how large the cap is. `max_body_bytes` defaults to 8 MiB - a
photograph off a phone is three to five.

**The server deletes the file once the handler returns**, however it returns. A
handler that wants to keep an upload moves or copies it (`fs_move`, `fs_copy`);
one that returns early, fails or times out leaves nothing behind.

A browser form with a file in it sends `multipart/form-data`, which is several
parts in one body. `http_multipart_extract(body_path, content_type, into_directory)`
takes it apart, reading in blocks and writing each file part as it finds it:

```nail
content_type:s = danger(hashmap_get(request.headers, `content-type`));
fields:h<s,s> = danger(http_multipart_extract(request.body_path, content_type, `uploads`));

caption:s = danger(hashmap_get(fields, `caption`));          // a text field's value
photo:s = danger(hashmap_get(fields, `photo`));              // the file's written path
original:s = danger(hashmap_get(fields, `photo.filename`));  // the name the client gave
kind:s = danger(hashmap_get(fields, `photo.type`));          // the type it declared
```

The written name is chosen by the server, not the client: a part claiming to be
called `../../etc/cron.d/anything` is written as `anything` under a fresh id, so a
file name can neither escape the directory nor overwrite somebody else's
upload.

This is why Nail has no bytes type. The bytes that cannot be a Nail string never
become one - they go to a file, and the program works in paths.

### CSV Operations (`csv_*`)
- `csv_parse(text:s, options:CSV_Options):a:h<s,s>!e` - Parse CSV text into one hashmap per row, keyed by the header row
- `csv_default_options():CSV_Options` - The defaults, since Nail has no default field values
- `csv_open(path:s, options:CSV_Options):CSV_Reader!e` - Open a file for batch reading
- `csv_next_rows(reader:CSV_Reader, count:i):a:h<s,s>!e` - Read up to `count` more rows
- `csv_close(reader:CSV_Reader):v!e` - Close a reader and release its file descriptor

All of these are quote-aware, so a field containing the delimiter or a newline
survives intact. Splitting on commas by hand corrupts every column after such
a field.

**Which one to use.** There are three layers, and picking the wrong one is the
usual mistake:

| Situation | Use |
| --- | --- |
| Text you already have in memory - an API response, a small file | `csv_parse` |
| A file too large to hold in memory, walked row by row | `csv_open` + `csv_next_rows` |
| A large file you want to *query* - filter, aggregate, join | `db_datafusion_register_csv` |

`csv_parse` takes the whole document as a string, so the file has to fit in
memory twice over - once as text, once as rows. For anything larger, open a
reader and pull batches:

```nail
reader:CSV_Reader = danger(csv_open(`big.csv`, csv_default_options()));
batch:a:h<s,s> = danger(csv_next_rows(reader, 10000));
// A batch shorter than the count asked for means the file is finished.
danger(csv_close(reader));
```

For analytical work over a large file - filtering, aggregating, joining - use
DataFusion instead, which streams, pushes projections down, and spills to disk:

```nail
session:DB_DataFusion = danger(db_datafusion_session());
danger(db_datafusion_register_csv(session, `words`, `big.csv`));
rows:a:h<s,s> = danger(db_datafusion_query(session, `SELECT word FROM words WHERE type = 'noun'`));
```

Do not reach for DataFusion to read a small file: it is behind a feature gate
because it is a whole query engine, and its typed columnar batches have to be
flattened back into row hashmaps, which costs more than the plain reader.

`CSV_Options` fields: `delimiter`, `quote`, `escape` (single characters, empty
means unset), `double_quote`, `has_headers`, `flexible`, `ignore_errors`
(booleans), `comment`, `eol_char`, `trim` (`CSV_Trim::None`, `CSV_Trim::Headers`, `CSV_Trim::Fields` or `CSV_Trim::All`), `skip_rows` and `n_rows` (0 means no limit), and `null_values` (texts
read as empty, e.g. `NA`).

### Time Operations (`time_*`)

A moment is a Unix timestamp - whole seconds since the start of 1970 - because
one number is the only representation two machines never disagree about. Every
calendar function below works in UTC: store UTC, convert once at the edge where
a person reads it, and daylight saving never costs you an hour.

- `time_sleep(seconds:f):v` - Sleep for specified seconds
- `time_now():i` - Current timestamp in seconds
- `time_now_millis():i` - Current timestamp in milliseconds
- `time_format(timestamp:i, format:TIME_Format):s!e` - Write a timestamp out in a standard spelling
- `time_parse(time_str:s, format:TIME_Format):i!e` - Read a timestamp back out of text
- `time_format_custom(timestamp:i, layout:s):s!e` - Write a timestamp in a layout of your own, in strftime notation (`%Y-%m-%d`, `%H:%M`, `%A %d %B %Y`)
- `time_parse_custom(time_str:s, layout:s):i!e` - Read a timestamp from text in that same notation
- `time_from_parts(year:i, month:i, day:i, hour:i, minute:i, second:i):i!e` - Build a moment from a UTC date; a day not on the calendar is an error
- `time_add_seconds(timestamp:i, seconds:i):i` - Shift by seconds (negative to subtract)
- `time_add_minutes(timestamp:i, minutes:i):i` - Shift by minutes
- `time_add_hours(timestamp:i, hours:i):i` - Shift by hours
- `time_add_days(timestamp:i, days:i):i` - Shift by days
- `time_add_months(timestamp:i, months:i):i!e` - Shift by months, keeping the day of the month where it can
- `time_start_of_day(timestamp:i):i!e` - Midnight UTC at the start of that day
- `time_diff(t1:i, t2:i):i` - Absolute difference between two timestamps
- `time_format_duration(seconds:i):s` - Write a length of time as a person says it: `2d 3h`, `1h 5m`, `45s`
- `time_year(timestamp:i):i!e`, `time_month`, `time_day`, `time_hour`, `time_minute`, `time_second` - The parts of a UTC date
- `time_weekday(timestamp:i):s!e` - The day of the week written out, `Monday` to `Sunday`
- `time_day_of_year(timestamp:i):i!e` - Which day of the year, from 1 to 366

**TIME_Format enum values:**
- `Unix` - Unix timestamp in seconds: `1234567890`
- `UnixMillis` - Unix timestamp in milliseconds: `1234567890000`
- `ISO8601` - `2009-02-13T23:31:30Z`
- `RFC3339` - `2009-02-13T23:31:30+00:00`
- `RFC2822` - `Fri, 13 Feb 2009 23:31:30 +0000`

Cron schedules are arithmetic here rather than a scheduler: a program asks when
the next run is, sleeps until then with `time_sleep`, does the work, and asks
again, so the loop and its error handling stay in the program.
- `time_cron_valid(expression:s):b` - Whether it is a five-field expression this understands
- `time_cron_matches(expression:s, timestamp:i):b!e` - Whether it matches a moment, to the minute
- `time_cron_next(expression:s, after_timestamp:i):i!e` - The next moment it matches

### Cryptography Operations (`crypto_*`)
- `crypto_hash_password(password:s):s!e` - Store a password. Argon2id with a fresh random salt
- `crypto_verify_password(password:s, stored_hash:s):b` - Check a password against a stored hash
- `crypto_hash_sha256(s:s):s` / `crypto_hash_sha512(s:s):s` - Digests, as hex
- `crypto_hash_md5(s:s):s` - MD5, for checksums, not security
- `crypto_hmac_sha256(key:s, message:s):s` - HMAC-SHA256 under a secret key, as hex
- `crypto_uuid_v4():s` - A random UUID
- `crypto_uuid_v7():s` - A UUID with the time it was made in the leading bits, so sorting the ids sorts them by age. The one to use for a database key
- `crypto_random_hex(bytes:i):s!e` - Operating-system random bytes as hex, for session ids, nonces and anything an attacker must not guess
- `crypto_secure_equal(left:s, right:s):b` - Compare two secrets in time that does not reveal how much of them matched

Three rules this library exists to enforce:

**Never store a password with `crypto_hash_sha256`.** A graphics card computes
SHA-256 billions of times a second, so a stolen table of digests is a stolen
table of passwords. `crypto_hash_password` is deliberately slow and
memory-hungry, which is what makes guessing at scale cost real money. Its
answer carries its own salt, so the same password hashed twice gives two
different strings - store the whole thing and hand it back unchanged.

**`math_random` is not for secrets.** It is a fast generator for simulations
and shuffles, and its output can be predicted from earlier output. Use
`crypto_random_hex`.

**Compare secrets with `crypto_secure_equal`, never with `==`.** A normal
comparison stops at the first differing byte, and how long it took says how
much of the value was right.

Identifiers for a URL, alongside the UUIDs:
- `crypto_ulid():s!e` - 26 typable characters, sorted by when it was made, no hyphens
- `crypto_random_id(length:i):s!e` - Letters, digits, hyphen and underscore, so it needs no escaping

### Configuration (`toml_*`)
- `toml_serialize(value):s!e` - Write a struct, hashmap or array out as TOML
- `toml_deserialize(text:s):T!e` - Read TOML into a value; the type on the left of the assignment says what to read it as

The same shape as `json_*`. TOML rather than JSON for anything a person edits:
it has comments, and nobody has to count closing braces.

`env_load_dotenv(path:s):h<s,s>!e` reads a `.env` file, sets every variable in
it, and returns what it read. Variables the process was already started with
always win over what the file says, which is what makes such a file safe to
leave lying around in production.

`yaml_serialize(value):s!e` and `yaml_deserialize(text:s):T!e` are the same two
functions again, for the documents a program does not get to choose the format
of - a CI file, a manifest, a compose file.

### Logging (`log_*`)

Logging goes to standard error, so whatever the program was actually asked for
stays pipeable on standard output.

- `log_debug(message:s):v` / `log_info` / `log_warn` / `log_error` - One line at a level
- `log_with_fields(level:LOG_Level, message:s, fields:h<s,s>):v` - A message with named values beside it, which is what turns a log line from prose into something searchable
- `log_set_level(level:LOG_Level):v` - Hide everything below this level. `Info` by default
- `log_set_json(enabled:b):v` - Switch to one JSON object per line, which is what a log collector wants

**LOG_Level values:** `Debug`, `Info`, `Warn`, `Error`.

### Terminals (`term_*`)

Everything that adds colour returns a string rather than printing it, so a
coloured value can be built up, joined, put in a table cell and printed once.
Colour written to a file is noise in the file - ask `term_is_tty` first, or
strip it back out with `term_strip_styles`.

- `term_paint(text:s, color:TERM_Color):s` / `term_background(text:s, color:TERM_Color):s`
- `term_bold(text:s):s` / `term_dim` / `term_italic` / `term_underline` / `term_inverse`
- `term_strip_styles(text:s):s` - Remove every escape sequence
- `term_display_width(text:s):i` - How wide the text is once printed, counting what a person sees
- `term_is_tty():b` - Whether output is a terminal rather than a file or a pipe
- `term_width():i` / `term_height():i` - The terminal's size, or 80 by 24 when there is none to ask
- `term_table(headers:a:s, rows:a:a:s):s!e` - A table with aligned columns, measured by visible width so a coloured cell does not throw the alignment out
- `term_progress_bar(share:f, width:i):s!e` - A bar filled to a share from 0.0 to 1.0
- `term_hyperlink(text:s, url:s):s` - A clickable link where the terminal supports it, plain text where it does not

**TERM_Color values:** `Black`, `Red`, `Green`, `Yellow`, `Blue`, `Magenta`,
`Cyan`, `White`, and a `Bright` form of each.

### Terminal Interfaces (`tui_*`)

Full-screen terminal programs, described rather than drawn. A program supplies
two ordinary functions and `tui_run` owns everything else:

```nail
struct App { count:i, finished:b }

f view(state:App):TUI_Screen {
    r TUI_Screen {
        title = `Counter`,
        lines = [tui_line(string_from(state.count))],
        status = `up and down to change, q to quit`,
        quit = state.finished
    };
}

f update(state:App, event:TUI_Event):App {
    if {
        event.key == `q` -> { r App { count = state.count, finished = true }; },
        event.key == `Up` -> { r App { count = state.count + 1, finished = false }; },
        else -> { r state; }
    }
}

final_state:App = danger(tui_run(App { count = 0, finished = false }));
```

- `tui_run(initial:T):T!e` - Run until `view` reports `quit`, and return the state it finished with
- `tui_line(text:s):TUI_Line` - A plain line in the terminal's own colour
- `tui_styled(text:s, color:TERM_Color, bold:b, selected:b):TUI_Line` - A line with its appearance said explicitly

`tui_run` is the only stdlib function that calls back into the program twice,
and the only one whose callbacks are written in terms of a type it cannot name:
`T` is bound from its own argument, so `App` is whatever struct the program
uses. The names `view` and `update` are fixed, the way `handle_request` is for
`http_server`.

**Why this is not how other terminal libraries work.** Every one of them is
retained-mode: build widget objects, hold onto them, mutate them as things
change. That cannot be the default in a language where nothing is mutable - and
it turns out not to be needed. `view` is a pure function of the state, so the
same state always draws the same screen, and a whole interface can be tested
without a terminal to test it on.

**What `tui_run` takes off your hands** is the part hand-written terminal
programs get wrong: raw mode, the alternate screen, polling for input without
blocking the async runtime, drawing each frame in one write so it does not
flicker, and putting the terminal back exactly as it was - on a normal exit, on
an early return, and while a panic unwinds. A Nail program cannot leave someone's
shell in raw mode with no echo, because it never turns raw mode on itself.

**Resizing needs no handling**: every event carries `width` and `height`, so
the next frame simply knows more. **Quitting is a field on the screen** rather
than a special return, so the state decides when it is over.

**TUI_Screen fields:** `title`, `lines` (`a:TUI_Line`), `status`, `quit`.
Lines past the bottom of the terminal are not drawn - that is the program's cue
to scroll by choosing different lines, a decision `view` is better placed to
make than the library.

**TUI_Line fields:** `text`, `color` (a `TERM_Color`), `bold`, `selected` (drawn
with foreground and background swapped).

**TUI_Event fields:** `key`, `tick`, `width`, `height`. `key` is a single
character for an ordinary key, or one of `Enter`, `Esc`, `Up`, `Down`, `Left`,
`Right`, `Backspace`, `Tab`, `Delete`, `Home`, `End`, `PageUp`, `PageDown`,
`F1`-`F12`, or a combination like `Ctrl+c`. `tick` is true for the event
delivered when nothing was pressed, which is what drives a clock or a spinner.

### Testing (`test_*`)

Each assertion either does nothing or stops the program with a message naming
what was expected and what turned up. A failing assertion exits with a non-zero
status, which is all any test runner needs to tell pass from fail. The message
argument is required on purpose: "assertion failed" tells you nothing at three
in the morning.

- `test_assert(condition:b, message:s):v` / `test_assert_false(condition:b, message:s):v`
- `test_assert_equal_int(actual:i, expected:i, message:s):v`
- `test_assert_equal_string(actual:s, expected:s, message:s):v`
- `test_assert_equal_bool(actual:b, expected:b, message:s):v`
- `test_assert_equal_float(actual:f, expected:f, tolerance:f, message:s):v` - Floats are never compared exactly; 0.1 plus 0.2 is not 0.3 on any machine with hardware floats
- `test_assert_contains(haystack:s, needle:s, message:s):v`
- `test_assert_equal_array(actual:a:T, expected:a:T, message:s):v` - Names the first position that differs

### Command-Line Arguments (`args_*`)

Two ways in, for two sizes of program.

A script that wants one setting reads it directly, with no description to keep
in step:

- `args_get(index:i):s!e` - The argument at a position
- `args_count():i` - How many there are
- `args_flag(name:s):b` - Whether a flag like `--verbose` was passed
- `args_value(name:s):s!e` - The value of `--name value` or `--name=value`
- `args_value_or(name:s, fallback:s):s` - The same, with a default instead of an error
- `args_value_int(name:s):i!e` / `args_value_float(name:s):f!e` - Read as a number
- `args_wants_help():b` - Whether `--help` or `-h` was passed. Check this first

A real command-line tool describes what it accepts once, as an array of
`ARGS_Option`, and hands that description to both functions below:

- `args_parse(options:a:ARGS_Option):ARGS_Parsed!e` - Read and check the whole command line in one call
- `args_help_text(program:s, description:s, options:a:ARGS_Option):s` - The `--help` page, built from that same description

`args_parse` errors on an unknown flag, an option that takes a value and was
not given one, a value handed to an option that takes none, and a missing
required option - all in one place, so a person fixes one thing and runs it
again.

The description is not ceremony. Given `mytool --output report.txt deploy`,
nothing can tell whether `report.txt` is the value of `--output` or the
subcommand without knowing that `--output` takes a value. Every argument parser
needs this; most hide it inside a builder. Here it is plain data, which is also
why the help page cannot describe a flag the program does not accept.

**ARGS_Option fields:** `name`, `short` (a single letter without its dash, or
empty), `description`, `takes_value`, `required`.

**ARGS_Parsed fields:** `command` (the first positional argument, or empty),
`positional` (everything that is neither a flag nor a flag's value, in order -
`command` is its first element, so `array_skip(parsed.positional, 1)` is what
came after), `values` (`h<s,s>`, keyed by long name however the flag was
written, so `-o x`, `--output x` and `--output=x` all arrive as `output`), and
`flags` (the long names of the present options that take no value).

### URL Operations (`url_*`)
- `url_encode(text:s):s` - Percent-encode a string for use in a URL
- `url_decode(text:s):s!e` - Decode a percent-encoded string
- `url_parse_query(query:s):h<s,s>` - Parse a query string or form body into a hashmap
- `url_build_query(params:h<s,s>):s` - Build a percent-encoded query string from a hashmap

### Base64 Operations (`base64_*`)
- `base64_encode(text:s):s` - Encode as base64, standard alphabet with padding
- `base64_decode(data:s):s!e` - Decode standard base64 back to text
- `base64_encode_url(text:s):s` - Encode as URL-safe base64 without padding (JWTs, URL parameters)
- `base64_decode_url(data:s):s!e` - Decode URL-safe base64, padded or not

### Hex Operations (`hex_*`)
- `hex_encode(text:s):s` - Encode text as hex, two lower-case characters per byte
- `hex_decode(data:s):s!e` - Decode hex back to text

### String Operations (`string_*`)
The string library is large; the one that matters for building pages:
- `string_escape_html(text:s):s` - Escape `&`, `<`, `>`, `"` and `'` so text a
  visitor supplied can be put in a page without becoming markup

Alongside the usual splitting, trimming and case conversions, the text
utilities every program ends up writing for itself:
- `string_slugify(text:s):s` - A title as a URL part: `Hello, World!` becomes `hello-world`
- `string_truncate(text:s, max_length:i, ellipsis:s):s` - Cut to a length, counting the ellipsis
- `string_word_wrap(text:s, width:i):s` - Break into lines, splitting between words
- `string_levenshtein(first:s, second:s):i` / `string_similarity(first:s, second:s):f` - How far apart two strings are
- `string_closest(text:s, candidates:a:s):s!e` - Answer a mistyped command with a suggestion
- `string_dedent(text:s):s` / `string_indent(text:s, prefix:s):s` - Reshape a block
- `string_normalize_whitespace(text:s):s` - Collapse every run of whitespace to one space
- `string_unescape_html(text:s):s` - The reverse of `string_escape_html`
- `string_mask(text:s, visible_tail:i, mask_character:s):s` - Show which secret is in use without printing it

### Formatting Numbers (`format_*`)
Nail has no format strings, so the roundings and comma insertions every program
writes for itself live here instead. Everything returns text for a reader, never
text to parse again:
- `format_decimals(value:f, places:i):s` - Fixed decimals, keeping trailing zeros
- `format_thousands(value:i):s` / `format_thousands_float(value:f, places:i):s` - Grouped digits
- `format_currency(amount:f, symbol:s):s` - A symbol and two decimals
- `format_percent(fraction:f, places:i):s` - `0.125` at one place is `12.5%`
- `format_bytes(count:i):s` - `1.5 KB`, in steps of 1024
- `format_compact(value:i):s` - `1.2k`, `3.4M`, in steps of 1000
- `format_ordinal(number:i):s` - `1st`, `2nd`, `13th`
- `format_plural(count:i, singular:s, plural:s):s` - Both forms given, since English guesses wrongly
- `format_list(items:a:s, conjunction:s):s` - `a, b and c`

### Money (`money_*`)
An amount is a whole number of cents, because a float cannot hold `0.10`.
Adding and subtracting need nothing from this module - cents are integers - so
these are the operations plain arithmetic cannot do safely:
- `money_from_dollars(dollars:f):i` / `money_to_dollars(cents:i):f` - The boundary where a float stops being involved
- `money_parse(text:s):i!e` - Read what a person typed; more precision than a cent is an error, not a rounding
- `money_format(cents:i, symbol:s):s` - Write it the way a receipt does
- `money_percent_of(cents:i, rate:f):i` - Tax or a discount, to the nearest cent
- `money_times(cents:i, count:i):i!e` - A line item
- `money_split(cents:i, ways:i):a:i!e` - Split evenly, handing out the leftover cents rather than dropping them
- `money_allocate(cents:i, weights:a:i):a:i!e` - Split in proportion, with every cent accounted for

### Templates (`template_*`)
Values filled into text, escaped on the way in, so no call site has to remember
to escape anything:
- `template_render(template:s, values:h<s,s>):s!e` - Fill in one set of values
- `template_render_rows(template:s, rows:a:h<s,s>):s!e` - Render once per row, for a table body
- `template_names_used(template:s):a:s!e` - The names a template asks for

The syntax is `{{name}}` for an escaped value, `{{{name}}}` for raw markup,
`{{#if name}}...{{else}}...{{/if}}`, `{{#unless name}}...{{/unless}}`, and
`{{! a comment }}`. There is no loop tag: a loop belongs in the program, where
Nail's own `map` already does it better. A name the values do not hold is an
error rather than a blank, because a blank in a page is a bug every time.

### Charts (`chart_*`)
Each returns a whole SVG document, which is a string like any other - written to
a file or put straight into a page:
- `chart_line(width:f, height:f, values:a:f, labels:a:s, colour:s, title:s):s!e`
- `chart_bar(width:f, height:f, values:a:f, labels:a:s, colour:s, title:s):s!e`
- `chart_scatter(width:f, height:f, x_values:a:f, y_values:a:f, colour:s, title:s):s!e`
- `chart_sparkline(width:f, height:f, values:a:f, colour:s):s!e` - No axis or labels, for beside a number

Anything more particular - a second axis, a legend, stacked bars - is built from
`draw_*` directly.

### Validation (`validate_*`)
The questions every program asks of input from outside itself. Each answers with
a boolean, because a failed check is an expected answer rather than an error:
- `validate_email(text:s):b`, `validate_url(text:s):b`, `validate_hostname(text:s):b`
- `validate_uuid(text:s):b`, `validate_ipv4(text:s):b`, `validate_ipv6(text:s):b`, `validate_port(number:i):b`
- `validate_credit_card(text:s):b` - The Luhn check, which catches a mistyped digit
- `validate_hex_color(text:s):b`, `validate_slug(text:s):b`, `validate_json(text:s):b`
- `validate_length_between(text:s, minimum:i, maximum:i):b` - Counted in characters
- `validate_password_strength(text:s):i` - 0 to 4, and 0 for the passwords everybody tries first

### Signed Tokens (`jwt_*`)
How a server recognises a visitor it has seen before without trusting what the
browser sent back. Claims go in and come out as JSON text, which
`json_serialize` and `json_deserialize` turn into a struct:
- `jwt_sign(claims_json:s, secret:s, expires_in_seconds:i):s!e` - Zero seconds means no expiry
- `jwt_verify(token:s, secret:s):s!e` - The claims, if the signature holds and the expiry has not passed
- `jwt_is_expired(token:s):b!e` - For deciding whether to refresh a token, not whether to trust one
- `jwt_read_unverified(token:s):s!e` - The claims without checking anything; nothing it returns has been verified

Only HS256 is accepted. A token arriving with any other algorithm is refused
rather than trusted, which is how JWT libraries come to accept `alg: none`.

### Version Numbers (`semver_*`)
Comparing versions as numbers rather than as text, since `1.10.0` is newer than
`1.9.0` but sorts before it:
- `semver_valid(version:s):b`, `semver_compare(first:s, second:s):i!e`
- `semver_is_newer(first:s, second:s):b!e`, `semver_is_older(first:s, second:s):b!e`
- `semver_major/minor/patch(version:s):i!e`, `semver_prerelease(version:s):s!e`
- `semver_bump_major/minor/patch(version:s):s!e`
- `semver_satisfies(version:s, requirement:s):b!e` - Exact, `>=1.2.0`, `^1.2.3`, `~1.2.3`, `*`, or several separated by commas
- `semver_sort(versions:a:s):a:s!e`, `semver_newest(versions:a:s):s!e`

### Pictures (`image_*`)
File to file, so nothing binary crosses into the program:
- `image_resize(from_path:s, to_path:s, width:i, height:i):v!e` - exactly that size, stretching if the shape differs
- `image_resize_within(from_path:s, to_path:s, width:i, height:i):v!e` - fits inside the box, keeps the proportions, never enlarges
- `image_convert(from_path:s, to_path:s):v!e` - the written extension decides the format
- `image_width(path:s):i!e` / `image_height(path:s):i!e`
- `image_format(path:s):s!e` - what the file really is, read from its bytes rather than its name

Nothing works on pixels one at a time: that needs the picture in memory as
values, and a Nail program has no use for four million of them.

### Files That Are Not Text
The three functions that handle a binary file without reading it into the program:
- `crypto_hash_file_sha256(path:s):s!e` - the checksum, read in blocks so the file never has to fit in memory
- `fs_read_base64(path:s):s!e` - a small file as base64, for a `data:` URI or a JSON field
- `fs_write_base64(path:s, data:s):v!e` - and back out again; text that is not base64 is an error

Alongside `HTTP_Request.body_path` for uploads and `archive_*` for zip and tar,
that is the whole binary story, and none of it needs a bytes type.

### Media Types (`mime_*`)
- `mime_for_path(path:s):s` - What to tell a browser a file is; anything unknown is `application/octet-stream`
- `mime_is_text(media_type:s):b` - Whether it is text a program could read as a string
- `mime_extension_for(media_type:s):s!e` - For naming a file that arrived with a type but no name

### Archives (`archive_*`)
Path to path throughout, since an archive worth making does not fit in memory:
- `archive_zip_create(zip_path:s, directory:s):v!e`, `archive_zip_extract(zip_path:s, directory:s):v!e`, `archive_zip_list(zip_path:s):a:s!e`
- `archive_targz_create(archive_path:s, directory:s):v!e`, `archive_targz_extract(archive_path:s, directory:s):v!e`, `archive_targz_list(archive_path:s):a:s!e`

An entry naming a path outside the directory being extracted into stops the
extraction rather than being written, and links and devices are skipped - a
downloaded archive is data from somewhere else.

### Networks (`net_*`)
The network below HTTP, for checking a port before deploying to it and speaking
the line-based protocols that predate HTTP. Every call takes a deadline in
milliseconds, because a network operation without one is how a program hangs
forever with nothing in the log:
- `net_tcp_request(host:s, port:i, text:s, timeout_milliseconds:i):s!e`
- `net_tcp_is_open(host:s, port:i, timeout_milliseconds:i):b!e` - A refused connection is false, not an error
- `net_udp_request(host:s, port:i, text:s, timeout_milliseconds:i):s!e`
- `net_dns_lookup(hostname:s):a:s!e`

### Reading HTML (`html_*`)
Reading HTML somebody else wrote - a fetched page, a feed, an export. Elements
are found by CSS selector, and real HTML is recovered from rather than refused:
- `html_text(html:s):s`, `html_title(html:s):s!e`, `html_meta(html:s, meta_name:s):s!e`
- `html_select_text(html:s, selector:s):a:s!e`, `html_select_html(html:s, selector:s):a:s!e`
- `html_select_attribute(html:s, selector:s, attribute:s):a:s!e`, `html_count(html:s, selector:s):i!e`
- `html_links(html:s):a:s!e`, `html_images(html:s):a:s!e`

Nothing here writes HTML: that is what `template_render` and `markdown_to_html`
are for.

### Live Updates (SSE and Websockets)
`http_server_realtime(port, config, live_path)` is `http_server` with a live
endpoint beside the ordinary routes. A GET to `live_path` is a server-sent-event
stream - what htmx's sse extension and the browser's EventSource consume - and a
websocket upgrade on the same path joins the same channel; `?channel=name` picks
the channel either way.

- `http_live_send(channel:s, message:s):i` - broadcast to every subscriber on the
  channel, SSE and websocket alike; returns how many heard it, and nobody is 0
- `http_live_count(channel:s):i` - subscribers right now

Each websocket text frame is answered by the program's `handle_message(message:s,
state:h<s,s>):s` - the return goes back to that one client, and the empty string
means no reply. Broadcasts stay in `http_live_send`, called from wherever the
program likes: a handler, a `spawn` loop, a watcher.

### XML (`xml_*`) and Feeds (`feed_*`)
`xml_serialize(value, root_name:s):s!e` and `xml_deserialize(text:s):T!e` are the
same two functions as TOML and YAML, for the systems that still want XML.
`feed_parse(text:s):FEED_Feed!e` reads RSS or Atom - whichever it is - into one
shape: title, link, description, and entries with id, title, link, summary and a
published timestamp. What a feed omits is empty rather than missing.

### Watching Files (`fs_watch_*`)
- `fs_watch_start(path:s):FS_Watcher!e` - directories are watched all the way down
- `fs_watch_next(watcher, timeout_milliseconds:i):a:s!e` - the paths that changed,
  or an empty array when the time passed quietly; changes pile up between calls
- `fs_watch_stop(watcher):v!e`

### PDFs (`pdf_*`)
- `pdf_text(path:s):s!e` - the text of a PDF; a scanned one is photographs and gives nothing
- `pdf_from_text(path:s, title:s, body:s):v!e` - a paginated A4 report of plain text

### Spreadsheets (`xlsx_*`)
The same shape as CSV on purpose - rows keyed by the header row, every cell text:
- `xlsx_sheets(path:s):a:s!e`, `xlsx_read(path:s, sheet:s):a:h<s,s>!e`
- `xlsx_write(path:s, sheet:s, headers:a:s, rows:a:h<s,s>):v!e`

### Sending Mail (`email_*`)
- `email_default_server():EMAIL_Server` - Port 587 with TLS, which is what almost every provider wants
- `email_send(server:EMAIL_Server, to:s, subject:s, body:s):v!e`
- `email_send_html(server:EMAIL_Server, to:s, subject:s, html:s):v!e`

Success means the server accepted the message, not that it was delivered. Only
sending is here; reading mail is not something a Nail program should be doing.

### Postgres (`db_postgres_*`)
The same shape as the SQLite module, for when the data lives on a server rather
than in a file. Placeholders are Postgres's own `$1`, `$2`:
- `db_postgres_connect(url:s):DB_Postgres!e`, `db_postgres_close(db:DB_Postgres):v!e`
- `db_postgres_execute(db:DB_Postgres, sql:s, params:a:s):DB_PostgresResult!e`
- `db_postgres_execute_batch(db:DB_Postgres, statements:s):v!e`
- `db_postgres_query(db:DB_Postgres, sql:s, params:a:s):a:T!e`
- `db_postgres_query_single(db:DB_Postgres, sql:s, params:a:s):T!e`

The connection is not encrypted, so it belongs on localhost or a private
network; across the internet, tunnel it.

### Error Handling
- `safe(result:T!e, handler:f(e:e):T):T` - Handle error with function
- `danger(result:T!e):T` - Unwrap or panic (use carefully)
- `expect(result:T!e):T` - Unwrap or panic (for impossible errors)
- `e(message:s):T!e` - Create an error result, inside a function whose return type is `T!e`

There is no wrapper for the success case: a function returning `T!e` returns the
value itself with `r value;`, and only the error case is written out with `e(...)`.

## Memory Management and Execution

- Automatic memory management.
- No garbage collection except for some reference counted objects that facilitate concurrency/async/parallelism - if you consider that garbage collection.
- Nail code is transpiled to Rust and then compiled to native executables.

## Development Environment

- Mandatory use of Nail's IDE on Linux.
- Opinionated code formatting enforced on save.

The EBNF specification in this repo provides a more formal and comprehensive overview of the Nail programming language.

#  Nail Language Grammar in EBNF


## Type System and Declarations


### EBNF
```ebnf
// Types
type := base_type ["!" "e"]
base_type := primitive_type | struct_type | enum_type | array_type | hashmap_type | void_type | any_of_type
result_type := base_type "!" error_type
primitive_type := "i" | "f" | "s" | "b"
struct_type := "struct" | pascal_identifier
struct_field_type = primitive_type | enum_type | array_type | hashmap_type
enum_type := pascal_identifier
array_type := "a" ":" base_type
hashmap_type := "h" "<" concrete_type "," concrete_type ">"
concrete_type := primitive_type | struct_type | enum_type | array_type | hashmap_type
void_type := "v"
any_of_type :="|" base_type ["|" base_type ["|" base_type]] "|"
error_type := "e"

// Note: hashmap_type uses concrete_type (excludes void and error types)
// as both keys and values must be concrete, storable data types


// Declarations
struct_decl := "struct" pascal_identifier "{" struct_field "," struct_field "}"
struct_field := snake_identifier ":" struct_field_type
enum_decl := "enum" pascal_identifier "{" enum_variant "," enum_variant "}"
enum_variant := pascal_identifier
const_decl := snake_identifier ":" type "=" expression ";"
```

### Lexical Elements

```ebnf
// Lexical Elements
pascal_identifier := uppercase_letter { letter | digit | "_" }
snake_identifier := lowercase_letter { lowercase_letter | digit | "_" }

uppercase_letter := "A" | "B" | "C" | ... | "Z"
lowercase_letter := "a" | "b" | "c" | ... | "z"
letter := "A" | "B" | "C" | ... | "Z" | "a" | "b" | "c" | ... | "z"
digit := "0" | "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9"
```



## Expressions and declarations

### EBNF

```js
expression :=
    literal                   // A constant value (e.g., numbers, strings)
    function_call             // Invoking a function (e.g., `foo(a, b)`)
    binary_expression         // Binary operations (e.g., `a + b`)
    unary_expression          // Unary operations (e.g., `-a`, `!b`)
    if_expression             // Conditional expression (e.g., `if { condition -> { block } }`)
    match_expression          // Pattern matching (e.g., `match x { ... }`)
    block                     // A sequence of statements inside `{}` (e.g., `{ stmt1; stmt2 }`)
    for_loop                  // For loop construct (e.g., `for i in array_range(0, 10) { ... }`)
    while_loop                // While loop construct (e.g., `while condition { ... }`)
    loop                      // Infinite loop construct (e.g., `loop { ... }`)
    break                     // Breaks out of a loop (e.g., `break`)
    continue                  // Skips to the next loop iteration (e.g., `continue`)
    return                    // Returns a value from a function (e.g., `r x`)
    assignment                // Assigning a value (e.g., `x = y`)
    error_handling            // All errors must be handled explicitly

declaration :=
    const_decl                  // Declaring a constant (e.g., `pi = 3.14;`)
    struct_decl               // Declaring a struct (e.g., `struct Point { ... }`)
    enum_decl                 // Declaring an enum (e.g., `enum Days { ... }`)
```


## Struct

### EBNF

```js
struct_decl := "struct" pascal_identifier "{" struct_field "," struct_field "}"
struct_field := snake_identifier ":" struct_field_type
struct_field_type = primitive_type | enum_type | array_type
```


### Nail:

```js
struct Point {
    x_coord:i,
    y_coord:i
}
```

### Transpilation
    
```js
struct Point {
    x_coord:i32,
    y_coord:i32,
}
```



## Enums

### EBNF

```js
enum_decl := "enum" identifier "{" enum_variant {"," enum_variant} "}"
enum_variant := identifier
identifier := letter {letter | digit | "_"}
letter := "A" | "B" | "C" | ... | "Z" | "a" | "b" | "c" | ... | "z"
digit := "0" | "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9"
```
#### notes 
- `enum_decl`: Defines an enum declaration, starting with the `enum` keyword, followed by an identifier (the enum's name), and a list of variants enclosed in curly braces.
- `enum_variant`: Each variant is simply an identifier.
- `identifier`: Follows the same rules as in struct declarations.

### Usage in Nail

```js
enum TrafficLight {
    Red,
    Yellow,
    Green
}

enum DaysOfWeek {
    Monday,
    Tuesday,
    Wednesday,
    Thursday,
    Friday,
    Saturday,
    Sunday
}

// Usage
current_light:TrafficLight = TrafficLight::Red;
today:DaysOfWeek = DaysOfWeek::Wednesday;

// if statement that must cover all enum cases since it has no else branch
if {
   current_light == TrafficLight::Red -> { print(`Stop!`); },
   current_light == TrafficLight::Yellow -> { print(`Prepare to stop`); },
   current_light == TrafficLight::Green -> { print(`Go!`); }
}

// If you have an else branch, you don't need to cover all cases
if {
   current_light == TrafficLight::Red -> { print(`Stop!`); },
   else -> { print(`It could be yellow or green...`); }
}
```

### Transpiled to Rust

```js
enum TrafficLight {
    Red,
    Yellow,
    Green
}

if current_light == TrafficLight::Red {
    println!(`Stop!`);
} else if current_light == TrafficLight::Yellow {
    println!(`Prepare to stop`);
} else if current_light == TrafficLight::Green {
    println!(`Go!`);
} 
```

### Key Points

- Enums are immutable and cannot be modified after declaration.
- When an enum is in the expression side of an if statement, all possible enum variants must be covered, unless there is an else branch. This allows simple refactoring when you want specifically ensure that all cases are covered.
- Enums in Nail are simple and don't support associated values, aligning with the language's simplicity principle.
- Enum variants are accessed using double colon notation in Nail (e.g., `TrafficLight::Red`), which transpiles to the same double colon notation in Rust.



## If Statements

### EBNF

```ebnf
if_expression :=
    "if" "{" if_branch {"," if_branch} ["else" "->" block] "}"

if_branch :=
    expression "->" block
```

### Notes:

- `if_expression`: Begins with the keyword `if`, followed by a list of branches enclosed in curly braces. Each branch consists of an expression, which is followed by a `->` and a block of code. If none of the conditions are met, the optional `else` branch will execute its block.
- `if_branch`: Each branch consists of an expression followed by a `->` and a block, which represents the code that should be executed if the condition is true.

### Usage in Nail

In Nail, if expressions are used similarly to other languages, but they offer concise syntax that ensures every condition leads to a valid block of code. The branches in an if statement are separated by commas, allowing for clean and readable code.

```js
// Basic if statement in Nail
if {
    today == DaysOfWeek::Monday -> {
        print(`Start of the week.`);
    },
    today == DaysOfWeek::Friday -> {
        print(`End of the workweek!`);
    },
    else -> {
        print(`It's a regular day.`);
    }
}
```

### Explanation:

- The if expression consists of multiple branches. The first condition checks if `today` is equal to `DaysOfWeek::Monday` and executes the corresponding block if true. The second branch checks for `DaysOfWeek::Friday`.
- The `else` branch provides a default case that will execute if none of the other conditions are met.

### Transpiled to Rust

In Rust, the if statement translates similarly, but the syntax is slightly different. Here's how the Nail example transpiles into Rust:

```js
if today == DaysOfWeek::Monday {
    println!("Start of the week.");
} else if today == DaysOfWeek::Friday {
    println!("End of the workweek!");
} else {
    println!("It's a regular day.");
}
```

### Handling All Enum Cases

In Nail, when using an if statement with an enum, you must ensure that all possible cases are handled unless an `else` branch is provided. If all cases are handled explicitly, it guarantees exhaustive matching of enum variants, preventing potential bugs from unhandled cases.

```js
if {
    current_light == TrafficLight::Red -> {
        print(`Stop!`);
    },
    current_light == TrafficLight::Yellow -> {
        print(`Prepare to stop.`);
    },
    current_light == TrafficLight::Green -> {
        print(`Go!`);
    }
}
```

### Transpiled to Rust:

```js
if current_light == TrafficLight::Red {
    println!("Stop!");
} else if current_light == TrafficLight::Yellow {
    println!("Prepare to stop.");
} else if current_light == TrafficLight::Green {
    println!("Go!");
}
```

In this example, all enum variants of `TrafficLight` are covered. If an enum variant were left out, an error would occur unless an `else` branch was provided.

### All Branches Must Return the Same Type

One important aspect of if expressions in Nail is that all branches must return the same type. This ensures consistency, especially when the result of the if expression is used in a larger context (e.g., assigned to a constant or returned from a function).

```js
// Example where all branches return the same type (in this case, a string)
message:s = if {
    today == DaysOfWeek::Monday -> { r `Start of the week`; },
    today == DaysOfWeek::Friday -> { r `End of the workweek`; },
    else -> { r `It's a regular day`; }
};

// This will work because all branches return a string.
```

However, if branches return different types, Nail will produce an error:

```js
// Example where branches return different types (this will cause an error)
message:s = if {
    today == DaysOfWeek::Monday -> { r `Start of the week`; },  // String
    today == DaysOfWeek::Friday -> { r 5; },  // Integer - ERROR!
    else -> { r `It's a regular day`; }  // String
};

// This will fail because one branch returns a string and another returns an integer.
```

### Transpiled to Rust:

```js
let message = if today == DaysOfWeek::Monday {
    "Start of the week"
} else if today == DaysOfWeek::Friday {
    "End of the workweek"
} else {
    "It's a regular day"
};
```

In Rust, similar rules apply: each branch must return the same type to maintain type safety.

### Key Points:

- Nail's if expressions ensure concise and readable branching logic.
- Comma-separated branches in if expressions reduce syntax noise.
- Enum-based if expressions must account for all cases unless an `else` branch is provided.
- All branches in an if expression must return the same type (exceptions if a branch of the if panics or similar)
- Nail transpiles directly to equivalent Rust if statements, preserving the logic and structure.


## Const Declarations

In Nail, all values are constants by default. There are no mutable variables.

Const declarations are written as:

```js
pi:f = 3.14159;
max_users:i = 100;
greeting:s = `Hello, World!`;
```

Key points about const declarations:
- They are immutable.
- To change a const value, you must use shadowing (redeclaration with the same name).
- Identifiers use snake_case, same as constant declarations (otherwise changing all the names for minor refactoring would be painful).

Example of shadowing:

```js
user_count:i = 5;
// Later in the code
user_count:i = 6;  // This shadows the previous declaration
```

### Examples

Const with shadowing:

```js
max_attempts:i = 3;
max_attempts:i = 5;  // Shadowing the previous declaration
max_attempts:s = `Three`; // Shadows can even change the type (like Rust)
```

### Key Takeaways

- Const values can be "changed" through shadowing, giving an impression of mutability, which helps ease of use.

## Functions and Closures

### Function Declaration Syntax

In Nail, function declarations are similar to Rust. The basic syntax is:

```js
f function_name(param_name:Type, another_param:Type):Type {
    // Function body
}
```

Example:

```js
f add(num_a:i, num_b:i):i {
   r num_a + num_b;
}
```

### Function Return Types and Void Functions

Functions in Nail can return values of any type, or they can be void functions that return nothing:

```js
// Function with return type
f calculate(x:i, y:i):i {
    r x + y;
}

// Void function (returns void type :v)
f print_message(msg:s):v {
    print(msg);
}

// Result type for error handling
f divide(a:i, b:i):i!e {
    if {
        b == 0 -> { r e(`Division by zero`); },
        else -> { r a / b; }
    }
}
```

**Important Rule**: Void functions cannot be assigned to variables. Since they don't return a value, attempting to capture their "result" is a type error:

```js
// This is INVALID - compile error
result:s = print(`Hello`);  // ERROR: Cannot assign void to variable

// This is valid - just call the function
print(`Hello`);  // OK

// Functions that return values can be assigned
sum:i = calculate(5, 3);  // OK - returns an integer
```

### Function Parameters

In Nail, function parameters must always be named, unless the name of the constant being passed is an exact match to the parameter name. This encourages clear and self-documenting function calls.

```js
f greet(name:s) {
    print(`Hello, ` + name + `!`);
}

user_name:s = `Alice`;
greet(name:user_name);  // Explicitly named parameter
greet(user_name);        // Allowed because constant name matches parameter name
```

### Loop-based Processing

Nail provides collection operations for iteration and data processing. Since variables are immutable, traditional imperative loops are replaced with functional operations:

```js
numbers:a:i = [1, 2, 3, 4, 5];

// Transform each element using map
doubled:a:i = map num in numbers {
    y num * 2;
};

// Filter elements using filter
even_numbers:a:i = filter num in numbers {
    y num % 2 == 0;
};

// Calculate sum using reduce
sum:i = reduce acc num in numbers from 0 {
    y acc + num;
};

// For iteration with side effects, use each
each num in numbers {
    print(`Number: `, num);
}
```


## Parallel Blocks

Nail's parallel blocks allow you to execute multiple operations concurrently, automatically leveraging async/await patterns when transpiled to Rust.

### Syntax

```js
p
    // Each statement runs concurrently
    task1:s = expensive_operation();
    task2:i = fetch_from_api();
    print(`Processing in parallel!`);
    calculation:i = compute_result();
/p
```

### Key Points:

- All statements inside a parallel block execute concurrently
- Variables declared inside can be used after the block completes
- Transpiles to Rust's `tokio::join!` for true parallelism
- No semicolon needed after the closing brace
- Ideal for I/O operations, API calls, or independent computations

### Realistic Examples:

```js
// Example 1: Parallel API calls for a dashboard
p
    user_profile:HTTP_Response = danger(http_request(HTTP_Method::Get, `https://api.example.com/user/123`, headers, ``));
    recent_orders:HTTP_Response = danger(http_request(HTTP_Method::Get, `https://api.example.com/orders?user=123`, headers, ``));
    account_balance:HTTP_Response = danger(http_request(HTTP_Method::Get, `https://api.example.com/balance/123`, headers, ``));
/p

// All data is available after parallel block completes
print(`Profile: `, user_profile.body);
print(`Orders: `, recent_orders);
print(`Balance: `, account_balance);

// Example 2: Parallel file processing
files:a:s = [`data1.txt`, `data2.txt`, `data3.txt`];
p
    content1:s = danger(fs_read_file(`data1.txt`));
    content2:s = danger(fs_read_file(`data2.txt`));
    content3:s = danger(fs_read_file(`data3.txt`));
/p

// Process all content together
all_content:s = array_join([content1, content2, content3], `\n`);
```

## Structs


```js
struct UserInput {
    full_name:s,
    email:s,
    age:i
}

struct UserRecord {
    id:i,
    first_name:s,
    last_name:s,
    email:s,
    age:i
}

f convert_user_input_to_record(input:UserInput, id:i):UserRecord {
   name_parts:a:s = split(input.full_name, ` `);
    r UserRecord {
        id:id,
        first_name: danger(get_index(name_parts, 0)), // This could error but we're just going to danger it.
        last_name: danger(get_index(name_parts, 1)), // Use danger or define a handler function for safe
        email:input.email,
        age:input.age
    }
}

// Usage
input:UserInput = UserInput { full_name = `John Doe`, email = `john@example.com`, age = 30 };
record:UserRecord = convert_user_input_to_record(input, 1);
```

### Struct Serialization and Deserialization

Nail provides built-in functions for serializing structs to JSON and deserializing JSON to structs:

```js
// Serialization - converts structs, enums, arrays to pretty JSON
user:User = User { name = `Bob`, age = 25, email = `bob@example.com` };
json_str:s = danger(json_serialize(user));  // Returns pretty-formatted JSON

// Arrays can also be serialized
users:a:User = [user1, user2, user3];
json_array:s = danger(json_serialize(users));

// If you need compact JSON (no whitespace), use string_minify
compact_json:s = string_minify(json_str);

// Deserialization - converts JSON string back to typed value
// Type is inferred from variable declaration
deserialized_user:User = danger(json_deserialize(json_str));
deserialized_users:a:User = danger(json_deserialize(json_array));
```

Note: These functions return Result types, use `danger()` to unwrap or handle errors appropriately.

### Database Operations

Nail provides built-in SQLite database support with type-safe struct-based queries:

```js
// Define a struct that matches your database table
struct Employee {
    id:i,
    name:s,
    email:s,
    salary:f,
    active:b,
    department:s
}

// Create an in-memory database
db:DB_SQLite = expect(db_sqlite_memory());

// Create table and insert data
expect(db_sqlite_execute(db, `CREATE TABLE employees (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL,
    email TEXT UNIQUE,
    salary REAL,
    active BOOLEAN,
    department TEXT
)`));

// Insert data
expect(db_sqlite_execute(db, `INSERT INTO employees (name, email, salary, active, department) VALUES 
    ('Alice Johnson', 'alice@company.com', 75000.50, 1, 'Engineering'),
    ('Bob Smith', 'bob@company.com', 82000.00, 1, 'Marketing')`));

// Query data - results automatically deserialize into structs
employees:a:Employee = expect(db_sqlite_query(db, `SELECT * FROM employees ORDER BY id`));

// Query single record
alice:Employee = expect(db_sqlite_query_single(db, `SELECT * FROM employees WHERE name = 'Alice Johnson'`));

// Query with filtering
engineers:a:Employee = expect(db_sqlite_query(db, `SELECT * FROM employees WHERE department = 'Engineering'`));

// Print employee information
each employee in employees {
    print(`Employee: `, employee.name, ` (`, employee.department, `) - $`, employee.salary);
}

// Close database connection
expect(db_sqlite_close(db));
```

Key features:
- **Type-safe queries**: Query results automatically deserialize into your defined structs
- **No manual parsing**: Database rows map directly to struct fields
- **Error handling**: All operations return Result types for proper error handling
- **Transaction support**: Use `db_sqlite_execute_batch` for atomic operations
- **Memory and file databases**: Support for both in-memory and persistent databases

These additional features and examples demonstrate how Nail can work with complex data structures and transformations. By providing these utilities and patterns, Nail enables developers to handle various data scenarios while maintaining its core principle of simplicity.

## Error Handling in Nail

Nail implements a unique error handling mechanism that promotes robust and maintainable code by ensuring errors are explicitly handled.

### Error 

Errors are panic'd on immediately whenever an "e" (which represents Rust's err type) type is returned from any function.

### Handling Errors

Nail provides several ways to handle errors:

#### Using `safe`

The `safe` function allows you to handle potential errors with a handler function:

```js
f handle_error(err:e):i {
    print(`An error occurred: ` + err);
    r -1;  // Return a default value or handle the error appropriately
}

result:i = safe(potentially_failing_function(), handle_error);
```

#### Using `danger`

The `danger` function allows you to assert that a function will not fail, and if it does, it will return the error to start it propogating up the stack. The difference between `danger` and `expect` is that `danger` is used when the programmer acknowledges this should and can be made safe, and it should be made safe. This way you can easily find all dangerous parts of a program, and make them safe.

```js
result:i = danger(potentially_failing_function());
```

#### Using `expect`

The `expect` function is identical to danger, but with a different semantic meaning. It is an error so catastrophic, there is no point in not crashing the program if it fails. Used for errors that should never happen in a well-functioning program. For example, you may have a program that displays data from a CSV. Instead of using `safe` to handle the error, which would display no data to the user anyway, you would likely prefer to crash the program so you actually are aware there is a massive problematic error occuring, rather than give users a terrible experience of seeing no data at all, and not trip any monitoring systems. The choice is up to the programmer of when to use which.


### Error Handler Function Types

**Important**: Error handling functions used with `safe()` must accept a parameter of type `:e` (error), not `:s` (string). The type checker enforces this requirement.

```js
// ✓ Correct - error handler accepts :e type
f handle_error(err:e):i {
    print(`Error: `, err);
    r 0;
}

// ✗ Incorrect - will cause type checker error
f bad_handler(err:s):i {
    print(`Error: `, err);
    r 0;
}
```

### Best Practices

- **Use Proper Error Types**: Always declare error handler parameters as `:e` type, not `:s` type.

- **Be Specific**: When adding to error messages, be as specific as possible about what operation failed and why.

- **Provide Context**: Include relevant constant values or state information in error messages to aid debugging.

- **Use Descriptive Error Messages**: Make your error messages clear and actionable to help with debugging and maintenance.

By following these practices and leveraging Nail's error handling features, you can create robust, maintainable code that gracefully handles unexpected situations and provides clear, traceable error information when things go wrong.

## Troubleshooting Common Issues

### Compilation Errors

#### Variable Name Too Short
```
Error: Variable name too short. Use descriptive names.
Found: 'x'
Suggestion: Use descriptive name like 'x_value' or 'x_coordinate'
```
**Solution**: All variable names must be descriptive. Use snake_case with meaningful names:
```js
// Wrong
x:i = 5;

// Correct
count:i = 5;
user_age:i = 25;
```

#### Traditional If Syntax Error
```
Error: Expected BlockOpen, found Identifier
```
**Solution**: Nail only supports match-like if syntax:
```js
// Wrong
if count > 0 {
    print(`Positive`);
}

// Correct
if {
    count > 0 -> { print(`Positive`); },
    else -> { print(`Non-positive`); }
}
```

#### Using Return in Collection Operations
```
Error: Cannot use 'r' (return) in collection operation
```
**Solution**: Use `y` (yield) in collection operations, `r` (return) in functions:
```js
// Wrong
doubled:a:i = map num in numbers {
    r num * 2;  // ERROR
};

// Correct
doubled:a:i = map num in numbers {
    y num * 2;  // Use yield
};
```

#### Void Function Assignment Error
```
Error: Cannot assign void to variable
```
**Solution**: Void functions cannot be assigned to variables:
```js
// Wrong
result:v = print(`Hello`);  // ERROR

// Correct
print(`Hello`);  // Just call the function
```

#### HashMap Type Errors
```
Error: Void type cannot be used as hashmap value
```
**Solution**: HashMap values must be concrete types:
```js
// Wrong
map:h<s,v> = hashmap_new();  // ERROR

// Correct
map:h<s,i> = hashmap_new();  // Use concrete types
```

### Runtime Issues

#### Error Propagation
When a function returns an error type, it must be explicitly handled:
```js
// This will panic if the function fails
result:i = danger(int_from(`abc`));

// Safe handling with error function
f handle_parse_error(err:e):i { r 0; }
result:i = safe(int_from(`123`), handle_parse_error);
```

#### Infinite Loops
Remember that `loop` and `loop index` are infinite by default:
```js
// This will run forever - BAD
loop {
    print(`Forever`);
}

// Always include a break condition
loop index {
    print(string_from(index));
    if {
        index >= 10 -> { break; },
        else -> { /* continue */ }
    }
}
```

### Best Practices

1. **Always handle errors explicitly** - Use `safe()`, `danger()`, or `expect()`
2. **Use descriptive variable names** - Avoid single letters or abbreviations
3. **Remember Nail's syntax** - Match-like if statements, yield in collections
4. **Type your variables** - Always include type annotations
5. **Use collection operations** - Prefer map/filter/reduce over manual loops
6. **Handle concurrency carefully** - Use parallel blocks for I/O, spawn for background tasks
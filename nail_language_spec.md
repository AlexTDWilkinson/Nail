
# Nail Programming Language Overview

- Nail takes inspiration from this blog post: https://grugbrain.dev/
- Nail is spiritually similar to HTMX, except for the obvious difference that it is an entire programming language and paradigm.

## Introduction

Nail is a programming language designed with a focus on simplicity, safety, and productivity. Its primary goal is to eliminate common sources of bugs and reduce cognitive load on developers by enforcing strict rules, a strict environment and by providing a consistent, straightforward syntax.

Nail runs on Linux only. The `nail` launcher installs everything: the IDE, the compiler, and the exact toolchain version each file pins.

Nail programs are transpiled to async, parallelized (when specified) Rust and then compiled to native executables.

Nail programs often exhibit superior performance compared to typical Rust implementations, as Nail easily incorporates asynchronous, concurrent, and parallel paradigms: optimizations that many developers might not take the time to implement in typical Rust programs. However, it's important to note that a meticulously optimized Rust program can likely exceed Nail's performance, given that Nail is ultimately transpiled to Rust.

## Core Design Principles

Nail adheres to the following core principles:

- Simplicity: The language includes only essential features, avoiding complexity.
- Safety: Strong typing and strict rules prevent common programming errors.
- Productivity: Consistent syntax and built-in best practices enhance developer efficiency.
- Explicitness: The language favors explicit declarations over implicit behavior.

## Language Restrictions

To achieve its goals, Nail imposes the following restrictions:

- Limited data types: integer, float, string, boolean, array, hashmap, struct, and enum.
- The simple parallel block keyword transforms into parallelized Rust.
- No package manager or external dependencies (The standard library is updated with every new version of Nail)
- No uninitialized constants (constants must be defined with a value)
- No null references.
- No mutability - variables are immutable, with no exceptions. Rebinding a name is done by shadowing (a fresh declaration in the same scope), and accumulating across iterations is what `reduce` and `scan` are for.
- No classes, inheritance, or traditional OOP constructs.
- No manual memory allocation or management.
- No loops except `forever`, a block that runs until the program ends. Walking a collection is `each`, and building one is `map`, `filter`, `reduce` or `scan`. No for loop, no while loop, no break, no continue.
- No traditional if statements (replaced by a pseudo match/switch expression).
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
- No single letter variable or parameter names (x, y, z and w are whitelisted, so a struct holding coordinates can name its fields honestly)
- No lambda functions or closures
- Explicit collection operation keywords (map, filter, reduce, scan, each, find, all, any) instead of generic functional methods
- Collection operations use 'y' (yield) to produce values, while 'r' (return) exits functions

## Lexical Structure

### 4.1 Keywords

Reserved keywords in Nail:

```
f if else struct enum import import_dangerous
forever in from
map filter reduce scan each find all any
p /p c /c r y
```

Rust's own keywords (`fn`, `let`, `match`, `impl` and the rest) are also
reserved, so an identifier that would collide with the generated Rust is
refused with a message saying so.

### 4.2 Identifiers

Identifiers follow snake_case convention:

```js
my_constant
calculate_total
```

### 4.3 Comments

Single-line comments only, preceded by `//`:

```nail
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
| Other languages | `py`, `python`, `rb`, `ruby`, `rs`, `rust`, `go`, `golang`, `java`, `cs`, `csharp`, `c`, `h`, `cpp`, `cc`, `hpp`, `cxx`, `php`, `swift`, `kt`, `kotlin`, `lua`, `graphql`, `gql` |
| Shaders | `wgsl` |
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
Nail provides range functions for creating sequences to walk:

```nail-fragment
// Range function creates arrays for iteration
numbers:a:i = array_range(1, 5);  // Creates [1, 2, 3, 4] (end not included)

// Walk a count with each
each index in array_range(0, 5) {
    print(string_from(index));  // Prints 0, 1, 2, 3, 4
}

// The index comes with the element, so there is no need to count by hand
each item index in numbers {
    print(index, `: `, item);
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

```nail
// Everything in nail is const.
age:i = 30;
name:s = `Grug`;
is_developer:b = true;
```

### 5.3 Type Checking and Conversion

Strict type checking is enforced:

```nail-refused
count:i = 5;  // Valid
count:i = 6.0;  // Error: Can't assign float to integer
count:f = 6.0;  // Valid
count:f!e = float_from(5);  // Invalid, all result type errors cannot be assigned to a variable. They must be handled explicitly.
count:f = danger(float_from(5));  // Valid, removes the error type.
count:f = expect(float_from(5));  // Valid, removes the error type (same as danger but different semantic meaning).
// Handler function must be defined separately
f handle_float_error(e:s):f { r 0.0; }
count:f = safe(float_from(5), handle_float_error);  // Valid, handles error safely.
```

Both sides of an operator have to be the same type. Nothing converts on its
own, so a whole number and a fraction are never compared or added directly:

```nail-refused
count:i = 5;
ratio:f = 2.5;
bigger:b = count > ratio;
```

Convert one of them first, and handle the conversion's error:

```nail
count:i = 5;
ratio:f = 2.5;
bigger:b = danger(float_from(count)) > ratio;
print(bigger);
```

### 5.4 Composite Types

#### 5.4.1 Arrays

Homogeneous, non-nested collections:

```nail
names:a:s = [`Alice`, `Bob`, `Charlie`];
```

#### 5.4.2 Structs

Custom data types with named fields:

```nail
struct Point {
    x_pos:i,
    y_pos:i
}
```

#### 5.4.3 HashMaps

Key-value collections with type-safe keys and values. Both keys and values must be concrete types (cannot be void or error types):

```nail
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

```nail
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

```nail-fragment
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

```nail
numbers:a:i = [1, 2, 3, 4, 5];

// Basic map - transform each element
doubled:a:i = map num in numbers {
    y num * 2;
};

// Map with index access (no comma between iterators)
indexed_values:a:s = map num index in numbers {
    y array_join([`Index `, string_from(index), `: `, string_from(num)], ``);
};

// Note: To map over characters in a string, first convert to array
// let chars:a:s = string_chars(`hello`);
// uppercase_chars:a:s = map char in chars { ... };
```

#### Filter Operation

Filter selects elements from a collection based on a condition:

```nail-fragment
// Filter even numbers
evens:a:i = filter num in numbers {
    y num % 2 == 0;
};

// Filter with index (no comma between iterators)
first_three:a:i = filter num index in numbers {
    y index < 3;
};
```

#### Reduce Operation

Reduce accumulates values from a collection into a single result:

```nail-fragment
// Sum all numbers
sum:i = reduce acc num in numbers from 0 {
    y acc + num;
};

// Find maximum: the yielded value is the whole if expression, and each
// branch returns from the branch with r
max_val:i = reduce acc num in numbers from danger(array_get(numbers, 0)) {
    y if { num > acc -> { r num; }, else -> { r acc; } };
};

// Build string ('+' adds numbers only, so text is joined with string_concat)
concatenated:s = reduce acc word in [`hello`, ` `, `world`] from `` {
    y string_concat([acc, word]);
};
```

#### Scan Operation

Scan is a reduce that keeps its work: the accumulator's value after every
element, so the result is an array as long as the one it scanned.

```nail-fragment
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

```nail-fragment
// Print each element (statement form, no assignment)
each num in numbers {
    print(array_join([`Number: `, string_from(num)], ``));
}

// With index (no comma between iterators)
each num index in numbers {
    print(array_join([`[`, string_from(index), `]: `, string_from(num)], ``));
}
```

#### Find Operation

Find returns the first element matching a condition:

```nail-fragment
// Find first even number
first_even:i = danger(find num in numbers {
    y num % 2 == 0;
});

// Find with index (no comma between iterators)
third_element:i = danger(find num index in numbers {
    y index == 2;
});
```

#### All/Any Operations

Check if all or any elements match a condition:

```nail-fragment
// Check if all positive
all_positive:b = all num in numbers {
    y num > 0;
};

// Check if any negative (with index access)
has_negative:b = any num index in numbers {
    y num < 0;
};
```

### 6.3 Array Function Operations

Standard library provides array functions for common operations:

```nail-fragment
numbers:a:i = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10];

// Take and skip operations
first_three:a:i = array_take(numbers, 3);  // [1, 2, 3]
skip_three:a:i = array_skip(numbers, 3);   // [4, 5, 6, 7, 8, 9, 10]

// Take/skip while operations with predicates
f less_than_five(num:i):b { r num < 5; }
small_nums:a:i = array_take_while(numbers, less_than_five);  // [1, 2, 3, 4]

// Array utilities
unique_nums:a:i = array_deduplicate([1, 2, 2, 3, 3, 3]);  // [1, 2, 3]
nested:a:a:i = [[1, 2], [3, 4]];
flat_array:a:i = array_flatten(nested);    // [1, 2, 3, 4]

// Finding elements
index:i = danger(array_find(numbers, 5));  // Returns 4 (0-based index)

// Transforming and selecting are the map and filter keywords
doubled:a:i = map num in numbers {
    y num * 2;
};

evens:a:i = filter num in numbers {
    y num % 2 == 0;
};
```

### 6.4 Loops

#### No For Loop

Nail has no `for` loop, and the keyword is refused with an error saying so.
Walking a collection for its side effects is `each`, which also hands over the
index (`each item index in items`), and building a value from a collection is
`map`, `filter`, `reduce`, `scan`, `find`, `all` or `any`. Every one of them
names the collection and the element the same way, so there is one shape to
learn. A count is a collection too: `each index in array_range(0, 5)`.

#### No While Loops

Nail has no `while` loop, and the keyword is refused with an error saying so.
Every job a while loop does has a better home: iterating a collection is
`each`, building a value up as you go is `reduce` or `scan`, repeating until
something outside changes is a function that calls itself, and running
until the program ends is `forever`. A while loop would also need mutable state to ever
terminate, and Nail has no mutation.

#### No Break or Continue

`break` and `continue` are refused the same way. A loop has no way to skip an
element or stop early, and does not need one: `filter` picks the elements
before the loop sees them, `array_take_while` cuts a collection off where a
condition stops holding, and inside a function `r` leaves a forever block with
the answer. A loop that has to stop is a collection being walked or a function
being left, never a loop with an exit bolted on.

#### Forever

`forever` is a block that runs until the program ends. It is for work that is
meant to never stop, like a server accepting connections or a heartbeat in the
background, and a program whose top level reaches a `forever` stays in it for
as long as it runs:

```nail-fragment
// A heartbeat that runs for as long as the program does, in a c block
// beside whatever else lives that long
f heartbeat():v {
    forever {
        print(`still here`);
        time_sleep(60.0);
    }
}
```

Inside a function, `r` leaves the block, and since a forever block never falls
through, a function whose body ends in one needs no return after it:

```nail
f wait_for(path:s):s {
    forever {
        if {
            path_exists(path) -> { r danger(fs_read(path)); },
            else -> { time_sleep(1.0); }
        }
    }
}
print(wait_for(`config.toml`));
```

There is no counter. A block that cares how many times it has run is keeping
state across passes, and state lives in a function's arguments (a function
that calls itself with `attempt + 1`), in the program's own struct (what
`game_run` and `tui_run` hand back to `update` every frame), or in the world.

#### No Background Blocks

Nail has no way to start work and walk away from it. Everything a program
starts, it waits for: `c` runs its statements at once and ends when all of
them have ended, and `p` does the same on threads. A server with a heartbeat
is a `c` block holding two functions that run `forever`, and the program
lives inside that block:

```nail-fragment
c
    heartbeat();
    serve();
/c
```

Work that must outlive a request, like sending the email after the answer has
gone out, is a job written somewhere a worker in that `c` block picks up,
which also means it survives a restart. A block that ran behind the program's
back could not be waited for, could not report an error, and was cut off
wherever it stood when the program ended.

### 6.4 Collection Operation Transpilation

The elementwise operations (map, filter, find, all, any) transpile to rayon
parallel iterators, so they use every core without the program saying so. In an
async context the chain is wrapped in `tokio::task::spawn_blocking` so the
runtime's threads are never blocked, and a body that itself does async work
drives each element with its own `block_on`. Sketched:

```js
// Nail
doubled:a:i = map num in numbers {
    y num * 2;
};

// Transpiles to Rust (sketch, async context)
let doubled = tokio::task::spawn_blocking(move || {
    numbers.into_par_iter().map(|num| num * 2).collect::<Vec<_>>()
}).await;

// Filter operation (block with yield statement)
evens:a:i = filter num in numbers {
    y num % 2 == 0;
};

// Transpiles to the same shape, with rayon's filter
let evens = tokio::task::spawn_blocking(move || {
    numbers.into_par_iter().filter(|num| num % 2 == 0).collect::<Vec<_>>()
}).await;
```

A reduce depends on the value before it, so it stays an ordered loop over the
elements, unless its step is provably associative (a sum, say), in which case
it too is split across cores. A scan's intermediate values are the point, so a
scan always runs in order.

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

forever :=
    "forever" block

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

statement :=
    const_decl | struct_decl | enum_decl | function_decl |
    if_expression | forever |
    parallel_block | concurrent_block |
    return_statement | yield_statement |
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

```nail
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

```nail-fragment
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

```nail-fragment
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

```nail-fragment
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
            denominator:f = 1.0 - math_pow(1.0 + monthly_rate, -payments);
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

```nail-refused
user_input:s!e = io_read_line();
user_input:s = danger(io_read_line());

// OR safely handle the error
f handle_input_error(e:e):s { r `default value`; }
user_input:s!e = io_read_line();
user_input:s = safe(io_read_line(), handle_input_error);

```

## File Inclusion

Nail brings other files in at compile time through the `import()` keyword,
which splices the file's contents in place as if the code was typed there.
Imported code is sandboxed by default: it may only compute, never touch the
world, and the compiler proves it. For your own trusted files there is
`import_dangerous()`, which includes a file the same way but lets it do
anything your own code can do.

The default is the safe one on purpose. A library that only computes needs no
trust and no explanation: you import it, the compiler proves it cannot phone
home. A library that genuinely needs the network or the file system has to ask
you for `import_dangerous`, and its documentation has to say why, before you
have granted it anything.

### Syntax

```nail
import(`downloaded_library.nail`)
import_dangerous(`your_own_helpers.nail`)
```

### Behavior

Both forms include a file the same way:

- The file path is resolved relative to the current file's directory
- The entire contents of the specified file are spliced in at the location of
  the statement, at compile time during lexical analysis
- Circular includes are detected and prevented
- There is no runtime cost, and the generated Rust is byte for byte identical
  between the two forms. They differ only in what the compiler will prove
  about the code

### The sandbox: what import proves

Everything from an imported file may only compute. The compiler guarantees it
cannot phone home, read your disk or environment, spy on global state, or
seize resources like stdout. This is Nail's supply chain answer: downloaded
code is pasted in with `import`, and the guarantee comes from the compiler,
not from the code's author.

```nail
import(`downloaded_library.nail`)

result:s = library_function(`input`);
print(result);
```

#### Rules for imported files

1. **Only declarations at the top level.** An imported file may declare
   functions, structs, enums, and constants. Any other top-level statement is
   a compile error: your program decides when sandboxed code runs, the imported
   file never runs anything by itself.
2. **Sandboxed code may only call computation.** Inside sandboxed functions and
   sandboxed constant initializers, standard library calls are checked against a
   deny list. Denied: anything that touches the machine (files, network,
   databases, processes, email), anything that reads machine or invocation
   state (environment, system facts, arguments, stdin), anything holding
   process-global state (cache, i18n), and anything that seizes a resource
   (stdout and print, the terminal, the scheduler, sleeping). Allowed: all
   pure computation (math, strings, arrays, parsing, crypto, regex,
   compression, and the rest), plus `log_*` and `print_error`, because stderr
   cannot exfiltrate anything to the code's author and keeps sandboxed code
   debuggable.
3. **Enforcement is transitive.** Any function reachable from sandboxed code is
   checked by the same rules, so sandboxed code cannot launder an effect through
   an unsandboxed helper. If a sandboxed function calls your `fetch_report()`
   helper and that helper performs a network call, the program is rejected:

   ```nail
   // downloaded.nail (brought in with import)
   f sandboxed_summary():s {
       r fetch_report(); // fetch_report is now reachable from sandboxed code
   }
   ```

   ```nail
   // main.nail
   import(`downloaded.nail`)

   f fetch_report():s {
       // Rejected: reachable from sandboxed code, and http touches the machine
       r danger(http_download_file(`https://example.com/r`, `r.bin`));
   }
   ```
4. **Nesting cannot escape.** An `import_dangerous()` inside an imported file
   is a compile error: the lexer refuses it with "import_dangerous is not
   allowed inside a sandboxed import", since a file brought in with `import()`
   can only use `import()` itself. An `import()` nested deeper simply stays
   sandboxed.

### Trusted inclusion: import_dangerous

`import_dangerous()` is for files you wrote yourself: splitting a large
program across files, or sharing helpers between your own programs. The name
is deliberately heavy. Reaching for it is how you say out loud that this file
may touch the world.

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
import_dangerous(`math_helpers.nail`)

result:i = add(5, 3);
product:i = multiply(result, 2);
print(product); // Outputs: 16
```

### Restrictions

- File paths must be string literals (not variables)
- Included files must contain valid Nail code
- No conditional includes (includes always happen)

## Error Message Style Guide

Friendly, detailed errors are a core feature of Nail, and their quality is
enforced by golden-file tests (`tests/errors/`, run via
`./scripts/test_error_messages.sh`). Every diagnostic the compiler emits must answer
four things:

1. **What is wrong**, stated in plain language, never internal jargon.
   Write "Expected '}' but the file ended first", not "Expected BlockClose".
2. **Where**: the failing line of the user's own code is shown with a
   caret underline pointing at the problem (rendered by `CodeError::render`).
   A diagnostic with a missing span (line 0) is a bug.
3. **Why, with the actual values involved**: name the real types, variables,
   and functions, e.g. "'count' is declared as an integer (i) but its value
   is a string (s)". Use `NailDataTypeDescriptor::describe()` for type names,
   never `{:?}` Debug formatting.
4. **How to fix it**: a concrete `help:` suggestion (the `help` field on
   `CodeError`) whenever a fix is knowable: a "did you mean 'x'?" for typos,
   a corrected code snippet, or the idiomatic alternative.

Standard library runtime errors follow the same rule: always name the
function and echo the offending input, e.g.
"int_from: could not parse 'abc' as an integer".

Example of the required shape:

```
error: 'count' is declared as an integer (i) but its value is a string (s)
  --> tests/errors/type_mismatch_declaration.nail:3:11
   |
 3 | count:i = `hello`;
   |           ^^^^^^^
help: either change the declaration to 'count:s' or make the value an integer (i)
```

## Versioning and Toolchain Pinning

A Nail file records which compiler version it was written for, and the
toolchain obtains and runs that **exact** version to compile it. A Nail program
that compiled once compiles forever. There are no dependency mismatches, no
"works on my machine", and no bit rot from language changes. Old code never
needs migration to keep working: migration is a choice, not a requirement.

The tool that does this is `nail` itself. It reads which version a file asks
for, makes sure that version is on the machine, and hands the file to it.
Nothing else. It is the only command, and the only thing on `PATH`.

### Why a version number is enough

In most languages a compiler version does not pin what a program compiles
against, so a lockfile has to carry the rest. Nail has no package manager and a
closed dependency set: the standard library registry declares every crate a
program can reach, the bundle vendors all of them, and `nailc
--cargo-toml-superset` emits the complete list. A version is therefore a total
pin, down to the byte, on its own. This is why nothing beside the file is
needed, and why a lockfile must never be added.

### The version line

A file declares its version on the first line:

```nail
nail 0.3.1
```

**The line is required.** Two states, both explicit:

| line one | means | who writes it |
|---|---|---|
| `nail 0.3.1` | archived and frozen, compiles the same forever | the IDE, on save |
| `nail latest` | tracks whatever is installed here | you, once |

There is no third state. A file with no version line is an error, not a
default, because a default would mean the file compiles against whatever
happens to be newest, which is the drift this whole design exists to prevent.
An implicit fallback here would be the same shape as a null or an implicit
conversion, both of which the language already refuses.

`latest` means the newest version **installed on this machine**, never the
newest published. Opening a file can therefore never start a download of a
compiler nobody asked for, and a `latest` file is fully deterministic on a
given machine while deliberately not being so across machines. Source you
maintain says `latest`. Source you ship or archive carries a triple.

Three scoping rules keep the requirement from being a burden:

- **Only the entry file needs one.** Imported files inherit it, since one
  compiler compiles everything it reaches and only the entry decides which.
- **It is enforced on files, not in the lexer.** `nailc` checks the file it was
  handed. Lexing a fragment of source is unaffected.
- **The IDE writes it on save**, so in practice nobody meets the error. A
  released IDE stamps its own version. A development checkout stamps `latest`,
  since its version was never published and pinning to it would produce a file
  nobody else could open. A file that already has a line is never rewritten,
  including a `latest` one, because re-stamping would silently migrate code
  somebody pinned on purpose. `nail new` creates a file already stamped.

**The launcher stays permissive where the compiler is strict.** A missing or garbled
first line makes the launcher fall back to the newest installed version and say so on
stderr, rather than refuse. Refusing to launch is a far worse failure than
compiling, the garbled line might be legitimate syntax from a release that does
not exist yet, and by the time `nailc` runs the correct compiler has already
been chosen. Strictness belongs where it can be acted on.

**A program runs in its own directory.** `nail run` starts the program with its
working directory set to the directory of the source file, the same rule
imports follow. A program that reads `data.csv` beside itself works no matter
where the command was typed. A program run as a bare compiled binary gets
whatever directory it was started from, like any other executable.

### The grammar, which can never change

A `nail` built today has to read a file written in ten years, and a compiler from
ten years ago has to read a file written today. So this grammar is frozen:

```text
file    = [ BOM ] [ shebang LF ] version line ( LF | EOF )
shebang = "#!" *( byte except LF )
version line  = "nail" SP ( "latest" / version ) [ CR ]
version = num "." num "." num [ "-" pre ]
num     = 1*9DIGIT
pre     = 1*( ALPHA / DIGIT / "-" / "." )
```

One ASCII space, no leading whitespace, no trailing comment, no quotes. A
shebang may sit above the version line so that `#!/usr/bin/env nail` scripts do not
fight it for line one, and nothing else may.

**There are no ranges.** `^0.3`, `>=0.3.1` and a bare `0.3` cannot be written.
A range would mean the same file compiles differently over time, which is the
rot this whole design exists to kill. The parser accepts one sentinel word or
an exact triple and nothing in between, so the rule is enforced by the shape of
the parser rather than by a check somewhere downstream.

Parsing reads bytes, not text, and never looks past the first 4 KB. A file
whose body is invalid UTF-8, or which uses syntax from a release that does not
exist yet, still resolves and launches. A malformed first line reads as
unpinned rather than as an error, for the same reason.

A prerelease suffix (`0.4.0-dev`) marks a locally built compiler that was never
published. the launcher refuses to fetch one and says why.

The implementation is `src/version_line.rs`, shared by the compiler and by the launcher.

### Resolution

In order, stopping at the first that applies:

1. `--nail-version=<v>` on the command line
2. the entry file's version line
3. the newest version installed

The **entry file decides the whole program**. One compiler compiles every
source it reaches, so an imported file pinned to something older must not drag
the entry file's compiler backwards with it.

### What the launcher owns, and what it forwards

`nail` is a multiplexer. It owns the commands that are about the *set* of
installed versions, because no single version can answer those, and the
everyday spellings that pick a version and hand the file over: `new`, `run`,
`build`, `check`, `test`, `docs`, `website`, `github`, `version`, `open` and
`help`. Anything it has never heard of is forwarded to the resolved version's
`nailc`:

```
nail --transpile old.nail     runs the transpiler that shipped with old.nail's compiler
```

So `--transpile`, `--cargo-toml` and anything invented in ten years work
through a `nail` built today, without it ever being taught about them.
Forwarding is also better than calling `nailc` directly, because the file's
own version does the work rather than the newest one.

The reserved list is frozen. Growing it later would shadow a subcommand some
future `nailc` wants for itself.

| | |
|---|---|
| `install` `remove` `list` `gc` | manage versions |
| `which <file>` | print the resolved version and why |
| `fetch <path>` | install every version a tree pins, so it can go offline |
| `update <path>` | migrate files that still compile |
| `export` / `import` | move a release to a machine with no network |
| `doctor` | check the install over |
| `self-update` | the only thing that rewrites the launcher |
| `config` | warn and gc thresholds |
| `new <file>` | create a file already stamped |
| `run` `build` `check` | compile a file through the version it pins |
| `test [pattern]` | run every file in tests/, or those matching |
| `docs [name]` | what the standard library says, whole or for one name |
| `website` | open the Nail website |
| `github` / `source` | open the repository |
| `version` | print the resolved compiler's version |
| `open` | open a file in the editor, the explicit form of the default |
| `help` | print usage |
| `--` | escape hatch, forwards something that collides with the above |

`nail update` splits across the same line. the launcher walks the tree and makes
sure the target version is present, and that version's `nailc --stamp=<v>` type
checks each file and rewrites line one **only if it passes**. A file that no
longer compiles keeps its old version line and keeps working, which is what makes
migration optional forever. Files that track `latest` are skipped, and the
vendor folder is left alone: source pinned by its author is not ours to
restamp.

### Distribution

Nail ships as **one immutable bundle per release**, installed at
`/opt/nail/versions/<version>`. That path is fixed on every machine, which is
what the install needs sudo for: a bundle carries its dependencies already
compiled, and cargo decides reuse by fingerprints holding absolute paths, so a
store anywhere else throws the shipped cache away and recompiles everything on
first build. The store is handed to whoever ran the install, so nothing after
it needs root. The promise: download, install, open, it works. Offline. No Rust
installation, no C compiler, no crates.io, nothing else on the machine.

The bundle contains everything a build touches:

- `bin/` the IDE (`nail`) and compiler (`nailc`)
- `toolchain/` a pinned Rust toolchain (rustc, cargo, rust-lld, std for the
  host and for `x86_64-unknown-linux-musl`)
- `cargo-home/` `config.toml` (the single source of build configuration) plus
  vendored sources for every crate the stdlib registry can emit
- `nail/` the nail crate source that generated programs depend on
- `cache/` a pre-warmed shared build cache, warmed under both build profiles
  (quick and release), so the first build on a fresh machine compiles only the
  user's program (seconds, not minutes)

Design decisions and why:

- **Relocated on install.** Cargo's build fingerprints embed absolute paths, so
  a bundle's warm cache is only valid at the path it was warmed at, and the
  release machine's path is not the user's. Two settings in `cargo-home/config.toml`
  hold that path (the vendored sources and the linker) and the launcher rewrites
  both as it installs, reading the path the bundle was built at back out of that
  same file. The warm cache does not survive the move, so the launcher spends it
  once by building one throwaway program under each build profile while the
  person is still waiting on the install, rather than leaving the cost in
  front of their first real build. An
  install at the path the bundle was built at skips both steps.
- **Full copy per version.** No layer sharing between versions. A pinned
  version must never meet a rustc it was not built against, and paying the full
  download size per version is the correct price for that.
- **Versions are never on `PATH`.** Only the launcher is. A version on `PATH` would
  shadow the launcher and the version line would stop deciding which compiler runs, which
  is the entire point.
- **Static musl output.** User programs target `x86_64-unknown-linux-musl`,
  linked by the bundled `rust-lld` with `link-self-contained=yes`. Linking
  needs zero system files, and the produced binaries are fully static, so they
  run on any Linux distribution, including inside empty containers.
- **Scrubbed build environment.** The IDE invokes the bundled cargo by absolute
  path with a clean environment (`RUSTFLAGS`, `CARGO_*`, rustup installs, and
  the user's `PATH` cannot leak in). All configuration lives in the bundle's
  `cargo-home/config.toml` rather than in code.
- **Closed dependency set.** Nail programs can only ever require crates the
  stdlib registry declares, which is why complete vendoring and cache
  pre-warming are possible at all. Registry crates must be pure Rust or bundle
  their C source. Crates that require system libraries at build time are not
  accepted.
- **Tools require a glibc distribution.** The bundled rustc is the official
  glibc build, so the IDE and toolchain run on mainstream distros (Ubuntu,
  Debian, Fedora, Arch) but not musl-based ones like Alpine. Output binaries,
  being static, run anywhere.
- **Development checkouts are unaffected.** A binary finds its bundle from its
  own location, so when there is none (a source checkout) builds fall back to
  the system cargo, which is the workflow in this repository.

### Where releases come from

`nail` bakes in **one origin, forever**, and asks it for a version by name:

```
GET  {origin}/versions/latest            the newest published version
GET  {origin}/versions/<v>/x86_64-linux  the bundle
GET  {origin}/nail/x86_64-linux          the launcher itself, for self-update
```

The origin serves those files itself, off disk, through the reverse proxy. What
a user downloads is a compiled artifact rather than a git tag, so a source host
has nothing that corresponds to a release and there is no reason to send anyone
to one. No index document exists either: a version is named by its URL, and
that is the whole protocol. The target is in the path so that adding a second
one later does not change a request already in the wild.

Releases are **not signed**. Publishing means writing a file to the release
box, so being able to publish is already the credential, and a key kept beside
the artifact it signs would add custody without adding much. A missing version
is a plain `404`, and pre-1.0 releases may be removed.

Nothing appears under `versions/` until it has been downloaded and unpacked
whole, because the unpack happens beside the final location and is moved in
with a rename. An interrupted download cannot leave something that looks
installed but is not.

### Disk

A version is gigabytes, and the largest part of it, the build cache, is
reconstructible. So reclaiming disk is tiered rather than all-or-nothing:
`nail gc --caches` drops stale caches (minutes to rebuild, nothing lost),
and `nail gc` also uninstalls versions abandoned for much longer. Both are
dry runs until `--yes`. The newest installed version and anything used inside
the keep window are never touched.

the launcher warns when there is enough to reclaim to be worth mentioning, and
deletes nothing on its own unless `nail config auto` says otherwise. Users
will not run `gc` and disks will fill, but deleting gigabytes that cost a long
download to restore is worse than nagging.

### Tooling

`bundle/build_bundle.sh` assembles and warms a bundle (the only step needing
network and a musl C compiler, a build-machine concern), `bundle/install.sh` is
the one-time bootstrap that installs the launcher into `/opt/nail` under sudo
and hands the store to the user who ran it,
and `bundle/test_bundle.sh` is the release gate: on a machine with no Rust, no
cc and no network, compile and run a Nail program using only the bundle. A
release that fails the gate does not ship. `deploy/releases.sh` uploads a built
bundle to the release box and points `latest` at it.

## Standard Library

Nail includes a comprehensive standard library with functions organized by category:

### Namespaces

A Nail program has one flat name space and no import list, so every standard
library name carries the namespace of the library it belongs to. Functions wear
it in lower case and types in upper case:

```nail-fragment
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
`print_no_newline`, `danger`, `safe`, `expect`, `panic` and `todo`. Two registry tests
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
    y function.module == `string`;
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
- `test_assert(condition:b, message:s):v` - Assert a condition is true, stop the program with the message if false
- `panic(message:s)` - Panic with a message
- `todo(message:s)` - Mark unimplemented code

### String Operations
- `string_from(value):s` - Convert any value to string
- `string_to_uppercase(s:s):s` - Convert to uppercase
- `string_to_lowercase(s:s):s` - Convert to lowercase
- `string_to_title_case(s:s):s` - Convert to title case (capitalize each word)
- `string_to_sentence_case(s:s):s` - Convert to sentence case (capitalize first letter)
- `string_to_snake_case(s:s):s` - Convert to snake_case
- `string_to_kebab_case(s:s):s` - Convert to kebab-case
- `string_contains(s:s, substring:s):b` - Check if string contains substring
- `string_replace(s:s, from:s, to:s):s` - Replace all occurrences of substring
- `string_replace_first(s:s, from:s, to:s):s` - Replace first occurrence of substring
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
- `array_push(arr:a:T, item:T):a:T` - The array with the item appended (arrays are immutable, so this builds a new one)
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
- `array_zip_with(first:a:A, second:a:B, combine:f(A,B):C):a:C!e` - Combine two arrays element-wise through a named function
- `array_flatten(arr:a:a:T):a:T` - Flatten nested array by one level
- `array_deduplicate(arr:a:T):a:T` - Remove duplicate elements
- `array_find(arr:a:T, value:T):i!e` - Find index of first occurrence (can fail)
- `array_find_last(arr:a:T, value:T):i!e` - Find index of last occurrence (can fail)
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

```nail-fragment
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

For `array_group_by`, `array_count_by` and `array_deduplicate_by` the key must
be `i`, `s` or `b` (the sorting and min/max functions take any key, and
`array_sum_by` wants `i` or `f`). A `b` key is how an array is split in two,
which other languages call partition:

```nail-fragment
f is_paid(invoice:Invoice):b { r invoice.paid_on > 0; }

split:h<b,a:Invoice> = array_group_by(invoices, is_paid);
paid:a:Invoice       = hashmap_get_or(split, true, []);
owing:a:Invoice      = hashmap_get_or(split, false, []);
```

Reach for `hashmap_get_or` with an empty array rather than `hashmap_get`: a side
with nothing on it has no bucket at all, and empty is the right answer there
rather than an error.

Nothing calls the key function while it works: the keys are worked out over the
array first, and only then is anything sorted, bucketed or totalled. So a key
function may read a file or make a request -
`array_sort_by(reports, report_size)` where `report_size` calls `fs_size` is fine,
and costs one read per element rather than one per comparison.

**Sorting is stable.** Elements whose keys are equal come out in the order they
went in, and that is a promise, not an implementation detail. It is what lets a
program sort on more than one key with no two-key sort function existing: sort
by the least important key first and the most important key last, and each pass
leaves the previous order standing wherever its own keys tie.

```nail-fragment
// By author, and within an author the newest first.
newest_per_author:a:Post = array_sort_by(array_sort_by_descending(posts, post_day), post_author);
```

That composes to any number of keys, and the keys may point in different
directions - `array_sort_by_descending` reverses the order of the keys, not the
order of the ties. A comparator taking two elements would say the same thing in
one pass, and Nail has no closures to write one with, so this is the way.

Three more take a named function and answer a question `filter` cannot:

- `array_take_while(arr:a:T, keep:f(T):b):a:T` - The front of the array, up to the first element that fails
- `array_skip_while(arr:a:T, skip:f(T):b):a:T` - The rest, from that element onwards
- `array_deduplicate_by(arr:a:T, key:f(T):K):a:T` - First of each key, in the order they came in

`filter` takes every element that passes wherever it sits. `array_take_while`
stops at the first failure and ignores everything after it, which is what
reading a header off the top of a file wants. Taking and skipping with the same
test put the array back together.

The same named-function idea pairs two arrays up:

- `array_zip_with(first:a:A, second:a:B, combine:f(A,B):C):a:C!e` - What the function makes of each pair, in order

```nail-fragment
f line_total(price:f, quantity:f):f { r price * quantity; }

totals:a:f = danger(array_zip_with(prices, quantities, line_total));
```

Arrays of different lengths are an error rather than a quiet stop at the shorter
one, the same choice `hashmap_from_arrays` makes: two lists meant to line up and
not lining up is a bug worth hearing about. There is no `array_zip` producing
pairs, because a pair would need a type to be, and Nail has neither tuples nor
generic structs. Combining as the arrays are walked needs no such type.

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

A hashmap has no order of its own, and `hashmap_keys` may hand the same keys
back in a different order on the next run. Anywhere the order is seen - a
listing, a report, a comparison against a previous run - reach for one of these
instead, which all break ties on the key so two runs agree:

- `hashmap_sorted_keys(map:h<K,V>):a:K` - The keys in order
- `hashmap_keys_by_value(map:h<K,V>):a:K` - The keys ordered by their value, smallest first
- `hashmap_keys_by_value_descending(map:h<K,V>):a:K` - The keys ordered by their value, largest first
- `hashmap_max_by_value(map:h<K,V>):K!e` - The key holding the largest value
- `hashmap_min_by_value(map:h<K,V>):K!e` - The key holding the smallest value

The top ten is those two together: count with `hashmap_increment`, order with
`hashmap_keys_by_value_descending`, take the front with `array_take`.

The rest read a hashmap into a new one, leaving the original alone:

- `hashmap_sum_values(map:h<K,V>):V` - The values added together
- `hashmap_invert(map:h<K,V>):h<V,K>` - The hashmap turned around, values becoming keys
- `hashmap_pick(map:h<K,V>, keys:a:K):h<K,V>` - Only the named keys
- `hashmap_omit(map:h<K,V>, keys:a:K):h<K,V>` - Everything except the named keys

### Graph Operations (`graph_*`)

A graph is two parallel arrays: `edges_from[i] -> edges_to[i]` is one directed
edge, the shape a language without tuples holds pairs in. A node exists by
appearing in an edge, so an isolated node is simply not in the graph. Node ids
are ints or strings (the `K` below). Every function answers in the order the
edge arrays first mention nodes, so the same input always gives the same
answer.

- `graph_topological_sort(edges_from:a:K, edges_to:a:K):a:K!e` - An order that puts each edge's first node before its second. When every edge points from a prerequisite to what needs it, this is the order to build, migrate or load in. A cycle is the error, and the error names it
- `graph_has_cycle(edges_from:a:K, edges_to:a:K):b!e` - Whether following the edges around can ever come back to a node already passed through
- `graph_connected_components(edges_from:a:K, edges_to:a:K):a:a:K!e` - The groups of nodes that touch through edges read in either direction. This is what union find computes, delivered in one call
- `graph_reachable(edges_from:a:K, edges_to:a:K, start:K):a:K!e` - Every node the edges lead to from the start, one way only, the start itself first. Swapping the two edge arrays turns the question around into what reaches this node
- `graph_shortest_path(edges_from:a:K, edges_to:a:K, start:K, goal:K):a:K!e` - The route crossing the fewest edges, both ends included. No route is the error
- `graph_shortest_path_weighted(edges_from:a:s, edges_to:a:s, weights:a:f, start:s, goal:s):GRAPH_Path!e` - The cheapest route when every edge carries a cost, one weight per edge by position, none of them negative

The weighted route answers with two things at once, so it returns a struct
rather than an array: `GRAPH_Path` carries the route in `nodes:a:s` and its
total in `cost:f`. Its node ids are strings because a struct's fields name
concrete types:

```nail-fragment
edges_from:a:s = [`home`, `home`, `park`];
edges_to:a:s = [`park`, `office`, `office`];
weights:a:f = [1.0, 5.0, 1.5];
cheapest:GRAPH_Path = danger(graph_shortest_path_weighted(edges_from, edges_to, weights, `home`, `office`));
print(cheapest.cost);
```

All of these mismatch-check the parallel arrays the way `array_zip_with` does,
which is why every one of them can fail. Anything a graph function does not
answer in one call - a spanning tree, weighted distances to everywhere - is a
sign the data wants a real store: register the edges as rows and write SQL.

### Type Conversion
- `int_from(value):i!e` - Convert to integer
- `float_from(value):f!e` - Convert to float
- `bool_from(value):b!e` - Convert to boolean. Text may be true, yes, y, on or 1 and their opposites false, no, n, off or 0, in any case. A number must be 1 or 0. Anything else is an error rather than a guess

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
- `math_asinh(value:f):f` - Inverse hyperbolic sine
- `math_acosh(value:f):f!e` / `math_atanh(value:f):f!e` - The other inverses, erroring where no real answer exists (below 1.0, and at or outside -1.0..1.0)
- `math_to_degrees(radians:f):f` / `math_to_radians(degrees:f):f` - Angle conversion
- `math_log(value:f):f!e` / `math_log2` / `math_log10` - Logarithms; error at or below zero
- `math_log_base(value:f, base:f):f!e` - Logarithm in a base of your choosing
- `math_exp(value:f):f` - e raised to a power
- `math_sigmoid(value:f):f` - 1 / (1 + e^-x)
- `math_lerp(start:f, end:f, t:f):f` - Linear interpolation
- `math_is_nan(value:f):b` / `math_is_infinite(value:f):b` / `math_is_finite(value:f):b` - What kind of number this is. Not-a-number is the one value not equal to itself, so `==` cannot be used to ask
- `math_random():f` - A fraction from 0.0 up to 1.0. **Not for secrets** - see `crypto_random_hex`
- `math_pi():f` / `math_e():f` - Constants

The `math_*` functions above work in floats. Where the answer to a question
about whole numbers is itself a whole number, the `int_*` version says so
without a trip out to floats and back:

- `int_abs(value:i):i!e` - Size without the sign. The most negative integer has no positive counterpart, so that one is an error
- `int_min(a:i, b:i):i` / `int_max(a:i, b:i):i` - Smaller and larger of two whole numbers
- `int_sign(value:i):i` - -1, 0 or 1 according to the direction of the value
- `int_clamp(value:i, low:i, high:i):i!e` - Restrict a whole number to a range
- `int_pow(base:i, exponent:i):i!e` - Raise to a power, erroring on a negative exponent or overflow
- `int_is_even(value:i):b` / `int_is_odd(value:i):b` - Whether the number divides evenly by two. Zero is even
- `int_from_hex(text:s):i!e` / `int_from_radix(text:s, base:i):i!e` / `int_to_radix(value:i, base:i):s!e` - Numbers written in another base, from 2 to 36

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
- `draw_text(x:f, y:f, content:s, size:f, fill:s, anchor:DRAW_Anchor):s!e` - The anchor is `DRAW_Anchor::Start`, `DRAW_Anchor::Middle` or `DRAW_Anchor::End`
- `draw_path(commands:s, stroke:s, stroke_width:f, fill:s):s!e` - SVG path notation, for a shape none of the others can make
- `draw_group(offset_x:f, offset_y:f, shapes:a:s):s` - Move several shapes together
- `draw_scale(value:f, from_low:f, from_high:f, to_low:f, to_high:f):f!e` - Move a value between ranges. To plot upward on a screen whose y grows downward, pass the height as `to_low` and `0.0` as `to_high`

Text is XML-escaped, so a label containing `&` or `<` cannot produce a document
nothing will open.

### Audio (`audio_*`)

Two things a program wants from sound: play this file, and beep when something
finishes. Playing is synchronous - the call returns when the sound has finished
- so a notification is one line. To carry on while it plays, run it in a `c`
block beside the other work.

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
`min_samples_leaf`, `bins`, `lambda_l2`, `objective`,
`early_stopping_rounds`. Start from `ml_boost_default_config()` and change
what you need, since Nail has no default field values and a literal must name
every field.

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
- `fs_is_dir(path:s):b!e` / `fs_is_file(path:s):b!e` - False for a path that is not there
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
- `http_path_params(pattern:s, path:s):h<s,s>!e` - The named segments a pattern binds, an error for a path the pattern does not match
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

```nail-fragment
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

```nail-fragment
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
            params:h<s,s> = danger(http_path_params(`/dictionary/:word`, request.path));
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
    state = hashmap_new(),
    cors_origins = [],
    security_headers = true,
    rate_limit_per_minute = 0,
    rate_limit_message = ``
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
including binary ones, an empty array serves none), `max_body_bytes` (0 means 8 MiB, larger bodies get 413), `timeout_seconds`
(0 means 30, a handler that overruns gives the client 504), `state`,
`cors_origins` (the origins allowed to call this server, empty allows none),
`security_headers` (the usual hardening headers on every response),
`rate_limit_per_minute` (0 means no limit) and `rate_limit_message` (what a
limited client is told).

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
    r HTTP_Response { status = 200, body = `saved`, content_type = `text/plain`, headers = hashmap_new() };
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

```nail-fragment
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
- `crypto_verify_password(password:s, stored_hash:s):b!e` - Check a password against a stored hash. False for a wrong password, an error for a stored value that is not a hash
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
- `log_set_file(path:s):v!e` - Send the lines to a file instead of standard error, for the rest of the run. The file is added to rather than replaced

A program run under a service manager usually wants the default, since the
manager collects standard error already. `log_set_file` is for the program that
has to keep its own file.

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

For text a person typed rather than text a machine stored, the difference
between a capital and a small letter usually should not count:
- `string_equals_ignore_case(first:s, second:s):b` - The comparison for a header name, a command, an answer at a prompt
- `string_contains_ignore_case(text:s, pattern:s):b` / `string_starts_with_ignore_case(text:s, prefix:s):b` / `string_ends_with_ignore_case(text:s, suffix:s):b`

### Formatting Numbers (`format_*`)
Nail has no format strings, so the roundings and comma insertions every program
writes for itself live here instead. Everything returns text for a reader, never
text to parse again:
- `format_decimals(value:f, places:i):s!e` - Fixed decimals, keeping trailing zeros
- `format_thousands(value:i):s` / `format_thousands_float(value:f, places:i):s!e` - Grouped digits
- `format_currency(amount:f, symbol:s):s` - A symbol and two decimals
- `format_percent(fraction:f, places:i):s!e` - `0.125` at one place is `12.5%`
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
- `template_render_or(template:s, values:h<s,s>, fallback:s):s!e` - Fill a missing name with the fallback instead of failing
- `template_names_used(template:s):a:s!e` - The names a template asks for
- `template_has(template:s, name:s):b!e` - Whether the template mentions that name

The syntax is `{{name}}` for an escaped value, `{{{name}}}` for raw markup,
`{{#if name}}...{{else}}...{{/if}}`, `{{#unless name}}...{{/unless}}`, and
`{{! a comment }}`. There is no loop tag: a loop belongs in the program, where
Nail's own `map` already does it better. A name the values do not hold is an
error rather than a blank, because a blank in a page is a bug every time. Where
a gap really is acceptable, `template_render_or` fills it with text you choose,
and still refuses a template it cannot read: a missing value is data, an
unclosed tag is a bug.

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
program likes: a handler, a `forever` function in a `c` block, a watcher.

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

- Nail's IDE on Linux is the default way to write Nail, and `nailc` with `nail run`, `nail build` and `nail check` is the command line route.
- Two build profiles, chosen by intent rather than by flag. `nail run` and the IDE's F7 use a quick profile that rebuilds in well under a second. `nail build` and Shift+F7 pay for the fully optimized release build and leave the binary beside the source. Only release builds put a binary there, so a binary next to a `.nail` file is always the shippable one.
- Opinionated code formatting enforced on save.

## Built-in Profiling

Every build is profiled by default. There is nothing to enable and nothing to configure.

- The compiler gives every user function a drop guard that records wall time on entry and exit. Recording a call costs two clock reads and three relaxed atomic updates, roughly 30 nanoseconds.
- When a program exits and stderr is a terminal, it prints a timing sheet: calls, total, average, max, and percent of wall time per function, sorted by total. Piped output and captured test output never see the sheet.
- While a program runs it rewrites `.nail_profile.json` in its working directory once a second, atomically. The IDE watches that file and annotates each function declaration with its live timings. When the source on screen no longer matches the build the program came from, the annotations show as stale.
- Times are cumulative wall time. A caller includes its callees, and an async function includes time spent awaiting.
- `nailc --no-profile` builds without any instrumentation, for deploys that want zero overhead. The Nail website deliberately ships profiled: its live timings section is the server reading its own dump. Browser builds are never instrumented.
- The compiler itself reports per-stage timings (lex, parse, check, transpile) on stderr when run at a terminal.

The grammar sections below state the language's rules formally.

#  Nail Language Grammar in EBNF


## Scope

A name lives from where it is declared to the end of the block that holds it.
A block is a function body, an `if` arm, a loop body, a collection operation's
body, or a bare `{ ... }`. Code after the block cannot see what the block
declared, and neither can the Rust the program becomes:

```nail-refused
if {
    true -> {
        answer:i = 42;
    },
    else -> { }
}
print(answer);
```

Declare it before the block when the code after the block needs it. A loop's
iterator belongs to the loop the same way, and a function body sees only its
own parameters and its own declarations: there are no globals inside a
function.

Structs and enums are declared at the top level of a file. A type belongs to
the whole program rather than to one block, so declaring one inside an `if` or
a loop is refused.

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

```ebnf
expression :=
    literal                   // A constant value (e.g., numbers, strings)
    function_call             // Invoking a function (e.g., `foo(a, b)`)
    binary_expression         // Binary operations (e.g., `a + b`)
    unary_expression          // Unary operations (e.g., `-a`, `!b`)
    if_expression             // Conditional expression (e.g., `if { condition -> { block } }`)
    block                     // A sequence of statements inside `{}` (e.g., `{ stmt1; stmt2 }`)
    forever                   // A block that runs until the program ends (e.g., `forever { ... }`)
    return                    // Returns a value from a function (e.g., `r x`)

declaration :=
    const_decl                  // Declaring a constant (e.g., `pi = 3.14;`)
    struct_decl               // Declaring a struct (e.g., `struct Point { ... }`)
    enum_decl                 // Declaring an enum (e.g., `enum Days { ... }`)
```


## Struct

### EBNF

```ebnf
struct_decl := "struct" pascal_identifier "{" struct_field "," struct_field "}"
struct_field := snake_identifier ":" struct_field_type
struct_field_type = primitive_type | enum_type | array_type
```


### Nail:

```nail
struct Point {
    x_coord:i,
    y_coord:i
}
```

### Transpilation
    
```nail
struct Point {
    x_coord:i,
    y_coord:i
}
```



## Enums

### EBNF

```ebnf
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

```nail
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

```nail-fragment
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

```nail-fragment
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

```nail-fragment
// Example where all branches return the same type (in this case, a string)
message:s = if {
    today == DaysOfWeek::Monday -> { r `Start of the week`; },
    today == DaysOfWeek::Friday -> { r `End of the workweek`; },
    else -> { r `It's a regular day`; }
};

// This will work because all branches return a string.
```

However, if branches return different types, Nail will produce an error:

```nail-fragment
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

```nail
pi:f = 3.14159;
max_users:i = 100;
greeting:s = `Hello, World!`;
```

Key points about const declarations:
- They are immutable.
- To change a const value, you must use shadowing (redeclaration with the same name, in the same scope).
- Identifiers use snake_case, same as constant declarations (otherwise changing all the names for minor refactoring would be painful).

Example of shadowing:

```nail
user_count:i = 5;
// Later in the code
user_count:i = 6;  // This shadows the previous declaration
```

### Examples

Const with shadowing:

```nail
max_attempts:i = 3;
max_attempts:i = 5;  // Shadowing the previous declaration
max_attempts:s = `Three`; // Shadows can even change the type (like Rust)
```

### Key Takeaways

- Const values can be "changed" through shadowing, giving an impression of mutability, which helps ease of use.
- Shadowing works in the SAME scope only. Declaring a name that already
  exists in an enclosing scope is a compile error, because such a shadow dies
  at the end of its block and code after the block silently sees the outer
  value again (`sum:i = sum + num;` in a loop body would quietly sum to
  nothing). The error's help names the fix: use `reduce` to accumulate, or
  pick a different name.
- There is no reassignment at all. A bare `count = 2;` (no type annotation)
  is a compile error everywhere, with a help line pointing at shadowing,
  `reduce`, and `==` for the comparison typo. Accumulating across iterations
  is `reduce` and `scan`, never mutation.

## Functions

### Function Declaration Syntax

In Nail, function declarations are similar to Rust. The basic syntax is:

```nail-fragment
f function_name(param_name:Type, another_param:Type):Type {
    // Function body
}
```

Example:

```nail
f add(num_a:i, num_b:i):i {
   r num_a + num_b;
}
```

### Function Return Types and Void Functions

Functions in Nail can return values of any type, or they can be void functions that return nothing:

```nail
// Function with return type
f calculate(left:i, right:i):i {
    r left + right;
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

**Important Rule**: A void call can only be assigned to a name declared `:v`. Assigning it to any other type is a mismatch, since there is no value of that type to hand over:

```nail-fragment
// This is INVALID - print returns void, not a string
result:s = print(`Hello`);  // ERROR

// This is valid - just call the function
print(`Hello`);  // OK

// Functions that return values can be assigned
sum:i = calculate(5, 3);  // OK - returns an integer
```

### Function Parameters

Arguments are positional: a call supplies one expression per parameter, in the order the declaration names them. There is no named-argument syntax.

```nail
f greet(name:s):v {
    print(string_concat([`Hello, `, name, `!`]));
}

user_name:s = `Alice`;
greet(user_name);
```

### Loop-based Processing

Nail provides collection operations for iteration and data processing. Since variables are immutable, traditional imperative loops are replaced with functional operations:

```nail
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


## Parallel and Concurrent Blocks

Two blocks run their statements at the same time, and one question decides
which you want: is that time spent computing, or waiting?

- `p ... /p` gives every statement its own operating system thread, so the
  statements run on different cores. Reach for it when the work is your own
  computation.
- `c ... /c` runs every statement as an async task on one thread, so their
  waits overlap. Reach for it when the work is a request, a file read, a
  database query, or a sleep.

```nail-fragment
// Waiting: three reads, all idle. One thread is plenty.
c
    weather:s = danger(fs_read(`weather.txt`));
    news:s = danger(fs_read(`news.txt`));
    prices:s = danger(fs_read(`prices.txt`));
/c

// Computing: three cores, three real workloads.
p
    hash_a:s = crypto_hash_sha256(block_one);
    hash_b:s = crypto_hash_sha256(block_two);
    hash_c:s = crypto_hash_sha256(block_three);
/p
```

Both blocks share one shape and one scoping rule: a name declared inside is an
ordinary immutable value on the line after the block, with no await left to
forget and nothing mutable to lock.

### What each one costs

`c` transpiles to Rust's `tokio::join!`. One thread starts every statement, and
each task hands that thread on the moment it waits, so three waits cost the
longest one instead of the sum. Waiting takes no CPU, which is why a single
thread can hold thousands of pending waits. What `c` cannot do is make
computation faster: one thread is still doing the computing, so a statement
that computes for two seconds without ever waiting holds the thread for two
seconds while the others sit behind it.

`p` transpiles to one `std::thread::spawn` per statement, and joins every
thread before the block ends. Three threads land on three cores, and three
cores really do compute at the same time. A thread costs microseconds to start
and a stack to keep, which is nothing for three computations and far too much
for three hundred sleeping reads.

Put another way: `c` buys overlap, `p` buys cores. If the machine's CPU meter
would sit near zero while the statement runs, reach for `c`. If it would peg a
core, reach for `p`.

### Syntax

```nail-fragment
p
    // Each statement gets its own thread
    parsed_rows:a:s = parse_every_line(raw_text);
    checksum:s = crypto_hash_sha256(raw_text);
    print(`Processing in parallel!`);
    sorted_names:a:s = array_sort(names);
/p
```

### Key Points:

- All statements inside the block start at the same time, and the block ends
  when the slowest one finishes
- Variables declared inside can be used after the block completes
- In `p`, each statement transpiles to its own `std::thread::spawn` and the
  block joins every thread before continuing, so the statements run on
  separate cores
- In `c`, each statement becomes an async task inside one `tokio::join!`, so
  their waits overlap on a single thread
- No semicolon needed after the closing `/p` or `/c`
- `p` is for independent computation, `c` is for I/O and API calls

### Realistic Examples:

```nail-fragment
// Example 1: a dashboard's three API calls, which are all waiting
c
    user_profile:HTTP_Response = danger(http_request(HTTP_Method::Get, `https://api.example.com/user/123`, headers, ``));
    recent_orders:HTTP_Response = danger(http_request(HTTP_Method::Get, `https://api.example.com/orders?user=123`, headers, ``));
    account_balance:HTTP_Response = danger(http_request(HTTP_Method::Get, `https://api.example.com/balance/123`, headers, ``));
/c

// All three responses are available after the block, and it cost one request
print(`Profile: `, user_profile.body);
print(`Orders: `, recent_orders);
print(`Balance: `, account_balance);

// Example 2: reading three files, which is also waiting
c
    content1:s = danger(fs_read(`data1.txt`));
    content2:s = danger(fs_read(`data2.txt`));
    content3:s = danger(fs_read(`data3.txt`));
/c

all_content:s = array_join([content1, content2, content3], `\n`);

// Example 3: hashing those three files, which is computation, so it wants cores
p
    digest1:s = crypto_hash_sha256(content1);
    digest2:s = crypto_hash_sha256(content2);
    digest3:s = crypto_hash_sha256(content3);
/p

report:s = array_join([digest1, digest2, digest3], `\n`);
```

## Structs


```nail
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
    name_parts:a:s = string_split(input.full_name, ` `);
    r UserRecord {
        id = id,
        first_name = danger(array_get(name_parts, 0)), // This could error, and danger says so out loud
        last_name = danger(array_get(name_parts, 1)),  // Use danger, or safe with a handler function
        email = input.email,
        age = input.age
    };
}

// Usage
input:UserInput = UserInput { full_name = `John Doe`, email = `john@example.com`, age = 30 };
record:UserRecord = convert_user_input_to_record(input, 1);
```

### Struct Serialization and Deserialization

Nail provides built-in functions for serializing structs to JSON and deserializing JSON to structs:

```nail-fragment
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

```nail-fragment
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

Returning `e(message)` from a function whose return type is `T!e` produces an error value, and nothing panics at that point. The error travels back to the caller as a value, and the program only stops if `danger` or `expect` meets it.

### Handling Errors

Nail provides several ways to handle errors:

#### Using `safe`

The `safe` function allows you to handle potential errors with a handler function:

```nail-fragment
f handle_error(err:e):i {
    print(`An error occurred: ` + err);
    r -1;  // Return a default value or handle the error appropriately
}

result:i = safe(potentially_failing_function(), handle_error);
```

#### Using `danger`

The `danger` function unwraps a result, and panics with the error if there is one. The difference between `danger` and `expect` is purely one of intent: `danger` is used when the programmer acknowledges this should and can be made safe, and it should be made safe. This way you can easily find all dangerous parts of a program, and make them safe.

```nail-fragment
result:i = danger(potentially_failing_function());
```

#### Using `expect`

The `expect` function is identical to danger, but with a different semantic meaning. It is an error so catastrophic, there is no point in not crashing the program if it fails. Used for errors that should never happen in a well-functioning program. For example, you may have a program that displays data from a CSV. Instead of using `safe` to handle the error, which would display no data to the user anyway, you would likely prefer to crash the program so you actually are aware there is a massive problematic error occuring, rather than give users a terrible experience of seeing no data at all, and not trip any monitoring systems. The choice is up to the programmer of when to use which.


### Error Handler Function Types

**Important**: Error handling functions used with `safe()` must accept a parameter of type `:e` (error), not `:s` (string). The type checker enforces this requirement.

```nail
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
```nail-refused
// Wrong
x:i = 5;

// Correct
count:i = 5;
user_age:i = 25;
```

#### Traditional If Syntax Error
```
Expected '{' here, but found the name 'count'
```
**Solution**: Nail only supports match-like if syntax:
```nail-refused
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
Use 'y' (yield) instead of 'r' (return) in collection operations
```
**Solution**: Use `y` (yield) in collection operations, `r` (return) in functions:
```nail-fragment
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
'result' is declared as a string (s) but print returns void (v)
```
**Solution**: A void call can only be assigned to a name declared `:v`:
```nail
// Wrong: result:s = print(`Hello`);

// Correct
print(`Hello`);  // Just call the function
```

### Runtime Issues

#### Error Propagation
When a function returns an error type, it must be explicitly handled:
```nail
// This will panic if the function fails
result:i = danger(int_from(`abc`));

// Safe handling with error function
f handle_parse_error(err:e):i { r 0; }
result:i = safe(int_from(`123`), handle_parse_error);
```

#### Forever Means Forever
`forever` never ends on its own, and there is no break. A program whose top
level reaches one stays there, which is right for a server and wrong for a
script. A block that has to stop belongs in a function, which `r` can leave,
and a search that stops is a function that calls itself:
```nail
f first_square_past(limit:i, candidate:i):i {
    if {
        candidate * candidate > limit -> { r candidate; },
        else -> { r first_square_past(limit, candidate + 1); }
    }
}
print(first_square_past(50, 0));
```

### Best Practices

1. **Always handle errors explicitly** - Use `safe()`, `danger()`, or `expect()`
2. **Use descriptive variable names** - Avoid single letters or abbreviations
3. **Remember Nail's syntax** - Match-like if statements, yield in collections
4. **Type your variables** - Always include type annotations
5. **Use collection operations** - Prefer map/filter/reduce over manual loops
6. **Handle concurrency carefully** - Use `p` for work that computes, `c` for work that waits
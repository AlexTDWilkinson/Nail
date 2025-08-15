
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
- Explicit collection operation keywords (map, filter, reduce, each, find, all, any) instead of generic functional methods
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

### 4.5 Operators

#### 4.5.1 Arithmetic Operators
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
for idx in array_range(0, array_len(my_array)) {
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
hashmap_insert(user_scores, `alice`, 100);
hashmap_insert(user_scores, `bob`, 85);

score:i = danger(hashmap_get(user_scores, `alice`));
has_charlie:b = hashmap_contains_key(user_scores, `charlie`);
map_size:i = hashmap_len(user_scores);

// Safe access with error handling
f handle_missing_key(err:e):i { r 0; }
alice_score:i = safe(hashmap_get(user_scores, `alice`), handle_missing_key);

// Example with struct values
struct Point { x_pos:i, y_pos:i }
origin:Point = Point { x_pos: 0, y_pos: 0 };
hashmap_insert(id_to_struct, 1, origin);
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
    status == 200 => { print(`OK`); },
    status == 404 => { print(`Not Found`); },
    else => { print(`Unknown Status`); }
}

// If as an expression (returns a value)
result:s = if {
    status == 200 => { r `Success`; },
    else => { r `Error`; }
};
```

**Important**: All branches use `=>` followed by blocks. When used as an expression, use `r` (return) to produce the value.

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
    y if { num > acc => { num }, else => { acc } };
};

// Build string
concatenated:s = reduce acc str in [`hello`, ` `, `world`] from `` {
    y acc + str;
};
```

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
for position in array_range(0, array_len(numbers)) {
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
        index >= 10 => { break; },     // Exits the loop
        index == 5 => { continue; },   // Skips to next iteration (index becomes 6)
        else => { /* keep looping */ }
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
        string_len(data) == 0 => { r err(`Empty data`); },
        else => { r ok(data); }
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
        annual_rate == 0 => { 
            r err(`Annual rate cannot be zero`); 
        },
        years <= 0 => { 
            r err(`Loan term must be positive`); 
        },
        else => {
            monthly_rate:f = expect(float_from(annual_rate)) / 12.0 / 100.0;
            payments:i = years * 12;
            
            // Division by zero check
            denominator:f = 1.0 - pow(1.0 + monthly_rate, -payments);
            if {
                denominator == 0.0 => { 
                    r err(`Cannot calculate payment: invalid parameters`);
                },
                else => {
                    payment:f = expect(float_from(principal)) * monthly_rate / denominator;
                    r ok(payment);
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

## Standard Library

Nail includes a comprehensive standard library with functions organized by category:

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
- `string_pad_left(s:s, length:i, pad:s):s` - Pad string on the left to specified length
- `string_pad_right(s:s, length:i, pad:s):s` - Pad string on the right to specified length
- `string_len(s:s):i` - Get string length
- `string_chars(s:s):a:s` - Convert string to array of single-character strings
- `string_starts_with(s:s, prefix:s):b` - Check if string starts with prefix
- `string_ends_with(s:s, suffix:s):b` - Check if string ends with suffix
- `string_index_of(s:s, substring:s):i!e` - Find index of first occurrence (can fail)
- `string_last_index_of(s:s, substring:s):i!e` - Find index of last occurrence (can fail)
- `string_substring(s:s, start:i, end:i):s!e` - Extract substring (can fail)
- `string_repeat(s:s, count:i):s` - Repeat string count times
- `string_reverse(s:s):s` - Reverse string characters
- `string_join(arr:a:s, separator:s):s` - Join array of strings with separator
- `string_is_alphabetic(s:s):b` - Check if string contains only alphabetic characters
- `string_is_digits_only(s:s):b` - Check if string contains only digit characters (0-9)
- `string_is_numeric(s:s):b` - Check if string can be parsed as a number (includes decimals, signs)
- `string_is_alphanumeric(s:s):b` - Check if string contains only alphanumeric characters

### Array Operations
- `array_len(arr:a:T):i` - Get array length
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
- `array_flatten_deep(arr:a:a:T):a:T` - Recursively flatten all nested arrays
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
- `array_partition(arr:a:T, predicate:f(T):b):a:a:T` - Partition array into [matching, non-matching]
- `array_group_by(arr:a:T, key_fn:f(T):K):h<K,a:T>` - Group elements by key function result

### HashMap Operations
**Note**: HashMap keys and values must be concrete types (i, f, s, b, arrays, structs, enums). Void type cannot be used as a value.

- `hashmap_new():h<K,V>` - Create new hashmap (K,V must be concrete types)
- `hashmap_insert(map:h<K,V>, key:K, value:V):v` - Insert key-value pair
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

### Math Operations (`math_*`)
- `math_abs(n:i):i` - Absolute value (integer)
- `math_abs(n:f):f` - Absolute value (float)
- `math_min(a:i, b:i):i` - Minimum of two integers
- `math_max(a:i, b:i):i` - Maximum of two integers
- `math_min(a:f, b:f):f` - Minimum of two floats
- `math_max(a:f, b:f):f` - Maximum of two floats
- `math_pow(base:f, exp:f):f` - Power operation
- `math_sqrt(n:f):f!e` - Square root (can fail for negative)
- `math_ceil(n:f):i` - Round up to nearest integer
- `math_floor(n:f):i` - Round down to nearest integer
- `math_round(n:f):i` - Round to nearest integer
- `math_gcd(a:i, b:i):i` - Greatest common divisor
- `math_lcm(a:i, b:i):i` - Least common multiple
- `math_factorial(n:i):i!e` - Factorial (can fail for negative or overflow)
- `math_is_prime(n:i):b` - Check if number is prime
- `math_sin(angle:f):f` - Sine function (radians)
- `math_cos(angle:f):f` - Cosine function (radians)
- `math_tan(angle:f):f` - Tangent function (radians)
- `math_log(n:f):f!e` - Natural logarithm (can fail for non-positive)
- `math_log10(n:f):f!e` - Base-10 logarithm (can fail for non-positive)
- `math_log2(n:f):f!e` - Base-2 logarithm (can fail for non-positive)
- `math_sigmoid(x:f):f` - Sigmoid function (1 / (1 + e^-x))
- `math_lerp(start:f, end:f, t:f):f` - Linear interpolation
- `math_clamp(value:f, min:f, max:f):f` - Clamp value between min and max

### I/O Operations (`io_*`)
- `io_read_line():s!e` - Read line from stdin
- `io_read_line_prompt(prompt:s):s!e` - Read with prompt
- `io_write_file(path:s, content:s):v!e` - Write to file
- `io_read_file(path:s):s!e` - Read file contents

### File System (`fs_*`)
- `fs_exists(path:s):b` - Check if path exists
- `fs_read_dir(path:s):a:s!e` - List directory contents
- `fs_create_dir(path:s):v!e` - Create directory
- `fs_remove_file(path:s):v!e` - Delete file

### HTTP Operations (`http_*`)
- `http_request(method:s, url:s, headers:h<s,s>, body:s):s!e` - Make HTTP request
- `http_get(url:s):s!e` - Simple GET request
- `http_post(url:s, body:s):s!e` - Simple POST request

### Time Operations (`time_*`)
- `time_sleep(seconds:f):v` - Sleep for specified seconds
- `time_now():i` - Current timestamp in seconds
- `time_now_millis():i` - Current timestamp in milliseconds
- `time_format(timestamp:i, format:TimeFormat):s` - Format timestamp using TimeFormat enum
- `time_parse(time_str:s, format:TimeFormat):i!e` - Parse time string using TimeFormat enum
- `time_add_seconds(timestamp:i, seconds:i):i` - Add seconds to timestamp
- `time_diff(t1:i, t2:i):i` - Get absolute difference between timestamps

**TimeFormat enum values:**
- `Unix` - Unix timestamp in seconds
- `UnixMillis` - Unix timestamp in milliseconds
- `ISO8601` - ISO 8601 format
- `RFC3339` - RFC 3339 format
- `RFC2822` - RFC 2822 format

### Cryptography Operations (`crypto_*`)
- `crypto_hash_sha256(s:s):s` - Calculate SHA-256 hash of string
- `crypto_hash_md5(s:s):s` - Calculate MD5 hash of string (for checksums, not security)
- `crypto_uuid_v4():s` - Generate a UUID v4 string

### Error Handling
- `safe(result:T!e, handler:f(e:e):T):T` - Handle error with function
- `danger(result:T!e):T` - Unwrap or panic (use carefully)
- `expect(result:T!e):T` - Unwrap or panic (for impossible errors)
- `ok(value:T):T!e` - Wrap value in Ok result
- `err(message:s):T!e` - Create error result

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
    if_expression             // Conditional expression (e.g., `if { condition => { block } }`)
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
   current_light == TrafficLight::Red => { print(`Stop!`); },
   current_light == TrafficLight::Yellow => { print(`Prepare to stop`); },
   current_light == TrafficLight::Green => { print(`Go!`); }
}

// If you have an else branch, you don't need to cover all cases
if {
   current_light == TrafficLight::Red => { print(`Stop!`); },
   else => { print(`It could be yellow or green...`); }
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
    "if" "{" if_branch {"," if_branch} ["else" "=>" block] "}"

if_branch :=
    expression "=>" block
```

### Notes:

- `if_expression`: Begins with the keyword `if`, followed by a list of branches enclosed in curly braces. Each branch consists of an expression, which is followed by a `=>` and a block of code. If none of the conditions are met, the optional `else` branch will execute its block.
- `if_branch`: Each branch consists of an expression followed by a `=>` and a block, which represents the code that should be executed if the condition is true.

### Usage in Nail

In Nail, if expressions are used similarly to other languages, but they offer concise syntax that ensures every condition leads to a valid block of code. The branches in an if statement are separated by commas, allowing for clean and readable code.

```js
// Basic if statement in Nail
if {
    today == DaysOfWeek::Monday => {
        print(`Start of the week.`);
    },
    today == DaysOfWeek::Friday => {
        print(`End of the workweek!`);
    },
    else => {
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
    current_light == TrafficLight::Red => {
        print(`Stop!`);
    },
    current_light == TrafficLight::Yellow => {
        print(`Prepare to stop.`);
    },
    current_light == TrafficLight::Green => {
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
    today == DaysOfWeek::Monday => { r `Start of the week`; },
    today == DaysOfWeek::Friday => { r `End of the workweek`; },
    else => { r `It's a regular day`; }
};

// This will work because all branches return a string.
```

However, if branches return different types, Nail will produce an error:

```js
// Example where branches return different types (this will cause an error)
message:s = if {
    today == DaysOfWeek::Monday => { r `Start of the week`; },  // String
    today == DaysOfWeek::Friday => { r 5; },  // Integer - ERROR!
    else => { r `It's a regular day`; }  // String
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
        b == 0 => { r err(`Division by zero`); },
        else => { r ok(a / b); }
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
    user_profile:s = http_get(`https://api.example.com/user/123`);
    recent_orders:s = http_get(`https://api.example.com/orders?user=123`);
    account_balance:s = http_get(`https://api.example.com/balance/123`);
/p

// All data is available after parallel block completes
print(`Profile: `, user_profile);
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
input:UserInput = UserInput { full_name: `John Doe`, email: `john@example.com`, age: 30 };
record:UserRecord = convert_user_input_to_record(input, 1);
```

### Struct Serialization and Deserialization

Nail provides built-in functions for serializing structs to JSON and deserializing JSON to structs:

```js
// Serialization
user:User = User { name:`Bob`, age:25, email:`bob@example.com` };
json_str:s = json_to_string(user);

// Deserialization
deserialized_user:User = json_to_type(json_str);
```

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
    count > 0 => { print(`Positive`); },
    else => { print(`Non-positive`); }
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
        index >= 10 => { break; },
        else => { /* continue */ }
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
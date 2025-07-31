
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

## Data Types and constants

### 5.1 Type System

Nail uses a prefix-based type system:

- `i`: Integer
- `f`: Float
- `s`: String
- `b`: Boolean
- `a`: Array
- `e`: Error
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
count:f = safe(to_float(5),f(e):f { r 0.0; });  // Valid, handles error safely.
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

Key-value collections with type-safe keys and values:

```js
// Create a new hashmap with string keys and string values
user_scores:h<s,s> = hashmap_new();

// Hashmaps with different type combinations
int_map:h<s,i> = hashmap_new();      // String keys, integer values
struct_map:h<s,Point> = hashmap_new(); // String keys, struct values
bool_map:h<i,b> = hashmap_new();     // Integer keys, boolean values

// Hashmap operations
hashmap_insert(user_scores, `alice`, `100`);
hashmap_insert(user_scores, `bob`, `85`);

score:s = danger(hashmap_get(user_scores, `alice`));
has_charlie:b = hashmap_contains_key(user_scores, `charlie`);
map_size:i = hashmap_len(user_scores);

// Safe access with error handling
alice_score:s = safe(hashmap_get(user_scores, `alice`), f(err:s):s { r `0`; });

origin:Point = Point { x_pos: 0, y_pos: 0 };
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

### 6.1 Match Statements

Used for control flow instead of if-else. Has some extra intelligence so if an enum is iffed, the compiler will check if all enum values are covered, unless there is an else branch, etc.

```js
status:i = get_http_status_code(response);

if {
    status == 200 { print(`OK`) }
    status == 404 { print(`Not Found`) }
    else { print(`Unknown Status`) }
}
```

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
    r acc + num;
};

// Find maximum (with index access)
max_val:i = reduce acc num idx in numbers from danger(array_get(numbers, 0)) {
    r if { num > acc => { num }, else => { acc } };
};

// Build string
concatenated:s = reduce acc str in [`hello`, ` `, `world`] from `` {
    r acc + str;
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

#### Zip Operation

Combine two arrays element-wise:

```js
names:a:s = [`Alice`, `Bob`, `Charlie`];
ages:a:i = [30, 25, 35];

// Zip arrays together
pairs:a:Pair = zip name, age in names, ages {
    r Pair { name: name, age: age };
};
```

#### Take/Skip Operations

Take or skip a certain number of elements:

```js
// Take first 3 elements
first_three:a:i = take 3 from numbers;

// Skip first 2 elements
remaining:a:i = skip 2 from numbers;

// Take while condition is true
small_nums:a:i = take_while num in numbers {
    y num < 4;
};
```

### 6.3 For Loops

Traditional for loops for more complex iteration patterns:

```js
// Range iteration
for idx in 0..5 {
    print(danger(string_from(idx)));
};

// Inclusive range
for idx in 1..=3 {
    print(danger(string_from(idx * idx)));
};
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

// Nail with index (no comma)
indexed:a:s = map num idx in numbers {
    y danger(string_from(idx)) + ": " + danger(string_from(num));
};

// Filter operation (block with yield statement)
evens:a:i = filter num in numbers {
    y num % 2 == 0;
};

// Transpiles to Rust (map operation)
let indexed = {
    let mut __result = Vec::new();
    for (idx, num) in numbers.iter().enumerate() {
        __result.push(format!("{}: {}", idx, num));
    }
    __result
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
// - ALL collection operations use blocks with return statements for consistency
// - This maintains Nail's principle of explicit returns everywhere

zip_expression :=
    "zip" identifier "," identifier "in" expression "," expression block

take_expression :=
    "take" expression "from" expression

skip_expression :=
    "skip" expression "from" expression

take_while_expression :=
    "take_while" identifier [identifier] "in" expression block

for_loop :=
    "for" identifier "in" expression block

while_loop :=
    "while" expression ["from" expression] ["max" expression] block

block :=
    "{" statement* "}"

return_statement :=
    "r" expression ";"
yield_statement :=
    "y" expression ";"
```

## Return vs Yield Statements

Nail uses two different keywords for different contexts:

### Return Statements (`r`)
- Used in functions to exit and return a value
- Always exits the entire function
- Required in all functions (no implicit returns)

```js
f add(num_a:i, num_b:i):i {
    r num_a + num_b;  // Exits function and returns result
}
```

### Yield Statements (`y`)
- Used in collection operations to produce a value for that iteration
- Does NOT exit the function - only provides the value for the current iteration
- Required in all collection operation blocks

```js
// Yield produces a value for each iteration
doubled:a:i = map num in numbers {
    y num * 2;  // Yields doubled value for this iteration
};

// Yield produces a boolean condition
evens:a:i = filter num in numbers {
    y num % 2 == 0;  // Yields true/false for this iteration
};
```

**Key Difference**: `r` exits functions, `y` produces iteration values. Using `r` in collection operations or `y` in functions is a compile error.

## Functions

Functions that can fail must return a result type (using the `!e` syntax).

```js
f calculate_monthly_payment(principal:i, annual_rate:i, years:i):i!e {
    if (annual_rate == 0) {
        return e(`Annual rate cannot be zero`);
    }
    if (years <= 0) {
        return e(`Loan term must be positive`);
    }
    
    monthly_rate:f = expect(float_from(annual_rate)) / 12.0 / 100.0;
    payments:i = years * 12;
    
    // Division by zero check
    denominator:f = 1.0 - pow(1.0 + monthly_rate, -payments);
    if (denominator == 0.0) {
        return e(`Cannot calculate payment: invalid parameters resulted in division by zero`);
    }
    
    payment:f = to_float(principal) * monthly_rate / denominator;
    return string_from(payment); 
}
```

## Error Handling

Errors must be explicitly handled:

```js
user_input:s!e = lib_io_readline();
user_input:s = danger(lib_io_readline());

// OR safely handle the error

user_input:s!e = lib_io_readline();
user_input:s = safe(lib_io_readline(), (e):s { r `default value`; });

```

## Standard Library

Nail includes a comprehensive standard library:

- Core operations: `print`, `assert`, `len`, `get_index`, etc.
- `io_*`: Input/output operations
- `math_*`: Mathematical operations
- `http_*`: HTTP operations
- `fs_*`: File system operations

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
base_type := primitive_type | struct_type | enum_type | array_type | void_type | any_of_type
result_type := base_type "!" error_type
primitive_type := "i" | "f" | "s" | "b"
struct_type := "struct" | pascal_identifier
struct_field_type = primitive_type | enum_type | array_type
enum_type := pascal_identifier
array_type := "a" ":" base_type
void_type := "v"
any_of_type :="|" base_type ["|" base_type ["|" base_type]] "|"
error_type := "e"


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
    if_expression             // Conditional expression (e.g., `if a > b then ...`)
    match_expression          // Pattern matching (e.g., `match x { ... }`)
    block                     // A sequence of statements inside `{}` (e.g., `{ stmt1; stmt2 }`)
    for_loop                  // For loop construct (e.g., `for (i in 0..10) { ... }`)
    while_loop                // While loop construct (e.g., `while (condition) { ... }`)
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
   current_light == TrafficLight::Red => { println!(`Stop!`) },
   current_light == TrafficLight::Yellow => { println!(`Prepare to stop`) },
   current_light == TrafficLight::Green => { println!(`Go!`) }
}

// If you have an else branch, you don't need to cover all cases
if {
   current_light == TrafficLight::Red => { println!(`Stop!`) },
   else { println!(`It could be yellow or green...`) }
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
    "if" "{" if_branch {"," if_branch} ["else" block] "}"

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
        println(`Start of the week.`);
    },
    today == DaysOfWeek::Friday => {
        println(`End of the workweek!`);
    },
    else {
        println(`It's a regular day.`);
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
        println(`Stop!`);
    },
    current_light == TrafficLight::Yellow => {
        println(`Prepare to stop.`);
    },
    current_light == TrafficLight::Green => {
        println(`Go!`);
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
    today == DaysOfWeek::Monday => `Start of the week`,
    today == DaysOfWeek::Friday => `End of the workweek`,
    else => `It's a regular day`
}

// This will work because all branches return a string.
```

However, if branches return different types, Nail will produce an error:

```js
// Example where branches return different types (this will cause an error)
message:s = if {
    today == DaysOfWeek::Monday => `Start of the week`,  // String
    today == DaysOfWeek::Friday => 5,  // Integer
    else => `It's a regular day`  // String
}

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

// Void function (no return type specified)
f print_message(msg:s) {
    print(msg);
}

// Result type for error handling
f divide(a:i, b:i):i!e {
    if b == 0 {
        r err(`Division by zero`);
    };
    r ok(a / b);
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

Nail uses traditional loop constructs for iteration and data processing:

```js
numbers:a:i = [1, 2, 3, 4, 5];
doubled:a:i = [];

// Transform each element
for (idx:i in 0..len(numbers)) {
    num:i = get_index(numbers, idx);
    push(doubled, num * 2);
}

// Filter elements  
even_numbers:a:i = [];
for (num:i in numbers) {
    if (num % 2 == 0) {
        push(even_numbers, num);
    }
}

// Calculate sum
sum:i = 0;
for (num:i in numbers) {
    sum = sum + num;
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

### Example:

```js
// Fetch data from multiple sources simultaneously
p
    user_data:s = fetch_user_profile();
    posts:a:s = fetch_user_posts();
    notifications:i = get_notification_count();
/p

// All variables are available here after parallel execution completes
print(user_data);
print(from(notifications));
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
        last_name: safe(get_index(name_parts, 1), (e):s {r ``;}), // We could also do this to avoid a potential program error at this point.
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

The `safe` function allows you to handle potential errors inline:

```js
result:i = safe(
    potentially_failing_function(),
    f (err:e):i {
        print(`An error occurred: ` + err);
        r -1;  // Return a default value or handle the error appropriately
    }
);
```

#### Using `danger`

The `danger` function allows you to assert that a function will not fail, and if it does, it will return the error to start it propogating up the stack. The difference between `danger` and `expect` is that `danger` is used when the programmer acknowledges this should and can be made safe, and it should be made safe. This way you can easily find all dangerous parts of a program, and make them safe.

```js
result:i = danger(potentially_failing_function());
```

#### Using `expect`

The `expect` function is identical to danger, but with a different semantic meaning. It is an error so catastrophic, there is no point in not crashing the program if it fails. Used for errors that should never happen in a well-functioning program. For example, you may have a program that displays data from a CSV. Instead of using `safe` to handle the error, which would display no data to the user anyway, you would likely prefer to crash the program so you actually are aware there is a massive problematic error occuring, rather than give users a terrible experience of seeing no data at all, and not trip any monitoring systems. The choice is up to the programmer of when to use which.


### Best Practices

- **Be Specific**: When adding to error messages, be as specific as possible about what operation failed and why.

- **Provide Context**: Include relevant constant values or state information in error messages to aid debugging.

- **Use Descriptive Error Messages**: Make your error messages clear and actionable to help with debugging and maintenance.

By following these practices and leveraging Nail's error handling features, you can create robust, maintainable code that gracefully handles unexpected situations and provides clear, traceable error information when things go wrong.
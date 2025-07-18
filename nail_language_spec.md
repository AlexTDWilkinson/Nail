
# Nail Programming Language Overview

- Nail takes a lot of inspiration from this blog post: https://grugbrain.dev/

## Introduction

Nail is a programming language designed with a focus on simplicity, safety, and productivity. Its primary goal is to eliminate common sources of bugs and reduce cognitive load on developers by enforcing strict rules, a strict enviroment and by providing a consistent, straightforward syntax.

Nail can ONLY be written and transpiled in the Nail IDE, which only runs on Linux.

Nail programs are transpiled to async, parallellized (when specified) Rust and then compiled to native executables.

Nail programs often exhibit superior performance compared to typical Rust implementations, as Nail easily incorporates asynchronous, concurrent, and parallel paradigms — optimizations that many developers might not take the time to implement in typical Rust programs. However, it's important to note that a meticulously optimized Rust program can likely exceed Nail's performance, given that Nail is ultimately transpiled to Rust. Rest assured, Nail is fast.

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
- No mutability (except for hashmaps and the like).
- No classes, inheritance, or traditional OOP constructs.
- No manual memory allocation or management.
- No traditional loops (for, while, etc.), replaced with ranges and for each.
- No traditional if statements (replaced by a psuedo match/switch expressions).
- No function or operator overloading.
- No implicit returns.
- No floating-point comparisons without epsilon.
- No magic numbers (enforced use of named constants).
- No default values.
- No compiler warnings (only errors).
- No recursive functions.
- No direct array indexing (only safe functional operations).
- No optional syntax (consistent, deterministic structure).
- No nested functions (except lambdas).
- No tuples (named structs only).
- No method attachment to structs or enums.
- No generics.
- No macros or metaprogramming.
- No single letter constant names (must be descriptive)

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
x:i = 5; // This is an inline comment
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
// Everythin in nail is const.
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
count:f!e = to_float(5);  // Valid, creates a result type for error handling
count:f = dangerous(to_float(5));  // Valid, dangerousing the conversion, will yeet the error up the stack if it fails
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
    x:i,
    y:i
}

origin:Point = Point { x: 0, y: 0 };
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

### 6.2 Functional Iteration

ALWAYS used instead of traditional loops:

```js
numbers:a:i = [1, 2, 3, 4, 5];
for_each(numbers, |num:i|:v { print(`Number: ` + to_string(num)); });
```

## Functions

Functions that can fail must return a result type (using the `!e` syntax).

```js
function calculate_monthly_payment(principal:i, annual_rate:i, years:i):i!e {
    if (annual_rate == 0) {
        return e(`Annual rate cannot be zero`);
    }
    if (years <= 0) {
        return e(`Loan term must be positive`);
    }
    
    monthly_rate:f = to_float(annual_rate) / 12.0 / 100.0;
    payments:i = years * 12;
    
    // Division by zero check
    denominator:f = 1.0 - pow(1.0 + monthly_rate, -payments);
    if (denominator == 0.0) {
        return e(`Cannot calculate payment: invalid parameters resulted in division by zero`);
    }
    
    payment:f = to_float(principal) * monthly_rate / denominator;
    return to_int(payment); 
}
```

## Error Handling

Errors must be explicitly handled:

```js
user_input:s!e = lib_io_readline();
user_input:s = dangerous(lib_io_readline());

// OR safely handle the error

user_input:s!e = lib_io_readline();
user_input:s = safe(lib_io_readline(), |e|:s { r `default value`; });

```

## Namespaces and Modules

Each file has a namespace directive:

```js
[!namespace math]

public function sum_two_i(a:i, b:i): i {
    return a + b;
}

// In another file:
sum:i = math_sum_two_i(5, 3);
```

## Standard Library

Nail includes a comprehensive standard library:

- Core operations: `print`, `assert`, `map`, `filter`, `reduce`, etc.
- `lib_io`: Input/output operations
- `lib_math`: Mathematical functions
- `lib_http`: HTTP client
- `lib_fs`: File system access

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
any_of_type :="(" base_type ["|" base_type ["|" base_type]] ")"
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
    literal                     // A constant value (e.g., numbers, strings)
    | const                     // Accessing a constant (e.g., `PI`)
    | function_call             // Invoking a function (e.g., `foo(a, b)`)
    | binary_expression         // Binary operations (e.g., `a + b`)
    | unary_expression          // Unary operations (e.g., `-a`, `!b`)
    | if_expression             // Conditional expression (e.g., `if a > b then ...`)
    | match_expression          // Pattern matching (e.g., `match x { ... }`)
    | block                     // A sequence of statements inside `{}` (e.g., `{ stmt1; stmt2 }`)
    | loop                      // Looping construct (e.g., `loop { ... }`)
    | break                     // Breaks out of a loop (e.g., `break`)
    | continue                  // Skips to the next loop iteration (e.g., `continue`)
    | return                    // Returns a value from a function (e.g., `r x`)
    | assignment                // Assigning a value (e.g., `x = y`)
    | error_handling            // All errors must be handled explicitly


declaration :=
    const_decl                  // Declaring a constant (e.g., `pi = 3.14;`)
    | struct_decl               // Declaring a struct (e.g., `struct Point { ... }`)
    | enum_decl                 // Declaring an enum (e.g., `enum Days { ... }`)
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
    x:i,
    y:i
}
```

### Transpilation
    
```js
struct Point {
    x:i32,
    y:i32,
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

- Enums can be declared either mutably or immutably.
- When an enum is in the expression side of an if statement, all possible enum variants must be covered, unless there is an else branch. This allows simple refactoring when you want specifically ensure that all cases are covered.
- Enums in Nail are simple and don't support associated values, aligning with the language's simplicity principle.
- Enums, like other types, cannot be nested, maintaining the language's flat structure principle.
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
- All branches in an if expression must return the same type.
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

In Nail, function declarations are similar to Rust but with simplified generics. The basic syntax is:

```js
fn function_name(param_name:Type, another_param:Type):Type {
    // Function body
}
```

Example:

```js
fn add(a:i, b:i):i {
   r a + b;
}
```

### Function Parameters

In Nail, function parameters must always be named, unless the name of the constant being passed is an exact match to the parameter name. This encourages clear and self-documenting function calls.

```js
fn greet(name:s) {
    print(`Hello, ` + name + `!`);
}

user_name:s = `Alice`;
greet(name:user_name);  // Explicitly named parameter
greet(user_name);        // Allowed because constant name matches parameter name
```

### Anonymous Functions / Lambdas

Nail supports anonymous functions (lambdas), INSIDE FUNCTION CALLS ONLY.
Note lambadas can only be used inside functions. They cannot be assigned to constants or passed around.
They must just be defined as regular functions if they need to be reused.

```js
// NOT ALLOWED.
multiply:i = |x:i, y:i|:i { r x * y; };

// ALLOWED. Normal function declaration of same thing.
fn multiply(x:i, y:i):i {
   r x * y;
}

// NOT ALLOWED. You cannot use a lambda inside a function declaration. You should make the lambda a seperate function and call it.
fn multiply(x:i, y:i):i {
  r |x:i, y:i|:i { r x * y; }
}

// ALLOWED. You can call functions inside other functions, as long as they are not lambdas in the declaration.
fn multiply_example(x:i, y:i):i {
 one_to_five:a:i = [1, 2, 3, 4, 5];
 x:a:i = map(one_to_five, |num:i|:i { r num * 2; }); // Allowed, because the lambda is inside a function call.
  r multiply(x, y);
}

// ALLOWED - note the lambda is inside a function call.
data:a:i = [1, 2, 3, 4, 5];
multiplied_array:a:i = map(data, |x:i, y:i|:i { r x * y; });
```

The lambda syntax in Nail is `|parameters|:return_type { body }`. This clearly specifies the input parameters, return type, and the function body.

### Higher-order Functions

Nail supports higher-order functions, allowing functions to be passed as arguments or returned from other functions:

```js
fn apply_i(f:fn(i):i, x:i):i {
   r f(x);
}

result:i = apply_i(|x:i|:i { r x * 2; }, 5);
```

### Generics

Nail simplifies generics by only supporting the `any_of` type.
It is only allowed in library functions. It is not allowed in user-defined functions.

The type checker will ensure that the type the type being passed in is one of the types specified in the `any_of` type.

Using this lets us avoid monomorphizing the library functions, which would be a huge pain -

For example instead of simply map(), you would have combinatorial explosion like map_ai_i(), map_af_i(), etc for many functions.

```js
fn generic_function(x:(i|f)):i {
    // Function body
}
```

## Functional Iteration

Nail uses functional paradigms for iteration instead of traditional loops. Each operation is performed separately, as there is no function chaining in Nail.

### Map

The `map` function transforms each element of a collection:

```js
numbers:a:i = [1, 2, 3, 4, 5];
doubled:a:i = map(numbers, |x:i|:i { r x * 2; });  // [2, 4, 6, 8, 10]
// Note they can only accept identifiers, not expressions.
// THIS IS NOT ALLOWED (it lacks clear type information, does a lot of steps at once, would take time to break into chunks for debug printing, and is not very "naily").
doubled:a:i = map([1, 2, 3, 4, 5], |x:i|:i { r x * 2; }); 
```

### Filter

The `filter` function selects elements based on a predicate:

```js
even_numbers:a:i = filter(numbers, |x:i|:b { r x % 2 == 0; });  // [2, 4]
```

### Reduce

The `reduce` function combines all elements into a single value:

```js
sum:i = reduce(numbers, 0, |acc:i, x:i|:i { r acc + x; });  // 15
```

All the other typical ones will be present in the standard library as well.

### Combining Operations

Since Nail doesn't support function chaining, operations must be performed step by step:

```js
numbers:a:i = [1, 2, 3, 4, 5];
even_numbers:a:i = filter(numbers, |num:i|:b { r num % 2 == 0; });
squared_evens:a:i = map(even_numbers, |num:i|:i { r num * num; });
sum_of_squared_evens:i = reduce(squared_evens, 0, |acc:i, num:i|:i { r acc + num; });
```

This approach, while more verbose than chaining, provides clarity and allows for intermediate results to be easily inspected or used elsewhere in the code.


## Parallel Blocks

Nail's parallel blocks allow you to execute multiple operations concurrently, automatically leveraging async/await patterns when transpiled to Rust.

### Syntax

```js
parallel {
    // Each statement runs concurrently
    task1:s = expensive_operation();
    task2:i = fetch_from_api();
    print(`Processing in parallel!`);
    calculation:i = compute_result();
}
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
parallel {
    user_data:s = fetch_user_profile();
    posts:a:s = fetch_user_posts();
    notifications:i = get_notification_count();
}

// All variables are available here after parallel execution completes
print(user_data);
print(to_string(notifications));
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

fn map_user_input_to_record(input:UserInput, id:i):UserRecord {
   name_parts:a:s = split(input.full_name, ` `);
    r UserRecord {
        id:id,
        first_name: dangerous(get_index(name_parts, 0)), // This could error but we're just going to dangerous it.
        last_name: safe(get_index(name_parts, 1), |e|:s {r ``;}), // We could also do this to avoid a potential program error at this point.
        email:input.email,
        age:input.age
    }
}

// Usage
input:UserInput = UserInput { full_name: `John Doe`, email: `john@example.com`, age: 30 };
record:UserRecord = map_user_input_to_record(input, 1);
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

These additional features and examples demonstrate how Nail can work with complex data structures and transformations, despite its limitations on nested structs. By providing these utilities and patterns, Nail enables developers to handle various data scenarios while maintaining its core principle of simplicity.

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
    |e| { 
        print(`An error occurred: ` + e);
        r -1;  // Return a default value
    }
);
```

#### Using `dangerous`

The `dangerous` function allows you to assert that a function will not fail, and if it does, it will return the error to start it propogating up the stack.

```js
result:i = dangerous(potentially_failing_function());
```

### Best Practices

- **Be Specific**: When adding to error messages, be as specific as possible about what operation failed and why.

- **Provide Context**: Include relevant constant values or state information in error messages to aid debugging.

- **Use Descriptive Error Messages**: Make your error messages clear and actionable to help with debugging and maintenance.

By following these practices and leveraging Nail's error handling features, you can create robust, maintainable code that gracefully handles unexpected situations and provides clear, traceable error information when things go wrong.
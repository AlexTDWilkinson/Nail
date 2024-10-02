
# Nail Programming Language Overview

- Nail takes a lot of inspiration from this blog post: https://grugbrain.dev/

## Introduction

Nail is a programming language designed with a focus on simplicity, safety, and productivity. Its primary goal is to eliminate common sources of bugs and reduce cognitive load on developers by enforcing strict rules, a strict enviroment and by providing a consistent, straightforward syntax.

Nail can ONLY be written and transpiled in the Nail IDE, which only runs on Linux.

Nail programs are transpiled to async, parallellized, concurrent Rust and then compiled to native executables.

Nail programs often exhibit superior performance compared to typical Rust implementations, as Nail automatically incorporates asynchronous, concurrent, and parallel paradigms — optimizations that many developers might not take the time to implement in typical Rust programs. However, it's important to note that a meticulously optimized Rust program can likely exceed Nail's performance, given that Nail is ultimately transpiled to Rust. Rest assured, Nail is fast.

## Core Design Principles

Nail adheres to the following core principles:

- Simplicity: The language includes only essential features, avoiding complexity.
- Safety: Strong typing and strict rules prevent common programming errors.
- Productivity: Consistent syntax and built-in best practices enhance developer efficiency.
- Explicitness: The language favors explicit declarations over implicit behavior.

## Language Restrictions

To achieve its goals, Nail imposes the following restrictions:

- Limited data types: integer, float, string, boolean, array, struct, and enum.
- No user-level async/concurrency/parallelism syntax, all is transpiled into async concurrent Rust.
- No package manager or external dependencies (The standard library is updated with every new version of Nail)
- No uninitialized variables (variables must be defined with a value)
- No null references.
- No mutability by default (must be explicitly initialized as mutable).
- No global variables.
- No classes, inheritance, or traditional OOP constructs.
- No manual memory allocation or management.
- No traditional loops (for, while, etc.).
- No traditional if statements (replaced by a psuedo match/switch expressions).
- No function or operator overloading.
- No implicit returns.
- No floating-point comparisons without epsilon.
- No magic numbers (enforced use of named constants).
- No default values.
- No compiler warnings (only errors).
- No recursive functions.
- No nested arrays or hashmaps.
- No direct array indexing (only iteration and safe functional operations).
- No optional syntax (consistent, deterministic structure).
- No nested functions (except lambdas).
- No tuples (named structs only).
- No method attachment to structs or enums.
- No nested structs or enums.
- No generics (except the 'any' type used for the standard library)
- No macros or metaprogramming.
- No single letter variable names.
- No returns that are not identifiers (must be const/variable name or function call).

## Lexical Structure

### 4.1 Keywords

Reserved keywords in Nail:

```
Meh, there's a bunch, see the EBNF file in this repo for specifics.
```

### 4.2 Identifiers

Identifiers follow snake_case convention:

```js
my_variable
calculate_total
```

### 4.3 Comments

Single-line comments only, preceded by `//`:

```js
// This is a comment
c x:i = 5; // This is an inline comment
```

### 4.4 Literals

- Integer literals: `42`, `-7`
- Floating-point literals: `3.14`, `-0.001`
- String literals: `"hello"`, `"nail is awesome"`
- Boolean literals: `true`, `false`

## Data Types and Variables

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

### 5.2 Variable Declaration

Variables must include type and initialization:

```js
// c is for const. There is also v for variable.
c age:i = 30;
c name:s = "Grug";
c is_developer:b = true;
```

### 5.3 Type Checking and Conversion

Strict type checking is enforced:

```js
c count:i = 5;  // Valid
c count:i = 6.0;  // Error: Can't assign float to integer
c count:f = 6.0;  // Valid
c count:f|e = to_float(5);  // Valid, will result in result type for error handling
c count:f = trust(to_float(5));  // Valid, trusting the conversion, will yeet the error up the stack if it fails
```

### 5.4 Composite Types

#### 5.4.1 Arrays

Homogeneous, non-nested collections:

```js
c names:a:s = ["Alice", "Bob", "Charlie"];
```

#### 5.4.2 Structs

Custom data types with named fields:

```js
struct Point {
    x:i,
    y:i
}

c origin:Point = Point { x: 0, y: 0 };
```

#### 5.4.3 Enums

Fixed set of possible values (no associated data):

```js
enum TrafficLight {
    Red,
    Yellow,
    Green
}

c current_light:TrafficLight = TrafficLight::Red;
```

## Control Structures

### 6.1 Match Statements

Used for control flow instead of if-else. Has some extra intelligence so if an enum is iffed, the compiler will check if all enum values are covered, unless there is an else branch, etc.

```js
c status:i = get_http_status_code(response);

if {
    status == 200 { print("OK") }
    status == 404 { print("Not Found") }
    else { print("Unknown Status") }
}
```

### 6.2 Functional Iteration

ALWAYS used instead of traditional loops:

```js
c numbers:a:i = [1, 2, 3, 4, 5];
each(numbers, |num|:_ {r print("Number: " + to_string(num));});
```

## Functions

Functions that can fail must return an optional error type.
Also assertion blocks that will return an error when any of the assertions fail.

```js
function calculate_monthly_payment(principal:i, annual_rate:i, years:i):i|e {
    c monthly_rate:i = annual_rate / 12 / 100;
    c payments:i = years * 12;
    c payment:f = principal * monthly_rate / (1 - (1 + monthly_rate).pow(-payments));
    return to_int(payment:f); 
};
```

## Error Handling

Errors must be explicitly handled:

```js
c user_input:s|e = lib_io_readline();
c user_input:s = if {
    is_string_type(user_input:s|e) { return trust(user_input:s|e); },
    is_error_type(user_input:s|e) { return `Use this default string instead`; },
    else { return e(`Unexpected error in input reading`); }
};
```

## Namespaces and Modules

Each file has a namespace directive:

```js
[!namespace math]

public function sum_two_i(a:i, b:i): i {
    return a + b;
}

// In another file:
c sum:i = math_sum_two_i(5, 3);
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


<br>

## Type System and Declarations


### EBNF
```ebnf
// Types
type := base_type ["|" "e"]
base_type := primitive_type | struct_type | enum_type | array_type | void_type | any_of_type
result_type := base_type "|" error_type
primitive_type := "i" | "f" | "s" | "b"
struct_type := "struct" | pascal_identifier
struct_field_type = primitive_type | enum_type | array_type
enum_type := pascal_identifier
array_type := "a" ":" base_type
void_type := "v"
any_of_type :="(" base_type ["|" base_type ["|" base_type]] ")"
error_type := "e"


// Declarations
struct_decl := "struct" pascal_identifier "{" struct_field "," struct_field "}" ";"
struct_field := snake_identifier ":" struct_field_type ";"
enum_decl := "enum" pascal_identifier "{" enum_variant "," enum_variant "}" ";"
enum_variant := pascal_identifier
const_decl := "c" snake_identifier ":" type "=" expression ";"
variable_decl := "v" snake_identifier ":" type "=" expression ";"
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

<br>

## Expressions and declarations

### EBNF

```js
expression :=
    literal                     // A constant value (e.g., numbers, strings)
    | variable                  // Accessing a variable (e.g., `x`, `y`)
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
    | error_handling            // Handling errors (e.g., `try { ... } catch { ... }`)


declaration :=
    const_decl                  // Declaring a constant (e.g., `const pi = 3.14;`)
    | variable_decl             // Declaring a variable (e.g., `var x = 10;`)
    | struct_decl               // Declaring a struct (e.g., `struct Point { ... };`)
    | enum_decl                 // Declaring an enum (e.g., `enum Days { ... };`)
```
<br>

## Struct

### EBNF

```js
struct_decl := "struct" pascal_identifier "{" struct_field "," struct_field "}" ";"
struct_field := snake_identifier ":" struct_field_type ";"
struct_field_type = primitive_type | enum_type | array_type
```


### Nail:

```js
struct Point {
    x:i,
    y:i,
};
```

### Transpilation
    
```js
struct Point {
    x:i32,
    y:i32,
}
```

<br>

## Enums

### EBNF

```js
enum_decl := "enum" identifier "{" enum_variant {"," enum_variant} "}" ";"
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
    Green,
}

enum DaysOfWeek {
    Monday,
    Tuesday,
    Wednesday,
    Thursday,
    Friday,
    Saturday,
    Sunday,
}

// Usage
c current_light:TrafficLight = TrafficLight::Red;
v today:DaysOfWeek = DaysOfWeek::Wednesday;

// if statement that must cover all enum cases since it has no else branch
if {
   current_light == TrafficLight::Red => { println!(`Stop!`) };
   current_light == TrafficLight::Yellow => { println!(`Prepare to stop`) };
   current_light == TrafficLight::Green => { println!(`Go!`) };
}

// If you have an else branch, you don't need to cover all cases
if {
   current_light == TrafficLight::Red => { println!(`Stop!`) };
   else { println!(`It could be yellow or green...`) };
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

<br>

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
};
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
};
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

One important aspect of if expressions in Nail is that all branches must return the same type. This ensures consistency, especially when the result of the if expression is used in a larger context (e.g., assigned to a variable or returned from a function).

```js
// Example where all branches return the same type (in this case, a string)
c message = if {
    today == DaysOfWeek::Monday => `Start of the week`,
    today == DaysOfWeek::Friday => `End of the workweek`,
    else => `It's a regular day`
};

// This will work because all branches return a string.
```

However, if branches return different types, Nail will produce an error:

```js
// Example where branches return different types (this will cause an error)
c message = if {
    today == DaysOfWeek::Monday => `Start of the week`,  // String
    today == DaysOfWeek::Friday => 5,  // Integer
    else => `It's a regular day`  // String
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
- All branches in an if expression must return the same type.
- Nail transpiles directly to equivalent Rust if statements, preserving the logic and structure.

<br>



## Const (`c`) vs Variable (`v`) Declarations

In Nail, there are two primary ways to declare values: using `c` for constants and `v` for variables. These declarations have distinct characteristics and use cases. Both use snake_case for identifiers.

### Const Declarations (`c`)

Const declarations are used for immutable values. They are defined using the `c` keyword.

```js
c pi:f = 3.14159;
c max_users:i = 100;
c greeting:s = `Hello, World!`;
```

Key points about const declarations:
- They are immutable.
- To change a const value, you must use shadowing (redeclaration with the same name).
- Identifiers use snake_case, same as variable declarations (otherwise changing all the names for minor refactoring would be painful).

Example of shadowing:

```js
c user_count:i = 5;
// Later in the code
c user_count:i = 6;  // This shadows the previous declaration
```

### Variable Declarations (`v`)

Variable declarations are used for mutable values. They are defined using the `v` keyword.

```js
v users:Users = Users::new();
v current_day:DaysOfWeek = DaysOfWeek::Monday;
v scores:a:i = [10, 20, 30];
```

Key points about variable declarations:
- <B>In Nail, all mutable values for each scope are placed in a DashMap to allow concurrency and parallelism by default. This is how we get circumvent Rust's borrow checker to run all Nail code as
parallel and concurrent by default.</B>
- Identifiers for variables also use snake_case, the same as const declarations.

### Comparison and Usage

- **Type Restrictions:**
   - `c` can be used with all types.
   - `v` can be used with all types, except for result types (e.g., `i|e`).

- **Mutability:**
   - `c` declarations are immutable, requiring shadowing for changes.
   - `v` declarations are mutable, allowing direct modifications. (They cannot be shadowed? Unsure atm. They can be in Rust but that may be hard for us to do)

- **Naming Convention:**
   - Both `c` and `v` declarations use snake_case for identifiers.
   - Const identifiers are not written in all caps, unlike some other languages.

### Examples

Const with shadowing:

```js
c max_attempts:i = 3;
c max_attempts:i = 5;  // Shadowing the previous declaration
c max_attempts:s = `Three`; // Shadows can even change the type (like Rust)
```

Variable with direct modification:

```js
v current_day:DaysOfWeek = DaysOfWeek::Monday;
current_day = DaysOfWeek::Tuesday;  // Direct modification

v user_scores:a:i = [75, 80, 85];
trust(update(scores, 0, 78));
// this will cause the error to be propogated up the stack and either handled in a parent scope 
// or panic if not handled.
trust(update(scores, 1, 85));
// This is a safe version.
// In the event the update fails, the error will be printed ("handled"), and the program will continue.
fail_safe(update(scores, 1, 85), |e| {r print(e);});

// if you want to handle the result explicitly with an if statement or something.
// Also note the type "v" is for 'void". It is used for functions that return no value at all. 
// Generally only library functions that perform side-effects return void.
// Assigning void to any variable will propogate the error up the stack.
c scores:v|e = update(scores, 0, 78);

if {
    // if any expression in an if statement contains a result type, all result outcomes must be handled.
    is_ok(scores) => { r println(`Failed to update scores`); }
    is_error(scores) => { r println(`Scores updated successfully`);},
}


v some_variable:i = fail_safe(func_than_could_fail(x), |e| {print(e); r 5;}); 
some_variable = 10; // Direct modification

// THIS IS NOT ALLOWED. IT IGNORES/DISPOSES OF AN UNHANDLED RESULT VALUE.
v some_variable:i|e = func_than_could_fail(x); 
some_variable = 10; // This would send the errror value to the void

// To enforce dealing with the result value, must be either:
c some_variable:i|e = func_than_could_fail(x); // note const declaration
// or 
v some_variable:i = fail_safe(func_than_could_fail(x), |e| {print(e); r 5;}); // note variable declaration
```

### Key Takeaways

- Use `c` for immutable values.
- Use `v` for mutable values.
- Result values can not be assigned to mutable variables, because it would permit ignoring errors through reassignment.
- An actual error value itself, as in "e" can be assigned to a const or variable - it's just a regular value.
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

In Nail, function parameters must always be named, unless the name of the variable being passed is an exact match to the parameter name. This encourages clear and self-documenting function calls.

```js
fn greet(name:s) {
    print(`Hello, ` + name + `!`);
}

c user_name = `Alice`;
greet(name:user_name);  // Explicitly named parameter
greet(user_name);        // Allowed because variable name matches parameter name
```

### Anonymous Functions / Lambdas

Nail supports anonymous functions (lambdas), INSIDE FUNCTION CALLS ONLY.
Note lambadas can only be used inside functions. They cannot be assigned to variables or passed around.
They must just be defined as regular functions if they need to be reused.

```js
// NOT ALLOWED.
c multiply:i = |x:i, y:i|:i { r x * y; };

// ALLOWED. Normal function declaration of same thing.
fn multiply(x:i, y:i):i {
   r x * y;
}

// NOT ALLOWED. You cannot use a lambda inside a function declaration. You should make the lambda a seperate function and call it.
fn multiply(x:i, y:i):i {
  r |x:i, y:i|:i { r x * y; };
}

// ALLOWED. You can call functions inside other functions, as long as they are not lambdas in the declaration.
fn multiply_example(x:i, y:i):i {
  c one_to_five:a:i = [1, 2, 3, 4, 5];
  c x = map(one_to_five, |num:i|:i { r num * 2; }); // Allowed, because the lambda is inside a function call.
  r multiply(x, y);
}

// ALLOWED - note the lambda is inside a function call.
c data:a:i = [1, 2, 3, 4, 5];
c multiplied_array:a:i = map(data, |x:i, y:i|:i { r x * y; });
```

The lambda syntax in Nail is `|parameters|:return_type { body }`. This clearly specifies the input parameters, return type, and the function body.

### Higher-order Functions

Nail supports higher-order functions, allowing functions to be passed as arguments or returned from other functions:

```js
fn apply_i(f:fn(i):i, x:i):i {
   r f(x);
}

c result = apply_i(fn: |x:i|:i { r x * 2; }, to: 5);
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
c numbers:a:i = [1, 2, 3, 4, 5];
c doubled:a:i = map(numbers, |x:i|:i { r x * 2; });  // [2, 4, 6, 8, 10]
// Note they can only accept identifiers, not expressions.
// THIS IS NOT ALLOWED (it lacks clear type information, does a lot of steps at once, would take time to break into chunks for debug printing, and is not very "naily").
c doubled:a:i = map([1, 2, 3, 4, 5], |x:i|:i { r x * 2; }); 
```

### Filter

The `filter` function selects elements based on a predicate:

```js
c even_numbers:a:i = filter(numbers, |x:i|:b { r x % 2 == 0; });  // [2, 4]
```

### Reduce

The `reduce` function combines all elements into a single value:

```js
c sum:i = reduce(numbers, 0, |acc:i, x:i|:i { r acc + x; });  // 15
```

All the other typical ones will be present in the standard library as well.

### Combining Operations

Since Nail doesn't support function chaining, operations must be performed step by step:

```js
c numbers:a:i = [1, 2, 3, 4, 5];
c even_numbers:a:i = filter(numbers, |num:i|:b { r num % 2 == 0; });
c squared_evens:a:i = map(even_numbers, |num:i|:i { r num * num; });
c sum_of_squared_evens:i = reduce(squared_evens, 0, |acc:i, num:i|:i { r acc + num; });
```

This approach, while more verbose than chaining, provides clarity and allows for intermediate results to be easily inspected or used elsewhere in the code.


## Rust Blocks and Nail Escapes

Nail provides a mechanism to incorporate Rust code directly within Nail code using Rust blocks. This feature allows developers to leverage Rust's capabilities while working primarily in Nail.

### Rust Blocks

Rust blocks are denoted by the `R{ }` syntax. any Rust code can be placed inside these blocks.

```js
R{
    // anything in here is transpiled literally to the Rust code.
    // It is simply injected.
    // Even this comment would survive the transpilation.
    println!("This is Rust code!");
}
```

### Nail Escapes within Rust Blocks

Within Rust blocks, you can inject Nail expressions using the `^[ ]^` syntax. This allows you to seamlessly integrate Nail values into your Rust code.

```js
c name:s = `Alice`;
c age:i = 30;

R{
    println!("Hello, ^[name]^! You are ^[age]^ years old.");
}
```

In this example, `^[name]^` and `^[age]^` will be replaced with the values of the Nail variables `name` and `age` respectively.

### Usage in Function Definitions

Rust blocks are particularly useful for defining library functions that require Rust's capabilities:

Note '_' can be used if the function only takes a single un-named parameter. This is useful for common functions like print,
where the name of the function would also typically be the name of the parameter.

```js


fn do_something_in_rust_to_a_nail_integer(input:i):i {
    R{
        match input {
                NailDataType::Int(value) => value * 2,
                _ => panic!("Expected an integer from Nail, but saw something different. This error should never occur, because Nail's type checker should've caught it in the Nail IDE. If you're seeing this, something has gone very wrong!"),
        }
    }
}


c every_nail_type:any_of = (i|f|s|b|a:i|a:f|a:struct|a:enum);

fn do_something_in_rust_to_any_kind_in_nail(input:every_nail_type):i {
    // parameters using the any_of type, like 'input' will always be seen as NailDataType<T> in Rust, which represents all the possible Nail types.
R{  
    match input {
        NailDataType::Int(value) => value * 2
        _ => 19,
        }
    }
}

// or alternatively, you can just declare the function signature in Nail, and define it completely in Rust.
// the '_' is transpiled as a parameter named "input" on the Rust side.

!DIRECT_RUST_TRANSLATION
fn print(_:every_nail_type):v;
fn print<T>(input: NailDataType<T>) {
    match input {
        NailDataType::Int(value) => println!("{}", value),
        NailDataType::Float(value) => println!("{}", value),
        NailDataType::String(value) => println!("{}", value),
        NailDataType::ArrayInt(value) => println!("{:?}", value),
        NailDataType::ArrayFloat(value) => println!("{:?}", value),
        NailDataType::Struct(value) => println!("{:?}", value),
        NailDataType::Enum(value) => println!("{:?}", value),
        NailDataType::Bool(value) => println!("{}", value),
    }
}
```

### Key Points

- Rust blocks allow direct inclusion of Rust code within Nail.
- Nail escapes (`^[ ]^`) inside Rust blocks enable integration of Nail expressions. The Nail expressions ARE NOT EVALUATED, but it provides the Nail lexer and parser to understand the Nail code within the Rust block, which can be very useful for catching bugs.
- Rust blocks are primarily used for library functions and advanced features.
- Everyday Nail users typically won't need/want to use Rust escapes directly.
- Rust blocks can be used to define complex functions or operations that require Rust's full capabilities.

### Examples

Basic Rust block with Nail escape:

```js
c user_name:s = "Bob";
R{ println!("Welcome, ^[user_name]^!"); }
```

Function using Rust block for implementation:

```js
fn calculate_sum(a:i, b:i):i {
    // Note you do not have to use Nail escapes in Rust blocks,
    // for example here we don't escape "a" or "b".
    // This works and is fine, but it is invisible to the Nail parser,
    // so 'intellisense' and error checking inside the Nail IDE will not work on those variables.
    c sum:i = a + b;
    R{ 
        println!("The sum of {} and {} is {}", a, b, sum);
        sum
    }
}
```

This approach allows Nail to leverage Rust's power while maintaining its own simplified syntax for most use cases. It also allows us to build Nail libraries around existing powerful Rust libraries.


## Structs

### Mapping Structs

Nail keeps things simple by not supporting nested structs, to encourage flat data structures wherever possible. Nested structs can have considerable mental overhead. If you absolutely need to. You can still work with complex data transformations using mapping functions that are similar to relational database type operations.

```js
struct UserInput {
    full_name:s,
    email:s,
    age:i,
};

struct UserRecord {
    id:i,
    first_name:s,
    last_name:s,
    email:s,
    age:i,
};

fn map_user_input_to_record(input:UserInput, id:i):UserRecord {
    c name_parts:a:s = split(input.full_name, " ");
    r UserRecord {
        id:id,
        first_name: trust(get_index(name_parts, 0)); // This could error but we're just going to trust it.
        last_name: fail_safe(get_index(name_parts, 1), |e|:s {r "";}); // We could also do this to avoid a potential program error at this point.
        email:input.email,
        age:input.age,
    };
}

// Usage
c input = UserInput { full_name: "John Doe", email: "john@example.com", age: 30 };
c record = map_user_input_to_record(input, 1);
```

### De-nesting JSON to Flat Structs

Nail provides utility functions to transform nested JSON structures into flat structs. This is particularly useful when working with complex external data that needs to be simplified for use within Nail.

```js
struct FlatUser {
    id:i,
    name:s,
    email:s,
    address_street:s,
    address_city:s,
    address_country:s,
};

// A Nail programmer generally wouldn't be programming things like this, they'd
// just use a library function, as seen below. This is just an example.
fn denest_json_to_flat_user(json:s):FlatUser {
    R{
        let json_value:serde_json::Value = serde_json::from_str(^[json]^).unwrap();
        FlatUser {
            id:json_value["id"].as_i64().unwrap() as i32,
            name:json_value["name"].as_str().unwrap().to_string(),
            email:json_value["email"].as_str().unwrap().to_string(),
            address_street:json_value["address"]["street"].as_str().unwrap().to_string(),
            address_city:json_value["address"]["city"].as_str().unwrap().to_string(),
            address_country:json_value["address"]["country"].as_str().unwrap().to_string(),
        }
    }
}

// Usage
c json_str = "{\"id\": 1, \"name\": \"John Doe\", \"email\": \"john@example.com\", \"address\": {\"street\": \"123 Main St\", \"city\": \"Anytown\", \"country\": \"USA\"}}";

// The standard library will have functions that just do this for you.
// This function will return all JSON fields that match the struct fields,
// and return default values for any fields that are missing.
c flat_user:FlatUser = trust(flatten_json_to(struct: FlatUser, json_string: json_str, allow_missing_fields: true));
```

### Re-nesting Flat Structs to JSON

Conversely, Nail also provides functionality to transform flat structs back into nested JSON structures. This is useful when you need to export data from Nail to systems expecting nested JSON.

```js
// A Nail programmer generally wouldn't be programming things like this, they'd
// just use a library function, as seen below. This is just an example.
fn nest_flat_user_to_json(user:FlatUser):s {
    R{
        let json_value = serde_json::json!({
            "id": ^[user.id]^,
            "name": ^[user.name]^,
            "email": ^[user.email]^,
            "address": {
                "street": ^[user.address_street]^,
                "city": ^[user.address_city]^,
                "country": ^[user.address_country]^
            }
        });
        json_value.to_string()
    }
}

// Usage
c nested_json:s = nest_flat_user_to_json(flat_user);

// The standard library will have functions that just do this for you.
c nested_json:s = to_json(flat_user);

```

### No Dynamic Struct Fields

While Nail doesn't support dynamic struct fields directly, you can use arrays to represent structures with variable fields. Although you can do this, though it's not very "Naily" and not recommended. The library will likely include a HashMap type function that will do this type of thing.

### Struct Serialization and Deserialization

Nail provides built-in functions for serializing structs to JSON and deserializing JSON to structs:

```js
// Serialization
c user:User = { name:"Bob", age:25, email:"bob@example.com" };
c json_str:s = to_json(user);

// Deserialization
c deserialized_user:User = flatten_json_to(json_str);
```

These additional features and examples demonstrate how Nail can work with complex data structures and transformations, despite its limitations on nested structs. By providing these utilities and patterns, Nail enables developers to handle various data scenarios while maintaining its core principle of simplicity.

## Error Handling in Nail

Nail implements a unique error handling mechanism that promotes robust and maintainable code by ensuring errors are explicitly handled or intentionally propagated. This approach combines aspects of Rust's `Result` type with automatic error propagation and comprehensive error tracking.

### Error Propagation and Accumulation

In Nail, when a function encounters an error, it automatically propagates up the call stack until it reaches a point where it's explicitly handled. This process continues through any intermediate functions that don't explicitly handle or return error types. 

Crucially, as the error propagates, Nail captures and accumulates error information at each step. The error type in Nail holds a string that continually gets appended to, creating a "story" of what happened to cause the errors. This provides a detailed trace of the error's journey through the program.

#### Basic Error Propagation and Accumulation

Consider the following example:

```js
fn read_file(path:s):s|e {
    // Imagine this function could fail if the file doesn't exist
    // If it fails, it might return an error like:
    // e(Unable to read file at path: /some/path)
}

fn process_file_contents(contents:s):s|e {
    // This function could fail during processing
    // If it fails, it might return an error like:
    // e(Failed to process file contents: Invalid format)
}

fn display_processed_contents(processed:s):s {
    print(processed);
}

fn get_files_and_display_contents():v {
    c result:v|e = fail_safe(
        {
            c file_contents = read_file("/some/path");
            c processed_contents = process_file_contents(file_contents);
            r display_processed_contents(processed_contents);
        },
        |e|:e {
            // Return the error to start the error propagation
            r e(Something went wrong while trying to get files and display their contents);

            // alternatively you could handle the error at this point by returning an approrpriate expected value like:
            // r v; 
            // though in this case that would make no sense and would just result in 
            // error propogation starting anyway as you cannot assign a void type to a variable.
            }
    );
}
```


In this example, if `read_file` fails, the error will propagate through `process_file_contents` and `display_processed_contents`, accumulating context along the way. The final error message might look like:

```
## Error in fn process_file_contents: Failed to process file contents: Invalid format ##

It caused a panic because it was unhandled by the time it reached fn get_files_and_display_contents: Something went wrong while trying to get files and display their contents
   ->  It was seen by fn read_file: Unable to read file at path: /some/path
    ->  The error originated in fn process_file_contents: Failed to process file contents: Invalid format
```

This comprehensive error message provides a clear, chronological trail of how the error propagated through the program, starting from the root cause and showing each function it passed through. This format makes it easier for developers to trace the error's path and understand the sequence of events that led to the failure.


### Handling Errors

Nail provides several ways to handle errors:

#### Using `fail_safe`

The `fail_safe` function allows you to handle potential errors inline:

```js
c result:i = fail_safe(
    potentially_failing_function(),
    |e| { 
        print("An error occurred: " + e);
        r -1;  // Return a default value
    }
);
```

#### Using `trust`

The `trust` function allows you to assert that a function will not fail, and if it does, it will return the error to start it propogating up the stack.

```js
c result:i = trust(potentially_failing_function());
```

### Explicit Handling with If Statements

You can also handle errors explicitly using if statements:

```js
c result:i|e = potentially_failing_function();
if {
    is_ok(result) => { print("Success: " + result); },
    is_error(result) => { print("Error: " + result); },
}
```

### Best Practices

- **Be Specific**: When adding to error messages, be as specific as possible about what operation failed and why.

- **Provide Context**: Include relevant variable values or state information in error messages to aid debugging.

- **Handle Errors Appropriately**: Choose the right error handling mechanism based on the severity and recoverability of the error.

- **Don't Swallow Errors**: Avoid ignoring errors without proper handling or logging.

- **Use Descriptive Error Messages**: Make your error messages clear and actionable to help with debugging and maintenance.

By following these practices and leveraging Nail's error handling features, you can create robust, maintainable code that gracefully handles unexpected situations and provides clear, traceable error information when things go wrong.
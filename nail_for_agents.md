# Nail for coding agents

This page is the shortest complete briefing on Nail. It exists to be loaded
into a model's context whole: run `nail agents` in a project to write it into
AGENTS.md, paste it into a CLAUDE.md, or fetch it from
https://nail.alex-wilkinson.ca/llms.txt. Locally, `nail docs primer` prints
it. Nail is small enough that everything a code generator must
know fits on this one page. If a construct is not here, the language does not
have it.

Nail is a simple language that transpiles to Rust. Everything is immutable,
every declaration carries its type, there is no null, and a program that
ignores an error does not compile. One file, top to bottom, is one program.

## The feedback loop

The first line of every Nail file is a version line: `nail latest` while you
are writing, or `nail 0.3.1` to pin. The compiler refuses a file without one.

- `nail check file.nail` type checks and prints `ok` or the errors
- `nailc file.nail --check-only --json` is the same answer as one line of
  JSON: status, stage, and each error's file, line, column, and help text
- `nail run file.nail` compiles quickly and runs, rebuilds are sub-second
- `nail build file.nail` does the full release build and leaves the binary
  beside the source
- `nail test` runs every Nail file under tests/, a failing program is a
  failing test
- `nail docs array_sort` explains one library function with a runnable
  example, `nail docs errors` prints a section of the specification,
  bare `nail docs` lists everything
- `nailc --docs-json` is the whole library registry as JSON
- `nailc fmt file.nail` formats a file in place

Errors are worth reading in full: they name the identifier, say what was
expected and what was found, and usually carry a `help:` line with the fix.

## The rules that are different

These are the traps for a generator that assumes Python, JavaScript, or Rust:

- No reassignment and no mutation, ever. `count = 2;` on an existing name is
  refused. Declaring the name again (shadowing) is legal. Accumulation across
  a collection is what `reduce` and `scan` are for.
- No `for` loop, no `while` loop, no `break`, no `continue`. `each` walks a
  collection, `map`, `filter`, `reduce` and `scan` build one, `forever` runs
  until the program ends (servers, background work), and a function that calls itself
  repeats until something changes.
- No lambdas and no closures. A callback is an ordinary named function,
  declared at the top level and passed by name.
- A function body sees only its parameters. There are no globals inside
  functions, so anything a function needs is passed in.
- No bare expression statements. A call that returns a value must be
  assigned. Only void calls like `print(...)` can stand alone.
- Strings use backticks. `+` adds numbers only, text is joined with
  `string_concat([a, b])` or `array_join(parts, separator)`.
- No `arr[0]`. Indexing is `array_get(numbers, 0)`, which returns a result
  because the index may be out of bounds.
- Types are one letter: `i` integer, `f` float, `s` string, `b` boolean,
  `v` void, `e` error. Compounds read left to right: `a:i` array of
  integers, `h<s,i>` hashmap, `i!e` an integer or an error.
- Declarations are `name:type = value;`. The type is not optional.
- `f` declares a function, `r` returns from one, `y` yields one element
  inside a collection operation. `r` and `y` are not interchangeable.
- `if` is an expression with match-like arms: `if { cond -> { ... }, else ->
  { ... } }`. There is no bare `if cond {`.
- Struct literals name every field with `=`, and every field is required:
  `Point { x_pos = 3, y_pos = 4 }`. Enums are `Direction::North`.
- No single-letter identifiers (`x`, `y`, `z`, `w` are allowed, so
  coordinates can be honest). No tuples, no generics, no classes, no
  methods on structs, no macros, no operator overloading, no null.

## The whole language in one program

```nail
nail latest
// Every construct Nail has, on one screen. If it is not here, the language
// does not have it.
//
// The short tokens, once: f declares a function, r returns, y yields one
// element to a collection operation, p opens a parallel block (real threads),
// c opens a concurrent block (overlapped waiting, one thread). The type
// letters: i integer, f float, s string, b boolean, a array, h hashmap,
// e error, v void.

struct Point {
    x_pos:i,
    y_pos:i
}

enum Direction {
    North,
    South
}

f add_coordinates(point:Point):i {
    r point.x_pos + point.y_pos;
}

f half_of(num:i):i!e {
    if {
        num % 2 == 1 -> { r e(`odd numbers do not halve cleanly`); },
        else -> { r num / 2; }
    }
}

f return_zero_if_error(err:e):i {
    r 0;
}

f announce(message:s):v {
    print(message);
}

f first_square_past(limit:i, candidate:i):i {
    if {
        candidate * candidate > limit -> { r candidate; },
        else -> { r first_square_past(limit, candidate + 1); }
    }
}

// Everything is immutable, and every declaration carries its type
coordinate_total:i = add_coordinates(Point { x_pos = 3, y_pos = 4 });
ratio:f = 2.5;
markup:s = html`<b>a tagged string tells highlighters its language</b>`;
is_ready:b = true && !false;
heading:Direction = Direction::North;

// The three ways out of an error type: handle it, crash on it, or insist
half:i = safe(half_of(coordinate_total), return_zero_if_error);
crash_if_odd:i = danger(half_of(2));
promised_even:i = expect(half_of(4));

// if is an expression too
heading_name:s = if {
    heading == Direction::North -> { r `north`; },
    else -> { r `south`; }
};

// Arrays and hashmaps are the collections
numbers:a:i = [1, 2, 3, 4, 5];
ages:h<s,i> = hashmap_new();
hashmap_set(ages, `grug`, 30);
grug_age:i = danger(hashmap_get(ages, `grug`));

// map, filter and reduce are the loops, and they run on every core
squares:a:i = map num in numbers { y num * num; };
evens:a:i = filter num in squares { y num % 2 == 0; };
total:i = reduce acc num in evens from 0 { y acc + num; };

// scan keeps every step, find, all and any answer questions, each is for
// side effects, and any of them can also take an index iterator
running:a:i = scan acc num in numbers from 0 { y acc + num; };
first_even:i = danger(find num index in numbers { y num % 2 == 0; });
all_positive:b = all num in numbers { y num > 0; };
any_negative:b = any num in numbers { y num < 0; };
each num in numbers { print(num); }

// a search that stops is a function that calls itself until it has the answer
print(first_square_past(50, 0));

// forever runs until the program ends: servers, watchers, heartbeats.
// Nothing runs behind the program's back, so a forever function runs in a
// c block beside whatever else lives as long as the program. This one is
// declared and not called, so the tour itself can end
f heartbeat():v {
    forever {
        announce(`still here`);
        time_sleep(60.0);
    }
}

// p gives each statement its own thread, for work that computes
p
    left:i = array_sum(array_range_inclusive(1, 1000));
    right:i = array_sum(array_range_inclusive(1, 2000));
/p

// c overlaps waiting on one thread, for reads, requests and sleeps, so the
// block costs its slowest wait instead of the sum of them
c
    time_sleep(0.01);
    time_sleep(0.01);
/c

// import splices in another file, sandboxed so it can only compute, and
// import_dangerous splices one in with the sandbox off. Each needs a second
// file, so they are the one pair not shown running on this screen.

print(total + left + right + grug_age + first_even + half);
```

## What the compiler refuses on purpose

Reassignment. Declare a new name, or shadow the old one with a fresh
declaration:

```nail-refused
count:i = 1;
count = 2;
```

A while loop. Use `each`, `forever`, or a collection operation:

```nail-refused
while true {
    print(`spin`);
}
```

A for loop. `each` walks a collection, with the index if wanted:

```nail-refused
for num in numbers {
    print(num);
}
```

A discarded value. If a call returns something, the program must catch it:

```nail-refused
array_length([1, 2, 3]);
```

An unhandled error. `array_get` returns `i!e`, and `i!e` is not `i` until
`safe`, `danger`, or `expect` has said what happens on the error path:

```nail-refused
numbers:a:i = [1, 2, 3];
first:i = array_get(numbers, 0);
```

## Finding library functions

The standard library is large (more than a thousand functions in over eighty
libraries) and there is no package manager: what ships with the compiler is
the whole ecosystem. Names are `library_verb`: `string_split`, `array_sort`,
`fs_read`, `http_server_start`, `json_get_string`, `db_query`. When unsure a
function exists, ask the compiler rather than guessing: `nail docs <word>`
searches names and descriptions, and every entry's example is a complete
program that compiles.

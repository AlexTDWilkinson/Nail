# Error message golden tests

Every `.nail` file in this directory is a program that is SUPPOSED to fail
compilation. Its sibling `.stderr` file is the exact diagnostic the compiler
must print, byte for byte.

Run with `./scripts/test_error_messages.sh` (from the repo root). After
changing a diagnostic on purpose, regenerate goldens with
`./scripts/test_error_messages.sh --bless` and READ the new `.stderr` files
before committing. A golden is a human-approved promise about error quality,
not just captured output.

Every message must follow the error style guide in
`nail_language_spec.md` ("Error Message Style Guide"): plain-language
problem statement, caret-underlined source line, real values/types named,
and a `help:` suggestion when a fix is knowable.

When you add a new diagnostic to the compiler, add a test here that
triggers it. One file per diagnostic, named after the mistake the user
made (e.g. `undefined_variable.nail`), kept as small as possible.

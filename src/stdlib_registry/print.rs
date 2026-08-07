//! Print module stdlib registry entries.

use super::*;

pub(super) fn register(m: &mut HashMap<&'static str, StdlibFunction>) {
    simple_fns! { m, Print:
        "print" => "print_macro!", (message: T) -> v,
            "Prints a value to stdout followed by a newline.",
            "print(`Hello, World!`);";
        "print_no_newline" => "std_lib::print::print_no_newline", (message: T) -> v,
            "Prints a value to stdout without a trailing newline.",
            "print_no_newline(`Loading... `);";
        "print_clear_screen" => "std_lib::print::print_clear_screen", () -> v,
            "Clears the terminal screen and moves the cursor to the top left.",
            "print_clear_screen();";
        "print_debug" => "std_lib::print::print_debug", (value: T) -> v,
            "Prints a value in expanded debug format, useful for structs and arrays.",
            "struct Person { name:s, age:i }\n\nperson:Person = Person { name = `Ada`, age = 36 };\nprint_debug(person);";
        "print_error" => "std_lib::print::print_error", (message: T) -> v,
            "Prints a value to stderr followed by a newline, so a note about the run does not land in the answer the run produced. For anything with a level or a timestamp, use the log module instead.",
            "print_error(`warning: low disk space`);";
    }
}

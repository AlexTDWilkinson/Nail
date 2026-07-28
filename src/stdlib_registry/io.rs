//! IO module stdlib registry entries - reading from stdin.

use super::*;

pub(super) fn register(m: &mut HashMap<&'static str, StdlibFunction>) {
    simple_fns! { m, IO:
        "io_read_line" => "std_lib::io::read_line", () -> (s!e),
            "Reads a line from stdin (without the trailing newline); errors if stdin is closed.",
            "name:s = danger(io_read_line());";
        "io_read_line_prompt" => "std_lib::io::read_line_prompt", (prompt: s) -> (s!e),
            "Prints a prompt, then reads a line from stdin.",
            "name:s = danger(io_read_line_prompt(`Name: `));";
        "io_read_int" => "std_lib::io::read_int", () -> (i!e),
            "Reads a line from stdin and parses it as an integer.",
            "age:i = danger(io_read_int());";
        "io_read_int_prompt" => "std_lib::io::read_int_prompt", (prompt: s) -> (i!e),
            "Prints a prompt, then reads an integer from stdin.",
            "age:i = danger(io_read_int_prompt(`Age: `));";
        "io_read_float" => "std_lib::io::read_float", () -> (f!e),
            "Reads a line from stdin and parses it as a float.",
            "price:f = danger(io_read_float());";
        "io_read_float_prompt" => "std_lib::io::read_float_prompt", (prompt: s) -> (f!e),
            "Prints a prompt, then reads a float from stdin.",
            "price:f = danger(io_read_float_prompt(`Price: `));";
    }
}

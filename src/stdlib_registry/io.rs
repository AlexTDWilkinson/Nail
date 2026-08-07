//! IO module stdlib registry entries - reading from stdin.

use super::*;

pub(super) fn register(m: &mut HashMap<&'static str, StdlibFunction>) {
    simple_fns! { m, IO:
        "io_read_all" => "std_lib::io::read_all", () -> (s!e),
            "Reads all of standard input to the end - what `cat data | program` hands over, and what makes a program usable in a pipe.",
            "piped:s = danger(io_read_all());";
        "io_is_piped" => "std_lib::io::is_piped", () -> b,
            "Whether standard input is a pipe or a file rather than a person typing. Check this to decide between reading input and prompting for it.",
            "from_a_pipe:b = io_is_piped();";
        "io_read_line" => "std_lib::io::read_line", () -> (s!e),
            "Reads a line from stdin (without the trailing newline). Errors if stdin is closed.",
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
        "io_confirm" [Tokio] => "std_lib::io::confirm", (question: s, default_answer: b) -> (s!e),
            "Asks a yes-or-no question until it gets an answer it understands, returning `yes` or `no`. An empty line means the default.",
            "answer:s = danger(io_confirm(`Deploy to production?`, false));";
        "io_select" [Tokio] => "std_lib::io::select", (question: s, options: [s]) -> (i!e),
            "Shows a numbered list and asks until one is picked, returning the index of the chosen option.",
            "environments:a:s = [`staging`, `production`];\nchosen:i = danger(io_select(`Which environment?`, environments));";
        "io_read_secret" [Tokio, Crossterm] => "std_lib::io::read_secret", (prompt: s) -> (s!e),
            "Reads a line with nothing shown as it is typed, for a password or a token pasted into a terminal.",
            "password:s = danger(io_read_secret(`Password: `));";
        "io_read_line_or" [Tokio] => "std_lib::io::read_line_or", (prompt: s, default_answer: s) -> (s!e),
            "Reads a line, returning the default when nothing is typed, so a setup script can be answered by holding down return.",
            "host:s = danger(io_read_line_or(`Database host`, `localhost`));";
    }
}

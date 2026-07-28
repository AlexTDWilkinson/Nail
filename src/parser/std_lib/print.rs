use std::fmt::{Debug, Display};
use std::io::{self, Write};

/// Invert the escaping `{:?}` applies to strings (char::escape_debug): turn
/// \n, \t, \r, \", \', \\ and \u{...} sequences back into their real
/// characters so printed strings match what the program built.
pub fn unescape_debug_string(escaped: &str) -> String {
    let mut output = String::with_capacity(escaped.len());
    let mut chars = escaped.chars();
    while let Some(current) = chars.next() {
        if current != '\\' {
            output.push(current);
            continue;
        }
        match chars.next() {
            Some('n') => output.push('\n'),
            Some('t') => output.push('\t'),
            Some('r') => output.push('\r'),
            Some('0') => output.push('\0'),
            Some('u') => {
                // \u{XXXX}
                let mut code = String::new();
                for hex_char in chars.by_ref() {
                    if hex_char == '{' {
                        continue;
                    }
                    if hex_char == '}' {
                        break;
                    }
                    code.push(hex_char);
                }
                if let Some(character) = u32::from_str_radix(&code, 16).ok().and_then(char::from_u32) {
                    output.push(character);
                }
            }
            Some(other) => output.push(other),
            None => output.push('\\'),
        }
    }
    output
}

/// Print macro wrapper that handles any number of arguments
#[macro_export]
macro_rules! print_macro {
    ($($arg:expr),*) => {
        {
            let mut _first = true;
            $(
                if !_first {
                    print!(" ");
                }
                let formatted = format!("{:?}", $arg);
                // Strings arrive Debug-escaped in quotes; strip the quotes and
                // undo the escaping so the real text is printed
                let output = if formatted.starts_with('"') && formatted.ends_with('"') && formatted.len() > 1 {
                    $crate::std_lib::print::unescape_debug_string(&formatted[1..formatted.len()-1])
                } else {
                    formatted.replace("\\n", "\n")
                };
                print!("{}", output);
                _first = false;
            )*
            println!();
        }
    };
}

/// Print with newline (aliased as "print" for convenience)
pub async fn print<T>(value: T)
where
    T: Debug
{
    let formatted = format!("{:?}", value);
    // Strings arrive Debug-escaped in quotes; strip the quotes and undo the
    // escaping so the real text is printed
    let output = if formatted.starts_with('"') && formatted.ends_with('"') && formatted.len() > 1 {
        unescape_debug_string(&formatted[1..formatted.len()-1])
    } else {
        formatted.replace("\\n", "\n")
    };
    println!("{}", output);
}

/// Print without newline
pub async fn print_no_newline<T: Display>(value: T) {
    print!("{}", value);
    // Flush to ensure output appears immediately
    let _ = io::stdout().flush();
}

/// Print with debug format for complex types
pub async fn print_debug<T: std::fmt::Debug>(value: T) {
    println!("{:#?}", value);
}

/// Clear the terminal screen
pub async fn print_clear_screen() {
    print!("\x1B[2J\x1B[1;1H");
    let _ = io::stdout().flush();
}
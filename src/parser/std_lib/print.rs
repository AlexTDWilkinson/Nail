use std::fmt::{Debug, Display};
use std::io::{self, Write};

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
                // For strings, remove surrounding quotes and replace \n with actual newlines
                let output = if formatted.starts_with('"') && formatted.ends_with('"') && formatted.len() > 1 {
                    let without_quotes = &formatted[1..formatted.len()-1];
                    without_quotes.replace("\\n", "\n")
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
    // For strings, remove surrounding quotes and replace \n with actual newlines
    let output = if formatted.starts_with('"') && formatted.ends_with('"') && formatted.len() > 1 {
        let without_quotes = &formatted[1..formatted.len()-1];
        without_quotes.replace("\\n", "\n")
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
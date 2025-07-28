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
                print!("{:?}", $arg);
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
    println!("{:?}", value);
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
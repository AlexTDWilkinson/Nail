use std::fmt::Display;
use std::io::{self, Write};

/// Print with newline (aliased as "print" for convenience)
pub fn print<T: Display>(value: T) {
    println!("{}", value);
}

/// Print without newline
pub fn print_no_newline<T: Display>(value: T) {
    print!("{}", value);
    // Flush to ensure output appears immediately
    let _ = io::stdout().flush();
}

/// Print with debug format for complex types
pub fn print_debug<T: std::fmt::Debug>(value: T) {
    println!("{:#?}", value);
}

/// Clear the terminal screen
pub fn print_clear_screen() {
    print!("\x1B[2J\x1B[1;1H");
    let _ = io::stdout().flush();
}
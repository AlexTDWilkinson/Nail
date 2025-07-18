pub mod common;
pub mod checker;
pub mod colorizer;
pub mod formatter;
pub mod lexer;
pub mod parser;
pub mod transpilier;
pub mod statics_for_tests;
pub mod stdlib_registry;
pub mod stdlib_types;

pub use common::{CodeError, CodeSpan};

// Re-export formatter functions
pub use formatter::format_nail_code;

// Re-export std_lib for easier access
pub use parser::std_lib;
pub mod common;
pub mod checker;
#[cfg(not(target_arch = "wasm32"))]
pub mod colorizer;
pub mod embedded;
pub mod formatter;
pub mod lexer;
pub mod parser;
pub mod prof;
pub mod transpiler;
pub mod statics_for_tests;
pub mod stdlib_registry;
#[cfg(not(target_arch = "wasm32"))]
pub mod toolchain;

pub use common::{CodeError, CodeSpan};

// Re-export formatter functions
pub use formatter::format_nail_code;

// Re-export std_lib for easier access
pub use parser::std_lib;

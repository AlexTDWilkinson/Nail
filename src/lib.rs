pub mod common;
pub mod checker;
#[cfg(not(target_arch = "wasm32"))]
pub mod colorizer;
pub mod docs;
pub mod embedded;
/// The compiler's own fuzzer. Behind a feature because it is a development
/// tool: nothing a shipped compiler does needs it, and a default build should
/// not carry it.
#[cfg(all(feature = "fuzz", not(target_arch = "wasm32")))]
pub mod fuzz;
pub mod formatter;
#[cfg(not(target_arch = "wasm32"))]
pub mod keymap;
pub mod lexer;
pub mod parser;
pub mod version_line;
pub mod prof;
#[cfg(not(target_arch = "wasm32"))]
pub mod threads;
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

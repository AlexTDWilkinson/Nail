pub mod http;
pub mod math;
pub mod string;
pub mod time;
pub mod array;
pub mod array_functional;
pub mod env;
pub mod process;
pub mod path;
pub mod error;
pub mod int;
pub mod float;
pub mod hashmap;
pub mod io;
pub mod print;

// Re-export HTTP functions so they're available from the transpiler
pub use http::{http_server_start, http_server_route};

// Re-export array functional functions
pub use array_functional::{reduce_struct};

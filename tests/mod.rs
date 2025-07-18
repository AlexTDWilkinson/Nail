// Test module declarations for Nail language tests

mod test_framework;
mod test_lexer;
mod test_parser;
mod test_checker;
mod test_boolean;

// Re-export the test framework for use in other modules
pub use test_framework::*;
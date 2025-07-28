/// Error handling functions for Nail
///
/// Note: These are placeholder implementations. The actual behavior is handled
/// specially by the transpiler since Nail's error handling works differently
/// than Rust's Result type.

/// The safe function handles potential errors by executing an error handler if the result is an error
/// In Nail, this prevents the immediate panic that would normally occur with error types
///
/// Usage: safe(divide(10, 0), |e| { 0 })  // Returns 0 if divide fails
/// The error handler receives the error message as a parameter
pub async fn safe<T, E>(value: Result<T, E>, error_handler: impl FnOnce(E) -> T) -> T {
    match value {
        Ok(v) => v,
        Err(e) => error_handler(e),
    }
}

/// The danger function asserts that a function will not fail
/// If it does fail, it allows the error to panic immediately (Nail's default behavior)
/// This is intended for temporary use and should be replaced with safe() later
///
/// Usage: danger(divide(10, 2))  // Panics if divide fails
pub async fn danger<T, E: std::fmt::Display>(value: Result<T, E>) -> T {
    value.unwrap_or_else(|e| panic!("🔨 Nail Error: {}", e))
}

/// The expect function is semantically identical to danger but with different intent
/// Use expect() when you believe the operation will never fail, or when failure would
/// make the program useless. Unlike danger(), expect() is not meant to be replaced later
///
/// Usage: expect(read_config())  // Panics if config is missing (program can't run without it)
pub async fn expect<T, E: std::fmt::Display>(value: Result<T, E>) -> T {
    value.unwrap_or_else(|e| panic!("🔨 Nail Error: {}", e))
}

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
pub fn safe<T, E>(value: Result<T, E>, error_handler: impl FnOnce(E) -> T) -> T {
    match value {
        Ok(v) => v,
        Err(e) => error_handler(e),
    }
}

/// The dangerous function asserts that a function will not fail
/// If it does fail, it allows the error to panic immediately (Nail's default behavior)
/// This is intended for temporary use and should be replaced with safe() later
/// 
/// Usage: dangerous(divide(10, 2))  // Panics if divide fails
pub fn dangerous<T, E: std::fmt::Display>(value: Result<T, E>) -> T {
    value.unwrap_or_else(|e| panic!("🔨 Nail Error: {}", e))
}

/// The expect function is semantically identical to dangerous but with different intent
/// Use expect() when you believe the operation will never fail, or when failure would
/// make the program useless. Unlike dangerous(), expect() is not meant to be replaced later
/// 
/// Usage: expect(read_config())  // Panics if config is missing (program can't run without it)
pub fn expect<T, E: std::fmt::Display>(value: Result<T, E>) -> T {
    value.unwrap_or_else(|e| panic!("🔨 Nail Error: {}", e))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_safe_with_ok_result() {
        let result: Result<i32, &str> = Ok(42);
        let value = safe(result, |_| 0);
        assert_eq!(value, 42);
    }

    #[test]
    fn test_safe_with_err_result() {
        let result: Result<i32, &str> = Err("test error");
        let value = safe(result, |e| {
            assert_eq!(e, "test error");
            -1
        });
        assert_eq!(value, -1);
    }

    #[test]
    fn test_safe_with_complex_error_handler() {
        let result: Result<String, i32> = Err(404);
        let value = safe(result, |error_code| {
            format!("Error code: {}", error_code)
        });
        assert_eq!(value, "Error code: 404");
    }

    #[test]
    fn test_dangerous_with_ok_result() {
        let result: Result<i32, &str> = Ok(100);
        let value = dangerous(result);
        assert_eq!(value, 100);
    }

    #[test]
    #[should_panic(expected = "🔨 Nail Error: test panic")]
    fn test_dangerous_with_err_result() {
        let result: Result<i32, &str> = Err("test panic");
        dangerous(result);
    }

    #[test]
    fn test_expect_with_ok_result() {
        let result: Result<String, &str> = Ok("success".to_string());
        let value = expect(result);
        assert_eq!(value, "success");
    }

    #[test]
    #[should_panic(expected = "🔨 Nail Error: critical failure")]
    fn test_expect_with_err_result() {
        let result: Result<i32, &str> = Err("critical failure");
        expect(result);
    }

    #[test]
    fn test_all_functions_with_different_types() {
        // Test with float
        let float_ok: Result<f64, String> = Ok(3.14);
        assert_eq!(safe(float_ok, |_| 0.0), 3.14);

        // Test with boolean
        let bool_ok: Result<bool, i32> = Ok(true);
        assert_eq!(dangerous(bool_ok), true);

        // Test with vector
        let vec_ok: Result<Vec<i32>, &str> = Ok(vec![1, 2, 3]);
        assert_eq!(expect(vec_ok), vec![1, 2, 3]);
    }

    #[test]
    fn test_error_handler_closure_capture() {
        let default_value = 999;
        let result: Result<i32, &str> = Err("test");
        let value = safe(result, |_| default_value);
        assert_eq!(value, default_value);
    }
}
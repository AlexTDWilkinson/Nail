// Integer conversion and utility functions

// Convert string to integer
pub fn int_from(s: String) -> Result<i64, String> {
    s.parse::<i64>().map_err(|e| e.to_string())
}

// Absolute value
pub fn abs(x: i64) -> i64 {
    x.abs()
}

// Minimum of two values
pub fn min(a: i64, b: i64) -> i64 {
    if a < b { a } else { b }
}

// Maximum of two values
pub fn max(a: i64, b: i64) -> i64 {
    if a > b { a } else { b }
}

// Power function
pub fn pow(base: i64, exp: u32) -> i64 {
    base.pow(exp)
}
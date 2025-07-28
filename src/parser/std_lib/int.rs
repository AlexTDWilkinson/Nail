// Integer conversion and utility functions
use std::fmt::Display;

// Convert value to integer
pub async fn from<T: Display>(v: T) -> Result<i64, String> {
    v.to_string().parse::<i64>().map_err(|e| e.to_string())
}

// Power function
pub async fn pow(base: i64, exp: u32) -> i64 {
    base.pow(exp)
}

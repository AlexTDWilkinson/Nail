use std::fmt::Display;

// Float conversion functions. Float arithmetic (abs, sqrt, round, etc.) lives
// in std_lib::math - there is exactly one way to do each operation.

// Convert a value to a float
pub async fn from<T: Display>(v: T) -> Result<f64, String> {
    v.to_string().parse::<f64>().map_err(|e| e.to_string())
}

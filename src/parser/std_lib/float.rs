use std::fmt::Display;

// Float conversion and utility functions

// Convert a value to a float
pub async fn from<T: Display>(v: T) -> Result<f64, String> {
    v.to_string().parse::<f64>().map_err(|e| e.to_string())
}

// Absolute value
pub async fn abs(x: f64) -> f64 {
    x.abs()
}

// Square root
pub async fn sqrt(x: f64) -> f64 {
    x.sqrt()
}

// Power function
pub async fn pow(base: f64, exp: f64) -> f64 {
    base.powf(exp)
}

// Round to nearest integer
pub async fn round(x: f64) -> f64 {
    x.round()
}

pub async fn round_to_int(x: f64) -> i64 {
    x.round() as i64
}

// Floor function
pub async fn floor(x: f64) -> f64 {
    x.floor()
}

// Ceiling function
pub async fn ceil(x: f64) -> f64 {
    x.ceil()
}

// Minimum of two values
pub async fn min(a: f64, b: f64) -> f64 {
    if a < b {
        a
    } else {
        b
    }
}

// Maximum of two values
pub async fn max(a: f64, b: f64) -> f64 {
    if a > b {
        a
    } else {
        b
    }
}

// Random number between 0 and 1
pub async fn random() -> f64 {
    rand::random::<f64>()
}

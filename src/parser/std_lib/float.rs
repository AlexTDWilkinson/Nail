// Float conversion and utility functions

// Convert string to float
pub fn float_from(s: String) -> Result<f64, String> {
    s.parse::<f64>().map_err(|e| e.to_string())
}

// Absolute value
pub fn abs(x: f64) -> f64 {
    x.abs()
}

// Square root
pub fn sqrt(x: f64) -> f64 {
    x.sqrt()
}

// Power function
pub fn pow(base: f64, exp: f64) -> f64 {
    base.powf(exp)
}

// Round to nearest integer
pub fn round(x: f64) -> f64 {
    x.round()
}

// Floor function
pub fn floor(x: f64) -> f64 {
    x.floor()
}

// Ceiling function
pub fn ceil(x: f64) -> f64 {
    x.ceil()
}

// Minimum of two values
pub fn min(a: f64, b: f64) -> f64 {
    if a < b { a } else { b }
}

// Maximum of two values
pub fn max(a: f64, b: f64) -> f64 {
    if a > b { a } else { b }
}

// Random number between 0 and 1
pub fn random() -> f64 {
    rand::random::<f64>()
}
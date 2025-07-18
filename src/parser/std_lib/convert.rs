use std::fmt::Display;

// Generic to_string that works with any type that implements Display
pub fn to_string<T: Display>(value: T) -> String {
    format!("{}", value)
}

pub fn to_int(s: String) -> Result<i64, String> {
    s.parse::<i64>().map_err(|e| e.to_string())
}

pub fn to_float(s: String) -> Result<f64, String> {
    s.parse::<f64>().map_err(|e| e.to_string())
}
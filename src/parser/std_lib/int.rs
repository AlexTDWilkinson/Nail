// Integer conversion and utility functions
use std::fmt::Display;

// Convert value to integer
pub async fn from<T: Display>(v: T) -> Result<i64, String> {
    v.to_string().parse::<i64>().map_err(|_| format!("int_from: could not parse '{}' as an integer", v))
}

// Integer power with overflow and negative-exponent checks
pub async fn pow(base: i64, exp: i64) -> Result<i64, String> {
    if exp < 0 {
        return Err(format!("int_pow: exponent cannot be negative, got {}", exp));
    }
    let exp = u32::try_from(exp).map_err(|_| format!("int_pow: exponent {} is too large", exp))?;
    base.checked_pow(exp).ok_or_else(|| format!("int_pow: {}^{} overflows a 64-bit integer", base, exp))
}

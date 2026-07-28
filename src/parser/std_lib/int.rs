// Integer conversion and utility functions
use std::fmt::Display;

// Convert value to integer
pub async fn from<T: Display>(v: T) -> Result<i64, String> {
    v.to_string().parse::<i64>().map_err(|e| e.to_string())
}

// Integer power with overflow and negative-exponent checks
pub async fn pow(base: i64, exp: i64) -> Result<i64, String> {
    if exp < 0 {
        return Err(format!("int_pow exponent cannot be negative: {}", exp));
    }
    let exp = u32::try_from(exp).map_err(|_| format!("int_pow exponent too large: {}", exp))?;
    base.checked_pow(exp).ok_or_else(|| format!("int_pow overflow: {}^{}", base, exp))
}

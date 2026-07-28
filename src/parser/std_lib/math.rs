// Generic math functions for Nail
use std::cmp::{Ord, Ordering};
use std::ops::Neg;

// Generic min function - returns minimum of two values
pub async fn min<T>(a: T, b: T) -> T
where
    T: PartialOrd,
{
    if a <= b {
        a
    } else {
        b
    }
}

// Generic max function - returns maximum of two values
pub async fn max<T>(a: T, b: T) -> T
where
    T: PartialOrd,
{
    if a >= b {
        a
    } else {
        b
    }
}

// Generic clamp function - clamps value between min and max
// (PartialOrd rather than Ord so it works for floats too)
pub async fn clamp<T>(value: T, min_val: T, max_val: T) -> T
where
    T: PartialOrd,
{
    if value < min_val {
        min_val
    } else if value > max_val {
        max_val
    } else {
        value
    }
}

// Generic sign function - returns -1, 0, or 1
pub async fn sign<T>(value: T) -> i64
where
    T: Ord + Default,
{
    let zero = T::default();
    match value.cmp(&zero) {
        Ordering::Less => -1,
        Ordering::Equal => 0,
        Ordering::Greater => 1,
    }
}

// Generic absolute value (PartialOrd rather than Ord so it works for floats)
pub async fn abs<T>(value: T) -> T
where
    T: PartialOrd + Default + Neg<Output = T>,
{
    let zero = T::default();
    if value < zero {
        -value
    } else {
        value
    }
}

// Square root
pub async fn sqrt(x: f64) -> f64 {
    x.sqrt()
}

// Power function
pub async fn pow(base: f64, exp: f64) -> f64 {
    base.powf(exp)
}

// Round to nearest whole number, as a float
pub async fn round(x: f64) -> f64 {
    x.round()
}

// Round to nearest whole number, as an integer
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

// Random float in [0.0, 1.0)
pub async fn random() -> f64 {
    rand::random::<f64>()
}

// The constant pi
pub async fn pi() -> f64 {
    std::f64::consts::PI
}

// Euler's number
pub async fn e() -> f64 {
    std::f64::consts::E
}

// Division with a divide-by-zero check
pub async fn divide<T>(numerator: T, denominator: T) -> Result<T, String>
where
    T: std::ops::Div<Output = T> + Default + PartialEq,
{
    if denominator == T::default() {
        return Err("Division by zero".to_string());
    }
    Ok(numerator / denominator)
}

// Greatest common divisor (Euclidean algorithm)
pub async fn gcd(mut a: i64, mut b: i64) -> i64 {
    a = a.abs();
    b = b.abs();
    while b != 0 {
        let temp = b;
        b = a % b;
        a = temp;
    }
    a
}

// Least common multiple
pub async fn lcm(a: i64, b: i64) -> i64 {
    let a_abs = a.abs();
    let b_abs = b.abs();
    if a_abs == 0 || b_abs == 0 {
        return 0;
    }
    (a_abs / gcd(a_abs, b_abs).await) * b_abs
}

// Factorial
pub async fn factorial(n: i64) -> Result<i64, String> {
    if n < 0 {
        return Err("Factorial is not defined for negative numbers".to_string());
    }
    if n > 20 {
        return Err("Factorial too large, would overflow".to_string());
    }
    
    let mut result = 1i64;
    for i in 2..=n {
        result = result.checked_mul(i)
            .ok_or_else(|| "Factorial overflow".to_string())?;
    }
    Ok(result)
}

// Check if number is prime
pub async fn is_prime(n: i64) -> bool {
    if n <= 1 {
        return false;
    }
    if n <= 3 {
        return true;
    }
    if n % 2 == 0 || n % 3 == 0 {
        return false;
    }
    
    let mut i = 5;
    while i * i <= n {
        if n % i == 0 || n % (i + 2) == 0 {
            return false;
        }
        i += 6;
    }
    true
}

// Sigmoid function (useful for ML)
pub async fn sigmoid(x: f64) -> f64 {
    1.0 / (1.0 + (-x).exp())
}

// Linear interpolation
pub async fn lerp(a: f64, b: f64, t: f64) -> f64 {
    a + (b - a) * t.clamp(0.0, 1.0)
}

// Trigonometric functions
pub async fn sin(x: f64) -> f64 {
    x.sin()
}

pub async fn cos(x: f64) -> f64 {
    x.cos()
}

pub async fn tan(x: f64) -> f64 {
    x.tan()
}

pub async fn asin(x: f64) -> Result<f64, String> {
    if x < -1.0 || x > 1.0 {
        return Err("asin input must be between -1 and 1".to_string());
    }
    Ok(x.asin())
}

pub async fn acos(x: f64) -> Result<f64, String> {
    if x < -1.0 || x > 1.0 {
        return Err("acos input must be between -1 and 1".to_string());
    }
    Ok(x.acos())
}

pub async fn atan(x: f64) -> f64 {
    x.atan()
}

// Logarithmic functions
pub async fn log(x: f64) -> Result<f64, String> {
    if x <= 0.0 {
        return Err("log input must be positive".to_string());
    }
    Ok(x.ln())
}

pub async fn log10(x: f64) -> Result<f64, String> {
    if x <= 0.0 {
        return Err("log10 input must be positive".to_string());
    }
    Ok(x.log10())
}

pub async fn log2(x: f64) -> Result<f64, String> {
    if x <= 0.0 {
        return Err("log2 input must be positive".to_string());
    }
    Ok(x.log2())
}

// Exponential function
pub async fn exp(x: f64) -> f64 {
    x.exp()
}

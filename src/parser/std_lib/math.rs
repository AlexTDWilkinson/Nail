// Generic math functions for Nail
use std::ops::Neg;

// Generic min function - returns minimum of two values
pub fn min<T>(a: T, b: T) -> T
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
pub fn max<T>(a: T, b: T) -> T
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
pub fn clamp<T>(value: T, min_val: T, max_val: T) -> T
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

// Generic sign function - returns -1, 0, or 1.
//
// PartialOrd rather than Ord, like the comparisons above it: floats are not
// totally ordered, because not-a-number compares false against everything
// including itself, so an Ord bound here would exclude the type this is most
// often asked about. That leaves one value with no sign to give - not-a-number
// itself, which is neither negative nor positive nor zero - and 0 is the
// answer for it, matching what every other language returns.
pub fn sign<T>(value: T) -> i64
where
    T: PartialOrd + Default,
{
    let zero = T::default();
    if value < zero {
        return -1;
    }
    if value > zero {
        return 1;
    }
    return 0;
}

// Generic absolute value (PartialOrd rather than Ord so it works for floats)
pub fn abs<T>(value: T) -> T
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
pub fn sqrt(x: f64) -> f64 {
    x.sqrt()
}

// Power function
pub fn pow(base: f64, exp: f64) -> f64 {
    base.powf(exp)
}

// Round to nearest whole number, as a float
pub fn round(x: f64) -> f64 {
    x.round()
}

// Round to nearest whole number, as an integer
pub fn round_to_int(x: f64) -> i64 {
    x.round() as i64
}

// Floor function
pub fn floor(x: f64) -> f64 {
    x.floor()
}

// Ceiling function
pub fn ceil(x: f64) -> f64 {
    x.ceil()
}

// Random float in [0.0, 1.0)
pub fn random() -> f64 {
    rand::random::<f64>()
}

// The constant pi
pub fn pi() -> f64 {
    std::f64::consts::PI
}

// Euler's number
pub fn e() -> f64 {
    std::f64::consts::E
}

// Division with a divide-by-zero check
pub fn divide<T>(numerator: T, denominator: T) -> Result<T, String>
where
    T: std::ops::Div<Output = T> + Default + PartialEq + std::fmt::Display,
{
    if denominator == T::default() {
        return Err(format!("math_divide: cannot divide {} by zero", numerator));
    }
    Ok(numerator / denominator)
}

// Greatest common divisor (Euclidean algorithm)
pub fn gcd(mut a: i64, mut b: i64) -> i64 {
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
pub fn lcm(a: i64, b: i64) -> i64 {
    let a_abs = a.abs();
    let b_abs = b.abs();
    if a_abs == 0 || b_abs == 0 {
        return 0;
    }
    (a_abs / gcd(a_abs, b_abs)) * b_abs
}

// Factorial
pub fn factorial(n: i64) -> Result<i64, String> {
    if n < 0 {
        return Err(format!("math_factorial: not defined for negative numbers, got {}", n));
    }
    if n > 20 {
        return Err(format!("math_factorial: {} is too large, factorials above 20 overflow a 64-bit integer", n));
    }
    
    let mut result = 1i64;
    for i in 2..=n {
        result = result.checked_mul(i)
            .ok_or_else(|| format!("math_factorial: {} is too large, the result overflows a 64-bit integer", n))?;
    }
    Ok(result)
}

// Check if number is prime
pub fn is_prime(n: i64) -> bool {
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
pub fn sigmoid(x: f64) -> f64 {
    1.0 / (1.0 + (-x).exp())
}

// Linear interpolation
pub fn lerp(a: f64, b: f64, t: f64) -> f64 {
    a + (b - a) * t.clamp(0.0, 1.0)
}

// Trigonometric functions
pub fn sin(x: f64) -> f64 {
    x.sin()
}

pub fn cos(x: f64) -> f64 {
    x.cos()
}

pub fn tan(x: f64) -> f64 {
    x.tan()
}

pub fn asin(x: f64) -> Result<f64, String> {
    if x < -1.0 || x > 1.0 {
        return Err(format!("math_asin: input must be between -1 and 1, got {}", x));
    }
    Ok(x.asin())
}

pub fn acos(x: f64) -> Result<f64, String> {
    if x < -1.0 || x > 1.0 {
        return Err(format!("math_acos: input must be between -1 and 1, got {}", x));
    }
    Ok(x.acos())
}

pub fn atan(x: f64) -> f64 {
    x.atan()
}

// Logarithmic functions
pub fn log(x: f64) -> Result<f64, String> {
    if x <= 0.0 {
        return Err(format!("math_log: input must be positive, got {}", x));
    }
    Ok(x.ln())
}

pub fn log10(x: f64) -> Result<f64, String> {
    if x <= 0.0 {
        return Err(format!("math_log10: input must be positive, got {}", x));
    }
    Ok(x.log10())
}

pub fn log2(x: f64) -> Result<f64, String> {
    if x <= 0.0 {
        return Err(format!("math_log2: input must be positive, got {}", x));
    }
    Ok(x.log2())
}

// Exponential function
pub fn exp(x: f64) -> f64 {
    x.exp()
}

/// The angle from the positive x axis to the point (x, y), from -pi to pi.
///
/// This is the one to reach for, not `math_atan`. A plain arc tangent is given
/// only the ratio y/x, so it cannot tell the second quadrant from the fourth -
/// (-1, 1) and (1, -1) have the same ratio and opposite angles. Given both
/// numbers separately, this can, and it is defined at x = 0 where the ratio is
/// not.
pub fn atan2(y: f64, x: f64) -> f64 {
    return y.atan2(x);
}

/// The length of the hypotenuse: the distance from the origin to (x, y).
/// Computed without squaring the inputs first, so it stays exact for very
/// large and very small distances where x*x would overflow to infinity.
pub fn hypot(x: f64, y: f64) -> f64 {
    return x.hypot(y);
}

/// The cube root, defined for negative numbers as well - unlike a fractional
/// power, which is not.
pub fn cbrt(x: f64) -> f64 {
    return x.cbrt();
}

/// Throws away the fractional part, towards zero: 2.7 becomes 2.0 and -2.7
/// becomes -2.0. That is what makes it different from `math_floor`, which
/// takes -2.7 to -3.0.
pub fn trunc(x: f64) -> f64 {
    return x.trunc();
}

/// Just the fractional part, keeping the sign: 2.75 gives 0.75.
pub fn fract(x: f64) -> f64 {
    return x.fract();
}

/// An angle in radians, written in degrees.
pub fn to_degrees(radians: f64) -> f64 {
    return radians.to_degrees();
}

/// An angle in degrees, written in radians - which is what every function here
/// that takes an angle expects.
pub fn to_radians(degrees: f64) -> f64 {
    return degrees.to_radians();
}

pub fn sinh(x: f64) -> f64 {
    return x.sinh();
}

pub fn cosh(x: f64) -> f64 {
    return x.cosh();
}

pub fn tanh(x: f64) -> f64 {
    return x.tanh();
}

/// The remainder, always with the sign of the divisor, so the answer for a
/// positive divisor is never negative: -1 modulo 12 is 11, not -1.
///
/// This is what clock arithmetic, wrapping an index round an array, and hue
/// angles all want, and it is not what Rust's `%` gives - that keeps the sign
/// of the left-hand side, which turns a wrapped index into a crash.
pub fn modulo(value: f64, divisor: f64) -> Result<f64, String> {
    if divisor == 0.0 {
        return Err("math_modulo: the divisor is zero, and nothing can be divided by zero".to_string());
    }
    return Ok(value.rem_euclid(divisor));
}

/// The logarithm in a base of your choosing.
pub fn log_base(value: f64, base: f64) -> Result<f64, String> {
    if value <= 0.0 {
        return Err(format!("math_log_base: the logarithm of {} is undefined - the value must be above zero", value));
    }
    if base <= 0.0 || base == 1.0 {
        return Err(format!("math_log_base: {} is not a usable base - it must be above zero and not 1", base));
    }
    return Ok(value.log(base));
}

/// Whether this is the not-a-number value, which arithmetic produces from
/// things like zero divided by zero. It is the one value not equal to itself,
/// so `value == value` cannot be used to ask.
pub fn is_nan(value: f64) -> bool {
    return value.is_nan();
}

/// Whether this is positive or negative infinity, which arithmetic produces
/// when a result is too large to write down.
pub fn is_infinite(value: f64) -> bool {
    return value.is_infinite();
}

/// Whether this is an ordinary number: not infinite and not not-a-number. The
/// check to make before trusting a computed value.
pub fn is_finite(value: f64) -> bool {
    return value.is_finite();
}

#[cfg(test)]
mod added_tests {
    use super::*;

    fn close(left: f64, right: f64) -> bool {
        return (left - right).abs() < 1e-9;
    }

    #[test]
    fn atan2_tells_the_quadrants_apart_where_atan_cannot() {
        // Same ratio, opposite angles - the whole reason atan2 exists.
        assert!(close(atan(1.0 / -1.0), atan(-1.0 / 1.0)));
        assert!(close(atan2(1.0, -1.0), 3.0 * std::f64::consts::PI / 4.0));
        assert!(close(atan2(-1.0, 1.0), -std::f64::consts::PI / 4.0));
        // Defined on the y axis, where the ratio is not.
        assert!(close(atan2(1.0, 0.0), std::f64::consts::PI / 2.0));
    }

    #[test]
    fn hypot_is_the_distance_to_the_point() {
        assert!(close(hypot(3.0, 4.0), 5.0));
        // Squaring these first would overflow to infinity.
        assert!(hypot(1e200, 1e200).is_finite());
    }

    #[test]
    fn cube_roots_work_for_negative_numbers() {
        assert!(close(cbrt(27.0), 3.0));
        assert!(close(cbrt(-27.0), -3.0));
        assert!(pow(-27.0, 1.0 / 3.0).is_nan(), "a fractional power cannot do this, which is why cbrt exists");
    }

    #[test]
    fn truncating_and_flooring_differ_below_zero() {
        assert!(close(trunc(2.7), 2.0));
        assert!(close(trunc(-2.7), -2.0));
        assert!(close(floor(-2.7), -3.0));
        assert!(close(fract(2.75), 0.75));
    }

    #[test]
    fn angles_convert_both_ways() {
        assert!(close(to_degrees(std::f64::consts::PI), 180.0));
        assert!(close(to_radians(180.0), std::f64::consts::PI));
        assert!(close(to_radians(to_degrees(1.234)), 1.234));
    }

    #[test]
    fn the_hyperbolics_hold_their_identity() {
        // cosh^2 - sinh^2 is 1 for every input.
        for x in [-2.0, -0.5, 0.0, 0.5, 2.0] {
            assert!(close(cosh(x) * cosh(x) - sinh(x) * sinh(x), 1.0), "failed at {}", x);
        }
        assert!(close(tanh(0.0), 0.0));
    }

    #[test]
    fn modulo_keeps_the_sign_of_the_divisor() {
        assert!(close(modulo(-1.0, 12.0).expect("a non-zero divisor"), 11.0));
        assert!(close(modulo(13.0, 12.0).expect("a non-zero divisor"), 1.0));
        assert!(close(modulo(12.0, 12.0).expect("a non-zero divisor"), 0.0));
        assert!(modulo(1.0, 0.0).unwrap_err().contains("divisor is zero"));
    }

    #[test]
    fn a_logarithm_can_take_any_usable_base() {
        assert!(close(log_base(8.0, 2.0).expect("a usable base"), 3.0));
        assert!(close(log_base(81.0, 3.0).expect("a usable base"), 4.0));
        assert!(log_base(0.0, 2.0).unwrap_err().contains("undefined"));
        assert!(log_base(8.0, 1.0).unwrap_err().contains("not a usable base"));
    }

    #[test]
    fn the_values_that_are_not_ordinary_numbers_can_be_asked_about() {
        let not_a_number = 0.0f64 / 0.0f64;
        assert!(is_nan(not_a_number));
        assert!(!is_finite(not_a_number));
        assert_ne!(not_a_number, not_a_number, "which is why is_nan has to exist");

        let too_large = 1.0f64 / 0.0f64;
        assert!(is_infinite(too_large));
        assert!(!is_finite(too_large));

        assert!(is_finite(1.5));
        assert!(!is_nan(1.5));
    }
}

#[cfg(test)]
mod sign_tests {
    use super::*;

    #[test]
    fn sign_reports_direction_for_whole_numbers_and_fractions_alike() {
        assert_eq!(sign(-7i64), -1);
        assert_eq!(sign(0i64), 0);
        assert_eq!(sign(7i64), 1);
        assert_eq!(sign(-4.2f64), -1);
        assert_eq!(sign(0.0f64), 0);
        assert_eq!(sign(4.2f64), 1);
    }

    #[test]
    fn the_value_with_no_sign_reports_zero_rather_than_lying() {
        // Not-a-number compares false against everything, so it is neither
        // negative nor positive; 0 is what every other language answers here.
        assert_eq!(sign(0.0f64 / 0.0f64), 0);
    }
}

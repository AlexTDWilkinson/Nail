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

/// Rounds to a fixed number of decimal places, halves away from zero: 2.5
/// becomes 3 and -2.5 becomes -3, the way people round on paper.
pub fn round_to(value: f64, decimals: i64) -> Result<f64, String> {
    if !(0..=12).contains(&decimals) {
        return Err(format!("math_round_to: decimals must be between 0 and 12, got {}", decimals));
    }
    let factor = 10f64.powi(decimals as i32);
    return Ok((value * factor).round() / factor);
}

/// How much a value grew or shrank, as a percentage of where it started:
/// from 50 to 75 is 50, from 50 to 25 is -50.
pub fn percent_change(old: f64, new: f64) -> Result<f64, String> {
    if old == 0.0 {
        return Err("math_percent_change: the old value is zero, and a change from zero has no percentage".to_string());
    }
    return Ok((new - old) / old * 100.0);
}

/// What percentage the part is of the whole: 30 of 120 is 25.
pub fn percent_of(part: f64, whole: f64) -> Result<f64, String> {
    if whole == 0.0 {
        return Err("math_percent_of: the whole is zero, and a part of zero has no percentage".to_string());
    }
    return Ok(part / whole * 100.0);
}

/// The nth root: degree 2 is the square root, degree 3 the cube root, and so
/// on. An odd root of a negative number is negative, as it should be - a
/// fractional power cannot do that. An even root of a negative number does not
/// exist, and is an error.
pub fn nth_root(value: f64, degree: i64) -> Result<f64, String> {
    if degree < 1 {
        return Err(format!("math_nth_root: the degree must be at least 1, got {}", degree));
    }
    if value < 0.0 && degree % 2 == 0 {
        return Err(format!("math_nth_root: a negative number has no even root - got degree {} of {}", degree, value));
    }
    let exponent = 1.0 / degree as f64;
    if value < 0.0 {
        return Ok(-((-value).powf(exponent)));
    }
    return Ok(value.powf(exponent));
}

/// How many ways to choose k things from n when order does not matter:
/// 10 choose 3 is 120. Choosing more than there are gives 0 ways.
pub fn combinations(n: i64, k: i64) -> Result<i64, String> {
    if n < 0 || k < 0 {
        return Err(format!("math_combinations: not defined for negative numbers, got n = {} and k = {}", n, k));
    }
    if k > n {
        return Ok(0);
    }
    // Choosing k is choosing which n - k to leave out, so count the smaller.
    let smaller = k.min(n - k);
    // Stepwise, multiplying in one factor and dividing out one before the
    // next: each intermediate value is itself a binomial coefficient, so the
    // division is always exact and nothing overflows that did not have to.
    let mut result: i128 = 1;
    for step in 1..=smaller {
        result = result
            .checked_mul((n - smaller + step) as i128)
            .ok_or_else(|| format!("math_combinations: {} choose {} is too large, the result overflows a 64-bit integer", n, k))?
            / step as i128;
    }
    return i64::try_from(result).map_err(|_| format!("math_combinations: {} choose {} is too large, the result overflows a 64-bit integer", n, k));
}

/// How many ways to arrange k things drawn from n when order matters:
/// 10 permute 3 is 720. Drawing more than there are gives 0 ways.
pub fn permutations(n: i64, k: i64) -> Result<i64, String> {
    if n < 0 || k < 0 {
        return Err(format!("math_permutations: not defined for negative numbers, got n = {} and k = {}", n, k));
    }
    if k > n {
        return Ok(0);
    }
    let mut result: i128 = 1;
    for factor in (n - k + 1)..=n {
        result = result
            .checked_mul(factor as i128)
            .ok_or_else(|| format!("math_permutations: {} permute {} is too large, the result overflows a 64-bit integer", n, k))?;
    }
    return i64::try_from(result).map_err(|_| format!("math_permutations: {} permute {} is too large, the result overflows a 64-bit integer", n, k));
}

/// Eases from 0 at the low edge to 1 at the high edge along a smooth S-curve,
/// flat at both ends - the classic easing function for fading and animation.
/// Outside the edges it holds at 0 or 1 rather than overshooting.
pub fn smoothstep(edge_low: f64, edge_high: f64, value: f64) -> Result<f64, String> {
    if edge_low == edge_high {
        return Err(format!("math_smoothstep: the edges must differ, both are {}", edge_low));
    }
    let t = ((value - edge_low) / (edge_high - edge_low)).clamp(0.0, 1.0);
    return Ok(t * t * (3.0 - 2.0 * t));
}

/// What a starting amount becomes after growing by a fixed rate for a number
/// of periods: 1000 at 5% (0.05) for 10 periods is about 1628.89. A negative
/// rate shrinks instead.
pub fn compound_growth(principal: f64, rate_per_period: f64, periods: i64) -> Result<f64, String> {
    if periods < 0 {
        return Err(format!("math_compound_growth: the number of periods cannot be negative, got {}", periods));
    }
    return Ok(principal * (1.0 + rate_per_period).powf(periods as f64));
}

/// ln(1 + x), computed accurately for x very close to zero - where adding 1
/// first would throw the small part away before the logarithm ever saw it.
pub fn log1p(value: f64) -> Result<f64, String> {
    if value <= -1.0 {
        return Err(format!("math_log1p: input must be above -1, got {}", value));
    }
    return Ok(value.ln_1p());
}

/// e^x - 1, computed accurately for x very close to zero - where subtracting
/// 1 from a number barely above 1 would cancel away all the precision.
pub fn expm1(value: f64) -> f64 {
    return value.exp_m1();
}

/// The first number wearing the sign of the second: copysign(3.0, -1.5)
/// is -3.0.
pub fn copysign(magnitude: f64, sign_source: f64) -> f64 {
    return magnitude.copysign(sign_source);
}

/// The digits of a number added together, ignoring its sign: 1234 gives 10.
pub fn sum_of_digits(value: i64) -> i64 {
    let mut remaining = value.unsigned_abs();
    let mut total: u64 = 0;
    while remaining > 0 {
        total += remaining % 10;
        remaining /= 10;
    }
    return total as i64;
}

/// How many decimal digits a number has, ignoring its sign: 0 has one digit,
/// -1234 has four.
pub fn digit_count(value: i64) -> i64 {
    let mut remaining = value.unsigned_abs();
    let mut count: i64 = 1;
    while remaining >= 10 {
        count += 1;
        remaining /= 10;
    }
    return count;
}

/// Whether the number is some integer multiplied by itself: 49 is, 50 is not,
/// and no negative number is.
pub fn is_perfect_square(value: i64) -> bool {
    if value < 0 {
        return false;
    }
    // The float square root can be off by one at the top of the i64 range, so
    // check the neighbours as well.
    let near = (value as f64).sqrt() as i64;
    for root in near.saturating_sub(1)..=near.saturating_add(1) {
        if let Some(squared) = root.checked_mul(root) {
            if squared == value {
                return true;
            }
        }
    }
    return false;
}

/// The Fibonacci number at a position, counting from fibonacci(0) = 0 and
/// fibonacci(1) = 1. Position 92 is the last that fits in a 64-bit integer.
pub fn fibonacci(position: i64) -> Result<i64, String> {
    if position < 0 {
        return Err(format!("math_fibonacci: not defined for negative positions, got {}", position));
    }
    if position == 0 {
        return Ok(0);
    }
    let mut previous: i64 = 0;
    let mut current: i64 = 1;
    for _ in 1..position {
        let next = previous
            .checked_add(current)
            .ok_or_else(|| format!("math_fibonacci: position {} is too large, Fibonacci numbers past position 92 overflow a 64-bit integer", position))?;
        previous = current;
        current = next;
    }
    return Ok(current);
}

/// The nth triangular number, 1 + 2 + ... + n: the count of dots in a
/// triangle n rows tall. triangular(4) is 10.
pub fn triangular(n: i64) -> Result<i64, String> {
    if n < 0 {
        return Err(format!("math_triangular: not defined for negative numbers, got {}", n));
    }
    let counted = n as i128 * (n as i128 + 1) / 2;
    return i64::try_from(counted).map_err(|_| format!("math_triangular: {} is too large, the result overflows a 64-bit integer", n));
}

/// Folds a value into the range from low up to but not including high, the
/// way an angle of 370 degrees is really 10: wrap(370.0, 0.0, 360.0) is 10.0,
/// and wrap(-30.0, 0.0, 360.0) is 330.0.
pub fn wrap(value: f64, low: f64, high: f64) -> Result<f64, String> {
    if low >= high {
        return Err(format!("math_wrap: the low edge must be below the high edge, got {} and {}", low, high));
    }
    return Ok(low + (value - low).rem_euclid(high - low));
}

/// Where erf and erfc switch from the Maclaurin series to the continued
/// fraction. Below this the series converges in a couple of dozen terms,
/// above it the fraction does.
const ERF_SERIES_LIMIT: f64 = 1.25;

/// The error function: the share of a Gaussian bell that lies within the
/// given distance of its centre, scaled to run from -1 to 1. It is odd, so
/// erf(-x) is exactly -erf(x).
///
/// Near zero it is summed from the Maclaurin series and further out it is
/// 1 minus the directly computed complement below. Both routes carry far
/// more digits than the classic Abramowitz and Stegun rational fit.
pub fn erf(value: f64) -> f64 {
    if value.abs() <= ERF_SERIES_LIMIT {
        return erf_series(value);
    }
    let tail = erfc_continued_fraction(value.abs());
    if value < 0.0 {
        return tail - 1.0;
    }
    return 1.0 - tail;
}

/// The complementary error function 1 - erf. Computed directly rather than
/// by subtracting from 1, so the tiny tail values for large inputs keep
/// their relative accuracy instead of cancelling away to noise.
pub fn erfc(value: f64) -> f64 {
    if value > ERF_SERIES_LIMIT {
        return erfc_continued_fraction(value);
    }
    if value < -ERF_SERIES_LIMIT {
        return 2.0 - erfc_continued_fraction(-value);
    }
    return 1.0 - erf_series(value);
}

/// The Maclaurin series for erf, one term folded in at a time until the
/// next would not move the total. Only called for small inputs, where the
/// alternating terms shrink from the start.
fn erf_series(x: f64) -> f64 {
    let squared = x * x;
    let mut term = x;
    let mut total = x;
    let mut n = 1.0;
    while term.abs() > total.abs() * 1e-17 {
        term *= -squared * (2.0 * n - 1.0) / (n * (2.0 * n + 1.0));
        total += term;
        n += 1.0;
    }
    return std::f64::consts::FRAC_2_SQRT_PI * total;
}

/// The continued fraction for the upper tail of the Gaussian, evaluated
/// with the modified Lentz algorithm - the same fraction that computes the
/// upper incomplete gamma function at one half. Only called for inputs
/// beyond the series limit, where it converges quickly.
fn erfc_continued_fraction(x: f64) -> f64 {
    let squared = x * x;
    if squared == f64::INFINITY {
        // So far out that the tail has nothing left in it at all
        return 0.0;
    }
    let floor_value = 1e-300;
    let mut b = squared + 0.5;
    let mut c = 1.0 / floor_value;
    let mut d = 1.0 / b;
    let mut h = d;
    for i in 1..300 {
        let a = -(i as f64) * (i as f64 - 0.5);
        b += 2.0;
        d = a * d + b;
        if d.abs() < floor_value {
            d = floor_value;
        }
        c = b + a / c;
        if c.abs() < floor_value {
            c = floor_value;
        }
        d = 1.0 / d;
        let delta = d * c;
        h *= delta;
        if (delta - 1.0).abs() < 1e-15 {
            break;
        }
    }
    return (-squared).exp() * x * (std::f64::consts::FRAC_2_SQRT_PI / 2.0) * h;
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

#[cfg(test)]
mod pure_addition_tests {
    use super::*;

    fn close(left: f64, right: f64) -> bool {
        return (left - right).abs() < 1e-9;
    }

    #[test]
    fn rounding_to_places_goes_half_away_from_zero() {
        assert!(close(round_to(2.34567, 2).expect("in range"), 2.35));
        assert!(close(round_to(2.5, 0).expect("in range"), 3.0));
        assert!(close(round_to(-2.5, 0).expect("in range"), -3.0));
        assert!(close(round_to(1.23456, 3).expect("in range"), 1.235));
        assert!(close(round_to(1.23456, 12).expect("in range"), 1.23456));
        assert!(round_to(1.0, 13).unwrap_err().contains("between 0 and 12"));
        assert!(round_to(1.0, -1).unwrap_err().contains("between 0 and 12"));
    }

    #[test]
    fn percentages_come_out_in_hundreds_and_zero_bases_are_errors() {
        assert!(close(percent_change(50.0, 75.0).expect("non-zero old"), 50.0));
        assert!(close(percent_change(50.0, 25.0).expect("non-zero old"), -50.0));
        assert!(percent_change(0.0, 10.0).unwrap_err().contains("zero"));
        assert!(close(percent_of(30.0, 120.0).expect("non-zero whole"), 25.0));
        assert!(percent_of(30.0, 0.0).unwrap_err().contains("zero"));
    }

    #[test]
    fn nth_roots_agree_with_their_named_cousins_and_odd_roots_go_negative() {
        assert!(close(nth_root(16.0, 2).expect("a valid degree"), 4.0));
        assert!(close(nth_root(27.0, 3).expect("a valid degree"), 3.0));
        assert!(close(nth_root(-27.0, 3).expect("odd degree of a negative"), -3.0));
        assert!(close(nth_root(32.0, 5).expect("a valid degree"), 2.0));
        assert!(nth_root(-16.0, 2).unwrap_err().contains("no even root"));
        assert!(nth_root(16.0, 0).unwrap_err().contains("at least 1"));
    }

    #[test]
    fn counting_choices_matches_the_textbook() {
        assert_eq!(combinations(10, 3).expect("small"), 120);
        assert_eq!(combinations(10, 7).expect("small"), 120, "choosing 7 is leaving out 3");
        assert_eq!(combinations(52, 5).expect("a poker hand"), 2_598_960);
        assert_eq!(combinations(5, 0).expect("choose nothing"), 1);
        assert_eq!(combinations(3, 5).expect("more than there are"), 0);
        assert_eq!(permutations(10, 3).expect("small"), 720);
        assert_eq!(permutations(5, 5).expect("all of them"), 120);
        assert_eq!(permutations(5, 0).expect("arrange nothing"), 1);
        assert_eq!(permutations(3, 5).expect("more than there are"), 0);
    }

    #[test]
    fn counting_choices_rejects_negatives_and_reports_overflow() {
        assert!(combinations(-1, 2).unwrap_err().contains("negative"));
        assert!(combinations(5, -2).unwrap_err().contains("negative"));
        assert!(permutations(-1, 2).unwrap_err().contains("negative"));
        // 67 choose 33 is the first row of Pascal's triangle whose middle
        // does not fit in a 64-bit integer; the row before it does.
        assert!(combinations(66, 33).is_ok());
        assert!(combinations(67, 33).unwrap_err().contains("overflows"));
        assert!(permutations(21, 21).unwrap_err().contains("overflows"), "21! is past the factorial limit");
    }

    #[test]
    fn smoothstep_eases_between_its_edges_and_holds_outside_them() {
        assert!(close(smoothstep(0.0, 1.0, 0.5).expect("edges differ"), 0.5));
        assert!(close(smoothstep(0.0, 1.0, -5.0).expect("edges differ"), 0.0));
        assert!(close(smoothstep(0.0, 1.0, 5.0).expect("edges differ"), 1.0));
        assert!(close(smoothstep(0.0, 1.0, 0.25).expect("edges differ"), 0.15625));
        // Flat at the ends: just inside an edge it has barely moved.
        assert!(smoothstep(0.0, 1.0, 0.01).expect("edges differ") < 0.001);
        assert!(smoothstep(2.0, 2.0, 2.0).unwrap_err().contains("must differ"));
    }

    #[test]
    fn compound_growth_multiplies_out_period_by_period() {
        assert!(close(compound_growth(1000.0, 0.05, 10).expect("periods in range"), 1628.894626777442));
        assert!(close(compound_growth(1000.0, 0.05, 0).expect("no periods"), 1000.0));
        assert!(close(compound_growth(1000.0, -0.5, 2).expect("shrinking"), 250.0));
        assert!(compound_growth(1000.0, 0.05, -1).unwrap_err().contains("negative"));
    }

    #[test]
    fn log1p_and_expm1_survive_where_the_naive_forms_lose_everything() {
        let tiny = 1e-18;
        assert!(close(log1p(tiny).expect("above -1") / tiny, 1.0), "ln(1+x) is x to first order");
        assert!(close(expm1(tiny) / tiny, 1.0), "e^x - 1 is x to first order");
        assert_eq!((1.0f64 + tiny).ln(), 0.0, "the naive form has already rounded 1 + x down to 1");
        assert!(close(log1p(std::f64::consts::E - 1.0).expect("above -1"), 1.0));
        assert!(log1p(-1.0).unwrap_err().contains("above -1"));
    }

    #[test]
    fn copysign_moves_only_the_sign() {
        assert_eq!(copysign(3.0, -1.5), -3.0);
        assert_eq!(copysign(-3.0, 2.0), 3.0);
        assert_eq!(copysign(3.0, 0.0), 3.0);
    }

    #[test]
    fn digits_are_summed_and_counted_without_their_sign() {
        assert_eq!(sum_of_digits(1234), 10);
        assert_eq!(sum_of_digits(-1234), 10);
        assert_eq!(sum_of_digits(0), 0);
        assert_eq!(sum_of_digits(i64::MIN), 89, "the one number with no positive counterpart");
        assert_eq!(digit_count(0), 1);
        assert_eq!(digit_count(-1234), 4);
        assert_eq!(digit_count(i64::MAX), 19);
        assert_eq!(digit_count(i64::MIN), 19);
    }

    #[test]
    fn perfect_squares_are_recognised_and_near_misses_are_not() {
        assert!(is_perfect_square(0));
        assert!(is_perfect_square(49));
        assert!(!is_perfect_square(50));
        assert!(!is_perfect_square(-49));
        let big_root = 3_037_000_499i64; // the largest root whose square fits
        assert!(is_perfect_square(big_root * big_root));
        assert!(!is_perfect_square(big_root * big_root - 1));
        assert!(!is_perfect_square(i64::MAX));
    }

    #[test]
    fn fibonacci_counts_from_zero_and_stops_exactly_where_the_integer_does() {
        assert_eq!(fibonacci(0).expect("in range"), 0);
        assert_eq!(fibonacci(1).expect("in range"), 1);
        assert_eq!(fibonacci(10).expect("in range"), 55);
        assert_eq!(fibonacci(92).expect("the last that fits"), 7_540_113_804_746_346_429);
        assert!(fibonacci(93).unwrap_err().contains("overflow"));
        assert!(fibonacci(-1).unwrap_err().contains("negative"));
    }

    #[test]
    fn triangular_numbers_stack_up_and_overflow_is_reported() {
        assert_eq!(triangular(0).expect("in range"), 0);
        assert_eq!(triangular(4).expect("in range"), 10);
        assert_eq!(triangular(100).expect("in range"), 5050);
        // 4294967295 is the largest n whose triangle still fits.
        assert!(triangular(4_294_967_295).is_ok());
        assert!(triangular(4_294_967_296).unwrap_err().contains("overflows"));
        assert!(triangular(-1).unwrap_err().contains("negative"));
    }

    #[test]
    fn wrapping_folds_angles_into_their_range() {
        assert!(close(wrap(370.0, 0.0, 360.0).expect("a proper range"), 10.0));
        assert!(close(wrap(-30.0, 0.0, 360.0).expect("a proper range"), 330.0));
        assert!(close(wrap(360.0, 0.0, 360.0).expect("a proper range"), 0.0), "the high edge itself wraps to the low");
        assert!(close(wrap(190.0, -180.0, 180.0).expect("a proper range"), -170.0));
        assert!(close(wrap(90.0, 0.0, 360.0).expect("a proper range"), 90.0), "already inside stays put");
        assert!(wrap(1.0, 5.0, 5.0).unwrap_err().contains("below the high edge"));
        assert!(wrap(1.0, 6.0, 5.0).unwrap_err().contains("below the high edge"));
    }
}

#[cfg(test)]
mod error_function_tests {
    use super::*;

    fn close(left: f64, right: f64) -> bool {
        return (left - right).abs() < 1e-9;
    }

    #[test]
    fn erf_matches_the_tables_at_the_textbook_points() {
        assert_eq!(erf(0.0), 0.0, "the series at zero is exactly zero");
        assert!((erf(1.0) - 0.8427007929497149).abs() < 1e-12);
        assert!(close(erf(0.5), 0.5204998778130465));
        assert!(close(erf(2.0), 0.9953222650189527));
        assert!(close(erf(3.0), 0.9999779095030014));
        assert_eq!(erf(f64::INFINITY), 1.0);
        assert_eq!(erf(f64::NEG_INFINITY), -1.0);
    }

    #[test]
    fn erf_is_odd_on_both_sides_of_the_series_limit() {
        for x in [0.1, 0.7, 1.0, 1.25, 1.3, 2.0, 5.0, 10.0] {
            assert_eq!(erf(-x), -erf(x), "failed at {}", x);
        }
    }

    #[test]
    fn erfc_complements_erf_across_the_whole_line() {
        assert_eq!(erfc(0.0), 1.0);
        assert!(close(erfc(1.0), 0.15729920705028513));
        assert!((erfc(3.0) - 2.2090496998585445e-5).abs() < 1e-10);
        for x in [-3.0, -1.0, -0.4, 0.3, 1.5, 4.0] {
            assert!((erf(x) + erfc(x) - 1.0).abs() < 1e-12, "failed at {}", x);
        }
    }

    #[test]
    fn erfc_keeps_relative_accuracy_far_into_the_tail() {
        // The asymptotic envelope brackets the true tail on both sides, so a
        // value computed with real relative accuracy must land between them.
        for x in [2.0f64, 5.0, 10.0, 15.0, 20.0] {
            let envelope = (-x * x).exp() / (x * std::f64::consts::PI.sqrt());
            let tail = erfc(x);
            assert!(tail < envelope, "failed the upper bound at {}", x);
            assert!(tail > envelope * (1.0 - 0.5 / (x * x)), "failed the lower bound at {}", x);
        }
        assert!(close(erfc(-4.0), 2.0 - erfc(4.0)));
        assert_eq!(erfc(f64::INFINITY), 0.0);
        assert_eq!(erfc(f64::NEG_INFINITY), 2.0);
    }
}

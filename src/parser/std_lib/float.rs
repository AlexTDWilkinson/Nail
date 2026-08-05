use std::fmt::Display;

// Float conversion functions. Float arithmetic (abs, sqrt, round, etc.) lives
// in std_lib::math - there is exactly one way to do each operation.

// Convert a value to a float
pub fn from<T: Display>(v: T) -> Result<f64, String> {
    v.to_string().parse::<f64>().map_err(|_| format!("float_from: could not parse '{}' as a float", v))
}

/// Whether two floats are within a tolerance of each other. This is how
/// floats should be compared: 0.1 + 0.2 is not equal to 0.3 in float
/// arithmetic, because neither side is stored exactly, so `==` on computed
/// floats is nearly always a bug. The sign of the tolerance is ignored, and
/// not-a-number is close to nothing, not even itself.
pub fn approx_equal(first: f64, second: f64, tolerance: f64) -> bool {
    return (first - second).abs() <= tolerance.abs();
}

/// Whether the float holds a whole number exactly: 3.0 is, 3.5 is not, and
/// neither infinity nor not-a-number is.
pub fn is_whole(value: f64) -> bool {
    return value.is_finite() && value.fract() == 0.0;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn approx_equal_is_the_comparison_that_survives_float_arithmetic() {
        assert_ne!(0.1 + 0.2, 0.3, "which is why approx_equal has to exist");
        assert!(approx_equal(0.1 + 0.2, 0.3, 1e-9));
        assert!(approx_equal(1.0, 1.5, 0.5), "the tolerance is inclusive");
        assert!(!approx_equal(1.0, 1.5001, 0.5));
        assert!(approx_equal(1.0, 1.5, -0.5), "the sign of the tolerance is ignored");
    }

    #[test]
    fn nothing_is_approximately_equal_to_not_a_number() {
        let not_a_number = 0.0f64 / 0.0f64;
        assert!(!approx_equal(not_a_number, not_a_number, 1.0));
        assert!(!approx_equal(not_a_number, 0.0, 1.0));
    }

    #[test]
    fn wholeness_is_about_the_stored_value_not_how_it_was_written() {
        assert!(is_whole(3.0));
        assert!(is_whole(-3.0));
        assert!(is_whole(0.0));
        assert!(is_whole(1.5 + 1.5));
        assert!(!is_whole(3.5));
        assert!(!is_whole(1.0f64 / 0.0f64));
        assert!(!is_whole(0.0f64 / 0.0f64));
    }
}

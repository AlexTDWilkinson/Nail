//! Assertions, so a Nail program can check itself.
//!
//! A language with no package manager has to ship the thing people test with,
//! or nobody tests. These are deliberately plain: each one either does nothing
//! or panics with a message naming what was expected and what turned up. A
//! failing assertion stops the program with a non-zero exit status, which is
//! all a test runner - `nail run tests/x.nail` in a loop, a shell script, CI -
//! needs in order to tell pass from fail.
//!
//! The message argument is not optional on purpose. "assertion failed" tells
//! you nothing at three in the morning; "user count after import" tells you
//! where to look.

/// The common failure path, so every assertion reports the same shape.
fn fail(check: &str, message: &str, detail: String) -> ! {
    panic!("{} failed: {}\n  {}", check, message, detail);
}

/// The condition must hold.
pub fn assert(condition: bool, message: String) {
    if !condition {
        fail("test_assert", &message, "expected the condition to be true".to_string());
    }
}

/// The condition must not hold.
pub fn assert_false(condition: bool, message: String) {
    if condition {
        fail("test_assert_false", &message, "expected the condition to be false".to_string());
    }
}

pub fn assert_equal_int(actual: i64, expected: i64, message: String) {
    if actual != expected {
        fail("test_assert_equal_int", &message, format!("expected {}, got {}", expected, actual));
    }
}

pub fn assert_equal_string(actual: String, expected: String, message: String) {
    if actual != expected {
        fail("test_assert_equal_string", &message, format!("expected `{}`, got `{}`", expected, actual));
    }
}

pub fn assert_equal_bool(actual: bool, expected: bool, message: String) {
    if actual != expected {
        fail("test_assert_equal_bool", &message, format!("expected {}, got {}", expected, actual));
    }
}

/// Floats are compared with a tolerance because exact equality is a trap:
/// 0.1 + 0.2 is not 0.3 in any language with hardware floats, including this
/// one. Pass the largest difference you are willing to call equal.
pub fn assert_equal_float(actual: f64, expected: f64, tolerance: f64, message: String) {
    if tolerance < 0.0 {
        fail("test_assert_equal_float", &message, format!("the tolerance {} is negative, so nothing could ever match", tolerance));
    }
    if (actual - expected).abs() > tolerance {
        fail("test_assert_equal_float", &message, format!("expected {} within {}, got {}", expected, tolerance, actual));
    }
}

/// The text must contain the fragment.
pub fn assert_contains(haystack: String, needle: String, message: String) {
    if !haystack.contains(&needle) {
        fail("test_assert_contains", &message, format!("expected to find `{}` in `{}`", needle, haystack));
    }
}

/// Two arrays must hold the same elements in the same order. Generic, so it
/// works for any array Nail can build.
pub fn assert_equal_array<T: PartialEq + std::fmt::Debug>(actual: &Vec<T>, expected: &Vec<T>, message: String) {
    if actual.len() != expected.len() {
        fail("test_assert_equal_array", &message, format!("expected {} elements, got {}", expected.len(), actual.len()));
    }
    for index in 0..actual.len() {
        if actual[index] != expected[index] {
            fail("test_assert_equal_array", &message, format!("element {} differs: expected {:?}, got {:?}", index, expected[index], actual[index]));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_holding_assertion_does_nothing() {
        assert(true, "a true condition".to_string());
        assert_false(false, "a false condition".to_string());
        assert_equal_int(2, 2, "equal integers".to_string());
        assert_equal_string("a".to_string(), "a".to_string(), "equal strings".to_string());
        assert_equal_bool(true, true, "equal booleans".to_string());
        assert_equal_float(0.1 + 0.2, 0.3, 1e-9, "floats within tolerance".to_string());
        assert_contains("hello world".to_string(), "lo wo".to_string(), "a substring".to_string());
        assert_equal_array(&vec![1, 2, 3], &vec![1, 2, 3], "equal arrays".to_string());
    }

    #[test]
    #[should_panic(expected = "expected 3, got 2")]
    fn a_failing_integer_assertion_names_both_values() {
        assert_equal_int(2, 3, "the count".to_string());
    }

    #[test]
    #[should_panic(expected = "element 1 differs")]
    fn a_failing_array_assertion_names_the_position() {
        assert_equal_array(&vec![1, 9, 3], &vec![1, 2, 3], "the rows".to_string());
    }

    #[test]
    #[should_panic(expected = "expected 4 elements, got 3")]
    fn a_length_mismatch_is_reported_as_a_length() {
        assert_equal_array(&vec![1, 2, 3], &vec![1, 2, 3, 4], "the rows".to_string());
    }

    #[test]
    #[should_panic(expected = "within 0.0001")]
    fn a_float_outside_the_tolerance_fails() {
        assert_equal_float(1.0, 2.0, 0.0001, "the ratio".to_string());
    }

    #[test]
    #[should_panic(expected = "the tolerance -1 is negative")]
    fn a_negative_tolerance_is_rejected() {
        assert_equal_float(1.0, 1.0, -1.0, "the ratio".to_string());
    }
}

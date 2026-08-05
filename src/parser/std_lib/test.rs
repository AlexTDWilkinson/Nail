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

use dashmap::DashMap;
use std::hash::Hash;

/// The common failure path, so every assertion reports the same shape.
fn report(check: &str, message: &str, detail: String) -> ! {
    panic!("{} failed: {}\n  {}", check, message, detail);
}

/// The condition must hold.
pub fn assert(condition: bool, message: String) {
    if !condition {
        report("test_assert", &message, "expected the condition to be true".to_string());
    }
}

/// The condition must not hold.
pub fn assert_false(condition: bool, message: String) {
    if condition {
        report("test_assert_false", &message, "expected the condition to be false".to_string());
    }
}

pub fn assert_equal_int(actual: i64, expected: i64, message: String) {
    if actual != expected {
        report("test_assert_equal_int", &message, format!("expected {}, got {}", expected, actual));
    }
}

pub fn assert_not_equal_int(actual: i64, unwanted: i64, message: String) {
    if actual == unwanted {
        report("test_assert_not_equal_int", &message, format!("expected anything but {}, got exactly that", unwanted));
    }
}

/// The value must be strictly greater than the threshold.
pub fn assert_greater_int(actual: i64, threshold: i64, message: String) {
    if actual <= threshold {
        report("test_assert_greater_int", &message, format!("expected a value greater than {}, got {}", threshold, actual));
    }
}

/// The value must be strictly less than the threshold.
pub fn assert_less_int(actual: i64, threshold: i64, message: String) {
    if actual >= threshold {
        report("test_assert_less_int", &message, format!("expected a value less than {}, got {}", threshold, actual));
    }
}

/// The value must be between low and high, both ends included. A range where
/// low exceeds high is rejected outright, because nothing could ever pass it.
pub fn assert_between_int(actual: i64, low: i64, high: i64, message: String) {
    if low > high {
        report("test_assert_between_int", &message, format!("the range {} to {} is empty, so nothing could ever pass", low, high));
    }
    if actual < low || actual > high {
        report("test_assert_between_int", &message, format!("expected a value from {} to {}, got {}", low, high, actual));
    }
}

pub fn assert_equal_string(actual: String, expected: String, message: String) {
    if actual != expected {
        report("test_assert_equal_string", &message, format!("expected `{}`, got `{}`", expected, actual));
    }
}

pub fn assert_not_equal_string(actual: String, unwanted: String, message: String) {
    if actual == unwanted {
        report("test_assert_not_equal_string", &message, format!("expected anything but `{}`, got exactly that", unwanted));
    }
}

pub fn assert_equal_bool(actual: bool, expected: bool, message: String) {
    if actual != expected {
        report("test_assert_equal_bool", &message, format!("expected {}, got {}", expected, actual));
    }
}

/// Floats are compared with a tolerance because exact equality is a trap:
/// 0.1 + 0.2 is not 0.3 in any language with hardware floats, including this
/// one. Pass the largest difference you are willing to call equal.
pub fn assert_equal_float(actual: f64, expected: f64, tolerance: f64, message: String) {
    if tolerance < 0.0 {
        report("test_assert_equal_float", &message, format!("the tolerance {} is negative, so nothing could ever match", tolerance));
    }
    if (actual - expected).abs() > tolerance {
        report("test_assert_equal_float", &message, format!("expected {} within {}, got {}", expected, tolerance, actual));
    }
}

/// Ordering comparisons need no tolerance: strictly greater is unambiguous
/// even on hardware floats.
pub fn assert_greater_float(actual: f64, threshold: f64, message: String) {
    if !(actual > threshold) {
        report("test_assert_greater_float", &message, format!("expected a value greater than {}, got {}", threshold, actual));
    }
}

pub fn assert_less_float(actual: f64, threshold: f64, message: String) {
    if !(actual < threshold) {
        report("test_assert_less_float", &message, format!("expected a value less than {}, got {}", threshold, actual));
    }
}

/// The text must contain the fragment.
pub fn assert_contains(haystack: String, needle: String, message: String) {
    if !haystack.contains(&needle) {
        report("test_assert_contains", &message, format!("expected to find `{}` in `{}`", needle, haystack));
    }
}

/// The text must not contain the fragment.
pub fn assert_not_contains(haystack: String, needle: String, message: String) {
    if haystack.contains(&needle) {
        report("test_assert_not_contains", &message, format!("expected not to find `{}` in `{}`", needle, haystack));
    }
}

pub fn assert_starts_with(text: String, prefix: String, message: String) {
    if !text.starts_with(&prefix) {
        report("test_assert_starts_with", &message, format!("expected the text to start with `{}`, got `{}`", prefix, text));
    }
}

pub fn assert_ends_with(text: String, suffix: String, message: String) {
    if !text.ends_with(&suffix) {
        report("test_assert_ends_with", &message, format!("expected the text to end with `{}`, got `{}`", suffix, text));
    }
}

/// Two arrays must hold the same elements in the same order. Generic, so it
/// works for any array Nail can build.
pub fn assert_equal_array<T: PartialEq + std::fmt::Debug>(actual: &Vec<T>, expected: &Vec<T>, message: String) {
    if actual.len() != expected.len() {
        report("test_assert_equal_array", &message, format!("expected {} elements, got {}", expected.len(), actual.len()));
    }
    for index in 0..actual.len() {
        if actual[index] != expected[index] {
            report("test_assert_equal_array", &message, format!("element {} differs: expected {:?}, got {:?}", index, expected[index], actual[index]));
        }
    }
}

pub fn assert_array_length<T>(array: &Vec<T>, expected: i64, message: String) {
    if array.len() as i64 != expected {
        report("test_assert_array_length", &message, format!("expected {} elements, got {}", expected, array.len()));
    }
}

pub fn assert_array_contains<T: PartialEq + std::fmt::Debug>(array: &Vec<T>, item: T, message: String) {
    if !array.iter().any(|element| element == &item) {
        report("test_assert_array_contains", &message, format!("expected to find {:?} among {} elements", item, array.len()));
    }
}

pub fn assert_array_empty<T>(array: &Vec<T>, message: String) {
    if !array.is_empty() {
        report("test_assert_array_empty", &message, format!("expected no elements, got {}", array.len()));
    }
}

pub fn assert_array_not_empty<T>(array: &Vec<T>, message: String) {
    if array.is_empty() {
        report("test_assert_array_not_empty", &message, "expected at least one element, got none".to_string());
    }
}

/// Two hashmaps must hold the same keys with the same values. Every
/// difference is collected and the list is sorted before it is reported, so
/// a failure reads the same on every run despite hashmap iteration order.
pub fn assert_equal_hashmap<K: Hash + Eq + Clone + std::fmt::Debug, V: Clone + PartialEq + std::fmt::Debug>(actual: &DashMap<K, V>, expected: &DashMap<K, V>, message: String) {
    let mut differences: Vec<String> = Vec::new();
    for entry in expected.iter() {
        match actual.get(entry.key()) {
            None => differences.push(format!("missing key {:?}", entry.key())),
            Some(value) if *value != *entry.value() => {
                differences.push(format!("key {:?}: expected {:?}, got {:?}", entry.key(), entry.value(), *value));
            }
            Some(_) => {}
        }
    }
    for entry in actual.iter() {
        if !expected.contains_key(entry.key()) {
            differences.push(format!("unexpected key {:?}", entry.key()));
        }
    }
    if !differences.is_empty() {
        differences.sort();
        report("test_assert_equal_hashmap", &message, differences.join("\n  "));
    }
}

/// An unconditional failure, for a branch a test must never reach.
pub fn fail(message: String) -> ! {
    report("test_fail", &message, "this branch must not run".to_string());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_holding_assertion_does_nothing() {
        assert(true, "a true condition".to_string());
        assert_false(false, "a false condition".to_string());
        assert_equal_int(2, 2, "equal integers".to_string());
        assert_not_equal_int(2, 3, "different integers".to_string());
        assert_greater_int(3, 2, "three exceeds two".to_string());
        assert_less_int(2, 3, "two is under three".to_string());
        assert_between_int(2, 1, 3, "two sits in the range".to_string());
        assert_between_int(1, 1, 3, "the low end is included".to_string());
        assert_between_int(3, 1, 3, "the high end is included".to_string());
        assert_equal_string("a".to_string(), "a".to_string(), "equal strings".to_string());
        assert_not_equal_string("a".to_string(), "b".to_string(), "different strings".to_string());
        assert_equal_bool(true, true, "equal booleans".to_string());
        assert_equal_float(0.1 + 0.2, 0.3, 1e-9, "floats within tolerance".to_string());
        assert_greater_float(0.3, 0.2, "the larger float".to_string());
        assert_less_float(0.2, 0.3, "the smaller float".to_string());
        assert_contains("hello world".to_string(), "lo wo".to_string(), "a substring".to_string());
        assert_not_contains("hello world".to_string(), "bye".to_string(), "an absent substring".to_string());
        assert_starts_with("hello world".to_string(), "hello".to_string(), "the opening".to_string());
        assert_ends_with("hello world".to_string(), "world".to_string(), "the closing".to_string());
        assert_equal_array(&vec![1, 2, 3], &vec![1, 2, 3], "equal arrays".to_string());
        assert_array_length(&vec![1, 2, 3], 3, "three elements".to_string());
        assert_array_contains(&vec![1, 2, 3], 2, "the middle element".to_string());
        assert_array_empty(&Vec::<i64>::new(), "an empty array".to_string());
        assert_array_not_empty(&vec![1], "a filled array".to_string());

        let left: DashMap<String, i64> = DashMap::new();
        left.insert("ada".to_string(), 36);
        let right: DashMap<String, i64> = DashMap::new();
        right.insert("ada".to_string(), 36);
        assert_equal_hashmap(&left, &right, "equal hashmaps".to_string());
    }

    #[test]
    #[should_panic(expected = "expected 3, got 2")]
    fn a_failing_integer_assertion_names_both_values() {
        assert_equal_int(2, 3, "the count".to_string());
    }

    #[test]
    #[should_panic(expected = "expected anything but 2")]
    fn a_failing_inequality_names_the_unwanted_value() {
        assert_not_equal_int(2, 2, "the count".to_string());
    }

    #[test]
    #[should_panic(expected = "expected a value greater than 3, got 3")]
    fn equal_is_not_greater() {
        assert_greater_int(3, 3, "the count".to_string());
    }

    #[test]
    #[should_panic(expected = "the range 3 to 1 is empty")]
    fn an_impossible_range_is_rejected() {
        assert_between_int(2, 3, 1, "the count".to_string());
    }

    #[test]
    #[should_panic(expected = "expected the text to start with `world`")]
    fn a_failing_prefix_assertion_names_the_prefix() {
        assert_starts_with("hello world".to_string(), "world".to_string(), "the opening".to_string());
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
    #[should_panic(expected = "expected to find 4 among 3 elements")]
    fn a_missing_element_reports_what_was_looked_for() {
        assert_array_contains(&vec![1, 2, 3], 4, "the new id".to_string());
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

    #[test]
    #[should_panic(expected = "missing key \"grace\"")]
    fn a_hashmap_difference_names_the_key() {
        let actual: DashMap<String, i64> = DashMap::new();
        actual.insert("ada".to_string(), 36);
        let expected: DashMap<String, i64> = DashMap::new();
        expected.insert("ada".to_string(), 36);
        expected.insert("grace".to_string(), 45);
        assert_equal_hashmap(&actual, &expected, "the ages".to_string());
    }

    #[test]
    #[should_panic(expected = "key \"ada\": expected 45, got 36")]
    fn a_differing_value_names_the_key_and_both_values() {
        let actual: DashMap<String, i64> = DashMap::new();
        actual.insert("ada".to_string(), 36);
        let expected: DashMap<String, i64> = DashMap::new();
        expected.insert("ada".to_string(), 45);
        assert_equal_hashmap(&actual, &expected, "the ages".to_string());
    }

    #[test]
    #[should_panic(expected = "test_fail failed: the fallback branch ran")]
    fn an_unconditional_failure_reports_its_message() {
        fail("the fallback branch ran".to_string());
    }
}

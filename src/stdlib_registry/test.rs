//! Test module stdlib registry entries.
//!
//! Every assertion takes the message describing what is being checked as its
//! last argument. It is required rather than optional because the message is
//! the only part of a failure that says where to look.

use super::*;

pub(super) fn register(m: &mut HashMap<&'static str, StdlibFunction>) {
    simple_fns! { m, Test:
        "test_assert" => "std_lib::test::assert", (condition: b, message: s) -> v,
            "Stops the program unless the condition is true, naming the check in the failure.",
            "test_assert(count > 0, `the import found rows`);";
        "test_assert_false" => "std_lib::test::assert_false", (condition: b, message: s) -> v,
            "Stops the program unless the condition is false.",
            "test_assert_false(string_is_empty(name), `the name is filled in`);";
        "test_assert_equal_int" => "std_lib::test::assert_equal_int", (actual: i, expected: i, message: s) -> v,
            "Stops the program unless the two whole numbers are equal, reporting both.",
            "test_assert_equal_int(total, 42, `the row count`);";
        "test_assert_equal_string" => "std_lib::test::assert_equal_string", (actual: s, expected: s, message: s) -> v,
            "Stops the program unless the two strings are equal, reporting both.",
            "test_assert_equal_string(greeting, `hello`, `the greeting`);";
        "test_assert_equal_bool" => "std_lib::test::assert_equal_bool", (actual: b, expected: b, message: s) -> v,
            "Stops the program unless the two booleans are equal.",
            "test_assert_equal_bool(finished, true, `the job finished`);";
        "test_assert_equal_float" => "std_lib::test::assert_equal_float", (actual: f, expected: f, tolerance: f, message: s) -> v,
            "Stops the program unless the two fractions are within the tolerance of each other. Floats are never compared exactly.",
            "test_assert_equal_float(ratio, 0.5, 0.0001, `the ratio`);";
        "test_assert_contains" => "std_lib::test::assert_contains", (haystack: s, needle: s, message: s) -> v,
            "Stops the program unless the text contains the fragment.",
            "test_assert_contains(page, `<title>`, `the page has a title`);";
        "test_assert_equal_array" => "std_lib::test::assert_equal_array", (actual: (&[T]), expected: (&[T]), message: s) -> v,
            "Stops the program unless the two arrays hold the same elements in the same order, naming the first position that differs.",
            "test_assert_equal_array(sorted, expected, `the sorted order`);";
    }
}

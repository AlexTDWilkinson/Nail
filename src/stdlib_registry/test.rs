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
            "count:i = 3;\ntest_assert(count > 0, `the import found rows`);";
        "test_assert_false" => "std_lib::test::assert_false", (condition: b, message: s) -> v,
            "Stops the program unless the condition is false.",
            "name:s = `Ada`;\ntest_assert_false(string_is_empty(name), `the name is filled in`);";
        "test_assert_equal_int" => "std_lib::test::assert_equal_int", (actual: i, expected: i, message: s) -> v,
            "Stops the program unless the two whole numbers are equal, reporting both.",
            "total:i = 42;\ntest_assert_equal_int(total, 42, `the row count`);";
        "test_assert_not_equal_int" => "std_lib::test::assert_not_equal_int", (actual: i, unwanted: i, message: s) -> v,
            "Stops the program if the two whole numbers are equal.",
            "new_id:i = 43;\nold_id:i = 42;\ntest_assert_not_equal_int(new_id, old_id, `the import made a fresh id`);";
        "test_assert_greater_int" => "std_lib::test::assert_greater_int", (actual: i, threshold: i, message: s) -> v,
            "Stops the program unless the value is strictly greater than the threshold.",
            "row_count:i = 12;\ntest_assert_greater_int(row_count, 0, `the import found rows`);";
        "test_assert_less_int" => "std_lib::test::assert_less_int", (actual: i, threshold: i, message: s) -> v,
            "Stops the program unless the value is strictly less than the threshold.",
            "retries:i = 2;\ntest_assert_less_int(retries, 5, `the retry loop stayed sane`);";
        "test_assert_between_int" => "std_lib::test::assert_between_int", (actual: i, low: i, high: i, message: s) -> v,
            "Stops the program unless the value is between low and high, both ends included.",
            "percent:i = 42;\ntest_assert_between_int(percent, 0, 100, `a percentage`);";
        "test_assert_equal_string" => "std_lib::test::assert_equal_string", (actual: s, expected: s, message: s) -> v,
            "Stops the program unless the two strings are equal, reporting both.",
            "greeting:s = `hello`;\ntest_assert_equal_string(greeting, `hello`, `the greeting`);";
        "test_assert_not_equal_string" => "std_lib::test::assert_not_equal_string", (actual: s, unwanted: s, message: s) -> v,
            "Stops the program if the two strings are equal.",
            "password:s = `hunter2`;\nhashed:s = danger(crypto_hash_password(password));\ntest_assert_not_equal_string(hashed, password, `the password is not stored bare`);";
        "test_assert_equal_bool" => "std_lib::test::assert_equal_bool", (actual: b, expected: b, message: s) -> v,
            "Stops the program unless the two booleans are equal.",
            "finished:b = true;\ntest_assert_equal_bool(finished, true, `the job finished`);";
        "test_assert_equal_float" => "std_lib::test::assert_equal_float", (actual: f, expected: f, tolerance: f, message: s) -> v,
            "Stops the program unless the two fractions are within the tolerance of each other. Floats are never compared exactly.",
            "ratio:f = 0.5;\ntest_assert_equal_float(ratio, 0.5, 0.0001, `the ratio`);";
        "test_assert_greater_float" => "std_lib::test::assert_greater_float", (actual: f, threshold: f, message: s) -> v,
            "Stops the program unless the value is strictly greater than the threshold. Ordering needs no tolerance.",
            "score:f = 0.62;\ntest_assert_greater_float(score, 0.0, `the model found some signal`);";
        "test_assert_less_float" => "std_lib::test::assert_less_float", (actual: f, threshold: f, message: s) -> v,
            "Stops the program unless the value is strictly less than the threshold. Ordering needs no tolerance.",
            "error_rate:f = 0.004;\ntest_assert_less_float(error_rate, 0.01, `the error rate stayed low`);";
        "test_assert_contains" => "std_lib::test::assert_contains", (haystack: s, needle: s, message: s) -> v,
            "Stops the program unless the text contains the fragment.",
            "page:s = `<title>Nail</title>`;\ntest_assert_contains(page, `<title>`, `the page has a title`);";
        "test_assert_not_contains" => "std_lib::test::assert_not_contains", (haystack: s, needle: s, message: s) -> v,
            "Stops the program if the text contains the fragment.",
            "page:s = `<title>Nail</title>`;\ntest_assert_not_contains(page, `Traceback`, `the page is not an error dump`);";
        "test_assert_starts_with" => "std_lib::test::assert_starts_with", (text: s, prefix: s, message: s) -> v,
            "Stops the program unless the text starts with the prefix.",
            "url:s = `https://nail-lang.org`;\ntest_assert_starts_with(url, `https://`, `the link is secure`);";
        "test_assert_ends_with" => "std_lib::test::assert_ends_with", (text: s, suffix: s, message: s) -> v,
            "Stops the program unless the text ends with the suffix.",
            "path:s = `website.nail`;\ntest_assert_ends_with(path, `.nail`, `a nail source file`);";
        "test_assert_equal_array" => "std_lib::test::assert_equal_array", (actual: (&[T]), expected: (&[T]), message: s) -> v,
            "Stops the program unless the two arrays hold the same elements in the same order, naming the first position that differs.",
            "sorted:a:i = [1, 2, 3];\nexpected:a:i = [1, 2, 3];\ntest_assert_equal_array(sorted, expected, `the sorted order`);";
        "test_assert_array_length" => "std_lib::test::assert_array_length", (array: (&[T]), expected: i, message: s) -> v,
            "Stops the program unless the array holds exactly that many elements.",
            "rows:a:s = [`one`, `two`, `three`];\ntest_assert_array_length(rows, 3, `every row survived the import`);";
        "test_assert_array_contains" => "std_lib::test::assert_array_contains", (array: (&[T]), item: T, message: s) -> v,
            "Stops the program unless the array contains the element.",
            "names:a:s = [`ada`, `grace`];\ntest_assert_array_contains(names, `ada`, `the founder is on the list`);";
        "test_assert_array_empty" => "std_lib::test::assert_array_empty", (array: (&[T]), message: s) -> v,
            "Stops the program unless the array is empty, reporting how many elements turned up.",
            "failures:a:s = [];\ntest_assert_array_empty(failures, `no import failed`);";
        "test_assert_array_not_empty" => "std_lib::test::assert_array_not_empty", (array: (&[T]), message: s) -> v,
            "Stops the program if the array is empty.",
            "rows:a:s = [`one`];\ntest_assert_array_not_empty(rows, `the query found something`);";
        "test_assert_equal_hashmap" [DashMap] => "std_lib::test::assert_equal_hashmap", (actual: (&(h K V)), expected: (&(h K V)), message: s) -> v,
            "Stops the program unless the two hashmaps hold the same keys with the same values, listing every difference by key in a stable order.",
            "counts:h<s,i> = hashmap_new();\nhashmap_set(counts, `nail`, 3);\nexpected:h<s,i> = hashmap_new();\nhashmap_set(expected, `nail`, 3);\ntest_assert_equal_hashmap(counts, expected, `the word counts`);";
    }

    m.insert("test_fail", StdlibFunction {
        rust_path: "std_lib::test::fail".to_string(),
        crate_deps: vec![],
        struct_derives: vec![],
        custom_type_imports: vec![],
        module: StdlibModule::Test,
        parameters: vec![nail_param!(message: s)],
        return_type: nail_type!(never),
        diverging: true,
        description: "Fails immediately, for a branch a test must never reach. Never returns.",
        example: "test_fail(`the fallback branch ran`);",
    });
}

//! Array module stdlib registry entries

use super::*;

pub(super) fn register(m: &mut HashMap<&'static str, StdlibFunction>) {
    simple_fns! { m, Array:
        "array_length" => "std_lib::array::len", (array: [T]) -> i,
            "Returns the number of elements in the array.",
            "count:i = array_length(numbers);";
        "array_push" => "std_lib::array::push", (array: [T], item: T) -> [T],
            "Returns a new array with the item appended to the end.",
            "more:a:i = array_push(numbers, 4);";
        "array_pop" => "std_lib::array::pop", (array: [T]) -> ([T]!e),
            "Returns a new array with the last element removed; errors if the array is empty.",
            "shorter:a:i = danger(array_pop(numbers));";
        "array_contains" => "std_lib::array::contains", (array: [T], item: T) -> b,
            "Returns true if the array contains the given item.",
            "found:b = array_contains(numbers, 3);";
        "array_join" => "std_lib::array::join", (array: [T], separator: s) -> s,
            "Converts each element to a string and joins them with the separator.",
            "csv:s = array_join(numbers, `, `);";
        "array_sort" => "std_lib::array::sort", (array: [T]) -> [T],
            "Returns a new array sorted in ascending order.",
            "sorted:a:i = array_sort(numbers);";
        "array_reverse" => "std_lib::array::reverse", (array: [T]) -> [T],
            "Returns a new array with the elements in reverse order.",
            "flipped:a:i = array_reverse(numbers);";
        "array_concat" => "std_lib::array::concat", (first: [T], second: [T]) -> [T],
            "Returns a new array containing all elements of the first array followed by the second.",
            "all:a:i = array_concat(evens, odds);";
        "array_get" => "std_lib::array::get", (array: [T], index: i) -> (T!e),
            "Returns the element at the given index, or an error if the index is out of bounds.",
            "item:i = danger(array_get(numbers, 0));";
        "array_first" => "std_lib::array::first", (array: [T]) -> (T!e),
            "Returns the first element, or an error if the array is empty.",
            "head:i = danger(array_first(numbers));";
        "array_last" => "std_lib::array::last", (array: [T]) -> (T!e),
            "Returns the last element, or an error if the array is empty.",
            "tail:i = danger(array_last(numbers));";
        "array_slice" => "std_lib::array::slice", (array: [T], start: i, end: i) -> ([T]!e),
            "Returns elements from start (inclusive) to end (exclusive), or an error if out of bounds.",
            "middle:a:i = danger(array_slice(numbers, 1, 3));";
        "array_take" => "std_lib::array::take", (array: [T], count: i) -> [T],
            "Returns a new array with the first count elements (fewer if the array is shorter).",
            "top:a:i = array_take(numbers, 3);";
        "array_skip" => "std_lib::array::skip", (array: [T], count: i) -> [T],
            "Returns a new array without the first count elements.",
            "rest:a:i = array_skip(numbers, 2);";
        "array_range" => "std_lib::array::array_range", (start: i, end: i) -> [i],
            "Returns integers from start (inclusive) to end (exclusive).",
            "nums:a:i = array_range(0, 5);";
        "array_range_inclusive" => "std_lib::array::array_range_inclusive", (start: i, end: i) -> [i],
            "Returns integers from start to end, both inclusive.",
            "nums:a:i = array_range_inclusive(1, 5);";
        "array_find" => "std_lib::array::find", (array: [T], value: T) -> (i!e),
            "Returns the index of the first occurrence of the value, or an error if not found.",
            "index:i = danger(array_find(numbers, 3));";
        "array_find_last" => "std_lib::array::find_last", (array: [T], value: T) -> (i!e),
            "Returns the index of the last occurrence of the value, or an error if not found.",
            "index:i = danger(array_find_last(numbers, 3));";
        "array_repeat" => "std_lib::array::repeat", (value: T, count: i) -> [T],
            "Returns an array containing the value repeated count times.",
            "zeros:a:i = array_repeat(0, 5);";
        "array_chunk" => "std_lib::array::chunk", (array: [T], size: i) -> ([[T]]!e),
            "Splits the array into chunks of the given size; errors if size is not positive.",
            "pairs:a:a:i = danger(array_chunk(numbers, 2));";
        "array_flatten" => "std_lib::array::flatten", (array: [[T]]) -> [T],
            "Flattens a nested array by one level.",
            "flat:a:i = array_flatten(nested);";
        "array_deduplicate" => "std_lib::array::deduplicate", (array: [T]) -> [T],
            "Removes consecutive duplicate elements.",
            "unique:a:i = array_deduplicate([1, 1, 2, 2, 3]);";
        "array_intersect" => "std_lib::array::intersect", (first: [T], second: [T]) -> [T],
            "Returns the elements present in both arrays, without duplicates.",
            "common:a:i = array_intersect(evens, primes);";
        "array_difference" => "std_lib::array::difference", (first: [T], second: [T]) -> [T],
            "Returns the elements of the first array that are not in the second.",
            "only:a:i = array_difference(all_ids, used_ids);";
        "array_union" => "std_lib::array::union", (first: [T], second: [T]) -> [T],
            "Returns all unique elements from both arrays.",
            "merged:a:i = array_union(evens, odds);";
        "array_rotate" => "std_lib::array::rotate", (array: [T], count: i) -> [T],
            "Rotates elements by count positions (positive rotates right, negative rotates left).",
            "moved:a:i = array_rotate(numbers, 2);";
        "array_shuffle" [Rand] => "std_lib::array::shuffle", (array: [T]) -> [T],
            "Returns a new array with the elements in random order.",
            "mixed:a:i = array_shuffle(numbers);";
        "array_rotate_left" => "std_lib::array::rotate_left", (array: [T], count: i) -> [T],
            "Rotates elements count positions to the left.",
            "moved:a:i = array_rotate_left(numbers, 1);";
        "array_rotate_right" => "std_lib::array::rotate_right", (array: [T], count: i) -> [T],
            "Rotates elements count positions to the right.",
            "moved:a:i = array_rotate_right(numbers, 1);";
        "array_sum" => "std_lib::array::sum", (array: [T]) -> T,
            "Returns the sum of all elements (0 for an empty array).",
            "total:i = array_sum(numbers);";
        "array_min" => "std_lib::array::min", (array: [T]) -> (T!e),
            "Returns the smallest element, or an error if the array is empty.",
            "lowest:i = danger(array_min(numbers));";
        "array_max" => "std_lib::array::max", (array: [T]) -> (T!e),
            "Returns the largest element, or an error if the array is empty.",
            "highest:i = danger(array_max(numbers));";
    }
}

//! Array module stdlib registry entries

use super::*;

pub(super) fn register(m: &mut HashMap<&'static str, StdlibFunction>) {
    simple_fns! { m, Array:
        "array_length" => "std_lib::array::len", (array: (&[T])) -> i,
            "Returns the number of elements in the array.",
            "count:i = array_length(numbers);";
        "array_push" => "std_lib::array::push", (array: [T], item: T) -> [T],
            "Returns a new array with the item appended to the end.",
            "more:a:i = array_push(numbers, 4);";
        "array_pop" => "std_lib::array::pop", (array: [T]) -> ([T]!e),
            "Returns a new array with the last element removed. Errors if the array is empty.",
            "shorter:a:i = danger(array_pop(numbers));";
        "array_contains" => "std_lib::array::contains", (array: (&[T]), item: T) -> b,
            "Returns true if the array contains the given item.",
            "found:b = array_contains(numbers, 3);";
        "array_join" => "std_lib::array::join", (array: (&[T]), separator: s) -> s,
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
            "evens:a:i = [2, 4];\nodds:a:i = [1, 3];\njoined:a:i = array_concat(evens, odds);";
        "array_get" => "std_lib::array::get", (array: (&[T]), index: i) -> (T!e),
            "Returns the element at the given index, or an error if the index is out of bounds.",
            "item:i = danger(array_get(numbers, 0));";
        "array_first" => "std_lib::array::first", (array: (&[T])) -> (T!e),
            "Returns the first element, or an error if the array is empty.",
            "head:i = danger(array_first(numbers));";
        "array_last" => "std_lib::array::last", (array: (&[T])) -> (T!e),
            "Returns the last element, or an error if the array is empty.",
            "tail:i = danger(array_last(numbers));";
        "array_slice" => "std_lib::array::slice", (array: (&[T]), start: i, end: i) -> ([T]!e),
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
        "array_find" => "std_lib::array::find", (array: (&[T]), value: T) -> (i!e),
            "Returns the index of the first occurrence of the value, or an error if not found.",
            "index:i = danger(array_find(numbers, 3));";
        "array_find_last" => "std_lib::array::find_last", (array: (&[T]), value: T) -> (i!e),
            "Returns the index of the last occurrence of the value, or an error if not found.",
            "index:i = danger(array_find_last(numbers, 3));";
        "array_repeat" => "std_lib::array::repeat", (value: T, count: i) -> [T],
            "Returns an array containing the value repeated count times.",
            "zeros:a:i = array_repeat(0, 5);";
        "array_chunk" => "std_lib::array::chunk", (array: (&[T]), size: i) -> ([[T]]!e),
            "Splits the array into chunks of the given size. Errors if size is not positive.",
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
        "array_sum" => "std_lib::array::sum", (array: (&[T])) -> T,
            "Returns the sum of all elements (0 for an empty array).",
            "total:i = array_sum(numbers);";
        "array_min" => "std_lib::array::min", (array: (&[T])) -> (T!e),
            "Returns the smallest element, or an error if the array is empty.",
            "lowest:i = danger(array_min(numbers));";
        "array_max" => "std_lib::array::max", (array: (&[T])) -> (T!e),
            "Returns the largest element, or an error if the array is empty.",
            "highest:i = danger(array_max(numbers));";
        "array_index_of" => "std_lib::array::index_of", (array: (&[T]), item: T) -> (i!e),
            "Returns the index where the item first appears, or an error if the array does not contain it.",
            "position:i = danger(array_index_of(names, `alice`));";
        "array_count_of" => "std_lib::array::count_of", (array: (&[T]), item: T) -> i,
            "Returns how many times the item appears in the array.",
            "repeats:i = array_count_of(rolls, 6);";
        "array_insert" => "std_lib::array::insert_at", (array: [T], index: i, item: T) -> ([T]!e),
            "Returns a new array with the item inserted at the index, moving the rest along. Errors if the index is past the end.",
            "queue:a:s = danger(array_insert(queue, 0, `urgent`));";
        "array_remove_at" => "std_lib::array::remove_at", (array: [T], index: i) -> ([T]!e),
            "Returns a new array without the element at the index. Errors if the index is out of bounds.",
            "rest:a:s = danger(array_remove_at(names, 2));";
        "array_replace_at" => "std_lib::array::replace_at", (array: [T], index: i, item: T) -> ([T]!e),
            "Returns a new array with the element at the index replaced. Errors if the index is out of bounds.",
            "fixed:a:s = danger(array_replace_at(names, 0, `alice`));";
        "array_swap" => "std_lib::array::swap", (array: [T], first: i, second: i) -> ([T]!e),
            "Returns a new array with the two elements exchanged. Errors if either index is out of bounds.",
            "reordered:a:i = danger(array_swap(numbers, 0, 1));";
        "array_all_equal" => "std_lib::array::all_equal", (array: (&[T])) -> b,
            "Returns true if every element equals the first one, and true for an empty array.",
            "uniform:b = array_all_equal(votes);";
        "array_is_empty" => "std_lib::array::is_empty", (array: (&[T])) -> b,
            "Returns true if the array has no elements.",
            "nothing_to_do:b = array_is_empty(queue);";
        "array_sort_descending" => "std_lib::array::sort_descending", (array: [T]) -> [T],
            "Returns a new array sorted from largest to smallest.",
            "leaderboard:a:i = array_sort_descending(scores);";
        "array_step_by" => "std_lib::array::step_by", (array: [T], step: i) -> ([T]!e),
            "Returns every step-th element starting with the first. Errors if step is less than 1.",
            "every_other:a:i = danger(array_step_by(numbers, 2));";
        "array_interleave" => "std_lib::array::interleave", (first: [T], second: [T]) -> [T],
            "Alternates elements from the two arrays. When one runs out, the rest of the other follows.",
            "woven:a:s = array_interleave(questions, answers);";
        "array_pad_end" => "std_lib::array::pad_end", (array: [T], length: i, value: T) -> [T],
            "Appends the value until the array reaches the length. An array already that long comes back unchanged.",
            "row:a:i = array_pad_end(numbers, 5, 0);";
        "array_pad_start" => "std_lib::array::pad_start", (array: [T], length: i, value: T) -> [T],
            "Prepends the value until the array reaches the length. An array already that long comes back unchanged.",
            "row:a:i = array_pad_start(numbers, 5, 0);";
        "array_is_sorted" => "std_lib::array::is_sorted", (array: (&[T])) -> b,
            "Returns true if each element is less than or equal to the next, and true for an empty array.",
            "ordered:b = array_is_sorted(scores);";
        "array_compact_strings" => "std_lib::array::compact_strings", (array: [s]) -> [s],
            "Removes empty strings from the array, keeping everything else in order.",
            "present:a:s = array_compact_strings(lines);";
        "array_middle" => "std_lib::array::middle", (array: (&[T])) -> (T!e),
            "Returns the middle element (the lower of the two middles when the length is even). Errors if the array is empty.",
            "median_name:s = danger(array_middle(names));";
        "array_take_last" => "std_lib::array::take_last", (array: [T], count: i) -> [T],
            "Returns a new array with the last count elements, in their original order (fewer if the array is shorter).",
            "recent:a:s = array_take_last(entries, 3);";
        "array_skip_last" => "std_lib::array::skip_last", (array: [T], count: i) -> [T],
            "Returns a new array without the last count elements.",
            "trimmed:a:s = array_skip_last(entries, 1);";
        "array_starts_with" => "std_lib::array::starts_with", (array: (&[T]), prefix: [T]) -> b,
            "Returns true if the array begins with the given prefix array, element for element.",
            "greeting:b = array_starts_with(words, [`hello`]);";
        "array_ends_with" => "std_lib::array::ends_with", (array: (&[T]), suffix: [T]) -> b,
            "Returns true if the array ends with the given suffix array, element for element.",
            "finished:b = array_ends_with(words, [`goodbye`]);";
        "array_is_unique" => "std_lib::array::is_unique", (array: (&[T])) -> b,
            "Returns true if no value appears more than once, and true for an empty array.",
            "no_repeats:b = array_is_unique(ids);";
        "array_count_runs" => "std_lib::array::count_runs", (array: (&[T])) -> i,
            "Returns how many runs of consecutive equal elements the array has (0 for an empty array).",
            "streaks:i = array_count_runs(results);";
        "array_common_prefix_length" => "std_lib::array::common_prefix_length", (first: (&[T]), second: (&[T])) -> i,
            "Returns how many elements the two arrays share at their start.",
            "shared:i = array_common_prefix_length(old_path, new_path);";
        "array_index_of_max" => "std_lib::array::index_of_max", (array: (&[T])) -> (i!e),
            "Returns the index of the largest element (the first one when tied). Errors if the array is empty.",
            "winner:i = danger(array_index_of_max(scores));";
        "array_index_of_min" => "std_lib::array::index_of_min", (array: (&[T])) -> (i!e),
            "Returns the index of the smallest element (the first one when tied). Errors if the array is empty.",
            "cheapest:i = danger(array_index_of_min(prices));";
        "array_sort_by" => "std_lib::array::sort_by_keys", (array: [T], key: (fn(T) -> K)) -> [T],
            "Returns the array sorted by what the named key function returns for each element, smallest first. The sort is stable, so elements with equal keys keep the order they came in. That is how to sort on more than one key: sort by the least important key first and the most important key last.",
            "by_year:a:Book = array_sort_by(books, book_year);";
        "array_sort_by_descending" => "std_lib::array::sort_by_keys_descending", (array: [T], key: (fn(T) -> K)) -> [T],
            "Returns the array sorted by the key function, largest first. Stable in the same way, and it reverses the order of the keys rather than the order of the ties, so one key can point down and another up in a stacked sort.",
            "newest_first:a:Book = array_sort_by_descending(books, book_year);";
        "array_min_by" => "std_lib::array::min_by_keys", (array: [T], key: (fn(T) -> K)) -> (T!e),
            "Returns the element whose key is smallest. An empty array is an error.",
            "oldest:Book = danger(array_min_by(books, book_year));";
        "array_max_by" => "std_lib::array::max_by_keys", (array: [T], key: (fn(T) -> K)) -> (T!e),
            "Returns the element whose key is largest. An empty array is an error.",
            "newest:Book = danger(array_max_by(books, book_year));";
        "array_sum_by" => "std_lib::array::sum_of_keys", (array: [T], key: (fn(T) -> (K: i|f))) -> K,
            "Returns every element's key added up, which is the total of a field over the array. An empty array sums to zero.",
            "total_pages:i = array_sum_by(books, book_pages);";
        "array_group_by" => "std_lib::array::group_by_keys", (array: [T], key: (fn(T) -> (K: i|s|b))) -> (h K [T]),
            "Buckets the elements by what the key function returns, keeping the order they appeared in inside each bucket. A key function returning true or false splits the array in two, which other languages call partition. For anything beyond bucketing, register the rows and write SQL.",
            "split:h<b,a:Invoice> = array_group_by(invoices, is_paid);";
        "array_count_by" => "std_lib::array::count_by_keys", (array: [T], key: (fn(T) -> (K: i|s|b))) -> (h K i),
            "Returns how many elements share each key, which is array_group_by when only the sizes matter.",
            "per_author:h<s,i> = array_count_by(books, book_author);";
        "array_take_while" => "std_lib::array::take_while_values", (array: [T], keep: (fn(T) -> b)) -> [T],
            "Returns the front of the array, up to the first element the named function says no to. Different from filter, which takes every element that passes wherever it sits - this stops at the first failure and ignores the rest.",
            "header:a:s = array_take_while(lines, line_is_not_blank);";
        "array_skip_while" => "std_lib::array::skip_while_values", (array: [T], skip: (fn(T) -> b)) -> [T],
            "Returns the rest of the array, from the first element the named function says no to onwards. The other half of array_take_while - the two together put the array back.",
            "body:a:s = array_skip_while(lines, line_is_not_blank);";
        "array_deduplicate_by" => "std_lib::array::deduplicate_by_keys", (array: [T], key: (fn(T) -> (K: i|s|b))) -> [T],
            "Returns the array with later elements dropped when their key has been seen before, keeping the first of each and the order they came in. Where array_deduplicate compares whole elements, this compares one thing about them, the way deduplicating records by address or id does.",
            "one_per_person:a:User = array_deduplicate_by(users, user_email);";
        "array_zip_with" => "std_lib::array::zip_with_values", (first: [A], second: [B], combine: (fn(A, B) -> C)) -> ([C]!e),
            "Walks two arrays in step and returns what the named function makes of each pair. Errors if the arrays are different lengths, since two lists meant to line up and not lining up is a bug worth hearing about.",
            "totals:a:f = danger(array_zip_with(prices, quantities, line_total));";
        "array_sort_natural" => "std_lib::array::sort_natural", (array: [s]) -> [s],
            "Sorts text the way a person reads names with numbers in them, so file2 comes before file10 instead of after it. Case is ignored, and names that differ only in case are settled by the text itself so the order never depends on the input order.",
            "in_order:a:s = array_sort_natural(filenames);";
        "array_binary_search" => "std_lib::array::binary_search", (array: (&[T]), item: T) -> (i!e),
            "Returns where the item sits in an already sorted array, found by halving the range rather than walking it. Errors when the array does not contain it. An unsorted array gets a wrong answer rather than an error, so use array_index_of when the order is not known.",
            "position:i = danger(array_binary_search(sorted_ids, 4096));";
        "array_insertion_point" => "std_lib::array::insertion_point", (array: (&[T]), item: T) -> i,
            "Returns the position the item would take in a sorted array, which is also how many elements come before it. Asking a sorted list of prices how many are under twenty, without a pass over the list.",
            "under:i = array_insertion_point(sorted_prices, 2000);";
        "array_insert_sorted" => "std_lib::array::insert_sorted", (array: [T], item: T) -> [T],
            "Returns the sorted array with one more item in it, still sorted. Keeps a leader board in order as scores arrive, without sorting the whole thing again.",
            "board:a:i = array_insert_sorted(board, new_score);";
        "array_page" => "std_lib::array::page", (array: (&[T]), page: i, per_page: i) -> ([T]!e),
            "Returns one page of the array, with pages numbered from 1. A page past the end is empty rather than an error, so a stale link shows nothing instead of breaking. Errors only when the page number or page size makes no sense.",
            "this_page:a:Post = danger(array_page(posts, 2, 20));";
        "array_windows" => "std_lib::array::windows", (array: (&[T]), size: i) -> ([[T]]!e),
            "Returns every run of neighbouring elements of that size, one step apart, so [1, 2, 3] in twos gives [1, 2] and [2, 3]. What a moving average or a three-in-a-row check reads. array_chunk is the one that cuts into pieces that do not overlap.",
            "pairs:a:a:i = danger(array_windows(readings, 2));";
        "array_combinations" => "std_lib::array::combinations", (array: (&[T]), size: i) -> ([[T]]!e),
            "Returns every way of choosing that many elements, order not counting. Refuses a request that would build more than a million arrays.",
            "pairings:a:a:s = danger(array_combinations(players, 2));";
        "array_permutations" => "std_lib::array::permutations", (array: (&[T])) -> ([[T]]!e),
            "Returns every ordering of the elements. Ten elements have three and a half million orderings, so anything that large is refused rather than attempted.",
            "orders:a:a:s = danger(array_permutations(stops));";
        "array_cartesian_product" => "std_lib::array::cartesian_product", (first: (&[T]), second: (&[T])) -> ([[T]]!e),
            "Returns every pairing of one element from each array, as two-element arrays, with the first array moving slowest. Sizes against colours, days against rooms.",
            "variants:a:a:s = danger(array_cartesian_product(sizes, colours));";
    }
}

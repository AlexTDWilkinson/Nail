//! Array module stdlib registry entries

use super::*;

pub(super) fn register(m: &mut HashMap<&'static str, StdlibFunction>) {
    simple_fns! { m, Array:
        "array_length" => "std_lib::array::len", (array: (&[T])) -> i,
            "Returns the number of elements in the array.",
            "numbers:a:i = [1, 2, 3];\ncount:i = array_length(numbers);";
        "array_push" => "std_lib::array::push", (array: [T], item: T) -> [T],
            "Returns a new array with the item appended to the end.",
            "numbers:a:i = [1, 2, 3];\nmore:a:i = array_push(numbers, 4);";
        "array_pop" => "std_lib::array::pop", (array: [T]) -> ([T]!e),
            "Returns a new array with the last element removed. Errors if the array is empty.",
            "numbers:a:i = [1, 2, 3];\nshorter:a:i = danger(array_pop(numbers));";
        "array_contains" => "std_lib::array::contains", (array: (&[T]), item: T) -> b,
            "Returns true if the array contains the given item.",
            "numbers:a:i = [1, 2, 3];\nfound:b = array_contains(numbers, 3);";
        "array_join" => "std_lib::array::join", (array: (&[T]), separator: s) -> s,
            "Converts each element to a string and joins them with the separator.",
            "numbers:a:i = [1, 2, 3];\ncsv:s = array_join(numbers, `, `);";
        "array_sort" => "std_lib::array::sort", (array: [T]) -> [T],
            "Returns a new array sorted in ascending order.",
            "numbers:a:i = [3, 1, 2];\nsorted:a:i = array_sort(numbers);";
        "array_reverse" => "std_lib::array::reverse", (array: [T]) -> [T],
            "Returns a new array with the elements in reverse order.",
            "numbers:a:i = [1, 2, 3];\nflipped:a:i = array_reverse(numbers);";
        "array_concat" => "std_lib::array::concat", (first: [T], second: [T]) -> [T],
            "Returns a new array containing all elements of the first array followed by the second.",
            "evens:a:i = [2, 4];\nodds:a:i = [1, 3];\njoined:a:i = array_concat(evens, odds);";
        "array_get" => "std_lib::array::get", (array: (&[T]), index: i) -> (T!e),
            "Returns the element at the given index, or an error if the index is out of bounds.",
            "numbers:a:i = [1, 2, 3];\nitem:i = danger(array_get(numbers, 0));";
        "array_first" => "std_lib::array::first", (array: (&[T])) -> (T!e),
            "Returns the first element, or an error if the array is empty.",
            "numbers:a:i = [1, 2, 3];\nhead:i = danger(array_first(numbers));";
        "array_last" => "std_lib::array::last", (array: (&[T])) -> (T!e),
            "Returns the last element, or an error if the array is empty.",
            "numbers:a:i = [1, 2, 3];\ntail:i = danger(array_last(numbers));";
        "array_slice" => "std_lib::array::slice", (array: (&[T]), start: i, end: i) -> ([T]!e),
            "Returns elements from start (inclusive) to end (exclusive), or an error if out of bounds.",
            "numbers:a:i = [1, 2, 3, 4];\nmiddle:a:i = danger(array_slice(numbers, 1, 3));";
        "array_take" => "std_lib::array::take", (array: [T], count: i) -> [T],
            "Returns a new array with the first count elements (fewer if the array is shorter).",
            "numbers:a:i = [1, 2, 3, 4];\ntop:a:i = array_take(numbers, 3);";
        "array_skip" => "std_lib::array::skip", (array: [T], count: i) -> [T],
            "Returns a new array without the first count elements.",
            "numbers:a:i = [1, 2, 3];\nrest:a:i = array_skip(numbers, 2);";
        "array_range" => "std_lib::array::array_range", (start: i, end: i) -> [i],
            "Returns integers from start (inclusive) to end (exclusive).",
            "nums:a:i = array_range(0, 5);";
        "array_range_inclusive" => "std_lib::array::array_range_inclusive", (start: i, end: i) -> [i],
            "Returns integers from start to end, both inclusive.",
            "nums:a:i = array_range_inclusive(1, 5);";
        "array_find" => "std_lib::array::find", (array: (&[T]), value: T) -> (i!e),
            "Returns the index of the first occurrence of the value, or an error if not found.",
            "numbers:a:i = [1, 2, 3];\nindex:i = danger(array_find(numbers, 3));";
        "array_find_last" => "std_lib::array::find_last", (array: (&[T]), value: T) -> (i!e),
            "Returns the index of the last occurrence of the value, or an error if not found.",
            "numbers:a:i = [1, 2, 3, 2];\nindex:i = danger(array_find_last(numbers, 2));";
        "array_repeat" => "std_lib::array::repeat", (value: T, count: i) -> [T],
            "Returns an array containing the value repeated count times.",
            "zeros:a:i = array_repeat(0, 5);";
        "array_chunk" => "std_lib::array::chunk", (array: (&[T]), size: i) -> ([[T]]!e),
            "Splits the array into chunks of the given size. Errors if size is not positive.",
            "numbers:a:i = [1, 2, 3, 4];\npairs:a:a:i = danger(array_chunk(numbers, 2));";
        "array_flatten" => "std_lib::array::flatten", (array: [[T]]) -> [T],
            "Flattens a nested array by one level.",
            "nested:a:a:i = [[1, 2], [3]];\nflat:a:i = array_flatten(nested);";
        "array_deduplicate" => "std_lib::array::deduplicate", (array: [T]) -> [T],
            "Removes consecutive duplicate elements.",
            "unique:a:i = array_deduplicate([1, 1, 2, 2, 3]);";
        "array_intersect" => "std_lib::array::intersect", (first: [T], second: [T]) -> [T],
            "Returns the elements present in both arrays, without duplicates.",
            "evens:a:i = [2, 4, 6];\nprimes:a:i = [2, 3, 5];\ncommon:a:i = array_intersect(evens, primes);";
        "array_difference" => "std_lib::array::difference", (first: [T], second: [T]) -> [T],
            "Returns the elements of the first array that are not in the second.",
            "all_ids:a:i = [1, 2, 3];\nused_ids:a:i = [2];\nonly:a:i = array_difference(all_ids, used_ids);";
        "array_union" => "std_lib::array::union", (first: [T], second: [T]) -> [T],
            "Returns all unique elements from both arrays.",
            "evens:a:i = [2, 4];\nodds:a:i = [1, 3];\nmerged:a:i = array_union(evens, odds);";
        "array_rotate" => "std_lib::array::rotate", (array: [T], count: i) -> [T],
            "Rotates elements by count positions (positive rotates right, negative rotates left).",
            "numbers:a:i = [1, 2, 3];\nmoved:a:i = array_rotate(numbers, 2);";
        "array_shuffle" [Rand] => "std_lib::array::shuffle", (array: [T]) -> [T],
            "Returns a new array with the elements in random order.",
            "numbers:a:i = [1, 2, 3];\nmixed:a:i = array_shuffle(numbers);";
        "array_rotate_left" => "std_lib::array::rotate_left", (array: [T], count: i) -> [T],
            "Rotates elements count positions to the left.",
            "numbers:a:i = [1, 2, 3];\nmoved:a:i = array_rotate_left(numbers, 1);";
        "array_rotate_right" => "std_lib::array::rotate_right", (array: [T], count: i) -> [T],
            "Rotates elements count positions to the right.",
            "numbers:a:i = [1, 2, 3];\nmoved:a:i = array_rotate_right(numbers, 1);";
        "array_sum" => "std_lib::array::sum", (array: (&[T])) -> T,
            "Returns the sum of all elements (0 for an empty array).",
            "numbers:a:i = [1, 2, 3];\ntotal:i = array_sum(numbers);";
        "array_min" => "std_lib::array::min", (array: (&[T])) -> (T!e),
            "Returns the smallest element, or an error if the array is empty.",
            "numbers:a:i = [1, 2, 3];\nlowest:i = danger(array_min(numbers));";
        "array_max" => "std_lib::array::max", (array: (&[T])) -> (T!e),
            "Returns the largest element, or an error if the array is empty.",
            "numbers:a:i = [1, 2, 3];\nhighest:i = danger(array_max(numbers));";
        "array_index_of" => "std_lib::array::index_of", (array: (&[T]), item: T) -> (i!e),
            "Returns the index where the item first appears, or an error if the array does not contain it.",
            "names:a:s = [`alice`, `bob`];\nposition:i = danger(array_index_of(names, `alice`));";
        "array_count_of" => "std_lib::array::count_of", (array: (&[T]), item: T) -> i,
            "Returns how many times the item appears in the array.",
            "rolls:a:i = [6, 2, 6];\nrepeats:i = array_count_of(rolls, 6);";
        "array_insert" => "std_lib::array::insert_at", (array: [T], index: i, item: T) -> ([T]!e),
            "Returns a new array with the item inserted at the index, moving the rest along. Errors if the index is past the end.",
            "queue:a:s = [`build`, `deploy`];\nwith_urgent:a:s = danger(array_insert(queue, 0, `urgent`));";
        "array_remove_at" => "std_lib::array::remove_at", (array: [T], index: i) -> ([T]!e),
            "Returns a new array without the element at the index. Errors if the index is out of bounds.",
            "names:a:s = [`alice`, `bob`, `carol`];\nrest:a:s = danger(array_remove_at(names, 2));";
        "array_replace_at" => "std_lib::array::replace_at", (array: [T], index: i, item: T) -> ([T]!e),
            "Returns a new array with the element at the index replaced. Errors if the index is out of bounds.",
            "names:a:s = [`alice`, `bob`];\nfixed:a:s = danger(array_replace_at(names, 0, `alice`));";
        "array_swap" => "std_lib::array::swap", (array: [T], first: i, second: i) -> ([T]!e),
            "Returns a new array with the two elements exchanged. Errors if either index is out of bounds.",
            "numbers:a:i = [1, 2, 3];\nreordered:a:i = danger(array_swap(numbers, 0, 1));";
        "array_all_equal" => "std_lib::array::all_equal", (array: (&[T])) -> b,
            "Returns true if every element equals the first one, and true for an empty array.",
            "votes:a:s = [`yes`, `yes`, `yes`];\nuniform:b = array_all_equal(votes);";
        "array_is_empty" => "std_lib::array::is_empty", (array: (&[T])) -> b,
            "Returns true if the array has no elements.",
            "queue:a:s = [];\nnothing_to_do:b = array_is_empty(queue);";
        "array_sort_descending" => "std_lib::array::sort_descending", (array: [T]) -> [T],
            "Returns a new array sorted from largest to smallest.",
            "scores:a:i = [11, 47, 22];\nleaderboard:a:i = array_sort_descending(scores);";
        "array_step_by" => "std_lib::array::step_by", (array: [T], step: i) -> ([T]!e),
            "Returns every step-th element starting with the first. Errors if step is less than 1.",
            "numbers:a:i = [1, 2, 3, 4];\nevery_other:a:i = danger(array_step_by(numbers, 2));";
        "array_interleave" => "std_lib::array::interleave", (first: [T], second: [T]) -> [T],
            "Alternates elements from the two arrays. When one runs out, the rest of the other follows.",
            "questions:a:s = [`name?`, `age?`];\nanswers:a:s = [`Ada`, `36`];\nwoven:a:s = array_interleave(questions, answers);";
        "array_pad_end" => "std_lib::array::pad_end", (array: [T], length: i, value: T) -> [T],
            "Appends the value until the array reaches the length. An array already that long comes back unchanged.",
            "numbers:a:i = [1, 2, 3];\nrow:a:i = array_pad_end(numbers, 5, 0);";
        "array_pad_start" => "std_lib::array::pad_start", (array: [T], length: i, value: T) -> [T],
            "Prepends the value until the array reaches the length. An array already that long comes back unchanged.",
            "numbers:a:i = [1, 2, 3];\nrow:a:i = array_pad_start(numbers, 5, 0);";
        "array_is_sorted" => "std_lib::array::is_sorted", (array: (&[T])) -> b,
            "Returns true if each element is less than or equal to the next, and true for an empty array.",
            "scores:a:i = [11, 22, 47];\nordered:b = array_is_sorted(scores);";
        "array_compact_strings" => "std_lib::array::compact_strings", (array: [s]) -> [s],
            "Removes empty strings from the array, keeping everything else in order.",
            "lines:a:s = [`first`, ``, `second`];\npresent:a:s = array_compact_strings(lines);";
        "array_middle" => "std_lib::array::middle", (array: (&[T])) -> (T!e),
            "Returns the middle element (the lower of the two middles when the length is even). Errors if the array is empty.",
            "names:a:s = [`alice`, `bob`, `carol`];\nmedian_name:s = danger(array_middle(names));";
        "array_take_last" => "std_lib::array::take_last", (array: [T], count: i) -> [T],
            "Returns a new array with the last count elements, in their original order (fewer if the array is shorter).",
            "entries:a:s = [`first`, `second`, `third`, `fourth`];\nrecent:a:s = array_take_last(entries, 3);";
        "array_skip_last" => "std_lib::array::skip_last", (array: [T], count: i) -> [T],
            "Returns a new array without the last count elements.",
            "entries:a:s = [`first`, `second`, `last`];\ntrimmed:a:s = array_skip_last(entries, 1);";
        "array_starts_with" => "std_lib::array::starts_with", (array: (&[T]), prefix: [T]) -> b,
            "Returns true if the array begins with the given prefix array, element for element.",
            "words:a:s = [`hello`, `goodbye`];\ngreeting:b = array_starts_with(words, [`hello`]);";
        "array_ends_with" => "std_lib::array::ends_with", (array: (&[T]), suffix: [T]) -> b,
            "Returns true if the array ends with the given suffix array, element for element.",
            "words:a:s = [`hello`, `goodbye`];\nfinished:b = array_ends_with(words, [`goodbye`]);";
        "array_is_unique" => "std_lib::array::is_unique", (array: (&[T])) -> b,
            "Returns true if no value appears more than once, and true for an empty array.",
            "ids:a:i = [1, 2, 3];\nno_repeats:b = array_is_unique(ids);";
        "array_count_runs" => "std_lib::array::count_runs", (array: (&[T])) -> i,
            "Returns how many runs of consecutive equal elements the array has (0 for an empty array).",
            "results:a:s = [`win`, `win`, `loss`];\nstreaks:i = array_count_runs(results);";
        "array_common_prefix_length" => "std_lib::array::common_prefix_length", (first: (&[T]), second: (&[T])) -> i,
            "Returns how many elements the two arrays share at their start.",
            "old_path:a:s = [`srv`, `app`, `old`];\nnew_path:a:s = [`srv`, `app`, `new`];\nshared:i = array_common_prefix_length(old_path, new_path);";
        "array_index_of_max" => "std_lib::array::index_of_max", (array: (&[T])) -> (i!e),
            "Returns the index of the largest element (the first one when tied). Errors if the array is empty.",
            "scores:a:i = [11, 47, 22];\nwinner:i = danger(array_index_of_max(scores));";
        "array_index_of_min" => "std_lib::array::index_of_min", (array: (&[T])) -> (i!e),
            "Returns the index of the smallest element (the first one when tied). Errors if the array is empty.",
            "prices:a:i = [1200, 850, 990];\ncheapest:i = danger(array_index_of_min(prices));";
        "array_sort_by" => "std_lib::array::sort_by_keys", (array: [T], key: (fn(T) -> K)) -> [T],
            "Returns the array sorted by what the named key function returns for each element, smallest first. The sort is stable, so elements with equal keys keep the order they came in. That is how to sort on more than one key: sort by the least important key first and the most important key last.",
            "struct Book { title:s, author:s, year:i, pages:i }\n\nbooks:a:Book = [\n    Book { title = `Middlemarch`, author = `Eliot`, year = 1871, pages = 904 },\n    Book { title = `Silas Marner`, author = `Eliot`, year = 1861, pages = 208 },\n];\n\nf book_year(book:Book):i { r book.year; }\n\nby_year:a:Book = array_sort_by(books, book_year);";
        "array_sort_by_descending" => "std_lib::array::sort_by_keys_descending", (array: [T], key: (fn(T) -> K)) -> [T],
            "Returns the array sorted by the key function, largest first. Stable in the same way, and it reverses the order of the keys rather than the order of the ties, so one key can point down and another up in a stacked sort.",
            "struct Book { title:s, author:s, year:i, pages:i }\n\nbooks:a:Book = [\n    Book { title = `Middlemarch`, author = `Eliot`, year = 1871, pages = 904 },\n    Book { title = `Silas Marner`, author = `Eliot`, year = 1861, pages = 208 },\n];\n\nf book_year(book:Book):i { r book.year; }\n\nnewest_first:a:Book = array_sort_by_descending(books, book_year);";
        "array_min_by" => "std_lib::array::min_by_keys", (array: [T], key: (fn(T) -> K)) -> (T!e),
            "Returns the element whose key is smallest. An empty array is an error.",
            "struct Book { title:s, author:s, year:i, pages:i }\n\nbooks:a:Book = [\n    Book { title = `Middlemarch`, author = `Eliot`, year = 1871, pages = 904 },\n    Book { title = `Silas Marner`, author = `Eliot`, year = 1861, pages = 208 },\n];\n\nf book_year(book:Book):i { r book.year; }\n\noldest:Book = danger(array_min_by(books, book_year));";
        "array_max_by" => "std_lib::array::max_by_keys", (array: [T], key: (fn(T) -> K)) -> (T!e),
            "Returns the element whose key is largest. An empty array is an error.",
            "struct Book { title:s, author:s, year:i, pages:i }\n\nbooks:a:Book = [\n    Book { title = `Middlemarch`, author = `Eliot`, year = 1871, pages = 904 },\n    Book { title = `Silas Marner`, author = `Eliot`, year = 1861, pages = 208 },\n];\n\nf book_year(book:Book):i { r book.year; }\n\nnewest:Book = danger(array_max_by(books, book_year));";
        "array_sum_by" => "std_lib::array::sum_of_keys", (array: [T], key: (fn(T) -> (K: i|f))) -> K,
            "Returns every element's key added up, which is the total of a field over the array. An empty array sums to zero.",
            "struct Book { title:s, author:s, year:i, pages:i }\n\nbooks:a:Book = [\n    Book { title = `Middlemarch`, author = `Eliot`, year = 1871, pages = 904 },\n    Book { title = `Silas Marner`, author = `Eliot`, year = 1861, pages = 208 },\n];\n\nf book_pages(book:Book):i { r book.pages; }\n\ntotal_pages:i = array_sum_by(books, book_pages);";
        "array_group_by" => "std_lib::array::group_by_keys", (array: [T], key: (fn(T) -> (K: i|s|b))) -> (h K [T]),
            "Buckets the elements by what the key function returns, keeping the order they appeared in inside each bucket. A key function returning true or false splits the array in two, which other languages call partition. For anything beyond bucketing, register the rows and write SQL.",
            "struct Invoice { reference:s, paid:b }\n\ninvoices:a:Invoice = [\n    Invoice { reference = `INV-1`, paid = true },\n    Invoice { reference = `INV-2`, paid = false },\n];\n\nf is_paid(invoice:Invoice):b { r invoice.paid; }\n\nsplit:h<b,a:Invoice> = array_group_by(invoices, is_paid);";
        "array_count_by" => "std_lib::array::count_by_keys", (array: [T], key: (fn(T) -> (K: i|s|b))) -> (h K i),
            "Returns how many elements share each key, which is array_group_by when only the sizes matter.",
            "struct Book { title:s, author:s, year:i, pages:i }\n\nbooks:a:Book = [\n    Book { title = `Middlemarch`, author = `Eliot`, year = 1871, pages = 904 },\n    Book { title = `Silas Marner`, author = `Eliot`, year = 1861, pages = 208 },\n];\n\nf book_author(book:Book):s { r book.author; }\n\nper_author:h<s,i> = array_count_by(books, book_author);";
        "array_take_while" => "std_lib::array::take_while_values", (array: [T], keep: (fn(T) -> b)) -> [T],
            "Returns the front of the array, up to the first element the named function says no to. Different from filter, which takes every element that passes wherever it sits - this stops at the first failure and ignores the rest.",
            "lines:a:s = [`title`, ``, `body`];\n\nf line_is_not_blank(line:s):b { r line != ``; }\n\nheader:a:s = array_take_while(lines, line_is_not_blank);";
        "array_skip_while" => "std_lib::array::skip_while_values", (array: [T], skip: (fn(T) -> b)) -> [T],
            "Returns the rest of the array, from the first element the named function says no to onwards. The other half of array_take_while - the two together put the array back.",
            "lines:a:s = [`title`, ``, `body`];\n\nf line_is_not_blank(line:s):b { r line != ``; }\n\nbody:a:s = array_skip_while(lines, line_is_not_blank);";
        "array_deduplicate_by" => "std_lib::array::deduplicate_by_keys", (array: [T], key: (fn(T) -> (K: i|s|b))) -> [T],
            "Returns the array with later elements dropped when their key has been seen before, keeping the first of each and the order they came in. Where array_deduplicate compares whole elements, this compares one thing about them, the way deduplicating records by address or id does.",
            "struct User { name:s, email:s }\n\nusers:a:User = [\n    User { name = `Ada`, email = `ada@example.com` },\n    User { name = `Ada Lovelace`, email = `ada@example.com` },\n];\n\nf user_email(user:User):s { r user.email; }\n\none_per_person:a:User = array_deduplicate_by(users, user_email);";
        "array_zip_with" => "std_lib::array::zip_with_values", (first: [A], second: [B], combine: (fn(A, B) -> C)) -> ([C]!e),
            "Walks two arrays in step and returns what the named function makes of each pair. Errors if the arrays are different lengths, since two lists meant to line up and not lining up is a bug worth hearing about.",
            "prices:a:f = [9.99, 4.50];\nquantities:a:i = [2, 1];\n\nf line_total(price:f, quantity:i):f {\n    count:f = danger(float_from(quantity));\n    r price * count;\n}\n\ntotals:a:f = danger(array_zip_with(prices, quantities, line_total));";
        "array_sort_natural" => "std_lib::array::sort_natural", (array: [s]) -> [s],
            "Sorts text the way a person reads names with numbers in them, so file2 comes before file10 instead of after it. Case is ignored, and names that differ only in case are settled by the text itself so the order never depends on the input order.",
            "filenames:a:s = [`page10.txt`, `page2.txt`];\nin_order:a:s = array_sort_natural(filenames);";
        "array_binary_search" => "std_lib::array::binary_search", (array: (&[T]), item: T) -> (i!e),
            "Returns where the item sits in an already sorted array, found by halving the range rather than walking it. Errors when the array does not contain it. An unsorted array gets a wrong answer rather than an error, so use array_index_of when the order is not known.",
            "sorted_ids:a:i = [12, 512, 4096];\nposition:i = danger(array_binary_search(sorted_ids, 4096));";
        "array_insertion_point" => "std_lib::array::insertion_point", (array: (&[T]), item: T) -> i,
            "Returns the position the item would take in a sorted array, which is also how many elements come before it. Asking a sorted list of prices how many are under twenty, without a pass over the list.",
            "sorted_prices:a:i = [900, 1500, 2500];\nunder:i = array_insertion_point(sorted_prices, 2000);";
        "array_insert_sorted" => "std_lib::array::insert_sorted", (array: [T], item: T) -> [T],
            "Returns the sorted array with one more item in it, still sorted. Keeps a leader board in order as scores arrive, without sorting the whole thing again.",
            "board:a:i = [10, 30, 50];\nnew_score:i = 40;\nupdated:a:i = array_insert_sorted(board, new_score);";
        "array_page" => "std_lib::array::page", (array: (&[T]), page: i, per_page: i) -> ([T]!e),
            "Returns one page of the array, with pages numbered from 1. A page past the end is empty rather than an error, so a stale link shows nothing instead of breaking. Errors only when the page number or page size makes no sense.",
            "struct Post { title:s }\n\nposts:a:Post = [\n    Post { title = `first` },\n    Post { title = `second` },\n];\nthis_page:a:Post = danger(array_page(posts, 1, 20));";
        "array_windows" => "std_lib::array::windows", (array: (&[T]), size: i) -> ([[T]]!e),
            "Returns every run of neighbouring elements of that size, one step apart, so [1, 2, 3] in twos gives [1, 2] and [2, 3]. What a moving average or a three-in-a-row check reads. array_chunk is the one that cuts into pieces that do not overlap.",
            "readings:a:i = [10, 12, 15];\npairs:a:a:i = danger(array_windows(readings, 2));";
        "array_combinations" => "std_lib::array::combinations", (array: (&[T]), size: i) -> ([[T]]!e),
            "Returns every way of choosing that many elements, order not counting. Refuses a request that would build more than a million arrays.",
            "players:a:s = [`ada`, `grace`, `alan`];\npairings:a:a:s = danger(array_combinations(players, 2));";
        "array_permutations" => "std_lib::array::permutations", (array: (&[T])) -> ([[T]]!e),
            "Returns every ordering of the elements. Ten elements have three and a half million orderings, so anything that large is refused rather than attempted.",
            "stops:a:s = [`home`, `depot`, `site`];\norders:a:a:s = danger(array_permutations(stops));";
        "array_cartesian_product" => "std_lib::array::cartesian_product", (first: (&[T]), second: (&[T])) -> ([[T]]!e),
            "Returns every pairing of one element from each array, as two-element arrays, with the first array moving slowest. Sizes against colours, days against rooms.",
            "sizes:a:s = [`small`, `large`];\ncolours:a:s = [`red`, `blue`];\nvariants:a:a:s = danger(array_cartesian_product(sizes, colours));";
    }
}

use rayon::prelude::*;
use std::cmp::Ordering;

// Thresholds below which a parallel pass loses to a sequential one.
//
// Entering rayon's pool costs on the order of 10 microseconds regardless of
// input size, so small collections pay coordination for nothing. These numbers
// are measured, not guessed (24-core machine, release build, i64 elements):
//
//     min/max    200k: 0.42x    500k: 0.75x    1M: 1.23x    4M: 2.64x
//     sum          1M: 0.63x      2M: 1.34x    4M: 1.98x
//
// Summing is memory-bandwidth bound and vectorises well sequentially, so it
// needs a larger input before threading pays; comparison-based reduction
// crosses over sooner. Elements whose comparison is expensive cross over much
// sooner still - String min is already 2.69x at 200k - so these thresholds are
// deliberately conservative for the cheapest element type rather than tuned
// for the best case.
const PARALLEL_MIN_LEN_REDUCE: usize = 1_000_000;
const PARALLEL_MIN_LEN_SUM: usize = 2_000_000;

/// Whether a collection of this size is worth handing to rayon.
///
/// Sequential cost is n*c, parallel cost is O + n*c/p, so parallelism wins
/// once n > O / (c * (1 - 1/p)) - where c is the per-element cost, p the
/// thread count and O rayon's overhead. The constants above fix O/c by
/// measurement; the 1/(1 - 1/p) term is applied here because it is the part
/// that depends on the machine rather than the operation. It is negligible on
/// many cores (1.04x at 24) but doubles the threshold on a dual-core box,
/// which is exactly where getting this wrong hurts most.
///
/// Single-core machines (small VMs, containers with one CPU) are never worth
/// it - there is no second thread to win anything back, only coordination to
/// pay for.
fn worth_parallel(len: usize, min_len: usize) -> bool {
    let threads = rayon::current_num_threads();
    if threads < 2 {
        return false;
    }
    let scaled = (min_len as u128 * threads as u128) / (threads as u128 - 1);
    len as u128 >= scaled
}

/// Whether a hand-written fold of this many elements is worth handing to rayon.
///
/// Called by transpiled code. The compiler parallelises a `reduce` only when it
/// can prove from the fold's shape that regrouping cannot change the answer,
/// and then defers the size question to the same measured crossover the stdlib
/// reductions use: an integer fold is the cheapest per element, so it needs the
/// largest input before threading pays for itself.
pub fn worth_parallel_fold(len: usize) -> bool {
    worth_parallel(len, PARALLEL_MIN_LEN_SUM)
}

pub fn len<T>(arr: &Vec<T>) -> i64 {
    arr.len() as i64
}

pub fn push<T: Clone>(mut arr: Vec<T>, item: T) -> Vec<T> {
    arr.push(item);
    arr
}

// Returns a new array with the last element removed; errors on empty arrays.
// Use array_last to read the final element - Nail arrays are immutable, so
// pop never mutates in place.
pub fn pop<T: Clone>(mut arr: Vec<T>) -> Result<Vec<T>, String> {
    if arr.pop().is_none() {
        return Err("array_pop: cannot pop from an empty array".to_string());
    }
    Ok(arr)
}

pub fn contains<T: PartialEq + Sync + Send>(arr: &Vec<T>, item: T) -> bool 
where T: Sync + Send
{
    use rayon::prelude::*;
    use rayon::iter::IntoParallelIterator;
    arr.par_iter().any(|x| x == &item)
}

pub fn join<T: std::fmt::Display + Send + Sync>(arr: &Vec<T>, separator: String) -> String {
    use rayon::prelude::*;
    use rayon::iter::IntoParallelIterator;
    
    arr.par_iter()
        .map(|item| format!("{}", item))
        .collect::<Vec<String>>()
        .join(&separator)
}

pub fn sort<T: Ord + Clone + Send>(mut arr: Vec<T>) -> Vec<T> {
    use rayon::prelude::*;
    use rayon::iter::IntoParallelIterator;
    arr.par_sort();
    arr
}

pub fn reverse<T: Clone>(mut arr: Vec<T>) -> Vec<T> {
    arr.reverse();
    arr
}

// Concatenate two arrays
pub fn concat<T: Clone>(mut first: Vec<T>, second: Vec<T>) -> Vec<T> {
    first.extend(second);
    first
}

// Safe array indexing - returns Result
pub fn get<T: Clone>(arr: &Vec<T>, index: i64) -> Result<T, String> {
    if index < 0 {
        return Err(format!("array_get: index cannot be negative, got {}", index));
    }

    let idx = index as usize;
    if idx >= arr.len() {
        return Err(format!("array_get: index {} is out of bounds for an array of length {}", index, arr.len()));
    }

    Ok(arr[idx].clone())
}

// Get first element
pub fn first<T: Clone>(arr: &Vec<T>) -> Result<T, String> {
    arr.first().cloned().ok_or_else(|| "array_first: cannot get the first element of an empty array".to_string())
}

// Get last element
pub fn last<T: Clone>(arr: &Vec<T>) -> Result<T, String> {
    arr.last().cloned().ok_or_else(|| "array_last: cannot get the last element of an empty array".to_string())
}

// Safe array slicing
pub fn slice<T: Clone>(arr: &Vec<T>, start: i64, end: i64) -> Result<Vec<T>, String> {
    if start < 0 || end < 0 {
        return Err(format!("array_slice: indices cannot be negative, got {}..{}", start, end));
    }

    let start_idx = start as usize;
    let end_idx = end as usize;

    if start_idx > arr.len() || end_idx > arr.len() {
        return Err(format!("array_slice: range {}..{} is out of bounds for an array of length {}", start, end, arr.len()));
    }

    if start_idx > end_idx {
        return Err(format!("array_slice: start index {} is greater than end index {}", start, end));
    }

    Ok(arr[start_idx..end_idx].to_vec())
}

// Take first n elements
pub fn take<T: Clone>(arr: Vec<T>, n: i64) -> Vec<T> {
    if n <= 0 {
        return Vec::new();
    }

    let count = (n as usize).min(arr.len());
    arr[..count].to_vec()
}

// Skip first n elements
pub fn skip<T: Clone>(arr: Vec<T>, n: i64) -> Vec<T> {
    if n <= 0 {
        return arr.clone();
    }

    let count = (n as usize).min(arr.len());
    arr[count..].to_vec()
}

// Generic array unique - returns array with unique elements only
pub fn unique<T>(arr: Vec<T>) -> Vec<T>
where
    T: PartialEq + Clone,
{
    let mut result = Vec::new();
    for item in arr {
        if !result.contains(&item) {
            result.push(item);
        }
    }
    result
}

// Flatten a nested array by one level
pub fn flatten<T>(arr: Vec<Vec<T>>) -> Vec<T> {
    arr.into_iter().flatten().collect()
}

// Generic array zip - combines two arrays into array of tuples
pub fn zip<T, U>(arr1: Vec<T>, arr2: Vec<U>) -> Vec<(T, U)> {
    arr1.into_iter().zip(arr2).collect()
}


// Generic min/max functions for arrays (PartialOrd so they work for floats)
//
// min, max and sum are associative, so splitting the work across cores and
// combining the pieces gives the same answer as a left-to-right fold. That is
// why these can be parallel while a user-written reduce cannot: the compiler
// wrote these operations and knows they hold, instead of having to take the
// programmer's word for it. Large inputs go to rayon, small ones stay
// sequential - see worth_parallel above.
pub fn min<T: Clone + Send + Sync>(arr: &Vec<T>) -> Result<T, String>
where
    T: PartialOrd,
{
    if arr.is_empty() {
        return Err("array_min: cannot get the minimum of an empty array".to_string());
    }
    if worth_parallel(arr.len(), PARALLEL_MIN_LEN_REDUCE) {
        // Reduce over references and clone only the winner. Reducing over
        // cloned values instead would allocate once per element, which for
        // String elements costs more than the parallelism saves.
        //
        // reduce_with needs no identity element, which matters here: there is
        // no universal "largest value" to seed a minimum with for arbitrary T.
        return arr
            .par_iter()
            .reduce_with(|a, b| if b < a { b } else { a })
            .cloned()
            .ok_or_else(|| "array_min: cannot get the minimum of an empty array".to_string());
    }
    let mut iter = arr.iter();
    let mut best = iter.next().ok_or_else(|| "array_min: cannot get the minimum of an empty array".to_string())?;
    for item in iter {
        if item < best {
            best = item;
        }
    }
    Ok(best.clone())
}

pub fn max<T: Clone + Send + Sync>(arr: &Vec<T>) -> Result<T, String>
where
    T: PartialOrd,
{
    if arr.is_empty() {
        return Err("array_max: cannot get the maximum of an empty array".to_string());
    }
    if worth_parallel(arr.len(), PARALLEL_MIN_LEN_REDUCE) {
        // Reduce over references, clone only the winner - see min above.
        return arr
            .par_iter()
            .reduce_with(|a, b| if b > a { b } else { a })
            .cloned()
            .ok_or_else(|| "array_max: cannot get the maximum of an empty array".to_string());
    }
    let mut iter = arr.iter();
    let mut best = iter.next().ok_or_else(|| "array_max: cannot get the maximum of an empty array".to_string())?;
    for item in iter {
        if item > best {
            best = item;
        }
    }
    Ok(best.clone())
}

// Sum of all elements (0 for an empty array)
pub fn sum<T: Clone + Send + Sync>(arr: &Vec<T>) -> T
where
    T: std::iter::Sum<T>,
{
    if worth_parallel(arr.len(), PARALLEL_MIN_LEN_SUM) {
        return arr.par_iter().cloned().sum();
    }
    arr.iter().cloned().sum()
}


// Range function - generates a range of integers (exclusive end, like Python)
pub fn array_range(start: i64, end: i64) -> Vec<i64> {
    (start..end).collect()
}

// Range inclusive 
pub fn array_range_inclusive(start: i64, end: i64) -> Vec<i64> {
    (start..=end).collect()
}


// Array take functions - returns first n elements
pub fn take_int(arr: Vec<i64>, n: i64) -> Vec<i64> {
    arr.into_iter().take(n as usize).collect()
}

pub fn take_float(arr: Vec<f64>, n: i64) -> Vec<f64> {
    arr.into_iter().take(n as usize).collect()
}

pub fn take_string(arr: Vec<String>, n: i64) -> Vec<String> {
    arr.into_iter().take(n as usize).collect()
}

// Find index of first occurrence of element
pub fn find<T: PartialEq + std::fmt::Debug>(arr: &Vec<T>, value: T) -> Result<i64, String> {
    for (idx, item) in arr.iter().enumerate() {
        if item == &value {
            return Ok(idx as i64);
        }
    }
    Err(format!("array_find: value {:?} not found in the array", value))
}

// Find index of last occurrence of element
pub fn find_last<T: PartialEq + std::fmt::Debug>(arr: &Vec<T>, value: T) -> Result<i64, String> {
    for (idx, item) in arr.iter().enumerate().rev() {
        if item == &value {
            return Ok(idx as i64);
        }
    }
    Err(format!("array_find_last: value {:?} not found in the array", value))
}

// Create array with value repeated count times
pub fn repeat<T: Clone>(value: T, count: i64) -> Vec<T> {
    if count <= 0 {
        return Vec::new();
    }
    vec![value; count as usize]
}

// Split array into chunks of specified size
pub fn chunk<T: Clone>(arr: &Vec<T>, size: i64) -> Result<Vec<Vec<T>>, String> {
    if size <= 0 {
        return Err(format!("array_chunk: chunk size must be positive, got {}", size));
    }
    
    let chunk_size = size as usize;
    let mut result = Vec::new();
    let mut i = 0;
    
    while i < arr.len() {
        let end = (i + chunk_size).min(arr.len());
        result.push(arr[i..end].to_vec());
        i = end;
    }
    
    Ok(result)
}

// Remove consecutive duplicates
pub fn deduplicate<T: PartialEq + Clone>(arr: Vec<T>) -> Vec<T> {
    if arr.is_empty() {
        return Vec::new();
    }
    
    let mut result = vec![arr[0].clone()];
    for i in 1..arr.len() {
        if arr[i] != arr[i - 1] {
            result.push(arr[i].clone());
        }
    }
    result
}

// Intersection of two arrays (common elements)
pub fn intersect<T: PartialEq + Clone>(arr1: Vec<T>, arr2: Vec<T>) -> Vec<T> {
    let mut result = Vec::new();
    for item in &arr1 {
        if arr2.contains(item) && !result.contains(item) {
            result.push(item.clone());
        }
    }
    result
}

// Difference of two arrays (elements in arr1 not in arr2)
pub fn difference<T: PartialEq + Clone>(arr1: Vec<T>, arr2: Vec<T>) -> Vec<T> {
    let mut result = Vec::new();
    for item in arr1 {
        if !arr2.contains(&item) {
            result.push(item);
        }
    }
    result
}

// Union of two arrays (all unique elements from both)
pub fn union<T: PartialEq + Clone>(arr1: Vec<T>, arr2: Vec<T>) -> Vec<T> {
    let mut result = arr1.clone();
    for item in arr2 {
        if !result.contains(&item) {
            result.push(item);
        }
    }
    result
}

// Rotate array elements by n positions (positive = right, negative = left)
pub fn rotate<T: Clone>(arr: Vec<T>, n: i64) -> Vec<T> {
    if arr.is_empty() {
        return Vec::new();
    }
    
    let len = arr.len() as i64;
    let shift = ((n % len) + len) % len; // Handle negative rotations
    let split_point = (len - shift) as usize;
    
    let mut result = Vec::new();
    result.extend_from_slice(&arr[split_point..]);
    result.extend_from_slice(&arr[..split_point]);
    result
}

// Shuffle array randomly
pub fn shuffle<T: Clone>(mut arr: Vec<T>) -> Vec<T> {
    use rand::seq::SliceRandom;
    use rand::thread_rng;
    
    let mut rng = thread_rng();
    arr.shuffle(&mut rng);
    arr
}


/// Where a value first appears in the array. An error rather than -1 when it
/// is not there, so the absence has to be handled instead of turning into an
/// index that reads the wrong end of the array.
pub fn index_of<T: PartialEq>(arr: &Vec<T>, item: T) -> Result<i64, String> {
    return match arr.iter().position(|candidate| *candidate == item) {
        Some(position) => Ok(position as i64),
        None => Err("array_index_of: the array does not contain that item".to_string()),
    };
}

/// How many times a value appears in the array. Zero is a fine answer here -
/// unlike a position, a count of none is still a count.
pub fn count_of<T: PartialEq>(arr: &Vec<T>, item: T) -> i64 {
    return arr.iter().filter(|candidate| **candidate == item).count() as i64;
}

/// A new array with the item put in at the given index, moving the rest along.
/// An index equal to the length appends, which is the one position past the end
/// that means something; anything beyond that is an error.
pub fn insert_at<T: Clone>(arr: Vec<T>, index: i64, item: T) -> Result<Vec<T>, String> {
    if index < 0 {
        return Err(format!("array_insert: index cannot be negative, got {}", index));
    }
    let position = index as usize;
    if position > arr.len() {
        return Err(format!("array_insert: index {} is past the end of an array of length {}", index, arr.len()));
    }
    let mut out = arr;
    out.insert(position, item);
    return Ok(out);
}

/// A new array without the element at the given index.
pub fn remove_at<T: Clone>(arr: Vec<T>, index: i64) -> Result<Vec<T>, String> {
    if index < 0 {
        return Err(format!("array_remove_at: index cannot be negative, got {}", index));
    }
    let position = index as usize;
    if position >= arr.len() {
        return Err(format!("array_remove_at: index {} is out of bounds for an array of length {}", index, arr.len()));
    }
    let mut out = arr;
    out.remove(position);
    return Ok(out);
}

/// A new array with the element at the given index replaced. Nail arrays are
/// immutable, so this is how a single element is changed: by building the array
/// that has the new value in it.
pub fn replace_at<T: Clone>(arr: Vec<T>, index: i64, item: T) -> Result<Vec<T>, String> {
    if index < 0 {
        return Err(format!("array_replace_at: index cannot be negative, got {}", index));
    }
    let position = index as usize;
    if position >= arr.len() {
        return Err(format!("array_replace_at: index {} is out of bounds for an array of length {}", index, arr.len()));
    }
    let mut out = arr;
    out[position] = item;
    return Ok(out);
}

/// A new array with two elements exchanged.
pub fn swap<T: Clone>(arr: Vec<T>, first: i64, second: i64) -> Result<Vec<T>, String> {
    if first < 0 || second < 0 {
        return Err(format!("array_swap: indexes cannot be negative, got {} and {}", first, second));
    }
    let (first_position, second_position) = (first as usize, second as usize);
    if first_position >= arr.len() || second_position >= arr.len() {
        return Err(format!("array_swap: indexes {} and {} are not both inside an array of length {}", first, second, arr.len()));
    }
    let mut out = arr;
    out.swap(first_position, second_position);
    return Ok(out);
}

/// Whether every element is the same as the first. An empty array is uniform
/// by the same reasoning that makes an empty sum zero: there is nothing in it
/// that differs.
pub fn all_equal<T: PartialEq>(arr: &Vec<T>) -> bool {
    let mut elements = arr.iter();
    return match elements.next() {
        Some(first) => elements.all(|element| element == first),
        None => true,
    };
}

/// Whether the array has no elements. `array_length(items) == 0` says the same
/// thing, but reads as arithmetic rather than as the question being asked.
pub fn is_empty<T>(arr: &Vec<T>) -> bool {
    return arr.is_empty();
}

/// The array sorted from largest to smallest - `array_sort` reversed, spelled
/// as one step because leaderboards and recent-first lists are the common case.
pub fn sort_descending<T: Clone + PartialOrd>(arr: Vec<T>) -> Vec<T> {
    let mut out = arr;
    out.sort_by(|left, right| right.partial_cmp(left).unwrap_or(std::cmp::Ordering::Equal));
    return out;
}

/// The operations that need to know something about each element that only the
/// program knows - which field to sort on, which value to group by. The `key`
/// argument is a function the program has already defined and named, which is
/// how these work in a language with no closures: `array_sort_by(books,
/// book_year)`, where `book_year` is an ordinary `f` taking a Book.
///
/// The key function must be a plain one - reading a field, doing arithmetic.
/// One that reads a file or makes a request becomes async in the generated Rust,
/// and a sort cannot wait for an answer in the middle of a comparison.
///
/// Sorting by a key rather than with a comparator is deliberate: a comparator
/// can be inconsistent with itself and produce an order that depends on where
/// the sort happened to start, and there is no way to check that it isn't.
pub fn sort_by<T: Clone, K: PartialOrd, F: Fn(T) -> K>(arr: Vec<T>, key: F) -> Vec<T> {
    let mut keyed: Vec<(K, T)> = arr.into_iter().map(|item| (key(item.clone()), item)).collect();
    keyed.sort_by(|left, right| left.0.partial_cmp(&right.0).unwrap_or(std::cmp::Ordering::Equal));
    return keyed.into_iter().map(|(_, item)| item).collect();
}

/// The same, largest first - a leaderboard, or newest-first.
pub fn sort_by_descending<T: Clone, K: PartialOrd, F: Fn(T) -> K>(arr: Vec<T>, key: F) -> Vec<T> {
    let mut keyed: Vec<(K, T)> = arr.into_iter().map(|item| (key(item.clone()), item)).collect();
    keyed.sort_by(|left, right| right.0.partial_cmp(&left.0).unwrap_or(std::cmp::Ordering::Equal));
    return keyed.into_iter().map(|(_, item)| item).collect();
}

/// The element whose key is smallest. An empty array is an error, because there
/// is no element to return and no sensible stand-in for one.
pub fn min_by<T: Clone, K: PartialOrd, F: Fn(T) -> K>(arr: Vec<T>, key: F) -> Result<T, String> {
    let mut best: Option<(K, T)> = None;
    for item in arr.into_iter() {
        let item_key = key(item.clone());
        let replace = match &best {
            Some((best_key, _)) => item_key < *best_key,
            None => true,
        };
        if replace {
            best = Some((item_key, item));
        }
    }
    return match best {
        Some((_, item)) => Ok(item),
        None => Err("array_min_by: the array is empty, so there is no smallest element".to_string()),
    };
}

/// The element whose key is largest.
pub fn max_by<T: Clone, K: PartialOrd, F: Fn(T) -> K>(arr: Vec<T>, key: F) -> Result<T, String> {
    let mut best: Option<(K, T)> = None;
    for item in arr.into_iter() {
        let item_key = key(item.clone());
        let replace = match &best {
            Some((best_key, _)) => item_key > *best_key,
            None => true,
        };
        if replace {
            best = Some((item_key, item));
        }
    }
    return match best {
        Some((_, item)) => Ok(item),
        None => Err("array_max_by: the array is empty, so there is no largest element".to_string()),
    };
}

/// Every element's key added up: the total of a field over an array. An empty
/// array sums to zero, the same as any other empty sum.
pub fn sum_by<T: Clone, K: Copy + Default + std::ops::Add<Output = K>, F: Fn(T) -> K>(arr: Vec<T>, key: F) -> K {
    let mut total = K::default();
    for item in arr.into_iter() {
        total = total + key(item);
    }
    return total;
}

/// The elements bucketed by their key: every element that shares a key ends up
/// in the same array, in the order they appeared.
///
/// This is what SQL's GROUP BY does to rows, and for anything more than
/// bucketing - counting inside groups, sorting groups by their totals - the
/// query engines are the better tool: register the rows and write the SQL.
pub fn group_by<T: Clone, K: std::hash::Hash + Eq + Clone, F: Fn(T) -> K>(arr: Vec<T>, key: F) -> dashmap::DashMap<K, Vec<T>> {
    let buckets: dashmap::DashMap<K, Vec<T>> = dashmap::DashMap::new();
    for item in arr.into_iter() {
        let bucket_key = key(item.clone());
        buckets.entry(bucket_key).or_insert_with(Vec::new).push(item);
    }
    return buckets;
}

/// How many elements share each key - `group_by` when only the sizes matter.
pub fn count_by<T: Clone, K: std::hash::Hash + Eq + Clone, F: Fn(T) -> K>(arr: Vec<T>, key: F) -> dashmap::DashMap<K, i64> {
    let counts: dashmap::DashMap<K, i64> = dashmap::DashMap::new();
    for item in arr.into_iter() {
        let bucket_key = key(item);
        *counts.entry(bucket_key).or_insert(0) += 1;
    }
    return counts;
}

#[cfg(test)]
mod key_function_tests {
    use super::*;

    #[derive(Clone, Debug, PartialEq)]
    struct Book {
        title: String,
        author: String,
        year: i64,
        price: f64,
    }

    fn library() -> Vec<Book> {
        return vec![
            Book { title: "Later".to_string(), author: "Ada".to_string(), year: 2001, price: 10.5 },
            Book { title: "Early".to_string(), author: "Bob".to_string(), year: 1999, price: 5.25 },
            Book { title: "Middle".to_string(), author: "Ada".to_string(), year: 2000, price: 7.0 },
        ];
    }

    fn year(book: Book) -> i64 {
        return book.year;
    }

    fn author(book: Book) -> String {
        return book.author;
    }

    fn price(book: Book) -> f64 {
        return book.price;
    }

    #[test]
    fn sorting_by_a_key_orders_by_that_field() {
        let sorted = sort_by(library(), year);
        assert_eq!(sorted.iter().map(|book| book.year).collect::<Vec<i64>>(), vec![1999, 2000, 2001]);
        let descending = sort_by_descending(library(), year);
        assert_eq!(descending.iter().map(|book| book.year).collect::<Vec<i64>>(), vec![2001, 2000, 1999]);
    }

    #[test]
    fn a_float_key_sorts_too() {
        let sorted = sort_by(library(), price);
        assert_eq!(sorted.first().expect("a book").title, "Early");
    }

    #[test]
    fn a_text_key_sorts_alphabetically() {
        let sorted = sort_by(library(), author);
        assert_eq!(sorted.iter().map(|book| book.author.clone()).collect::<Vec<String>>(), vec!["Ada".to_string(), "Ada".to_string(), "Bob".to_string()]);
    }

    #[test]
    fn the_smallest_and_largest_by_key_are_found() {
        assert_eq!(min_by(library(), year).expect("a book").year, 1999);
        assert_eq!(max_by(library(), year).expect("a book").year, 2001);
    }

    #[test]
    fn an_empty_array_has_no_smallest_element() {
        let empty: Vec<Book> = vec![];
        assert!(min_by(empty.clone(), year).is_err());
        assert!(max_by(empty, year).is_err());
    }

    #[test]
    fn a_field_can_be_totalled_over_the_array() {
        assert_eq!(sum_by(library(), year), 6000);
        assert_eq!(sum_by(library(), price), 22.75);
        let empty: Vec<Book> = vec![];
        assert_eq!(sum_by(empty, year), 0);
    }

    #[test]
    fn grouping_buckets_by_key_and_keeps_order_inside_a_bucket() {
        let buckets = group_by(library(), author);
        assert_eq!(buckets.len(), 2);
        let ada = buckets.get("Ada").expect("Ada wrote some");
        assert_eq!(ada.value().iter().map(|book| book.title.clone()).collect::<Vec<String>>(), vec!["Later".to_string(), "Middle".to_string()]);
        assert_eq!(buckets.get("Bob").expect("Bob wrote one").value().len(), 1);
    }

    #[test]
    fn counting_by_key_gives_the_bucket_sizes() {
        let counts = count_by(library(), author);
        assert_eq!(*counts.get("Ada").expect("Ada wrote some").value(), 2);
        assert_eq!(*counts.get("Bob").expect("Bob wrote one").value(), 1);
    }

    #[test]
    fn an_empty_array_groups_into_nothing() {
        let empty: Vec<Book> = vec![];
        assert!(group_by(empty.clone(), author).is_empty());
        assert!(count_by(empty, author).is_empty());
    }
}


/// The keyed half of the `_by` family: the caller has already worked out one key
/// per element, and these do the rest.
///
/// Splitting it this way is what lets a key function do I/O. The transpiler emits
/// the keys first - a loop it can await in - and then calls these, which never
/// call back into the program at all. So `array_sort_by(files, file_size)` works
/// whether `file_size` reads a field or reads the disk, and there is no rule
/// anybody has to be told about.
///
/// Every one of these takes the elements and their keys as two arrays of the same
/// length, in the same order.
pub fn sort_by_keys<T: Clone, K: PartialOrd>(arr: Vec<T>, keys: Vec<K>) -> Vec<T> {
    let mut keyed: Vec<(K, T)> = keys.into_iter().zip(arr.into_iter()).collect();
    keyed.sort_by(|left, right| left.0.partial_cmp(&right.0).unwrap_or(std::cmp::Ordering::Equal));
    return keyed.into_iter().map(|(_, item)| item).collect();
}

pub fn sort_by_keys_descending<T: Clone, K: PartialOrd>(arr: Vec<T>, keys: Vec<K>) -> Vec<T> {
    let mut keyed: Vec<(K, T)> = keys.into_iter().zip(arr.into_iter()).collect();
    keyed.sort_by(|left, right| right.0.partial_cmp(&left.0).unwrap_or(std::cmp::Ordering::Equal));
    return keyed.into_iter().map(|(_, item)| item).collect();
}

pub fn min_by_keys<T: Clone, K: PartialOrd>(arr: Vec<T>, keys: Vec<K>) -> Result<T, String> {
    let mut best: Option<(K, T)> = None;
    for (key, item) in keys.into_iter().zip(arr.into_iter()) {
        let replace = match &best {
            Some((best_key, _)) => key < *best_key,
            None => true,
        };
        if replace {
            best = Some((key, item));
        }
    }
    return match best {
        Some((_, item)) => Ok(item),
        None => Err("array_min_by: the array is empty, so there is no smallest element".to_string()),
    };
}

pub fn max_by_keys<T: Clone, K: PartialOrd>(arr: Vec<T>, keys: Vec<K>) -> Result<T, String> {
    let mut best: Option<(K, T)> = None;
    for (key, item) in keys.into_iter().zip(arr.into_iter()) {
        let replace = match &best {
            Some((best_key, _)) => key > *best_key,
            None => true,
        };
        if replace {
            best = Some((key, item));
        }
    }
    return match best {
        Some((_, item)) => Ok(item),
        None => Err("array_max_by: the array is empty, so there is no largest element".to_string()),
    };
}

/// The elements play no part in a total, but the pair of arrays is what every
/// one of these takes, so the shape stays the same across the family.
pub fn sum_of_keys<T, K: Copy + Default + std::ops::Add<Output = K>>(_arr: Vec<T>, keys: Vec<K>) -> K {
    let mut total = K::default();
    for key in keys.into_iter() {
        total = total + key;
    }
    return total;
}

pub fn group_by_keys<T: Clone, K: std::hash::Hash + Eq + Clone>(arr: Vec<T>, keys: Vec<K>) -> dashmap::DashMap<K, Vec<T>> {
    let buckets: dashmap::DashMap<K, Vec<T>> = dashmap::DashMap::new();
    for (key, item) in keys.into_iter().zip(arr.into_iter()) {
        buckets.entry(key).or_insert_with(Vec::new).push(item);
    }
    return buckets;
}

pub fn count_by_keys<T, K: std::hash::Hash + Eq + Clone>(_arr: Vec<T>, keys: Vec<K>) -> dashmap::DashMap<K, i64> {
    let counts: dashmap::DashMap<K, i64> = dashmap::DashMap::new();
    for key in keys.into_iter() {
        *counts.entry(key).or_insert(0) += 1;
    }
    return counts;
}

#[cfg(test)]
mod keyed_tests {
    use super::*;

    #[test]
    fn sorting_follows_the_keys_it_was_given() {
        let books = vec!["later".to_string(), "early".to_string(), "middle".to_string()];
        let years = vec![2001, 1999, 2000];
        assert_eq!(sort_by_keys(books.clone(), years.clone()), vec!["early".to_string(), "middle".to_string(), "later".to_string()]);
        assert_eq!(sort_by_keys_descending(books, years), vec!["later".to_string(), "middle".to_string(), "early".to_string()]);
    }

    #[test]
    fn the_extremes_follow_the_keys_too() {
        let books = vec!["later".to_string(), "early".to_string()];
        let years = vec![2001, 1999];
        assert_eq!(min_by_keys(books.clone(), years.clone()).expect("a book"), "early");
        assert_eq!(max_by_keys(books, years).expect("a book"), "later");
        assert!(min_by_keys::<String, i64>(vec![], vec![]).is_err());
    }

    #[test]
    fn totals_and_buckets_follow_the_keys() {
        let books = vec!["one".to_string(), "two".to_string(), "three".to_string()];
        assert_eq!(sum_of_keys(books.clone(), vec![1, 2, 3]), 6);
        let authors = vec!["ada".to_string(), "bob".to_string(), "ada".to_string()];
        let buckets = group_by_keys(books.clone(), authors.clone());
        assert_eq!(buckets.get("ada").expect("ada wrote two").value().len(), 2);
        assert_eq!(*count_by_keys(books, authors).get("ada").expect("ada wrote two").value(), 2);
    }

    /// The keyed and key-function forms must agree, since the same Nail call can
    /// reach either depending on whether its key function does I/O.
    #[test]
    fn the_keyed_form_agrees_with_the_key_function_form() {
        let numbers = vec![3, 1, 2];
        fn negate(value: i64) -> i64 {
            return -value;
        }
        let keys: Vec<i64> = numbers.iter().map(|value| negate(*value)).collect();
        assert_eq!(sort_by_keys(numbers.clone(), keys.clone()), sort_by(numbers.clone(), negate));
        assert_eq!(sum_of_keys(numbers.clone(), keys), sum_by(numbers, negate));
    }
}

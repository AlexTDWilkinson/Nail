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

/// Rotate elements count positions toward the front: the first count elements
/// move to the back. The count wraps, and a negative count rotates the other
/// way, matching array_rotate.
pub fn rotate_left<T: Clone>(arr: Vec<T>, count: i64) -> Vec<T> {
    if arr.is_empty() {
        return Vec::new();
    }
    let len = arr.len() as i64;
    let shift = ((count % len) + len) % len;
    let mut out = arr;
    out.rotate_left(shift as usize);
    return out;
}

/// Rotate elements count positions toward the back: the last count elements
/// move to the front.
pub fn rotate_right<T: Clone>(arr: Vec<T>, count: i64) -> Vec<T> {
    if arr.is_empty() {
        return Vec::new();
    }
    let len = arr.len() as i64;
    let shift = ((count % len) + len) % len;
    let mut out = arr;
    out.rotate_right(shift as usize);
    return out;
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

/// Every step-th element, starting with the first. A step of 1 copies the
/// array. A step below 1 is an error rather than an empty answer, because it
/// almost always means a miscalculation upstream.
pub fn step_by<T: Clone>(arr: Vec<T>, step: i64) -> Result<Vec<T>, String> {
    if step < 1 {
        return Err(format!("array_step_by: step must be at least 1, got {}", step));
    }
    return Ok(arr.into_iter().step_by(step as usize).collect());
}

/// The two arrays woven together - first[0], second[0], first[1], second[1] -
/// and when one runs out, the rest of the other follows.
pub fn interleave<T: Clone>(first: Vec<T>, second: Vec<T>) -> Vec<T> {
    let mut out = Vec::with_capacity(first.len() + second.len());
    let mut first_items = first.into_iter();
    let mut second_items = second.into_iter();
    loop {
        match (first_items.next(), second_items.next()) {
            (Some(from_first), Some(from_second)) => {
                out.push(from_first);
                out.push(from_second);
            }
            (Some(from_first), None) => {
                out.push(from_first);
                out.extend(first_items);
                break;
            }
            (None, Some(from_second)) => {
                out.push(from_second);
                out.extend(second_items);
                break;
            }
            (None, None) => break,
        }
    }
    return out;
}

/// The array grown to the given length by appending the value; an array
/// already that long (or longer) comes back unchanged.
pub fn pad_end<T: Clone>(arr: Vec<T>, length: i64, value: T) -> Vec<T> {
    let target = if length < 0 { 0 } else { length as usize };
    let mut out = arr;
    while out.len() < target {
        out.push(value.clone());
    }
    return out;
}

/// The array grown to the given length by prepending the value - fixed-width
/// alignment, where the existing elements keep the right-hand end.
pub fn pad_start<T: Clone>(arr: Vec<T>, length: i64, value: T) -> Vec<T> {
    let target = if length < 0 { 0 } else { length as usize };
    if arr.len() >= target {
        return arr;
    }
    let mut out = vec![value; target - arr.len()];
    out.extend(arr);
    return out;
}

/// Whether each element is less than or equal to the next, all the way
/// through - the order array_sort would produce. Empty and single-element
/// arrays are sorted; there is nothing in them that is out of place.
pub fn is_sorted<T: PartialOrd>(arr: &Vec<T>) -> bool {
    return arr.windows(2).all(|pair| pair[0] <= pair[1]);
}

/// The strings that actually say something: empty strings dropped, everything
/// else kept in order. The usual cleanup after splitting text on a separator.
pub fn compact_strings(arr: Vec<String>) -> Vec<String> {
    return arr.into_iter().filter(|item| !item.is_empty()).collect();
}

/// The middle element - the lower-index of the two middles when the length is
/// even. An empty array is an error, the same as array_first and array_last.
pub fn middle<T: Clone>(arr: &Vec<T>) -> Result<T, String> {
    if arr.is_empty() {
        return Err("array_middle: cannot get the middle element of an empty array".to_string());
    }
    return Ok(arr[(arr.len() - 1) / 2].clone());
}

/// The last count elements, in their original order - array_take from the
/// other end. Fewer come back if the array is shorter.
pub fn take_last<T: Clone>(arr: Vec<T>, count: i64) -> Vec<T> {
    if count <= 0 {
        return Vec::new();
    }
    let keep = (count as usize).min(arr.len());
    return arr[arr.len() - keep..].to_vec();
}

/// The array without its last count elements - array_skip from the other end.
/// Skipping more than the array holds leaves nothing, not an error.
pub fn skip_last<T: Clone>(arr: Vec<T>, count: i64) -> Vec<T> {
    if count <= 0 {
        return arr;
    }
    let drop = (count as usize).min(arr.len());
    return arr[..arr.len() - drop].to_vec();
}

/// Whether the array begins with the given prefix, element for element. An
/// empty prefix matches anything, the same way every string starts with "".
pub fn starts_with<T: PartialEq>(arr: &Vec<T>, prefix: Vec<T>) -> bool {
    return arr.starts_with(&prefix);
}

/// Whether the array ends with the given suffix, element for element.
pub fn ends_with<T: PartialEq>(arr: &Vec<T>, suffix: Vec<T>) -> bool {
    return arr.ends_with(&suffix);
}

/// Whether no value appears more than once. An empty array is unique - there
/// is nothing in it to repeat.
pub fn is_unique<T: PartialEq>(arr: &Vec<T>) -> bool {
    for (index, item) in arr.iter().enumerate() {
        if arr[index + 1..].contains(item) {
            return false;
        }
    }
    return true;
}

/// How many runs of consecutive equal elements the array has. [1, 1, 2, 1] is
/// three runs; an empty array is zero. Counting the runs without building them
/// - the run arrays themselves would be a nested collection.
pub fn count_runs<T: PartialEq>(arr: &Vec<T>) -> i64 {
    if arr.is_empty() {
        return 0;
    }
    let mut runs = 1i64;
    for index in 1..arr.len() {
        if arr[index] != arr[index - 1] {
            runs += 1;
        }
    }
    return runs;
}

/// How many elements the two arrays share at their start - the point where two
/// paths, or two versions of a list, begin to differ.
pub fn common_prefix_length<T: PartialEq>(first: &Vec<T>, second: &Vec<T>) -> i64 {
    return first.iter().zip(second.iter()).take_while(|(from_first, from_second)| from_first == from_second).count() as i64;
}

/// Where the largest element sits - array_max when the position matters more
/// than the value. Ties go to the first occurrence; an empty array is an error.
pub fn index_of_max<T: PartialOrd>(arr: &Vec<T>) -> Result<i64, String> {
    if arr.is_empty() {
        return Err("array_index_of_max: the array is empty, so there is no largest element".to_string());
    }
    let mut best = 0;
    for index in 1..arr.len() {
        if arr[index] > arr[best] {
            best = index;
        }
    }
    return Ok(best as i64);
}

/// Where the smallest element sits. Ties go to the first occurrence; an empty
/// array is an error.
pub fn index_of_min<T: PartialOrd>(arr: &Vec<T>) -> Result<i64, String> {
    if arr.is_empty() {
        return Err("array_index_of_min: the array is empty, so there is no smallest element".to_string());
    }
    let mut best = 0;
    for index in 1..arr.len() {
        if arr[index] < arr[best] {
            best = index;
        }
    }
    return Ok(best as i64);
}

/// How many arrays the combining functions may produce before they refuse.
///
/// Combinations and permutations grow faster than anybody expects: ten things
/// arranged every way is three and a half million arrays, and asking for it by
/// accident should say so rather than take the machine's memory. A million is
/// far past anything a program does something useful with one at a time.
const LARGEST_PRODUCED_COUNT: usize = 1_000_000;

/// The array sorted the way a person reads names with numbers in them, so
/// `file2` comes before `file10` instead of after it. Plain sorting compares
/// text one character at a time, which puts `10` before `2` because `1` is
/// before `2`, and that is wrong for every list of versions, chapters, or
/// numbered files anybody looks at.
pub fn sort_natural(mut arr: Vec<String>) -> Vec<String> {
    arr.sort_by(|first, second| crate::parser::std_lib::string::natural_ordering(first, second));
    return arr;
}

/// Where in a sorted array the item sits, found by halving the range rather
/// than walking it - the answer in twenty steps for a million elements, where
/// array_index_of takes a million.
///
/// The array must already be sorted, which is the whole bargain: this cannot
/// check that without the walk it exists to avoid. An unsorted array gets an
/// answer that is simply wrong, not an error.
pub fn binary_search<T: PartialOrd>(arr: &Vec<T>, item: T) -> Result<i64, String> {
    let mut low = 0usize;
    let mut high = arr.len();
    while low < high {
        let middle = low + (high - low) / 2;
        if arr[middle] < item {
            low = middle + 1;
        } else if arr[middle] > item {
            high = middle;
        } else {
            return Ok(middle as i64);
        }
    }
    return Err("array_binary_search: the array does not contain that item".to_string());
}

/// The position the item would go into a sorted array at, which is also how
/// many elements come before it. Asking a sorted list of prices how many are
/// under twenty is this function, not a pass over the whole list.
///
/// Equal elements are counted as coming after, so inserting here keeps a run of
/// equal values in the order they arrived.
pub fn insertion_point<T: PartialOrd>(arr: &Vec<T>, item: T) -> i64 {
    let mut low = 0usize;
    let mut high = arr.len();
    while low < high {
        let middle = low + (high - low) / 2;
        if arr[middle] > item {
            high = middle;
        } else {
            low = middle + 1;
        }
    }
    return low as i64;
}

/// The sorted array with one more item in it, still sorted. Keeping a leader
/// board or a queue in order as things arrive, without sorting the whole thing
/// again each time.
pub fn insert_sorted<T: PartialOrd + Clone>(mut arr: Vec<T>, item: T) -> Vec<T> {
    let at = insertion_point(&arr, item.clone()) as usize;
    arr.insert(at, item);
    return arr;
}

/// One page of a list, numbered from one, for a page of results with links to
/// the next. A page past the end is empty rather than an error: a listing that
/// has shrunk since the link was made shows nothing, it does not break.
pub fn page<T: Clone>(arr: &Vec<T>, page: i64, per_page: i64) -> Result<Vec<T>, String> {
    if page < 1 {
        return Err(format!("array_page: pages are numbered from 1, got {}", page));
    }
    if per_page < 1 {
        return Err(format!("array_page: a page has to hold at least one item, got {}", per_page));
    }
    let first = (page as usize - 1).saturating_mul(per_page as usize);
    if first >= arr.len() {
        return Ok(Vec::new());
    }
    let last = first.saturating_add(per_page as usize).min(arr.len());
    return Ok(arr[first..last].to_vec());
}

/// Every run of neighbouring elements of the given size, one step apart -
/// `[1, 2, 3, 4]` in twos is `[1, 2]`, `[2, 3]`, `[3, 4]`. This is what a
/// moving average, a pairwise difference, or a "three in a row" check reads.
///
/// array_chunk is the one that cuts a list into pieces that do not overlap.
/// An array shorter than the window has no runs of that size, which is no
/// windows rather than an error.
pub fn windows<T: Clone>(arr: &Vec<T>, size: i64) -> Result<Vec<Vec<T>>, String> {
    if size <= 0 {
        return Err(format!("array_windows: the window has to hold at least one element, got {}", size));
    }
    let width = size as usize;
    if width > arr.len() {
        return Ok(Vec::new());
    }
    return Ok(arr.windows(width).map(|window| window.to_vec()).collect());
}

/// Every way of choosing that many elements, order not counting: three people
/// out of ten for a rota, two cards out of a hand. Each choice keeps the order
/// the elements came in, and the choices come out in the order their positions
/// do.
///
/// Choosing none is one empty choice, and choosing more than there are is no
/// choices at all.
pub fn combinations<T: Clone>(arr: &Vec<T>, size: i64) -> Result<Vec<Vec<T>>, String> {
    if size < 0 {
        return Err(format!("array_combinations: cannot choose {} elements", size));
    }
    let choose = size as usize;
    if choose > arr.len() {
        return Ok(Vec::new());
    }
    let total = combination_count(arr.len(), choose);
    if total > LARGEST_PRODUCED_COUNT {
        return Err(format!("array_combinations: choosing {} of {} elements is {} arrays, more than the {} this will build", choose, arr.len(), total, LARGEST_PRODUCED_COUNT));
    }

    let mut chosen: Vec<usize> = (0..choose).collect();
    let mut produced = Vec::with_capacity(total);
    loop {
        produced.push(chosen.iter().map(|position| arr[*position].clone()).collect());
        // Step the rightmost position that still has room, then repack the ones
        // after it against it - the standard walk through combinations in the
        // order their positions read.
        let mut position = choose;
        while position > 0 {
            position -= 1;
            if chosen[position] != position + arr.len() - choose {
                chosen[position] += 1;
                for later in position + 1..choose {
                    chosen[later] = chosen[later - 1] + 1;
                }
                break;
            }
            if position == 0 {
                return Ok(produced);
            }
        }
        if choose == 0 {
            return Ok(produced);
        }
    }
}

/// How many ways there are to choose that many of that many, worked out without
/// building any of them, so an impossible request can be refused before it
/// allocates. Saturates rather than overflowing, since anything past the cap is
/// refused anyway.
fn combination_count(total: usize, choose: usize) -> usize {
    if choose > total {
        return 0;
    }
    let choose = choose.min(total - choose);
    let mut count = 1usize;
    for step in 0..choose {
        count = count.saturating_mul(total - step) / (step + 1);
        if count > LARGEST_PRODUCED_COUNT {
            return usize::MAX;
        }
    }
    return count;
}

/// Every ordering of the elements. Ten elements is three and a half million
/// orderings, so this refuses anything that large rather than trying.
pub fn permutations<T: Clone>(arr: &Vec<T>) -> Result<Vec<Vec<T>>, String> {
    let mut total = 1usize;
    for step in 1..=arr.len() {
        total = total.saturating_mul(step);
        if total > LARGEST_PRODUCED_COUNT {
            return Err(format!("array_permutations: {} elements have more orderings than the {} this will build", arr.len(), LARGEST_PRODUCED_COUNT));
        }
    }

    let mut produced = Vec::with_capacity(total);
    let mut chosen: Vec<T> = Vec::with_capacity(arr.len());
    let mut left = arr.clone();
    fill_permutations(&mut chosen, &mut left, &mut produced);
    return Ok(produced);
}

/// Takes each remaining element in turn as the next one chosen, which produces
/// the orderings in the order their positions read.
fn fill_permutations<T: Clone>(chosen: &mut Vec<T>, left: &mut Vec<T>, produced: &mut Vec<Vec<T>>) {
    if left.is_empty() {
        produced.push(chosen.clone());
        return;
    }
    for position in 0..left.len() {
        let item = left.remove(position);
        chosen.push(item.clone());
        fill_permutations(chosen, left, produced);
        chosen.pop();
        left.insert(position, item);
    }
}

/// Every pairing of one element from each array, as two-element arrays: sizes
/// against colours, days against rooms. The first array moves slowest, so the
/// pairs come out grouped by their first element.
pub fn cartesian_product<T: Clone>(first: &Vec<T>, second: &Vec<T>) -> Result<Vec<Vec<T>>, String> {
    let total = first.len().saturating_mul(second.len());
    if total > LARGEST_PRODUCED_COUNT {
        return Err(format!("array_cartesian_product: {} by {} is {} pairs, more than the {} this will build", first.len(), second.len(), total, LARGEST_PRODUCED_COUNT));
    }
    let mut produced = Vec::with_capacity(total);
    for from_first in first.iter() {
        for from_second in second.iter() {
            produced.push(vec![from_first.clone(), from_second.clone()]);
        }
    }
    return Ok(produced);
}

#[cfg(test)]
mod pure_function_tests {
    use super::*;

    fn words(items: &[&str]) -> Vec<String> {
        return items.iter().map(|item| item.to_string()).collect();
    }

    #[test]
    fn stepping_keeps_the_first_and_every_step_th_after() {
        assert_eq!(step_by(vec![1, 2, 3, 4, 5], 2).expect("a valid step"), vec![1, 3, 5]);
        assert_eq!(step_by(words(&["a", "b", "c", "d"]), 3).expect("a valid step"), words(&["a", "d"]));
        assert_eq!(step_by(vec![1, 2, 3], 1).expect("a valid step"), vec![1, 2, 3]);
        assert_eq!(step_by(vec![1, 2, 3], 10).expect("a valid step"), vec![1]);
        assert_eq!(step_by(Vec::<i64>::new(), 2).expect("a valid step"), Vec::<i64>::new());
    }

    #[test]
    fn a_step_below_one_is_an_error() {
        assert!(step_by(vec![1, 2, 3], 0).is_err());
        assert!(step_by(vec![1, 2, 3], -2).is_err());
    }

    #[test]
    fn interleaving_alternates_and_appends_the_leftover_tail() {
        assert_eq!(interleave(vec![1, 3, 5], vec![2, 4, 6]), vec![1, 2, 3, 4, 5, 6]);
        assert_eq!(interleave(vec![1, 3, 5, 7, 9], vec![2, 4]), vec![1, 2, 3, 4, 5, 7, 9]);
        assert_eq!(interleave(vec![1], vec![2, 4, 6]), vec![1, 2, 4, 6]);
        assert_eq!(interleave(words(&["a", "b"]), words(&["x"])), words(&["a", "x", "b"]));
        assert_eq!(interleave(Vec::<i64>::new(), vec![1, 2]), vec![1, 2]);
        assert_eq!(interleave(Vec::<i64>::new(), Vec::<i64>::new()), Vec::<i64>::new());
    }

    #[test]
    fn padding_grows_short_arrays_and_leaves_long_ones_alone() {
        assert_eq!(pad_end(vec![1, 2], 4, 0), vec![1, 2, 0, 0]);
        assert_eq!(pad_end(vec![1, 2, 3], 3, 0), vec![1, 2, 3]);
        assert_eq!(pad_end(vec![1, 2, 3], 2, 0), vec![1, 2, 3]);
        assert_eq!(pad_end(Vec::<String>::new(), 2, "x".to_string()), words(&["x", "x"]));
        assert_eq!(pad_end(vec![1], -3, 0), vec![1]);
        assert_eq!(pad_start(vec![1, 2], 4, 0), vec![0, 0, 1, 2]);
        assert_eq!(pad_start(vec![1, 2, 3], 2, 0), vec![1, 2, 3]);
        assert_eq!(pad_start(words(&["end"]), 3, "-".to_string()), words(&["-", "-", "end"]));
    }

    #[test]
    fn sortedness_allows_equal_neighbours_and_holds_for_the_trivial_arrays() {
        assert!(is_sorted(&vec![1, 2, 2, 3]));
        assert!(!is_sorted(&vec![3, 1, 2]));
        assert!(is_sorted(&Vec::<i64>::new()));
        assert!(is_sorted(&vec![42]));
        assert!(is_sorted(&words(&["apple", "banana", "banana"])));
        assert!(!is_sorted(&words(&["banana", "apple"])));
        assert!(is_sorted(&vec![1.0, 1.5, 2.0]));
    }

    #[test]
    fn compacting_drops_only_the_empty_strings() {
        assert_eq!(compact_strings(words(&["a", "", "b", ""])), words(&["a", "b"]));
        assert_eq!(compact_strings(words(&["", ""])), Vec::<String>::new());
        assert_eq!(compact_strings(Vec::new()), Vec::<String>::new());
        assert_eq!(compact_strings(words(&[" "])), words(&[" "]));
    }

    #[test]
    fn the_middle_element_is_the_lower_of_two_for_even_lengths() {
        assert_eq!(middle(&vec![1, 2, 3]).expect("a middle"), 2);
        assert_eq!(middle(&vec![1, 2, 3, 4]).expect("a middle"), 2);
        assert_eq!(middle(&vec![7]).expect("a middle"), 7);
        assert_eq!(middle(&words(&["a", "b", "c"])).expect("a middle"), "b");
        assert!(middle(&Vec::<i64>::new()).is_err());
    }

    #[test]
    fn taking_and_skipping_from_the_end_mirror_take_and_skip() {
        assert_eq!(take_last(vec![1, 2, 3, 4], 2), vec![3, 4]);
        assert_eq!(take_last(vec![1, 2], 5), vec![1, 2]);
        assert_eq!(take_last(vec![1, 2], 0), Vec::<i64>::new());
        assert_eq!(take_last(vec![1, 2], -1), Vec::<i64>::new());
        assert_eq!(take_last(words(&["a", "b", "c"]), 1), words(&["c"]));
        assert_eq!(skip_last(vec![1, 2, 3, 4], 2), vec![1, 2]);
        assert_eq!(skip_last(vec![1, 2], 5), Vec::<i64>::new());
        assert_eq!(skip_last(vec![1, 2], 0), vec![1, 2]);
        assert_eq!(skip_last(Vec::<i64>::new(), 3), Vec::<i64>::new());
    }

    #[test]
    fn prefix_and_suffix_checks_match_element_for_element() {
        assert!(starts_with(&vec![1, 2, 3], vec![1, 2]));
        assert!(!starts_with(&vec![1, 2, 3], vec![2]));
        assert!(starts_with(&vec![1, 2], Vec::new()));
        assert!(!starts_with(&vec![1], vec![1, 2]));
        assert!(starts_with(&words(&["a", "b"]), words(&["a"])));
        assert!(ends_with(&vec![1, 2, 3], vec![2, 3]));
        assert!(!ends_with(&vec![1, 2, 3], vec![1]));
        assert!(ends_with(&vec![1, 2], Vec::new()));
        assert!(ends_with(&words(&["a", "b"]), words(&["b"])));
        assert!(!ends_with(&Vec::<i64>::new(), vec![1]));
    }

    #[test]
    fn uniqueness_means_no_value_repeats_anywhere() {
        assert!(is_unique(&vec![1, 2, 3]));
        assert!(!is_unique(&vec![1, 2, 1]));
        assert!(is_unique(&Vec::<i64>::new()));
        assert!(is_unique(&words(&["a", "b"])));
        assert!(!is_unique(&words(&["a", "a"])));
    }

    #[test]
    fn runs_count_stretches_of_equal_neighbours() {
        assert_eq!(count_runs(&vec![1, 1, 2, 1]), 3);
        assert_eq!(count_runs(&vec![5, 5, 5]), 1);
        assert_eq!(count_runs(&vec![1, 2, 3]), 3);
        assert_eq!(count_runs(&Vec::<i64>::new()), 0);
        assert_eq!(count_runs(&words(&["a", "a", "b"])), 2);
    }

    #[test]
    fn the_common_prefix_stops_at_the_first_difference() {
        assert_eq!(common_prefix_length(&vec![1, 2, 3], &vec![1, 2, 9]), 2);
        assert_eq!(common_prefix_length(&vec![1, 2], &vec![3, 4]), 0);
        assert_eq!(common_prefix_length(&vec![1, 2], &vec![1, 2]), 2);
        assert_eq!(common_prefix_length(&vec![1, 2, 3], &vec![1, 2]), 2);
        assert_eq!(common_prefix_length(&Vec::<i64>::new(), &vec![1]), 0);
        assert_eq!(common_prefix_length(&words(&["usr", "bin"]), &words(&["usr", "lib"])), 1);
    }

    #[test]
    fn extreme_positions_go_to_the_first_of_a_tie() {
        assert_eq!(index_of_max(&vec![1, 9, 3]).expect("a position"), 1);
        assert_eq!(index_of_max(&vec![9, 2, 9]).expect("a position"), 0);
        assert_eq!(index_of_min(&vec![4, 1, 7]).expect("a position"), 1);
        assert_eq!(index_of_min(&vec![2, 5, 2]).expect("a position"), 0);
        assert_eq!(index_of_max(&words(&["ant", "zebra", "cat"])).expect("a position"), 1);
        assert_eq!(index_of_min(&vec![2.5, 1.5, 3.5]).expect("a position"), 1);
        assert!(index_of_max(&Vec::<i64>::new()).is_err());
        assert!(index_of_min(&Vec::<i64>::new()).is_err());
    }

    #[test]
    fn rotations_wrap_and_agree_with_their_directions() {
        assert_eq!(rotate_left(vec![1, 2, 3], 1), vec![2, 3, 1]);
        assert_eq!(rotate_right(vec![1, 2, 3], 1), vec![3, 1, 2]);
        assert_eq!(rotate_left(vec![1, 2, 3], 4), vec![2, 3, 1]);
        assert_eq!(rotate_right(vec![1, 2, 3], 4), vec![3, 1, 2]);
        assert_eq!(rotate_left(vec![1, 2, 3], -1), rotate_right(vec![1, 2, 3], 1));
        assert_eq!(rotate_left(Vec::<i64>::new(), 2), Vec::<i64>::new());
        assert_eq!(rotate_right(words(&["a", "b"]), 1), words(&["b", "a"]));
    }
}

#[cfg(test)]
mod ordering_paging_and_combining_tests {
    use super::*;

    fn words(items: &[&str]) -> Vec<String> {
        return items.iter().map(|item| item.to_string()).collect();
    }

    #[test]
    fn natural_order_reads_the_numbers_in_names() {
        assert_eq!(sort_natural(words(&["file10", "file2", "file1"])), words(&["file1", "file2", "file10"]));
        assert_eq!(sort_natural(words(&["v1.10.0", "v1.9.0", "v1.2.0"])), words(&["v1.2.0", "v1.9.0", "v1.10.0"]));
        assert_eq!(sort_natural(words(&["b", "A", "a"])), words(&["A", "a", "b"]), "case-insensitive, then the text itself breaks the tie");
        assert_eq!(sort_natural(Vec::new()), Vec::<String>::new());
    }

    #[test]
    fn halving_the_range_finds_what_walking_it_would() {
        let sorted = vec![1, 3, 5, 7, 9, 11];
        for (position, value) in sorted.iter().enumerate() {
            assert_eq!(binary_search(&sorted, *value).expect("a present value"), position as i64);
        }
        assert!(binary_search(&sorted, 4).is_err());
        assert!(binary_search(&Vec::<i64>::new(), 1).is_err());
        assert_eq!(binary_search(&words(&["ant", "bee", "cow"]), "bee".to_string()).expect("a present word"), 1);
        assert_eq!(binary_search(&vec![0.5, 1.5, 2.5], 2.5).expect("a present number"), 2);
    }

    #[test]
    fn the_insertion_point_counts_what_comes_before() {
        let sorted = vec![10, 20, 20, 30];
        assert_eq!(insertion_point(&sorted, 5), 0);
        assert_eq!(insertion_point(&sorted, 15), 1);
        assert_eq!(insertion_point(&sorted, 20), 3, "equal values are passed, so an insert keeps arrival order");
        assert_eq!(insertion_point(&sorted, 99), 4);
        assert_eq!(insertion_point(&Vec::<i64>::new(), 1), 0);
    }

    #[test]
    fn inserting_keeps_the_array_sorted() {
        assert_eq!(insert_sorted(vec![1, 3, 5], 4), vec![1, 3, 4, 5]);
        assert_eq!(insert_sorted(vec![1, 3, 5], 0), vec![0, 1, 3, 5]);
        assert_eq!(insert_sorted(vec![1, 3, 5], 9), vec![1, 3, 5, 9]);
        assert_eq!(insert_sorted(Vec::<i64>::new(), 2), vec![2]);
        assert_eq!(insert_sorted(words(&["ant", "cow"]), "bee".to_string()), words(&["ant", "bee", "cow"]));
    }

    #[test]
    fn pages_are_numbered_from_one_and_run_out_quietly() {
        let items = vec![1, 2, 3, 4, 5];
        assert_eq!(page(&items, 1, 2).expect("a real page"), vec![1, 2]);
        assert_eq!(page(&items, 3, 2).expect("a real page"), vec![5], "the last page is short");
        assert_eq!(page(&items, 4, 2).expect("a real page"), Vec::<i64>::new(), "past the end is empty, not an error");
        assert_eq!(page(&items, 1, 99).expect("a real page"), items);
        assert!(page(&items, 0, 2).unwrap_err().contains("numbered from 1"));
        assert!(page(&items, 1, 0).unwrap_err().contains("at least one item"));
    }

    #[test]
    fn windows_overlap_where_chunks_do_not() {
        assert_eq!(windows(&vec![1, 2, 3, 4], 2).expect("a real size"), vec![vec![1, 2], vec![2, 3], vec![3, 4]]);
        assert_eq!(windows(&vec![1, 2, 3], 3).expect("a real size"), vec![vec![1, 2, 3]]);
        assert_eq!(windows(&vec![1, 2], 3).expect("a real size"), Vec::<Vec<i64>>::new(), "no run that long exists");
        assert!(windows(&vec![1, 2], 0).unwrap_err().contains("at least one element"));
    }

    #[test]
    fn combinations_choose_without_order_and_permutations_with_it() {
        assert_eq!(combinations(&vec![1, 2, 3], 2).expect("a real size"), vec![vec![1, 2], vec![1, 3], vec![2, 3]]);
        assert_eq!(combinations(&vec![1, 2, 3], 3).expect("a real size"), vec![vec![1, 2, 3]]);
        assert_eq!(combinations(&vec![1, 2], 0).expect("a real size"), vec![Vec::<i64>::new()], "choosing none is one empty choice");
        assert_eq!(combinations(&vec![1, 2], 3).expect("a real size"), Vec::<Vec<i64>>::new());
        // 52 cards chosen 5 at a time is nearly three million hands.
        let too_many: Vec<i64> = (0..52).collect();
        assert!(combinations(&too_many, 5).unwrap_err().contains("more than the"));

        assert_eq!(permutations(&vec![1, 2, 3]).expect("few enough"), vec![vec![1, 2, 3], vec![1, 3, 2], vec![2, 1, 3], vec![2, 3, 1], vec![3, 1, 2], vec![3, 2, 1]]);
        assert_eq!(permutations(&Vec::<i64>::new()).expect("few enough"), vec![Vec::<i64>::new()]);
        let ten: Vec<i64> = (0..10).collect();
        assert!(permutations(&ten).unwrap_err().contains("more orderings than"));
    }

    #[test]
    fn every_pairing_comes_out_grouped_by_the_first_array() {
        assert_eq!(cartesian_product(&vec![1, 2], &vec![8, 9]).expect("a small product"), vec![vec![1, 8], vec![1, 9], vec![2, 8], vec![2, 9]]);
        assert_eq!(cartesian_product(&vec![1], &Vec::<i64>::new()).expect("a small product"), Vec::<Vec<i64>>::new());
        assert_eq!(cartesian_product(&words(&["s", "m"]), &words(&["red"])).expect("a small product"), vec![words(&["s", "red"]), words(&["m", "red"])]);
    }
}

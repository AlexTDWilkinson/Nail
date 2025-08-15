use rayon::prelude::*;
use std::cmp::Ordering;

pub async fn len<T>(arr: Vec<T>) -> i64 {
    arr.len() as i64
}

pub async fn push<T: Clone>(mut arr: Vec<T>, item: T) -> Vec<T> {
    arr.push(item);
    arr
}

pub async fn pop<T: Clone>(mut arr: Vec<T>) -> (Vec<T>, Option<T>) {
    let item = arr.pop();
    (arr, item)
}

pub async fn contains<T: PartialEq + Sync + Send>(arr: Vec<T>, item: T) -> bool 
where T: Sync + Send
{
    use rayon::prelude::*;
    use rayon::iter::IntoParallelIterator;
    arr.par_iter().any(|x| x == &item)
}

pub async fn join<T: std::fmt::Display + Send + Sync>(arr: Vec<T>, separator: String) -> String {
    use rayon::prelude::*;
    use rayon::iter::IntoParallelIterator;
    
    arr.par_iter()
        .map(|item| format!("{}", item))
        .collect::<Vec<String>>()
        .join(&separator)
}

pub async fn sort<T: Ord + Clone + Send>(mut arr: Vec<T>) -> Vec<T> {
    use rayon::prelude::*;
    use rayon::iter::IntoParallelIterator;
    arr.par_sort();
    arr
}

pub async fn reverse<T: Clone>(mut arr: Vec<T>) -> Vec<T> {
    arr.reverse();
    arr
}

// Concatenate two arrays
pub async fn concat<T: Clone>(mut first: Vec<T>, second: Vec<T>) -> Vec<T> {
    first.extend(second);
    first
}

// Safe array indexing - returns Result
pub async fn get<T: Clone>(arr: Vec<T>, index: i64) -> Result<T, String> {
    if index < 0 {
        return Err(format!("Array index cannot be negative: {}", index));
    }

    let idx = index as usize;
    if idx >= arr.len() {
        return Err(format!("Array index out of bounds: {} (array length: {})", index, arr.len()));
    }

    Ok(arr[idx].clone())
}

// Get first element
pub async fn first<T: Clone>(arr: Vec<T>) -> Result<T, String> {
    arr.first().cloned().ok_or_else(|| "Cannot get first element of empty array".to_string())
}

// Get last element
pub async fn last<T: Clone>(arr: Vec<T>) -> Result<T, String> {
    arr.last().cloned().ok_or_else(|| "Cannot get last element of empty array".to_string())
}

// Safe array slicing
pub async fn slice<T: Clone>(arr: Vec<T>, start: i64, end: i64) -> Result<Vec<T>, String> {
    if start < 0 || end < 0 {
        return Err("Slice indices cannot be negative".to_string());
    }

    let start_idx = start as usize;
    let end_idx = end as usize;

    if start_idx > arr.len() || end_idx > arr.len() {
        return Err(format!("Slice indices out of bounds: {}..{} (array length: {})", start, end, arr.len()));
    }

    if start_idx > end_idx {
        return Err(format!("Invalid slice range: {}..{}", start, end));
    }

    Ok(arr[start_idx..end_idx].to_vec())
}

// Take first n elements
pub async fn take<T: Clone>(arr: Vec<T>, n: i64) -> Vec<T> {
    if n <= 0 {
        return Vec::new();
    }

    let count = (n as usize).min(arr.len());
    arr[..count].to_vec()
}

// Skip first n elements
pub async fn skip<T: Clone>(arr: Vec<T>, n: i64) -> Vec<T> {
    if n <= 0 {
        return arr.clone();
    }

    let count = (n as usize).min(arr.len());
    arr[count..].to_vec()
}

// Generic array unique - returns array with unique elements only
pub async fn unique<T>(arr: Vec<T>) -> Vec<T>
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

// Generic array flatten - flattens nested arrays by one level
pub async fn flatten<T>(arr: Vec<Vec<T>>) -> Vec<T> {
    arr.into_iter().flatten().collect()
}

// Generic array zip - combines two arrays into array of tuples
pub async fn zip<T, U>(arr1: Vec<T>, arr2: Vec<U>) -> Vec<(T, U)> {
    arr1.into_iter().zip(arr2).collect()
}


// Generic min/max functions for arrays
pub async fn min<T>(arr: Vec<T>) -> Result<T, String>
where
    T: Ord,
{
    arr.into_iter().min().ok_or_else(|| "Array is empty".to_string())
}

pub async fn max<T>(arr: Vec<T>) -> Result<T, String>
where
    T: Ord,
{
    arr.into_iter().max().ok_or_else(|| "Array is empty".to_string())
}


// Range function - generates a range of integers (exclusive end, like Python)
pub async fn array_range(start: i64, end: i64) -> Vec<i64> {
    (start..end).collect()
}

// Range inclusive 
pub async fn array_range_inclusive(start: i64, end: i64) -> Vec<i64> {
    (start..=end).collect()
}


// Array take functions - returns first n elements
pub async fn take_int(arr: Vec<i64>, n: i64) -> Vec<i64> {
    arr.into_iter().take(n as usize).collect()
}

pub async fn take_float(arr: Vec<f64>, n: i64) -> Vec<f64> {
    arr.into_iter().take(n as usize).collect()
}

pub async fn take_string(arr: Vec<String>, n: i64) -> Vec<String> {
    arr.into_iter().take(n as usize).collect()
}

// Find index of first occurrence of element
pub async fn find<T: PartialEq>(arr: Vec<T>, value: T) -> Result<i64, String> {
    for (idx, item) in arr.iter().enumerate() {
        if item == &value {
            return Ok(idx as i64);
        }
    }
    Err(format!("Element not found in array"))
}

// Find index of last occurrence of element
pub async fn find_last<T: PartialEq>(arr: Vec<T>, value: T) -> Result<i64, String> {
    for (idx, item) in arr.iter().enumerate().rev() {
        if item == &value {
            return Ok(idx as i64);
        }
    }
    Err(format!("Element not found in array"))
}

// Create array with value repeated count times
pub async fn repeat<T: Clone>(value: T, count: i64) -> Vec<T> {
    if count <= 0 {
        return Vec::new();
    }
    vec![value; count as usize]
}

// NOTE: Removed sum/product/average functions - use reduce instead:
// sum:i = reduce acc, num in nums from 0 { y acc + num; };
// product:i = reduce acc, num in nums from 1 { y acc * num; };

// Split array into chunks of specified size
pub async fn chunk<T: Clone>(arr: Vec<T>, size: i64) -> Result<Vec<Vec<T>>, String> {
    if size <= 0 {
        return Err("Chunk size must be positive".to_string());
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

// Flatten nested arrays recursively (deep flatten)
pub async fn flatten_deep<T: Clone>(arr: Vec<Vec<T>>) -> Vec<T> {
    arr.into_iter().flatten().collect()
}

// Partition array into two based on predicate (returns [matching, non-matching])
// Note: In Nail, use filter for this - keeping for completeness
pub async fn partition<T: Clone + Send + Sync>(arr: Vec<T>, predicate: impl Fn(&T) -> bool + Send + Sync) -> (Vec<T>, Vec<T>) {
    let mut matching = Vec::new();
    let mut non_matching = Vec::new();
    
    for item in arr {
        if predicate(&item) {
            matching.push(item);
        } else {
            non_matching.push(item);
        }
    }
    
    (matching, non_matching)
}

// Remove consecutive duplicates
pub async fn deduplicate<T: PartialEq + Clone>(arr: Vec<T>) -> Vec<T> {
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
pub async fn intersect<T: PartialEq + Clone>(arr1: Vec<T>, arr2: Vec<T>) -> Vec<T> {
    let mut result = Vec::new();
    for item in &arr1 {
        if arr2.contains(item) && !result.contains(item) {
            result.push(item.clone());
        }
    }
    result
}

// Difference of two arrays (elements in arr1 not in arr2)
pub async fn difference<T: PartialEq + Clone>(arr1: Vec<T>, arr2: Vec<T>) -> Vec<T> {
    let mut result = Vec::new();
    for item in arr1 {
        if !arr2.contains(&item) {
            result.push(item);
        }
    }
    result
}

// Union of two arrays (all unique elements from both)
pub async fn union<T: PartialEq + Clone>(arr1: Vec<T>, arr2: Vec<T>) -> Vec<T> {
    let mut result = arr1.clone();
    for item in arr2 {
        if !result.contains(&item) {
            result.push(item);
        }
    }
    result
}

// Rotate array elements by n positions (positive = right, negative = left)
pub async fn rotate<T: Clone>(arr: Vec<T>, n: i64) -> Vec<T> {
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
pub async fn shuffle<T: Clone>(mut arr: Vec<T>) -> Vec<T> {
    use rand::seq::SliceRandom;
    use rand::thread_rng;
    
    let mut rng = thread_rng();
    arr.shuffle(&mut rng);
    arr
}


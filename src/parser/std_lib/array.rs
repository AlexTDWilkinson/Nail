use rayon::prelude::*;
use std::cmp::Ordering;

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
pub fn min<T: Clone>(arr: &Vec<T>) -> Result<T, String>
where
    T: PartialOrd,
{
    let mut iter = arr.iter();
    let mut best = iter.next().ok_or_else(|| "array_min: cannot get the minimum of an empty array".to_string())?;
    for item in iter {
        if item < best {
            best = item;
        }
    }
    Ok(best.clone())
}

pub fn max<T: Clone>(arr: &Vec<T>) -> Result<T, String>
where
    T: PartialOrd,
{
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
pub fn sum<T: Clone>(arr: &Vec<T>) -> T
where
    T: std::iter::Sum<T>,
{
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


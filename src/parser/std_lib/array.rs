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


// Range function - generates a range of integers
pub async fn range(start: i64, end: i64) -> Vec<i64> {
    (start..=end).collect()
}

// Range exclusive (like Python's range)
pub async fn range_exclusive(start: i64, end: i64) -> Vec<i64> {
    (start..end).collect()
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


// Functional array operations for Nail

// Map function - transforms each element in an array
pub async fn map_int<F>(arr: Vec<i64>, f: F) -> Vec<i64> 
where
    F: Fn(i64) -> i64 + Send + Sync
{
    arr.into_iter().map(|x| f(x)).collect()
}

pub async fn map_float<F>(arr: Vec<f64>, f: F) -> Vec<f64>
where
    F: Fn(f64) -> f64 + Send + Sync
{
    arr.into_iter().map(|x| f(x)).collect()
}

pub async fn map_string<F>(arr: Vec<String>, f: F) -> Vec<String>
where 
    F: Fn(String) -> String + Send + Sync
{
    arr.into_iter().map(|x| f(x)).collect()
}

// Filter function - keeps only elements that match predicate
pub async fn filter_int<F>(arr: Vec<i64>, f: F) -> Vec<i64>
where
    F: Fn(i64) -> bool + Send + Sync
{
    arr.into_iter().filter(|&x| f(x)).collect()
}

pub async fn filter_float<F>(arr: Vec<f64>, f: F) -> Vec<f64>
where
    F: Fn(f64) -> bool + Send + Sync
{
    arr.into_iter().filter(|&x| f(x)).collect()
}

pub async fn filter_string<F>(arr: Vec<String>, f: F) -> Vec<String>
where
    F: Fn(&String) -> bool + Send + Sync
{
    arr.into_iter().filter(|x| f(&x)).collect()
}

// Reduce function - combines all elements into a single value
pub async fn reduce_int<F>(arr: Vec<i64>, initial: i64, f: F) -> i64
where
    F: Fn(i64, i64) -> i64 + Send + Sync
{
    arr.into_iter().fold(initial, |acc, x| f(acc, x))
}

pub async fn reduce_float<F>(arr: Vec<f64>, initial: f64, f: F) -> f64
where
    F: Fn(f64, f64) -> f64 + Send + Sync
{
    arr.into_iter().fold(initial, |acc, x| f(acc, x))
}

pub async fn reduce_string<F>(arr: Vec<String>, initial: String, f: F) -> String
where
    F: Fn(String, String) -> String + Send + Sync
{
    arr.into_iter().fold(initial, |acc, x| f(acc, x))
}

// Each function - performs side effects for each element
pub async fn each_int<F>(arr: Vec<i64>, f: F)
where
    F: Fn(i64) + Send + Sync
{
    arr.into_iter().for_each(|x| f(x))
}

pub async fn each_float<F>(arr: Vec<f64>, f: F)
where
    F: Fn(f64) + Send + Sync
{
    arr.into_iter().for_each(|x| f(x))
}

pub async fn each_string<F>(arr: Vec<String>, f: F)
where
    F: Fn(String) + Send + Sync
{
    arr.into_iter().for_each(|x| f(x))
}

// Range function - generates a range of integers
pub fn range(start: i64, end: i64) -> Vec<i64> {
    (start..=end).collect()
}

// Range exclusive (like Python's range)
pub fn range_exclusive(start: i64, end: i64) -> Vec<i64> {
    (start..end).collect()
}
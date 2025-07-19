pub fn len<T>(arr: Vec<T>) -> i64 {
    arr.len() as i64
}

pub fn push<T: Clone>(mut arr: Vec<T>, item: T) -> Vec<T> {
    arr.push(item);
    arr
}

pub fn pop<T: Clone>(mut arr: Vec<T>) -> (Vec<T>, Option<T>) {
    let item = arr.pop();
    (arr, item)
}

pub fn contains<T: PartialEq>(arr: Vec<T>, item: T) -> bool {
    arr.contains(&item)
}

pub fn join(arr: Vec<String>, separator: String) -> String {
    arr.join(&separator)
}

pub fn sort<T: Ord + Clone>(mut arr: Vec<T>) -> Vec<T> {
    arr.sort();
    arr
}

pub fn reverse<T: Clone>(mut arr: Vec<T>) -> Vec<T> {
    arr.reverse();
    arr
}

// Safe array indexing - returns Result
pub fn get<T: Clone>(arr: &Vec<T>, index: i64) -> Result<T, String> {
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
pub fn first<T: Clone>(arr: &Vec<T>) -> Result<T, String> {
    arr.first()
        .cloned()
        .ok_or_else(|| "Cannot get first element of empty array".to_string())
}

// Get last element
pub fn last<T: Clone>(arr: &Vec<T>) -> Result<T, String> {
    arr.last()
        .cloned()
        .ok_or_else(|| "Cannot get last element of empty array".to_string())
}

// Safe array slicing
pub fn slice<T: Clone>(arr: &Vec<T>, start: i64, end: i64) -> Result<Vec<T>, String> {
    if start < 0 || end < 0 {
        return Err("Slice indices cannot be negative".to_string());
    }
    
    let start_idx = start as usize;
    let end_idx = end as usize;
    
    if start_idx > arr.len() || end_idx > arr.len() {
        return Err(format!("Slice indices out of bounds: {}..{} (array length: {})", 
                          start, end, arr.len()));
    }
    
    if start_idx > end_idx {
        return Err(format!("Invalid slice range: {}..{}", start, end));
    }
    
    Ok(arr[start_idx..end_idx].to_vec())
}

// Take first n elements
pub fn take<T: Clone>(arr: &Vec<T>, n: i64) -> Vec<T> {
    if n <= 0 {
        return Vec::new();
    }
    
    let count = (n as usize).min(arr.len());
    arr[..count].to_vec()
}

// Skip first n elements
pub fn skip<T: Clone>(arr: &Vec<T>, n: i64) -> Vec<T> {
    if n <= 0 {
        return arr.clone();
    }
    
    let count = (n as usize).min(arr.len());
    arr[count..].to_vec()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_array_get() {
        let arr = vec![10, 20, 30, 40, 50];
        
        // Test valid indices
        assert_eq!(get(&arr, 0).unwrap(), 10);
        assert_eq!(get(&arr, 2).unwrap(), 30);
        assert_eq!(get(&arr, 4).unwrap(), 50);
        
        // Test out of bounds
        assert!(get(&arr, 5).is_err());
        assert!(get(&arr, -1).is_err());
    }

    #[test]
    fn test_array_first_last() {
        let arr = vec![1, 2, 3, 4, 5];
        assert_eq!(first(&arr).unwrap(), 1);
        assert_eq!(last(&arr).unwrap(), 5);
        
        let empty: Vec<i32> = vec![];
        assert!(first(&empty).is_err());
        assert!(last(&empty).is_err());
    }

    #[test]
    fn test_array_slice() {
        let arr = vec![1, 2, 3, 4, 5];
        
        assert_eq!(slice(&arr, 1, 4).unwrap(), vec![2, 3, 4]);
        assert_eq!(slice(&arr, 0, 5).unwrap(), vec![1, 2, 3, 4, 5]);
        assert_eq!(slice(&arr, 2, 2).unwrap(), Vec::<i32>::new());
        
        // Test errors
        assert!(slice(&arr, -1, 3).is_err());
        assert!(slice(&arr, 0, 10).is_err());
        assert!(slice(&arr, 3, 2).is_err());
    }

    #[test]
    fn test_array_take_skip() {
        let arr = vec![1, 2, 3, 4, 5];
        
        assert_eq!(take(&arr, 3), vec![1, 2, 3]);
        assert_eq!(take(&arr, 0), Vec::<i32>::new());
        assert_eq!(take(&arr, 10), vec![1, 2, 3, 4, 5]);
        
        assert_eq!(skip(&arr, 2), vec![3, 4, 5]);
        assert_eq!(skip(&arr, 0), vec![1, 2, 3, 4, 5]);
        assert_eq!(skip(&arr, 10), Vec::<i32>::new());
    }
}
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
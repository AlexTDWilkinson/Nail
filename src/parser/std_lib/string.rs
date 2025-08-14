pub async fn concat(strings: Vec<String>) -> String {
    strings.join("")
}

pub async fn split(s: String, delimiter: String) -> Vec<String> {
    s.split(&delimiter).map(|s| s.to_string()).collect()
}

pub async fn trim(s: String) -> String {
    s.trim().to_string()
}

pub async fn contains(s: String, pattern: String) -> bool {
    s.contains(&pattern)
}

pub async fn replace(s: String, from: String, to: String) -> String {
    s.replace(&from, &to)
}

pub async fn len(s: String) -> i64 {
    s.len() as i64
}

pub async fn to_uppercase(s: String) -> String {
    s.to_uppercase()
}

pub async fn to_lowercase(s: String) -> String {
    s.to_lowercase()
}

// Convert any type that implements Debug to string
pub async fn from<T: std::fmt::Debug>(value: T) -> String {
    format!("{:?}", value)
}

// Convert array of integers to string
pub async fn from_array_i64(arr: Vec<i64>) -> String {
    format!("{:?}", arr)
}

// Convert array of floats to string
pub async fn from_array_f64(arr: Vec<f64>) -> String {
    format!("{:?}", arr)
}

// Convert array of strings to string
pub async fn from_array_string(arr: Vec<String>) -> String {
    format!("{:?}", arr)
}

// Convert array of booleans to string
pub async fn from_array_bool(arr: Vec<bool>) -> String {
    format!("{:?}", arr)
}

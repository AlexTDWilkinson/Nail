pub fn concat(strings: Vec<String>) -> String {
    strings.join("")
}

pub fn split(s: String, delimiter: String) -> Vec<String> {
    s.split(&delimiter).map(|s| s.to_string()).collect()
}

pub fn trim(s: String) -> String {
    s.trim().to_string()
}

pub fn contains(s: String, pattern: String) -> bool {
    s.contains(&pattern)
}

pub fn replace(s: String, from: String, to: String) -> String {
    s.replace(&from, &to)
}

pub fn len(s: String) -> i64 {
    s.len() as i64
}

pub fn to_uppercase(s: String) -> String {
    s.to_uppercase()
}

pub fn to_lowercase(s: String) -> String {
    s.to_lowercase()
}

// Convert any type that implements Display to string
pub fn string_from<T: std::fmt::Display>(value: T) -> String {
    format!("{}", value)
}
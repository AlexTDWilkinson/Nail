pub fn concat(strings: Vec<String>) -> String {
    strings.join("")
}

pub fn split(s: String, delimiter: String) -> Vec<String> {
    s.split(&delimiter).map(|s| s.to_string()).collect()
}

pub fn trim(s: String) -> String {
    s.trim().to_string()
}

pub fn contains(s: &String, pattern: String) -> bool {
    s.contains(&pattern)
}

pub fn replace(s: String, from: String, to: String) -> String {
    s.replace(&from, &to)
}

pub fn len(s: &String) -> i64 {
    s.len() as i64
}

pub fn to_uppercase(s: String) -> String {
    s.to_uppercase()
}

pub fn to_lowercase(s: String) -> String {
    s.to_lowercase()
}

// Convert any type that implements Debug to string
pub fn from<T: std::fmt::Debug>(value: T) -> String {
    format!("{:?}", value)
}

// Convert array of integers to string
pub fn from_array_i64(arr: Vec<i64>) -> String {
    format!("{:?}", arr)
}

// Convert array of floats to string
pub fn from_array_f64(arr: Vec<f64>) -> String {
    format!("{:?}", arr)
}

// Convert array of strings to string
pub fn from_array_string(arr: Vec<String>) -> String {
    format!("{:?}", arr)
}

// Convert array of booleans to string
pub fn from_array_bool(arr: Vec<bool>) -> String {
    format!("{:?}", arr)
}

// Check if string starts with prefix
pub fn starts_with(s: &String, prefix: String) -> bool {
    s.starts_with(&prefix)
}

// Check if string ends with suffix
pub fn ends_with(s: &String, suffix: String) -> bool {
    s.ends_with(&suffix)
}

// Find index of first occurrence of substring
pub fn index_of(s: &String, substring: String) -> Result<i64, String> {
    match s.find(&substring) {
        Some(idx) => Ok(idx as i64),
        None => Err(format!("string_index_of: substring '{}' not found in the string", substring))
    }
}

// Find index of last occurrence of substring
pub fn last_index_of(s: &String, substring: String) -> Result<i64, String> {
    match s.rfind(&substring) {
        Some(idx) => Ok(idx as i64),
        None => Err(format!("string_last_index_of: substring '{}' not found in the string", substring))
    }
}

// Extract substring from start to end index
pub fn substring(s: String, start: i64, end: i64) -> Result<String, String> {
    if start < 0 || end < 0 {
        return Err(format!("string_substring: indices cannot be negative, got {} and {}", start, end));
    }
    
    let start_idx = start as usize;
    let end_idx = end as usize;
    let len = s.len();
    
    if start_idx > len {
        return Err(format!("string_substring: start index {} is beyond the string length {}", start, len));
    }
    
    if end_idx > len {
        return Err(format!("string_substring: end index {} is beyond the string length {}", end, len));
    }
    
    if start_idx > end_idx {
        return Err(format!("string_substring: start index {} is greater than end index {}", start, end));
    }
    
    // Handle UTF-8 properly
    let chars: Vec<char> = s.chars().collect();
    if start_idx > chars.len() || end_idx > chars.len() {
        return Err(format!("string_substring: range {}..{} is out of bounds for the string's {} characters", start, end, chars.len()));
    }
    
    Ok(chars[start_idx..end_idx].iter().collect())
}

// Repeat string n times
pub fn repeat(s: String, count: i64) -> String {
    if count <= 0 {
        return String::new();
    }
    s.repeat(count as usize)
}

// Reverse string
pub fn reverse(s: String) -> String {
    s.chars().rev().collect()
}

pub fn minify(s: String) -> String {
    // Minify JSON or any string by removing unnecessary whitespace
    // This preserves whitespace inside quoted strings
    let mut result = String::new();
    let mut in_string = false;
    let mut escape_next = false;
    
    for ch in s.chars() {
        if escape_next {
            result.push(ch);
            escape_next = false;
            continue;
        }
        
        if ch == '\\' && in_string {
            result.push(ch);
            escape_next = true;
            continue;
        }
        
        if ch == '"' {
            in_string = !in_string;
            result.push(ch);
            continue;
        }
        
        if in_string {
            // Inside strings, keep everything including whitespace
            result.push(ch);
        } else {
            // Outside strings, skip all whitespace
            if !ch.is_whitespace() {
                result.push(ch);
            }
        }
    }
    
    result
}

// Join array of strings with separator
pub fn join(arr: Vec<String>, separator: String) -> String {
    arr.join(&separator)
}

// Convert string to array of single-character strings
pub fn chars(s: String) -> Vec<String> {
    s.chars().map(|c| c.to_string()).collect()
}

// Check if string is empty
pub fn is_empty(s: &String) -> bool {
    s.is_empty()
}

// Pad string on the left to reach target length
pub fn pad_start(s: String, target_length: i64, pad_str: String) -> String {
    let current_len = s.len();
    let target = target_length as usize;
    
    if current_len >= target || pad_str.is_empty() {
        return s;
    }
    
    let pad_needed = target - current_len;
    let pad_chars: Vec<char> = pad_str.chars().collect();
    let pad_len = pad_chars.len();
    
    let mut result = String::new();
    
    // Add padding
    let full_repeats = pad_needed / pad_len;
    let partial = pad_needed % pad_len;
    
    for _ in 0..full_repeats {
        result.push_str(&pad_str);
    }
    
    for i in 0..partial {
        result.push(pad_chars[i]);
    }
    
    result.push_str(&s);
    result
}

// Pad string on the right to reach target length
pub fn pad_end(s: String, target_length: i64, pad_str: String) -> String {
    let current_len = s.len();
    let target = target_length as usize;
    
    if current_len >= target || pad_str.is_empty() {
        return s;
    }
    
    let pad_needed = target - current_len;
    let pad_chars: Vec<char> = pad_str.chars().collect();
    let pad_len = pad_chars.len();
    
    let mut result = s.clone();
    
    // Add padding
    let full_repeats = pad_needed / pad_len;
    let partial = pad_needed % pad_len;
    
    for _ in 0..full_repeats {
        result.push_str(&pad_str);
    }
    
    for i in 0..partial {
        result.push(pad_chars[i]);
    }
    
    result
}

// Remove leading whitespace
pub fn trim_start(s: String) -> String {
    s.trim_start().to_string()
}

// Remove trailing whitespace
pub fn trim_end(s: String) -> String {
    s.trim_end().to_string()
}

// Replace first occurrence of substring
pub fn replace_first(s: String, from: String, to: String) -> String {
    s.replacen(&from, &to, 1)
}

// Convert to snake_case (handles camelCase, spaces, and dashes)
pub fn to_snake_case(s: String) -> String {
    let mut result = String::with_capacity(s.len());
    let mut prev_was_separator = true;
    for c in s.chars() {
        if c.is_uppercase() {
            if !prev_was_separator && !result.is_empty() {
                result.push('_');
            }
            for lower in c.to_lowercase() {
                result.push(lower);
            }
            prev_was_separator = false;
        } else if c == ' ' || c == '-' || c == '_' {
            if !prev_was_separator && !result.is_empty() {
                result.push('_');
            }
            prev_was_separator = true;
        } else {
            result.push(c);
            prev_was_separator = false;
        }
    }
    result.trim_end_matches('_').to_string()
}

// Convert to kebab-case (handles camelCase, spaces, and underscores)
pub fn to_kebab_case(s: String) -> String {
    to_snake_case(s).replace('_', "-")
}

// Convert to title case (capitalize first letter of each word)
pub fn to_title_case(s: String) -> String {
    s.split_whitespace()
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                None => String::new(),
                Some(first) => first.to_uppercase().chain(chars.as_str().to_lowercase().chars()).collect(),
            }
        })
        .collect::<Vec<String>>()
        .join(" ")
}

// Convert to sentence case (capitalize first letter, rest lowercase)
pub fn to_sentence_case(s: String) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(first) => first.to_uppercase().chain(chars.as_str().to_lowercase().chars()).collect(),
    }
}

// Split string by newlines
pub fn split_lines(s: String) -> Vec<String> {
    s.lines().map(|line| line.to_string()).collect()
}

// Split string by whitespace
pub fn split_whitespace(s: String) -> Vec<String> {
    s.split_whitespace().map(|word| word.to_string()).collect()
}

// Count occurrences of substring
pub fn count(s: &String, substring: String) -> i64 {
    s.matches(&substring).count() as i64
}

// Capitalize first letter only (rest unchanged)
pub fn capitalize(s: String) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
    }
}

// Slice string with negative index support (Python-style)
pub fn slice(s: String, start: i64, end: i64) -> Result<String, String> {
    let chars: Vec<char> = s.chars().collect();
    let len = chars.len() as i64;
    
    // Handle negative indices
    let actual_start = if start < 0 {
        (len + start).max(0) as usize
    } else {
        start.min(len) as usize
    };
    
    let actual_end = if end < 0 {
        (len + end).max(0) as usize
    } else {
        end.min(len) as usize
    };
    
    if actual_start > actual_end {
        return Err(format!("string_slice: start index {} is greater than end index {}", start, end));
    }
    
    Ok(chars[actual_start..actual_end].iter().collect())
}

// Check if string is numeric (can be parsed as a number, includes decimals and signs)
pub fn is_numeric(s: &String) -> bool {
    s.chars().all(|c| c.is_numeric() || c == '.' || c == '-' || c == '+')
        && s.parse::<f64>().is_ok()
}

// Check if string contains only digit characters (0-9)
pub fn is_digits_only(s: &String) -> bool {
    !s.is_empty() && s.chars().all(|c| c.is_ascii_digit())
}

// Check if string contains only alphabetic characters
pub fn is_alphabetic(s: &String) -> bool {
    !s.is_empty() && s.chars().all(|c| c.is_alphabetic())
}

// Check if string contains only alphanumeric characters
pub fn is_alphanumeric(s: &String) -> bool {
    !s.is_empty() && s.chars().all(|c| c.is_alphanumeric())
}

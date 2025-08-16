use regex::Regex;

/// Check if a regex pattern matches the text
pub async fn match_pattern(pattern: String, text: String) -> Result<bool, String> {
    match Regex::new(&pattern) {
        Ok(re) => Ok(re.is_match(&text)),
        Err(e) => Err(format!("Invalid regex pattern: {}", e))
    }
}

/// Find the first match of a pattern in text
pub async fn find(pattern: String, text: String) -> Result<String, String> {
    let re = Regex::new(&pattern)
        .map_err(|e| format!("Invalid regex pattern: {}", e))?;
    
    match re.find(&text) {
        Some(mat) => Ok(mat.as_str().to_string()),
        None => Err("No match found".to_string())
    }
}

/// Find all matches of a pattern in text
pub async fn find_all(pattern: String, text: String) -> Result<Vec<String>, String> {
    let re = Regex::new(&pattern)
        .map_err(|e| format!("Invalid regex pattern: {}", e))?;
    
    let matches: Vec<String> = re.find_iter(&text)
        .map(|mat| mat.as_str().to_string())
        .collect();
    
    if matches.is_empty() {
        Err("No matches found".to_string())
    } else {
        Ok(matches)
    }
}

/// Replace all matches of a pattern with replacement text
pub async fn replace(pattern: String, text: String, replacement: String) -> Result<String, String> {
    let re = Regex::new(&pattern)
        .map_err(|e| format!("Invalid regex pattern: {}", e))?;
    
    Ok(re.replace_all(&text, replacement.as_str()).to_string())
}

/// Split text by a regex pattern
pub async fn split(pattern: String, text: String) -> Result<Vec<String>, String> {
    let re = Regex::new(&pattern)
        .map_err(|e| format!("Invalid regex pattern: {}", e))?;
    
    let parts: Vec<String> = re.split(&text)
        .map(|s| s.to_string())
        .collect();
    
    Ok(parts)
}
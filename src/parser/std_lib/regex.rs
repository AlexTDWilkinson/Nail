use regex::Regex;

/// Check if a regex pattern matches the text
pub fn match_pattern(pattern: String, text: String) -> Result<bool, String> {
    match Regex::new(&pattern) {
        Ok(re) => Ok(re.is_match(&text)),
        Err(e) => Err(format!("regex_match: invalid pattern '{}': {}", pattern, e))
    }
}

/// Find the first match of a pattern in text
pub fn find(pattern: String, text: String) -> Result<String, String> {
    let re = Regex::new(&pattern)
        .map_err(|e| format!("regex_find: invalid pattern '{}': {}", pattern, e))?;

    match re.find(&text) {
        Some(mat) => Ok(mat.as_str().to_string()),
        None => Err(format!("regex_find: no match for pattern '{}' in the text", pattern))
    }
}

/// Find all matches of a pattern in text
pub fn find_all(pattern: String, text: String) -> Result<Vec<String>, String> {
    let re = Regex::new(&pattern)
        .map_err(|e| format!("regex_find_all: invalid pattern '{}': {}", pattern, e))?;

    let matches: Vec<String> = re.find_iter(&text)
        .map(|mat| mat.as_str().to_string())
        .collect();

    if matches.is_empty() {
        Err(format!("regex_find_all: no matches for pattern '{}' in the text", pattern))
    } else {
        Ok(matches)
    }
}

/// Replace all matches of a pattern with replacement text
pub fn replace(pattern: String, text: String, replacement: String) -> Result<String, String> {
    let re = Regex::new(&pattern)
        .map_err(|e| format!("regex_replace: invalid pattern '{}': {}", pattern, e))?;
    
    Ok(re.replace_all(&text, replacement.as_str()).to_string())
}

/// Split text by a regex pattern
pub fn split(pattern: String, text: String) -> Result<Vec<String>, String> {
    let re = Regex::new(&pattern)
        .map_err(|e| format!("regex_split: invalid pattern '{}': {}", pattern, e))?;
    
    let parts: Vec<String> = re.split(&text)
        .map(|s| s.to_string())
        .collect();
    
    Ok(parts)
}
/// The capture groups of the first match, in order, with the whole match first.
/// A group that took part in the match but captured nothing comes back as the
/// empty string, because Nail has no way to say "there is no string here".
pub fn captures(pattern: String, text: String) -> Result<Vec<String>, String> {
    let expression = Regex::new(&pattern).map_err(|failure| format!("regex_captures: invalid pattern '{}': {}", pattern, failure))?;
    return match expression.captures(&text) {
        Some(found) => Ok(found.iter().map(|group| group.map_or(String::new(), |group| group.as_str().to_string())).collect()),
        None => Err(format!("regex_captures: no match for pattern '{}' in the text", pattern)),
    };
}

/// One named capture group of the first match, for patterns written with
/// `(?<name>...)`. Naming the groups keeps a pattern readable when it has more
/// than two of them, and means adding a group does not renumber the rest.
pub fn capture_named(pattern: String, text: String, name: String) -> Result<String, String> {
    let expression = Regex::new(&pattern).map_err(|failure| format!("regex_capture_named: invalid pattern '{}': {}", pattern, failure))?;
    let found = match expression.captures(&text) {
        Some(found) => found,
        None => return Err(format!("regex_capture_named: no match for pattern '{}' in the text", pattern)),
    };
    return match found.name(&name) {
        Some(group) => Ok(group.as_str().to_string()),
        None => Err(format!("regex_capture_named: the pattern '{}' has no group named '{}'", pattern, name)),
    };
}

/// Replaces only the first match, where `regex_replace` replaces every one.
pub fn replace_first(pattern: String, text: String, replacement: String) -> Result<String, String> {
    let expression = Regex::new(&pattern).map_err(|failure| format!("regex_replace_first: invalid pattern '{}': {}", pattern, failure))?;
    return Ok(expression.replace(&text, replacement.as_str()).to_string());
}

/// How many times the pattern matches. Zero is an answer here rather than an
/// error, because counting nothing is a useful thing to have counted.
pub fn count(pattern: String, text: String) -> Result<i64, String> {
    let expression = Regex::new(&pattern).map_err(|failure| format!("regex_count: invalid pattern '{}': {}", pattern, failure))?;
    return Ok(expression.find_iter(&text).count() as i64);
}

/// Whether the pattern is a pattern at all. A program taking a search from a
/// visitor asks this before searching, so a stray bracket is a message rather
/// than a failed request.
pub fn is_valid(pattern: &String) -> bool {
    return Regex::new(pattern).is_ok();
}

/// Text with every regex character escaped, so it can be put inside a pattern
/// and match only itself. This is what makes a user-supplied search term safe
/// to build a pattern around.
pub fn escape(text: String) -> String {
    return regex::escape(&text);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capture_groups_come_back_with_the_whole_match_first() {
        let found = captures("(\\d+)-(\\d+)".to_string(), "order 12-34 shipped".to_string()).expect("a match");
        assert_eq!(found, vec!["12-34".to_string(), "12".to_string(), "34".to_string()]);
    }

    #[test]
    fn a_group_that_captured_nothing_is_an_empty_string() {
        let found = captures("a(x)?(b)".to_string(), "ab".to_string()).expect("a match");
        assert_eq!(found, vec!["ab".to_string(), String::new(), "b".to_string()]);
    }

    #[test]
    fn named_groups_are_read_by_name() {
        let pattern = "(?<year>\\d{4})-(?<month>\\d{2})".to_string();
        assert_eq!(capture_named(pattern.clone(), "2026-08".to_string(), "year".to_string()).expect("a match"), "2026");
        assert_eq!(capture_named(pattern.clone(), "2026-08".to_string(), "month".to_string()).expect("a match"), "08");
        assert!(capture_named(pattern, "2026-08".to_string(), "day".to_string()).is_err());
    }

    #[test]
    fn replacing_the_first_leaves_the_rest() {
        assert_eq!(replace_first("\\d".to_string(), "a1b2".to_string(), "#".to_string()).expect("valid"), "a#b2");
        assert_eq!(replace("\\d".to_string(), "a1b2".to_string(), "#".to_string()).expect("valid"), "a#b#");
    }

    #[test]
    fn counting_no_matches_is_zero_rather_than_an_error() {
        assert_eq!(count("\\d".to_string(), "a1b2".to_string()).expect("valid"), 2);
        assert_eq!(count("\\d".to_string(), "abc".to_string()).expect("valid"), 0);
    }

    #[test]
    fn a_broken_pattern_is_reported_rather_than_matched() {
        assert!(is_valid(&"\\d+".to_string()));
        assert!(!is_valid(&"(unclosed".to_string()));
        assert!(captures("(unclosed".to_string(), "text".to_string()).is_err());
    }

    #[test]
    fn escaped_text_matches_only_itself() {
        let literal = escape("1+1 (really)".to_string());
        assert!(match_pattern(literal.clone(), "1+1 (really)".to_string()).expect("valid"));
        assert!(!match_pattern(literal, "11 really".to_string()).expect("valid"));
    }
}

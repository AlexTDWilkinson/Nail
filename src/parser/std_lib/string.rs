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

/// Both quote characters are escaped as well as the three markup ones, because
/// text is just as likely to be written into an attribute as between tags -
/// `value="{}"` is the usual case - and there a bare quote ends the attribute
/// and lets whatever follows become markup of its own.
pub(crate) fn push_escaped_html(ch: char, out: &mut String) {
    match ch {
        '&' => out.push_str("&amp;"),
        '<' => out.push_str("&lt;"),
        '>' => out.push_str("&gt;"),
        '"' => out.push_str("&quot;"),
        '\'' => out.push_str("&#39;"),
        _ => out.push(ch),
    }
}

/// Escape text so it can be put in a page without becoming part of the markup.
/// Anything a visitor supplied - a name, a comment, a search term echoed back -
/// goes through here on its way into HTML.
pub fn escape_html(text: &String) -> String {
    let mut out = String::with_capacity(text.len() + 16);
    for ch in text.chars() {
        push_escaped_html(ch, &mut out);
    }
    return out;
}

/// The reverse of `escape_html`, for text that arrived as markup and has to be
/// shown as itself again. Only the entities `escape_html` writes are
/// recognised, plus the numeric form a browser may have sent instead.
pub fn unescape_html(text: String) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text.as_str();
    while let Some(start) = rest.find('&') {
        out.push_str(&rest[..start]);
        rest = &rest[start..];
        let end = match rest.find(';') {
            // A bare ampersand with no semicolon after it is just an
            // ampersand, so it is kept and the scan moves past it.
            None => {
                out.push_str(rest);
                return out;
            }
            Some(end) => end,
        };
        let entity = &rest[1..end];
        match entity {
            "amp" => out.push('&'),
            "lt" => out.push('<'),
            "gt" => out.push('>'),
            "quot" => out.push('"'),
            "apos" => out.push('\''),
            "nbsp" => out.push(' '),
            numeric if numeric.starts_with('#') => {
                let (digits, radix) = match numeric.strip_prefix("#x").or_else(|| numeric.strip_prefix("#X")) {
                    Some(hex) => (hex, 16),
                    None => (&numeric[1..], 10),
                };
                match u32::from_str_radix(digits, radix).ok().and_then(char::from_u32) {
                    Some(character) => out.push(character),
                    None => out.push_str(&rest[..=end]),
                }
            }
            _ => out.push_str(&rest[..=end]),
        }
        rest = &rest[end + 1..];
    }
    out.push_str(rest);
    return out;
}

/// The words of a string, split on anything that is not a letter or digit.
/// Every case conversion below works from this, so `to_camel_case` and
/// `to_snake_case` agree on where one word ends and the next begins - and both
/// agree with what `slugify` does to a title.
fn words(text: &str) -> Vec<String> {
    let mut found: Vec<String> = Vec::new();
    let mut current = String::new();
    let mut previous_was_lowercase = false;
    for character in text.chars() {
        if character.is_alphanumeric() {
            // A capital after a small letter starts a new word, so that
            // `parseHTTPResponse` splits as parse / HTTP / Response.
            if character.is_uppercase() && previous_was_lowercase && !current.is_empty() {
                found.push(std::mem::take(&mut current));
            }
            current.push(character);
            previous_was_lowercase = character.is_lowercase() || character.is_numeric();
        } else if !current.is_empty() {
            found.push(std::mem::take(&mut current));
            previous_was_lowercase = false;
        }
    }
    if !current.is_empty() {
        found.push(current);
    }
    return found;
}

/// `some text here` becomes `someTextHere` - the spelling JavaScript APIs and
/// JSON keys use.
pub fn to_camel_case(text: String) -> String {
    let mut out = String::with_capacity(text.len());
    for (position, word) in words(&text).iter().enumerate() {
        if position == 0 {
            out.push_str(&word.to_lowercase());
        } else {
            let mut characters = word.chars();
            if let Some(first) = characters.next() {
                out.extend(first.to_uppercase());
                out.push_str(&characters.as_str().to_lowercase());
            }
        }
    }
    return out;
}

/// `some text here` becomes `SomeTextHere` - the spelling type names use.
pub fn to_pascal_case(text: String) -> String {
    let mut out = String::with_capacity(text.len());
    for word in words(&text) {
        let mut characters = word.chars();
        if let Some(first) = characters.next() {
            out.extend(first.to_uppercase());
            out.push_str(&characters.as_str().to_lowercase());
        }
    }
    return out;
}

/// A title turned into the part of a URL that carries it: lowercase, words
/// joined by single hyphens, everything else dropped. `Hello, World!` becomes
/// `hello-world`.
pub fn slugify(text: String) -> String {
    return words(&text).iter().map(|word| word.to_lowercase()).collect::<Vec<String>>().join("-");
}

/// Text cut to a maximum length, with the ellipsis counted as part of that
/// length so the result never exceeds it. Cutting happens on a character
/// boundary, and a string already short enough comes back untouched.
pub fn truncate(text: String, max_length: i64, ellipsis: String) -> String {
    let max_length = max_length.max(0) as usize;
    let characters: Vec<char> = text.chars().collect();
    if characters.len() <= max_length {
        return text;
    }
    let ellipsis_length = ellipsis.chars().count();
    // No room for text alongside the ellipsis - give back as much of the
    // ellipsis as fits rather than something longer than asked for.
    if ellipsis_length >= max_length {
        return ellipsis.chars().take(max_length).collect();
    }
    let kept: String = characters[..max_length - ellipsis_length].iter().collect();
    return format!("{}{}", kept.trim_end(), ellipsis);
}

/// Text broken into lines no longer than the given width, splitting between
/// words. Line breaks already in the text are kept, so paragraphs stay
/// paragraphs. A single word longer than the width is left whole rather than
/// cut in half.
pub fn word_wrap(text: String, width: i64) -> String {
    let width = width.max(1) as usize;
    let mut wrapped: Vec<String> = Vec::new();
    for existing_line in text.split('\n') {
        let mut line = String::new();
        for word in existing_line.split_whitespace() {
            let word_length = word.chars().count();
            if line.is_empty() {
                line.push_str(word);
            } else if line.chars().count() + 1 + word_length <= width {
                line.push(' ');
                line.push_str(word);
            } else {
                wrapped.push(std::mem::take(&mut line));
                line.push_str(word);
            }
        }
        wrapped.push(line);
    }
    return wrapped.join("\n");
}

/// How many single-character edits - insert, delete, substitute - turn one
/// string into the other. Zero means they are the same string.
pub fn levenshtein(first: &String, second: &String) -> i64 {
    let first_characters: Vec<char> = first.chars().collect();
    let second_characters: Vec<char> = second.chars().collect();
    if first_characters.is_empty() {
        return second_characters.len() as i64;
    }
    if second_characters.is_empty() {
        return first_characters.len() as i64;
    }

    // Only the previous row of the edit-distance table is ever needed, so one
    // row is kept rather than the whole grid.
    let mut previous_row: Vec<usize> = (0..=second_characters.len()).collect();
    let mut current_row: Vec<usize> = vec![0; second_characters.len() + 1];
    for (first_index, first_character) in first_characters.iter().enumerate() {
        current_row[0] = first_index + 1;
        for (second_index, second_character) in second_characters.iter().enumerate() {
            let substitution_cost = if first_character == second_character { 0 } else { 1 };
            current_row[second_index + 1] = (current_row[second_index] + 1)
                .min(previous_row[second_index + 1] + 1)
                .min(previous_row[second_index] + substitution_cost);
        }
        std::mem::swap(&mut previous_row, &mut current_row);
    }
    return previous_row[second_characters.len()] as i64;
}

/// How alike two strings are, from 0.0 for nothing in common to 1.0 for the
/// same string. This is the edit distance scaled by the longer length, which
/// is what makes it comparable between pairs of different sizes.
pub fn similarity(first: &String, second: &String) -> f64 {
    let longest = first.chars().count().max(second.chars().count());
    if longest == 0 {
        return 1.0;
    }
    return 1.0 - (levenshtein(first, second) as f64 / longest as f64);
}

/// The candidate most like the given text, for answering a mistyped command
/// with "did you mean". An empty list of candidates is an error, because there
/// is no answer to give.
pub fn closest(text: String, candidates: Vec<String>) -> Result<String, String> {
    let mut best: Option<(i64, &String)> = None;
    for candidate in candidates.iter() {
        let distance = levenshtein(&text, candidate);
        if best.is_none() || distance < best.expect("checked above").0 {
            best = Some((distance, candidate));
        }
    }
    return match best {
        Some((_, candidate)) => Ok(candidate.clone()),
        None => Err("string_closest: there were no candidates to choose from".to_string()),
    };
}

/// How many words the text holds, counting a word as a run of non-whitespace.
pub fn word_count(text: &String) -> i64 {
    return text.split_whitespace().count() as i64;
}

/// Every line of the text with the prefix in front of it. Blank lines are left
/// blank rather than being given trailing whitespace.
pub fn indent(text: String, prefix: String) -> String {
    return text.split('\n').map(|line| if line.trim().is_empty() { line.to_string() } else { format!("{}{}", prefix, line) }).collect::<Vec<String>>().join("\n");
}

/// The opposite of `indent` for text that was written inside an indented
/// block: the whitespace every non-blank line shares is removed from all of
/// them, so the relative shape of the text is kept.
pub fn dedent(text: String) -> String {
    let common = text
        .split('\n')
        .filter(|line| !line.trim().is_empty())
        .map(|line| line.len() - line.trim_start().len())
        .min()
        .unwrap_or(0);
    return text.split('\n').map(|line| if line.len() >= common { line[common..].to_string() } else { line.trim_start().to_string() }).collect::<Vec<String>>().join("\n");
}

/// Every run of whitespace - including line breaks and tabs - collapsed to one
/// space, with the ends trimmed. What text pulled out of HTML needs before it
/// can be compared or displayed.
pub fn normalize_whitespace(text: String) -> String {
    return text.split_whitespace().collect::<Vec<&str>>().join(" ");
}

/// The single character at an index, as a string, since Nail has no character
/// type. An index past the end is an error rather than an empty string, so a
/// mistake is not silently carried along.
pub fn char_at(text: String, index: i64) -> Result<String, String> {
    if index < 0 {
        return Err(format!("string_char_at: the index {} is negative", index));
    }
    let characters: Vec<char> = text.chars().collect();
    return match characters.get(index as usize) {
        Some(character) => Ok(character.to_string()),
        None => Err(format!("string_char_at: the index {} is past the end of a string of {} characters", index, characters.len())),
    };
}

/// The beginning that every one of the strings shares, which is how a set of
/// paths is reduced to the directory holding them all. No strings, or nothing
/// in common, gives the empty string.
pub fn common_prefix(strings: Vec<String>) -> String {
    let mut candidates = strings.iter();
    let mut shared: Vec<char> = match candidates.next() {
        Some(first) => first.chars().collect(),
        None => return String::new(),
    };
    for candidate in candidates {
        let mut matched = 0;
        for (position, character) in candidate.chars().enumerate() {
            if position >= shared.len() || shared[position] != character {
                break;
            }
            matched = position + 1;
        }
        shared.truncate(matched);
        if shared.is_empty() {
            return String::new();
        }
    }
    return shared.into_iter().collect();
}

/// The text without the given prefix, or the text unchanged if it did not
/// start with it.
pub fn strip_prefix(text: String, prefix: String) -> String {
    return match text.strip_prefix(&prefix) {
        Some(rest) => rest.to_string(),
        None => text,
    };
}

/// The text without the given suffix, or the text unchanged if it did not end
/// with it.
pub fn strip_suffix(text: String, suffix: String) -> String {
    return match text.strip_suffix(&suffix) {
        Some(rest) => rest.to_string(),
        None => text,
    };
}

/// A secret with all but the last few characters replaced, for showing which
/// key or card is in use without printing it. Everything is hidden when the
/// text is no longer than the part that would have been shown.
pub fn mask(text: String, visible_tail: i64, mask_character: String) -> String {
    let characters: Vec<char> = text.chars().collect();
    let visible = (visible_tail.max(0) as usize).min(characters.len());
    let hidden = characters.len() - visible;
    let symbol = mask_character.chars().next().unwrap_or('*');
    let mut out: String = std::iter::repeat(symbol).take(hidden).collect();
    out.extend(characters[hidden..].iter());
    return out;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn html_entities_go_back_to_the_characters_they_stood_for() {
        assert_eq!(unescape_html("&lt;b&gt;hi&lt;/b&gt;".to_string()), "<b>hi</b>");
        assert_eq!(unescape_html("a &amp; b".to_string()), "a & b");
        assert_eq!(unescape_html("&quot;quoted&quot;".to_string()), "\"quoted\"");
        assert_eq!(unescape_html("&#39;&#x27;".to_string()), "''");
    }

    #[test]
    fn text_that_only_looks_like_an_entity_is_left_alone() {
        assert_eq!(unescape_html("a & b".to_string()), "a & b");
        assert_eq!(unescape_html("&unknown; thing".to_string()), "&unknown; thing");
        assert_eq!(unescape_html("100% & rising".to_string()), "100% & rising");
    }

    #[test]
    fn escaping_and_unescaping_round_trip() {
        let original = "<a href=\"x\">it's 5 > 3 & true</a>".to_string();
        assert_eq!(unescape_html(escape_html(&original)), original);
    }

    #[test]
    fn case_conversions_agree_on_where_words_end() {
        assert_eq!(to_camel_case("some text here".to_string()), "someTextHere");
        assert_eq!(to_pascal_case("some text here".to_string()), "SomeTextHere");
        assert_eq!(to_camel_case("user_id_number".to_string()), "userIdNumber");
        assert_eq!(to_pascal_case("parse-html-tag".to_string()), "ParseHtmlTag");
        assert_eq!(to_camel_case("alreadyCamel".to_string()), "alreadyCamel");
        assert_eq!(to_snake_case("parseHTTPResponse".to_string()), "parse_h_t_t_p_response");
    }

    #[test]
    fn a_title_becomes_a_url_part() {
        assert_eq!(slugify("Hello, World!".to_string()), "hello-world");
        assert_eq!(slugify("  Multiple   Spaces  ".to_string()), "multiple-spaces");
        assert_eq!(slugify("Nail 1.0 Released".to_string()), "nail-1-0-released");
        assert_eq!(slugify("".to_string()), "");
    }

    #[test]
    fn truncation_counts_the_ellipsis_in_the_limit() {
        assert_eq!(truncate("hello world".to_string(), 8, "...".to_string()), "hello...");
        assert_eq!(truncate("hello".to_string(), 8, "...".to_string()), "hello");
        assert_eq!(truncate("hello world".to_string(), 3, "...".to_string()), "...");
        assert_eq!(truncate("hello world".to_string(), 2, "...".to_string()), "..");
        assert!(truncate("hello world".to_string(), 8, "...".to_string()).chars().count() <= 8);
    }

    #[test]
    fn wrapping_breaks_between_words_and_keeps_paragraphs() {
        assert_eq!(word_wrap("the quick brown fox".to_string(), 10), "the quick\nbrown fox");
        assert_eq!(word_wrap("one\ntwo".to_string(), 10), "one\ntwo");
        assert_eq!(word_wrap("unsplittableword".to_string(), 5), "unsplittableword");
        for line in word_wrap("a b c d e f g h".to_string(), 3).split('\n') {
            assert!(line.chars().count() <= 3, "line too long: {}", line);
        }
    }

    #[test]
    fn edit_distance_counts_single_character_changes() {
        assert_eq!(levenshtein(&"kitten".to_string(), &"sitting".to_string()), 3);
        assert_eq!(levenshtein(&"same".to_string(), &"same".to_string()), 0);
        assert_eq!(levenshtein(&"".to_string(), &"abc".to_string()), 3);
        assert_eq!(levenshtein(&"abc".to_string(), &"".to_string()), 3);
    }

    #[test]
    fn similarity_runs_from_zero_to_one() {
        assert_eq!(similarity(&"same".to_string(), &"same".to_string()), 1.0);
        assert_eq!(similarity(&"".to_string(), &"".to_string()), 1.0);
        assert_eq!(similarity(&"abc".to_string(), &"xyz".to_string()), 0.0);
        let close = similarity(&"transpile".to_string(), &"transpiles".to_string());
        assert!(close > 0.8 && close < 1.0, "got {}", close);
    }

    #[test]
    fn the_closest_candidate_answers_a_typo() {
        let commands = vec!["transpile".to_string(), "check".to_string(), "format".to_string()];
        assert_eq!(closest("transpil".to_string(), commands.clone()).expect("a candidate"), "transpile");
        assert_eq!(closest("chekc".to_string(), commands).expect("a candidate"), "check");
        assert!(closest("anything".to_string(), vec![]).is_err());
    }

    #[test]
    fn words_are_runs_of_non_whitespace() {
        assert_eq!(word_count(&"one two three".to_string()), 3);
        assert_eq!(word_count(&"  padded  out  ".to_string()), 2);
        assert_eq!(word_count(&"".to_string()), 0);
    }

    #[test]
    fn indent_and_dedent_are_opposites() {
        let text = "first\nsecond".to_string();
        assert_eq!(indent(text.clone(), "  ".to_string()), "  first\n  second");
        assert_eq!(dedent(indent(text.clone(), "    ".to_string())), text);
    }

    #[test]
    fn indenting_leaves_blank_lines_blank() {
        assert_eq!(indent("a\n\nb".to_string(), "> ".to_string()), "> a\n\n> b");
    }

    #[test]
    fn dedent_keeps_the_relative_shape() {
        assert_eq!(dedent("    outer\n        inner".to_string()), "outer\n    inner");
    }

    #[test]
    fn whitespace_collapses_to_single_spaces() {
        assert_eq!(normalize_whitespace("  a\n\tb   c  ".to_string()), "a b c");
        assert_eq!(normalize_whitespace("".to_string()), "");
    }

    #[test]
    fn a_character_can_be_read_by_index() {
        assert_eq!(char_at("hello".to_string(), 1).expect("in range"), "e");
        assert!(char_at("hello".to_string(), 5).is_err());
        assert!(char_at("hello".to_string(), -1).is_err());
    }

    #[test]
    fn the_shared_beginning_of_strings_is_found() {
        assert_eq!(common_prefix(vec!["/srv/app/one".to_string(), "/srv/app/two".to_string()]), "/srv/app/");
        assert_eq!(common_prefix(vec!["abc".to_string(), "xyz".to_string()]), "");
        assert_eq!(common_prefix(vec!["only".to_string()]), "only");
        assert_eq!(common_prefix(vec![]), "");
    }

    #[test]
    fn stripping_an_absent_affix_changes_nothing() {
        assert_eq!(strip_prefix("prefix-body".to_string(), "prefix-".to_string()), "body");
        assert_eq!(strip_prefix("body".to_string(), "prefix-".to_string()), "body");
        assert_eq!(strip_suffix("body.txt".to_string(), ".txt".to_string()), "body");
        assert_eq!(strip_suffix("body".to_string(), ".txt".to_string()), "body");
    }

    #[test]
    fn masking_shows_only_the_tail() {
        assert_eq!(mask("sk_live_abcd1234".to_string(), 4, "*".to_string()), "************1234");
        assert_eq!(mask("short".to_string(), 10, "*".to_string()), "short");
        assert_eq!(mask("secret".to_string(), 0, "x".to_string()), "xxxxxx");
    }
}

/// The text with any of the given characters shaved off both ends. `string_trim`
/// takes whitespace off; this takes whatever you name, which is how a trailing
/// `/` comes off a URL or the quotes come off a quoted field.
pub fn trim_chars(text: String, characters: String) -> String {
    let unwanted: Vec<char> = characters.chars().collect();
    return text.trim_matches(|character| unwanted.contains(&character)).to_string();
}

/// The text with any of the given characters shaved off the front only.
pub fn trim_start_chars(text: String, characters: String) -> String {
    let unwanted: Vec<char> = characters.chars().collect();
    return text.trim_start_matches(|character| unwanted.contains(&character)).to_string();
}

/// The text with any of the given characters shaved off the end only.
pub fn trim_end_chars(text: String, characters: String) -> String {
    let unwanted: Vec<char> = characters.chars().collect();
    return text.trim_end_matches(|character| unwanted.contains(&character)).to_string();
}

/// Splits at the FIRST separator only, and returns the two halves - which is
/// what reading `key=value`, `name: value` or `path?query` needs, and what
/// `string_split` gets wrong the moment the value contains the separator too.
/// Errors when the separator is not in the text at all, since there is no
/// sensible second half to invent.
pub fn split_once(text: String, separator: String) -> Result<Vec<String>, String> {
    if separator.is_empty() {
        return Err("string_split_once: the separator is empty, so there is nothing to split at".to_string());
    }
    return match text.split_once(&separator) {
        Some((before, after)) => Ok(vec![before.to_string(), after.to_string()]),
        None => Err(format!("string_split_once: '{}' is not in the text", separator)),
    };
}

/// Splits at the LAST separator instead of the first - how a file name comes
/// apart from its extension, or a host from its port.
pub fn split_last(text: String, separator: String) -> Result<Vec<String>, String> {
    if separator.is_empty() {
        return Err("string_split_last: the separator is empty, so there is nothing to split at".to_string());
    }
    return match text.rsplit_once(&separator) {
        Some((before, after)) => Ok(vec![before.to_string(), after.to_string()]),
        None => Err(format!("string_split_last: '{}' is not in the text", separator)),
    };
}

/// The Unicode code point of the character at the index - `A` is 65. The other
/// half of `string_from_char_code`, and how character arithmetic gets done at
/// all without a character type.
pub fn char_code(text: String, index: i64) -> Result<i64, String> {
    if index < 0 {
        return Err(format!("string_char_code: the index {} is negative", index));
    }
    let characters: Vec<char> = text.chars().collect();
    return match characters.get(index as usize) {
        Some(character) => Ok(*character as i64),
        None => Err(format!("string_char_code: the index {} is past the end of a string of {} characters", index, characters.len())),
    };
}

/// The one-character string for a Unicode code point - 65 gives `A`. Errors on
/// a number that is not a character, such as a surrogate half or anything past
/// the top of the range.
pub fn from_char_code(code: i64) -> Result<String, String> {
    let candidate = u32::try_from(code).map_err(|_| format!("string_from_char_code: {} is not a Unicode code point", code))?;
    return match char::from_u32(candidate) {
        Some(character) => Ok(character.to_string()),
        None => Err(format!("string_from_char_code: {} is not a Unicode code point", code)),
    };
}

#[cfg(test)]
mod trim_split_and_code_tests {
    use super::*;

    #[test]
    fn named_characters_come_off_the_ends() {
        assert_eq!(trim_chars("/path/".to_string(), "/".to_string()), "path");
        assert_eq!(trim_chars("\"quoted\"".to_string(), "\"".to_string()), "quoted");
        assert_eq!(trim_chars("xxhixx".to_string(), "x".to_string()), "hi");
        assert_eq!(trim_start_chars("00042".to_string(), "0".to_string()), "42");
        assert_eq!(trim_end_chars("line...".to_string(), ".".to_string()), "line");
        // Characters that are not there change nothing.
        assert_eq!(trim_chars("hi".to_string(), "/".to_string()), "hi");
    }

    #[test]
    fn splitting_once_keeps_the_separator_in_the_second_half() {
        let parts = split_once("key=a=b".to_string(), "=".to_string()).expect("a separator");
        assert_eq!(parts, vec!["key".to_string(), "a=b".to_string()]);
        let last = split_last("key=a=b".to_string(), "=".to_string()).expect("a separator");
        assert_eq!(last, vec!["key=a".to_string(), "b".to_string()]);
    }

    #[test]
    fn splitting_at_a_separator_that_is_not_there_is_an_error() {
        assert!(split_once("plain".to_string(), "=".to_string()).unwrap_err().contains("not in the text"));
        assert!(split_last("plain".to_string(), "=".to_string()).unwrap_err().contains("not in the text"));
        assert!(split_once("plain".to_string(), "".to_string()).unwrap_err().contains("empty"));
    }

    #[test]
    fn code_points_go_both_ways() {
        assert_eq!(char_code("ABC".to_string(), 0).expect("in range"), 65);
        assert_eq!(from_char_code(65).expect("a code point"), "A");
        assert_eq!(char_code("héllo".to_string(), 1).expect("in range"), 233);
        assert_eq!(from_char_code(233).expect("a code point"), "é");
    }

    #[test]
    fn an_index_or_code_out_of_range_is_an_error() {
        assert!(char_code("ab".to_string(), 5).unwrap_err().contains("past the end"));
        assert!(char_code("ab".to_string(), -1).unwrap_err().contains("negative"));
        assert!(from_char_code(-1).unwrap_err().contains("not a Unicode code point"));
        assert!(from_char_code(0xD800).unwrap_err().contains("not a Unicode code point"));
    }
}

/// The characters a person sees, one string each. An emoji with skin tone or
/// a flag is one grapheme even though it is several code points.
pub fn graphemes(input: String) -> Vec<String> {
    use unicode_segmentation::UnicodeSegmentation;
    return input.graphemes(true).map(|g| g.to_string()).collect();
}

/// How many characters a person sees - the length string_length can overcount
/// when emoji and accents are involved.
pub fn grapheme_length(input: String) -> i64 {
    use unicode_segmentation::UnicodeSegmentation;
    return input.graphemes(true).count() as i64;
}

/// Unicode NFC normalization - the composed form. Two spellings of `café`
/// compare equal after both pass through here; normalize before comparing or
/// storing anything people typed.
pub fn normalize_nfc(input: String) -> String {
    use unicode_normalization::UnicodeNormalization;
    return input.nfc().collect();
}

/// Unicode NFKC normalization - compatibility form. Fullwidth letters,
/// ligatures and font variants all collapse to their plain equivalents, which
/// is what searching and usernames usually want.
pub fn normalize_nfkc(input: String) -> String {
    use unicode_normalization::UnicodeNormalization;
    return input.nfkc().collect();
}

/// The text with its accents dropped: `café` becomes `cafe`. Decomposes,
/// removes the combining marks, and recomposes what is left.
pub fn remove_accents(input: String) -> String {
    use unicode_normalization::UnicodeNormalization;
    return input.nfd().filter(|c| !unicode_normalization::char::is_combining_mark(*c)).collect::<String>().nfc().collect();
}

#[cfg(test)]
mod unicode_tests {
    use super::*;

    #[test]
    fn graphemes_count_what_a_person_sees() {
        assert_eq!(grapheme_length("héllo".to_string()), 5);
        assert_eq!(grapheme_length("🇨🇦".to_string()), 1);
        assert_eq!(graphemes("a👍b".to_string()), vec!["a", "👍", "b"]);
    }

    #[test]
    fn the_two_spellings_of_cafe_meet_in_the_middle() {
        let composed = "caf\u{e9}".to_string();
        let decomposed = "cafe\u{301}".to_string();
        assert_ne!(composed, decomposed);
        assert_eq!(normalize_nfc(decomposed.clone()), composed);
        assert_eq!(remove_accents(composed), "cafe");
        assert_eq!(remove_accents(decomposed), "cafe");
    }

    #[test]
    fn nfkc_flattens_the_fancy_forms() {
        assert_eq!(normalize_nfkc("ﬁle".to_string()), "file");
        assert_eq!(normalize_nfkc("Ｎａｉｌ".to_string()), "Nail");
    }
}

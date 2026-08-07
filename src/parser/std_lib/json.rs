use serde::{Deserialize, Serialize};
use serde_json;

/// Serialize a value (struct, enum, or array) to a pretty-formatted JSON string
pub fn json_serialize<T: Serialize>(value: T) -> Result<String, String> {
    serde_json::to_string_pretty(&value)
        .map_err(|e| {
            // Provide more helpful error messages
            if e.to_string().contains("key must be a string") {
                format!("json_serialize: hashmap keys must be strings to serialize to JSON: {}", e)
            } else {
                format!("json_serialize: only structs, enums, arrays, and basic types (string, int, float, bool) can be serialized: {}", e)
            }
        })
}

/// Deserialize a JSON string to a value (struct, enum, or array)
pub fn json_deserialize<T: for<'de> Deserialize<'de>>(json_string: String) -> Result<T, String> {
    // First check if the JSON is valid
    if let Err(e) = serde_json::from_str::<serde_json::Value>(&json_string) {
        return Err(format!("json_deserialize: invalid JSON syntax: {}", e));
    }
    
    serde_json::from_str(&json_string)
        .map_err(|e| {
            // Provide context about what went wrong
            if e.to_string().contains("missing field") {
                format!("json_deserialize: {}. Make sure all required struct fields are present in the JSON.", e)
            } else if e.to_string().contains("unknown field") {
                format!("json_deserialize: {}. The JSON contains fields not present in the target struct.", e)
            } else if e.to_string().contains("invalid type") {
                format!("json_deserialize: {}. Type mismatch between the JSON and the target struct's fields.", e)
            } else {
                format!("json_deserialize: the JSON structure does not match the expected struct/enum/array format: {}", e)
            }
        })
}


/// Walks a dotted path into a parsed JSON value. A path segment that is a
/// number indexes an array, so `items.0.name` is the first item's name.
fn walk<'value>(function: &str, document: &'value serde_json::Value, path: &str) -> Result<&'value serde_json::Value, String> {
    let mut current = document;
    if path.is_empty() {
        return Ok(current);
    }

    for segment in path.split('.') {
        current = match current {
            serde_json::Value::Object(fields) => fields.get(segment).ok_or_else(|| format!("{}: '{}' has no field '{}'", function, path, segment))?,
            serde_json::Value::Array(items) => {
                let index: usize = segment.parse().map_err(|_| format!("{}: '{}' is a list, and '{}' is not a position in it", function, path, segment))?;
                items.get(index).ok_or_else(|| format!("{}: '{}' has no item {} - the list holds {}", function, path, index, items.len()))?
            }
            _ => return Err(format!("{}: '{}' is a value, so it has nothing called '{}' inside it", function, path, segment)),
        };
    }
    return Ok(current);
}

/// The parsed document, or an error naming what is wrong with it.
fn document(function: &str, text: &str) -> Result<serde_json::Value, String> {
    return serde_json::from_str(text).map_err(|e| format!("{}: the text is not valid JSON: {}", function, e));
}

/// The text at a dotted path - `json_get_string(body, "user.name")`. For
/// reading a few fields out of an answer nobody wants to describe as a struct
/// first, which is most of what talking to somebody else's API is. A number or
/// a boolean at the path is returned as it is written, so a field that changes
/// type does not become an error.
pub fn get_string(text: String, path: String) -> Result<String, String> {
    let found = document("json_get_string", &text)?;
    let value = walk("json_get_string", &found, &path)?;
    return match value {
        serde_json::Value::String(found) => Ok(found.clone()),
        serde_json::Value::Null => Err(format!("json_get_string: '{}' is null", path)),
        serde_json::Value::Object(_) | serde_json::Value::Array(_) => Err(format!("json_get_string: '{}' holds a list or an object, not a piece of text", path)),
        other => Ok(other.to_string()),
    };
}

/// The whole number at a dotted path. A fraction is an error rather than a
/// silent rounding.
pub fn get_int(text: String, path: String) -> Result<i64, String> {
    let found = document("json_get_int", &text)?;
    let value = walk("json_get_int", &found, &path)?;
    return match value {
        serde_json::Value::Number(number) => number.as_i64().ok_or_else(|| format!("json_get_int: '{}' is {}, which is not a whole number", path, number)),
        serde_json::Value::String(written) => written.parse::<i64>().map_err(|_| format!("json_get_int: '{}' holds the text '{}', which is not a whole number", path, written)),
        _ => Err(format!("json_get_int: '{}' is not a number", path)),
    };
}

/// The number at a dotted path, whole or fractional.
pub fn get_float(text: String, path: String) -> Result<f64, String> {
    let found = document("json_get_float", &text)?;
    let value = walk("json_get_float", &found, &path)?;
    return match value {
        serde_json::Value::Number(number) => number.as_f64().ok_or_else(|| format!("json_get_float: '{}' is {}, which is not a number this machine can hold", path, number)),
        serde_json::Value::String(written) => written.parse::<f64>().map_err(|_| format!("json_get_float: '{}' holds the text '{}', which is not a number", path, written)),
        _ => Err(format!("json_get_float: '{}' is not a number", path)),
    };
}

/// The true or false at a dotted path. The strings "true" and "false" count,
/// since plenty of APIs send them that way.
pub fn get_bool(text: String, path: String) -> Result<bool, String> {
    let found = document("json_get_bool", &text)?;
    let value = walk("json_get_bool", &found, &path)?;
    return match value {
        serde_json::Value::Bool(found) => Ok(*found),
        serde_json::Value::String(written) if written == "true" => Ok(true),
        serde_json::Value::String(written) if written == "false" => Ok(false),
        _ => Err(format!("json_get_bool: '{}' is not true or false", path)),
    };
}

/// Whether there is anything at a dotted path. A field that is present but
/// null counts as missing, because for a reader they amount to the same thing.
pub fn has(text: String, path: String) -> bool {
    return match document("json_has", &text) {
        Ok(found) => match walk("json_has", &found, &path) {
            Ok(value) => !value.is_null(),
            Err(_) => false,
        },
        Err(_) => false,
    };
}

/// How many items the list at a dotted path holds - the number to count up to
/// when reading them one at a time. An empty path asks about the whole
/// document, which is how a top-level list is read.
pub fn array_length(text: String, path: String) -> Result<i64, String> {
    let found = document("json_array_length", &text)?;
    let value = walk("json_array_length", &found, &path)?;
    return match value {
        serde_json::Value::Array(items) => Ok(items.len() as i64),
        _ => Err(format!("json_array_length: '{}' is not a list", path)),
    };
}

/// The same JSON, indented - for writing a file a person will read, or a diff
/// that shows which field changed rather than one enormous line.
pub fn pretty(text: String) -> Result<String, String> {
    let found = document("json_pretty", &text)?;
    return serde_json::to_string_pretty(&found).map_err(|e| format!("json_pretty: could not write the JSON back out: {}", e));
}

/// The same JSON with every space between its values taken out - the form to
/// send over a network or store.
pub fn compact(text: String) -> Result<String, String> {
    let found = document("json_compact", &text)?;
    return serde_json::to_string(&found).map_err(|e| format!("json_compact: could not write the JSON back out: {}", e));
}

/// A changed document written back out as one compact line.
fn write_back(function: &str, value: &serde_json::Value) -> Result<String, String> {
    return serde_json::to_string(value).map_err(|e| format!("{}: could not write the JSON back out: {}", function, e));
}

/// Lays the overlay over the base, one field at a time. Two objects merge
/// field by field; anything else - a list, a number, a piece of text - is
/// simply replaced by what the overlay holds.
fn lay_over(base: &mut serde_json::Value, overlay: serde_json::Value) {
    match (base, overlay) {
        (serde_json::Value::Object(base_fields), serde_json::Value::Object(overlay_fields)) => {
            for (key, value) in overlay_fields {
                match base_fields.get_mut(&key) {
                    Some(existing) => lay_over(existing, value),
                    None => {
                        base_fields.insert(key, value);
                    }
                }
            }
        }
        (slot, value) => *slot = value,
    }
}

/// Two objects folded into one - the overlay's fields win, objects inside
/// them merge the same way, and lists and plain values are replaced whole.
/// For laying a handful of changes over a default configuration.
pub fn merge(base: String, overlay: String) -> Result<String, String> {
    let mut base_document = document("json_merge", &base)?;
    let overlay_document = document("json_merge", &overlay)?;
    if !base_document.is_object() {
        return Err("json_merge: the base is not a JSON object, so there are no fields to merge into".to_string());
    }
    if !overlay_document.is_object() {
        return Err("json_merge: the overlay is not a JSON object, so there are no fields to merge in".to_string());
    }
    lay_over(&mut base_document, overlay_document);
    return write_back("json_merge", &base_document);
}

/// Pours one value into the flat map, walking into objects and lists and
/// naming each leaf by the dotted path that reaches it.
fn pour_flat(prefix: &str, value: &serde_json::Value, flat: &mut serde_json::Map<String, serde_json::Value>) {
    match value {
        serde_json::Value::Object(fields) if !fields.is_empty() => {
            for (key, inner) in fields {
                let named = if prefix.is_empty() { key.clone() } else { format!("{}.{}", prefix, key) };
                pour_flat(&named, inner, flat);
            }
        }
        serde_json::Value::Array(items) if !items.is_empty() => {
            for (index, inner) in items.iter().enumerate() {
                let named = if prefix.is_empty() { index.to_string() } else { format!("{}.{}", prefix, index) };
                pour_flat(&named, inner, flat);
            }
        }
        leaf => {
            flat.insert(prefix.to_string(), leaf.clone());
        }
    }
}

/// A nested object pressed into one flat object whose keys are dotted paths -
/// {"a":{"b":1}} becomes {"a.b":1}, and a list contributes numbered segments
/// like "items.0". The keys it makes are the paths the json_get functions
/// read, which is what makes a flat form worth having.
pub fn flatten(text: String) -> Result<String, String> {
    let found = document("json_flatten", &text)?;
    if !found.is_object() {
        return Err("json_flatten: the document is not a JSON object, so there is nothing to flatten".to_string());
    }
    let mut flat = serde_json::Map::new();
    pour_flat("", &found, &mut flat);
    return write_back("json_flatten", &serde_json::Value::Object(flat));
}

/// The top-level field names of an object, sorted - for looking over an
/// answer whose shape nobody wrote down.
pub fn keys(text: String) -> Result<Vec<String>, String> {
    let found = document("json_keys", &text)?;
    return match found {
        serde_json::Value::Object(fields) => {
            let mut names: Vec<String> = fields.keys().cloned().collect();
            names.sort();
            Ok(names)
        }
        _ => Err("json_keys: the document is not a JSON object, so it has no keys".to_string()),
    };
}

/// The document with one field set. The value argument is itself JSON text,
/// so `"hi"` sets a piece of text and 5 sets a number. Objects along the path
/// that do not exist yet are created; walking through a plain value is an
/// error, since there is nothing inside it to step into.
pub fn set(text: String, path: String, value_text: String) -> Result<String, String> {
    let mut found = document("json_set", &text)?;
    let value: serde_json::Value = serde_json::from_str(&value_text).map_err(|e| format!("json_set: the value is not valid JSON: {}", e))?;
    if path.is_empty() {
        return Err("json_set: the path is empty, so there is nowhere to put the value".to_string());
    }

    let segments: Vec<&str> = path.split('.').collect();
    let mut current = &mut found;
    for segment in &segments[..segments.len() - 1] {
        current = match current {
            serde_json::Value::Object(fields) => fields.entry(segment.to_string()).or_insert_with(|| serde_json::Value::Object(serde_json::Map::new())),
            serde_json::Value::Array(items) => {
                let count = items.len();
                let index: usize = segment.parse().map_err(|_| format!("json_set: '{}' is a list, and '{}' is not a position in it", path, segment))?;
                items.get_mut(index).ok_or_else(|| format!("json_set: '{}' has no item {} - the list holds {}", path, index, count))?
            }
            _ => return Err(format!("json_set: '{}' is a value, so it has nothing called '{}' inside it", path, segment)),
        };
    }

    let last = segments[segments.len() - 1];
    match current {
        serde_json::Value::Object(fields) => {
            fields.insert(last.to_string(), value);
        }
        serde_json::Value::Array(items) => {
            let count = items.len();
            let index: usize = last.parse().map_err(|_| format!("json_set: '{}' is a list, and '{}' is not a position in it", path, last))?;
            let slot = items.get_mut(index).ok_or_else(|| format!("json_set: '{}' has no item {} - the list holds {}", path, index, count))?;
            *slot = value;
        }
        _ => return Err(format!("json_set: '{}' is a value, so it has nothing called '{}' inside it", path, last)),
    }
    return write_back("json_set", &found);
}

/// The document with one field dropped. Removing a field that was never there
/// is fine - the caller wanted it gone, and gone it is.
pub fn remove(text: String, path: String) -> Result<String, String> {
    let mut found = document("json_remove", &text)?;
    if path.is_empty() {
        return Err("json_remove: the path is empty, so there is nothing to remove".to_string());
    }

    let segments: Vec<&str> = path.split('.').collect();
    let mut current = &mut found;
    for segment in &segments[..segments.len() - 1] {
        let next = match current {
            serde_json::Value::Object(fields) => fields.get_mut(*segment),
            serde_json::Value::Array(items) => match segment.parse::<usize>() {
                Ok(index) => items.get_mut(index),
                Err(_) => None,
            },
            _ => None,
        };
        current = match next {
            Some(inner) => inner,
            // The path never reaches a field, so there is nothing to drop.
            None => return write_back("json_remove", &found),
        };
    }

    let last = segments[segments.len() - 1];
    match current {
        serde_json::Value::Object(fields) => {
            fields.remove(last);
        }
        serde_json::Value::Array(items) => {
            if let Ok(index) = last.parse::<usize>() {
                if index < items.len() {
                    items.remove(index);
                }
            }
        }
        _ => {}
    }
    return write_back("json_remove", &found);
}

/// What kind of value sits at a dotted path - `object`, `array`, `string`,
/// `number`, `boolean` or `null`. An empty path asks about the whole
/// document. For deciding how to read a field that is sometimes one thing
/// and sometimes another.
pub fn type_of(text: String, path: String) -> Result<String, String> {
    let found = document("json_type_of", &text)?;
    let value = walk("json_type_of", &found, &path)?;
    let name = match value {
        serde_json::Value::Object(_) => "object",
        serde_json::Value::Array(_) => "array",
        serde_json::Value::String(_) => "string",
        serde_json::Value::Number(_) => "number",
        serde_json::Value::Bool(_) => "boolean",
        serde_json::Value::Null => "null",
    };
    return Ok(name.to_string());
}

/// The list of strings at a dotted path - `json_get_array_strings(body,
/// "user.tags")`. Every item must be a piece of text; a list that mixes in
/// numbers or objects is an error naming the first item that is not.
pub fn get_array_strings(text: String, path: String) -> Result<Vec<String>, String> {
    let found = document("json_get_array_strings", &text)?;
    let value = walk("json_get_array_strings", &found, &path)?;
    let items = match value {
        serde_json::Value::Array(items) => items,
        _ => return Err(format!("json_get_array_strings: '{}' is not a list", path)),
    };
    let mut strings = Vec::with_capacity(items.len());
    for (index, item) in items.iter().enumerate() {
        match item {
            serde_json::Value::String(written) => strings.push(written.clone()),
            _ => return Err(format!("json_get_array_strings: item {} of '{}' is not a piece of text", index, path)),
        }
    }
    return Ok(strings);
}

/// The list at a dotted path, for the four get_array_ functions to read item
/// by item. Kept apart so all four give the same message for a path that is
/// not a list at all.
fn list_at(function: &str, text: &str, path: &str) -> Result<Vec<serde_json::Value>, String> {
    let found = document(function, text)?;
    let value = walk(function, &found, path)?;
    return match value {
        // Cloned because `walk` borrows from `found`, which ends with this call.
        serde_json::Value::Array(items) => Ok(items.clone()),
        _ => Err(format!("{}: '{}' is not a list", function, path)),
    };
}

/// The list of whole numbers at a dotted path. Each item follows the same rule
/// as json_get_int: a number written as text is read, and a fraction is an
/// error rather than a silent rounding.
pub fn get_array_ints(text: String, path: String) -> Result<Vec<i64>, String> {
    let items = list_at("json_get_array_ints", &text, &path)?;
    let mut numbers = Vec::with_capacity(items.len());
    for (index, item) in items.iter().enumerate() {
        let read = match item {
            serde_json::Value::Number(number) => number.as_i64(),
            serde_json::Value::String(written) => written.parse::<i64>().ok(),
            _ => None,
        };
        match read {
            Some(number) => numbers.push(number),
            None => return Err(format!("json_get_array_ints: item {} of '{}' is not a whole number", index, path)),
        }
    }
    return Ok(numbers);
}

/// The list of numbers at a dotted path, whole or fractional. Each item
/// follows the same rule as json_get_float.
pub fn get_array_floats(text: String, path: String) -> Result<Vec<f64>, String> {
    let items = list_at("json_get_array_floats", &text, &path)?;
    let mut numbers = Vec::with_capacity(items.len());
    for (index, item) in items.iter().enumerate() {
        let read = match item {
            serde_json::Value::Number(number) => number.as_f64(),
            serde_json::Value::String(written) => written.parse::<f64>().ok(),
            _ => None,
        };
        match read {
            Some(number) => numbers.push(number),
            None => return Err(format!("json_get_array_floats: item {} of '{}' is not a number", index, path)),
        }
    }
    return Ok(numbers);
}

/// The list of true-or-false values at a dotted path. Each item follows the
/// same rule as json_get_bool.
pub fn get_array_bools(text: String, path: String) -> Result<Vec<bool>, String> {
    let items = list_at("json_get_array_bools", &text, &path)?;
    let mut flags = Vec::with_capacity(items.len());
    for (index, item) in items.iter().enumerate() {
        match item {
            serde_json::Value::Bool(flag) => flags.push(*flag),
            serde_json::Value::String(written) if written == "true" => flags.push(true),
            serde_json::Value::String(written) if written == "false" => flags.push(false),
            _ => return Err(format!("json_get_array_bools: item {} of '{}' is not true or false", index, path)),
        }
    }
    return Ok(flags);
}

/// Whether two pieces of JSON say the same thing, however they are written -
/// spacing, indentation and the order of an object's fields do not count.
/// Text that does not parse as JSON is equal to nothing.
pub fn equal(first: String, second: String) -> bool {
    let first_document: serde_json::Value = match serde_json::from_str(&first) {
        Ok(found) => found,
        Err(_) => return false,
    };
    let second_document: serde_json::Value = match serde_json::from_str(&second) {
        Ok(found) => found,
        Err(_) => return false,
    };
    return first_document == second_document;
}

/// How many entries the object or list at a dotted path holds - fields for an
/// object, items for a list. An empty path asks about the whole document.
pub fn count(text: String, path: String) -> Result<i64, String> {
    let found = document("json_count", &text)?;
    let value = walk("json_count", &found, &path)?;
    return match value {
        serde_json::Value::Object(fields) => Ok(fields.len() as i64),
        serde_json::Value::Array(items) => Ok(items.len() as i64),
        _ => Err(format!("json_count: '{}' is not an object or a list, so it has nothing to count", path)),
    };
}

#[cfg(test)]
mod reading_tests {
    use super::*;

    const ANSWER: &str = r#"{
        "user": {"name": "Ada", "age": 36, "score": 9.5, "active": true, "nickname": null},
        "items": [{"name": "first"}, {"name": "second"}],
        "count": "12",
        "flagged": "false"
    }"#;

    #[test]
    fn a_dotted_path_reaches_a_field() {
        assert_eq!(get_string(ANSWER.to_string(), "user.name".to_string()).expect("a field"), "Ada");
        assert_eq!(get_int(ANSWER.to_string(), "user.age".to_string()).expect("a field"), 36);
        assert_eq!(get_float(ANSWER.to_string(), "user.score".to_string()).expect("a field"), 9.5);
        assert!(get_bool(ANSWER.to_string(), "user.active".to_string()).expect("a field"));
    }

    #[test]
    fn a_number_in_the_path_indexes_a_list() {
        assert_eq!(get_string(ANSWER.to_string(), "items.0.name".to_string()).expect("a field"), "first");
        assert_eq!(get_string(ANSWER.to_string(), "items.1.name".to_string()).expect("a field"), "second");
        assert_eq!(array_length(ANSWER.to_string(), "items".to_string()).expect("a list"), 2);
        assert_eq!(array_length("[1, 2, 3]".to_string(), "".to_string()).expect("a list"), 3);
    }

    #[test]
    fn a_value_written_as_text_still_reads_as_what_it_is() {
        assert_eq!(get_int(ANSWER.to_string(), "count".to_string()).expect("a field"), 12);
        assert!(!get_bool(ANSWER.to_string(), "flagged".to_string()).expect("a field"));
        // And a number asked for as text comes back as it was written.
        assert_eq!(get_string(ANSWER.to_string(), "user.age".to_string()).expect("a field"), "36");
    }

    #[test]
    fn what_is_missing_says_so_rather_than_guessing() {
        assert!(get_string(ANSWER.to_string(), "user.email".to_string()).unwrap_err().contains("no field 'email'"));
        assert!(get_string(ANSWER.to_string(), "user.nickname".to_string()).unwrap_err().contains("is null"));
        assert!(get_int(ANSWER.to_string(), "user.name".to_string()).unwrap_err().contains("not a whole number"));
        assert!(get_int(ANSWER.to_string(), "user.score".to_string()).unwrap_err().contains("not a whole number"));
        assert!(get_string(ANSWER.to_string(), "items.9.name".to_string()).unwrap_err().contains("no item 9"));
        assert!(get_string(ANSWER.to_string(), "user".to_string()).unwrap_err().contains("not a piece of text"));
        assert!(array_length(ANSWER.to_string(), "user".to_string()).unwrap_err().contains("not a list"));
        assert!(get_string("not json".to_string(), "a".to_string()).unwrap_err().contains("not valid JSON"));
    }

    #[test]
    fn asking_whether_something_is_there_never_fails() {
        assert!(has(ANSWER.to_string(), "user.name".to_string()));
        assert!(has(ANSWER.to_string(), "items.1".to_string()));
        assert!(!has(ANSWER.to_string(), "user.email".to_string()));
        assert!(!has(ANSWER.to_string(), "user.nickname".to_string()));
        assert!(!has("not json".to_string(), "user".to_string()));
    }

    #[test]
    fn the_same_document_can_be_written_wide_or_narrow() {
        let narrow = compact(ANSWER.to_string()).expect("valid JSON");
        assert!(!narrow.contains('\n'));
        let wide = pretty(narrow.clone()).expect("valid JSON");
        assert!(wide.contains('\n'));
        assert_eq!(compact(wide).expect("valid JSON"), narrow);
        assert!(pretty("{".to_string()).unwrap_err().contains("not valid JSON"));
    }
}

#[cfg(test)]
mod shaping_tests {
    use super::*;

    /// Whether two pieces of JSON text say the same thing, for checking a
    /// function's answer without caring how it was formatted.
    fn same(actual: &str, expected: &str) -> bool {
        return equal(actual.to_string(), expected.to_string());
    }

    #[test]
    fn a_merge_lets_the_overlay_win_without_forgetting_the_base() {
        let base = r#"{"name": "Ada", "settings": {"theme": "light", "wide": true}, "tags": ["old"]}"#;
        let overlay = r#"{"settings": {"theme": "dark"}, "tags": ["new"], "age": 36}"#;
        let merged = merge(base.to_string(), overlay.to_string()).expect("two objects");
        // A nested object merges field by field, so `wide` survives...
        assert!(same(&merged, r#"{"name": "Ada", "age": 36, "settings": {"theme": "dark", "wide": true}, "tags": ["new"]}"#));
        // ...while a list is replaced whole, not spliced.
        assert_eq!(get_string(merged.clone(), "tags.0".to_string()).expect("a field"), "new");
        assert_eq!(array_length(merged, "tags".to_string()).expect("a list"), 1);
    }

    #[test]
    fn a_merge_wants_two_objects_and_says_so() {
        assert!(merge("[1]".to_string(), "{}".to_string()).unwrap_err().contains("base is not a JSON object"));
        assert!(merge("{}".to_string(), "5".to_string()).unwrap_err().contains("overlay is not a JSON object"));
        assert!(merge("not json".to_string(), "{}".to_string()).unwrap_err().contains("not valid JSON"));
    }

    #[test]
    fn flattening_names_every_leaf_by_its_dotted_path() {
        let nested = r#"{"a": {"b": 1, "c": {"d": "deep"}}, "items": [{"name": "first"}, 2], "plain": true}"#;
        let flat = flatten(nested.to_string()).expect("an object");
        assert!(same(&flat, r#"{"a.b": 1, "a.c.d": "deep", "items.0.name": "first", "items.1": 2, "plain": true}"#));
        // The keys it makes are paths the json_get functions can read.
        assert_eq!(get_string(nested.to_string(), "a.c.d".to_string()).expect("a field"), "deep");
        assert!(flatten("[1, 2]".to_string()).unwrap_err().contains("not a JSON object"));
        assert!(flatten("not json".to_string()).unwrap_err().contains("not valid JSON"));
    }

    #[test]
    fn the_keys_of_an_object_come_back_sorted() {
        let found = keys(r#"{"zebra": 1, "apple": 2, "mango": 3}"#.to_string()).expect("an object");
        assert_eq!(found, vec!["apple".to_string(), "mango".to_string(), "zebra".to_string()]);
        assert_eq!(keys("{}".to_string()).expect("an object"), Vec::<String>::new());
        assert!(keys("[1]".to_string()).unwrap_err().contains("has no keys"));
    }

    #[test]
    fn setting_a_deep_field_builds_the_objects_on_the_way() {
        let built = set("{}".to_string(), "a.b.c".to_string(), "5".to_string()).expect("a fresh path");
        assert!(same(&built, r#"{"a": {"b": {"c": 5}}}"#));
        // The value argument is itself JSON, so a quoted value sets text...
        let named = set(built, "a.b.name".to_string(), "\"Ada\"".to_string()).expect("a sibling");
        assert_eq!(get_string(named.clone(), "a.b.name".to_string()).expect("a field"), "Ada");
        // ...and an existing field is simply replaced.
        let replaced = set(named, "a.b.c".to_string(), "[1, 2]".to_string()).expect("a rewrite");
        assert_eq!(array_length(replaced, "a.b.c".to_string()).expect("a list"), 2);
        // A number in the path still indexes a list, as it does everywhere else.
        let listed = set(r#"{"items": [1, 2]}"#.to_string(), "items.1".to_string(), "9".to_string()).expect("a slot");
        assert_eq!(get_int(listed, "items.1".to_string()).expect("a field"), 9);
    }

    #[test]
    fn setting_refuses_what_it_cannot_walk() {
        assert!(set(r#"{"a": 5}"#.to_string(), "a.b".to_string(), "1".to_string()).unwrap_err().contains("nothing called 'b' inside it"));
        assert!(set("{}".to_string(), "a".to_string(), "not json".to_string()).unwrap_err().contains("value is not valid JSON"));
        assert!(set("{}".to_string(), "".to_string(), "1".to_string()).unwrap_err().contains("path is empty"));
        assert!(set("[1]".to_string(), "x".to_string(), "1".to_string()).unwrap_err().contains("not a position"));
    }

    #[test]
    fn removing_drops_a_field_and_forgives_a_missing_one() {
        let trimmed = remove(r#"{"keep": 1, "drop": {"deep": 2}}"#.to_string(), "drop.deep".to_string()).expect("a field to drop");
        assert!(same(&trimmed, r#"{"keep": 1, "drop": {}}"#));
        // What was never there is already gone, which is what the caller wanted.
        let unchanged = remove(r#"{"keep": 1}"#.to_string(), "ghost.deep".to_string()).expect("nothing to drop");
        assert!(same(&unchanged, r#"{"keep": 1}"#));
        // Dropping a list item closes the gap.
        let shorter = remove("[1, 2, 3]".to_string(), "1".to_string()).expect("an item to drop");
        assert!(same(&shorter, "[1, 3]"));
        assert!(remove("{}".to_string(), "".to_string()).unwrap_err().contains("path is empty"));
        assert!(remove("not json".to_string(), "a".to_string()).unwrap_err().contains("not valid JSON"));
    }

    #[test]
    fn every_kind_of_value_gives_its_name() {
        let sampler = r#"{"o": {}, "a": [], "s": "hi", "n": 1.5, "b": false, "z": null}"#;
        assert_eq!(type_of(sampler.to_string(), "o".to_string()).expect("a field"), "object");
        assert_eq!(type_of(sampler.to_string(), "a".to_string()).expect("a field"), "array");
        assert_eq!(type_of(sampler.to_string(), "s".to_string()).expect("a field"), "string");
        assert_eq!(type_of(sampler.to_string(), "n".to_string()).expect("a field"), "number");
        assert_eq!(type_of(sampler.to_string(), "b".to_string()).expect("a field"), "boolean");
        assert_eq!(type_of(sampler.to_string(), "z".to_string()).expect("a field"), "null");
        // An empty path asks about the whole document.
        assert_eq!(type_of(sampler.to_string(), "".to_string()).expect("the root"), "object");
        assert!(type_of(sampler.to_string(), "ghost".to_string()).unwrap_err().contains("no field 'ghost'"));
    }

    #[test]
    fn a_list_of_text_comes_back_whole_and_a_mixed_one_does_not() {
        let body = r#"{"user": {"tags": ["red", "green"]}, "mixed": ["ok", 5], "count": 3}"#;
        assert_eq!(get_array_strings(body.to_string(), "user.tags".to_string()).expect("a list of text"), vec!["red".to_string(), "green".to_string()]);
        assert_eq!(get_array_strings("[]".to_string(), "".to_string()).expect("an empty list"), Vec::<String>::new());
        assert!(get_array_strings(body.to_string(), "mixed".to_string()).unwrap_err().contains("item 1 of 'mixed' is not a piece of text"));
        assert!(get_array_strings(body.to_string(), "count".to_string()).unwrap_err().contains("not a list"));
    }

    #[test]
    fn a_list_of_numbers_reads_the_way_the_single_number_getters_do() {
        let body = r#"{"ids": [1, 2, 3], "written": ["4", "5"], "prices": [1.5, 2], "mixed": [1, "x"], "fraction": [1.5], "count": 3}"#;
        assert_eq!(get_array_ints(body.to_string(), "ids".to_string()).expect("whole numbers"), vec![1i64, 2, 3]);
        assert_eq!(get_array_ints(body.to_string(), "written".to_string()).expect("numbers written as text"), vec![4i64, 5]);
        assert_eq!(get_array_floats(body.to_string(), "prices".to_string()).expect("numbers"), vec![1.5f64, 2.0]);
        assert_eq!(get_array_ints("[]".to_string(), "".to_string()).expect("an empty list"), Vec::<i64>::new());

        assert!(get_array_ints(body.to_string(), "fraction".to_string()).unwrap_err().contains("item 0 of 'fraction' is not a whole number"));
        assert!(get_array_ints(body.to_string(), "mixed".to_string()).unwrap_err().contains("item 1 of 'mixed' is not a whole number"));
        assert!(get_array_floats(body.to_string(), "mixed".to_string()).unwrap_err().contains("item 1 of 'mixed' is not a number"));
        assert!(get_array_ints(body.to_string(), "count".to_string()).unwrap_err().contains("not a list"));
        assert!(get_array_floats(body.to_string(), "count".to_string()).unwrap_err().contains("not a list"));
    }

    #[test]
    fn a_list_of_flags_reads_the_way_the_single_flag_getter_does() {
        let body = r#"{"flags": [true, false], "written": ["true", "false"], "mixed": [true, 1], "count": 3}"#;
        assert_eq!(get_array_bools(body.to_string(), "flags".to_string()).expect("flags"), vec![true, false]);
        assert_eq!(get_array_bools(body.to_string(), "written".to_string()).expect("flags written as text"), vec![true, false]);
        assert_eq!(get_array_bools("[]".to_string(), "".to_string()).expect("an empty list"), Vec::<bool>::new());
        assert!(get_array_bools(body.to_string(), "mixed".to_string()).unwrap_err().contains("item 1 of 'mixed' is not true or false"));
        assert!(get_array_bools(body.to_string(), "count".to_string()).unwrap_err().contains("not a list"));
    }

    #[test]
    fn equality_ignores_formatting_and_the_order_of_fields() {
        assert!(equal(r#"{"a": 1, "b": [1, 2]}"#.to_string(), "{\"b\":[1,2],\n  \"a\": 1}".to_string()));
        assert!(!equal(r#"{"a": 1}"#.to_string(), r#"{"a": 2}"#.to_string()));
        // A list's order is meaning, so it still counts.
        assert!(!equal("[1, 2]".to_string(), "[2, 1]".to_string()));
        assert!(!equal("not json".to_string(), "{}".to_string()));
        assert!(!equal("{}".to_string(), "not json".to_string()));
    }

    #[test]
    fn counting_covers_objects_and_lists_alike() {
        let body = r#"{"user": {"name": "Ada", "age": 36}, "items": [1, 2, 3]}"#;
        assert_eq!(count(body.to_string(), "user".to_string()).expect("an object"), 2);
        assert_eq!(count(body.to_string(), "items".to_string()).expect("a list"), 3);
        assert_eq!(count(body.to_string(), "".to_string()).expect("the root"), 2);
        assert!(count(body.to_string(), "user.name".to_string()).unwrap_err().contains("nothing to count"));
        assert!(count(body.to_string(), "ghost".to_string()).unwrap_err().contains("no field 'ghost'"));
    }
}

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

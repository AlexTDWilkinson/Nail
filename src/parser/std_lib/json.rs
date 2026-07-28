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


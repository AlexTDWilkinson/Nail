//! TOML, the format configuration files are written in.
//!
//! The same shape as the JSON module: a Nail struct goes in, text comes out,
//! and text goes in with the struct type on the left of the assignment saying
//! what to read it as. TOML rather than JSON because a person edits a config
//! file - it has comments, it does not mind a trailing comma it never had, and
//! it does not make anyone count closing braces.

use serde::{Deserialize, Serialize};

/// Writes a value out as TOML.
///
/// TOML puts every table after the plain values that share its parent, which
/// is why a struct holding both simple fields and nested structs comes back
/// with the simple ones first. That is the format's rule, not a choice made
/// here, and a document written any other way is not valid TOML.
pub fn toml_serialize<T: Serialize>(value: T) -> Result<String, String> {
    return ::toml::to_string_pretty(&value).map_err(|e| format!("toml_serialize: only structs, hashmaps and arrays of them can be written as TOML: {}", e));
}

/// Reads TOML back into a value. The type on the left of the assignment says
/// what to read it as, and a document that does not match that type is an
/// error naming the field that did not fit.
pub fn toml_deserialize<T: for<'de> Deserialize<'de>>(toml_string: String) -> Result<T, String> {
    return ::toml::from_str(&toml_string).map_err(|e| {
        let detail = e.to_string();
        if detail.contains("missing field") {
            format!("toml_deserialize: {}. Every field of the target struct must be present in the document.", detail)
        } else if detail.contains("unknown field") {
            format!("toml_deserialize: {}. The document has a key the target struct does not.", detail)
        } else if detail.contains("invalid type") {
            format!("toml_deserialize: {}. A value in the document is not the type the target struct expects.", detail)
        } else {
            format!("toml_deserialize: could not read the document: {}", detail)
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, PartialEq, Serialize, Deserialize)]
    struct Server {
        host: String,
        port: i64,
        debug: bool,
        allowed: Vec<String>,
    }

    fn example() -> Server {
        return Server { host: "127.0.0.1".to_string(), port: 8080, debug: false, allowed: vec!["/".to_string(), "/health".to_string()] };
    }

    #[test]
    fn a_value_round_trips_through_the_text() {
        let written = toml_serialize(&example()).expect("a writable struct");
        let read: Server = toml_deserialize(written).expect("what we just wrote");
        assert_eq!(read, example());
    }

    #[test]
    fn the_text_is_what_a_person_would_have_typed() {
        let written = toml_serialize(&example()).expect("a writable struct");
        assert!(written.contains("host = \"127.0.0.1\""), "got: {}", written);
        assert!(written.contains("port = 8080"), "got: {}", written);
        assert!(written.contains("debug = false"), "got: {}", written);
    }

    #[test]
    fn comments_and_spacing_in_the_document_are_ignored() {
        let document = r#"
            # which interface to bind
            host = "0.0.0.0"

            port   = 9000    # the port
            debug = true
            allowed = ["/"]
        "#;
        let read: Server = toml_deserialize(document.to_string()).expect("a valid document");
        assert_eq!(read.host, "0.0.0.0");
        assert_eq!(read.port, 9000);
        assert!(read.debug);
    }

    #[test]
    fn a_missing_field_names_the_field() {
        let document = "host = \"0.0.0.0\"\nport = 9000\ndebug = true\n";
        let failure = toml_deserialize::<Server>(document.to_string()).unwrap_err();
        assert!(failure.contains("missing field"), "got: {}", failure);
        assert!(failure.contains("allowed"), "got: {}", failure);
    }

    #[test]
    fn a_value_of_the_wrong_type_is_an_error() {
        let document = "host = \"0.0.0.0\"\nport = \"not a number\"\ndebug = true\nallowed = []\n";
        let failure = toml_deserialize::<Server>(document.to_string()).unwrap_err();
        assert!(failure.contains("toml_deserialize"), "got: {}", failure);
    }

    #[test]
    fn text_that_is_not_toml_at_all_is_an_error() {
        let failure = toml_deserialize::<Server>("{\"host\": \"0.0.0.0\"}".to_string()).unwrap_err();
        assert!(failure.contains("could not read the document"), "got: {}", failure);
    }
}

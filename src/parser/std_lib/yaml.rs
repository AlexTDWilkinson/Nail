//! YAML, the format the rest of the world writes configuration in.
//!
//! Nail's own preference is TOML - it has one way to write anything, and no
//! rule anybody has to memorise about when `no` means false. But a program that
//! has to read a CI file, a Kubernetes manifest or a Docker Compose file does
//! not get to choose the format, and hand-parsing YAML is not something anyone
//! should be doing.
//!
//! Same shape as the JSON and TOML modules: a struct goes in and text comes
//! out, text goes in and the type on the left of the assignment says what to
//! read it as.

use serde::{Deserialize, Serialize};

/// Writes a value out as YAML.
pub fn yaml_serialize<T: Serialize>(value: T) -> Result<String, String> {
    return serde_yaml::to_string(&value).map_err(|failure| format!("yaml_serialize: only structs, hashmaps and arrays of them can be written as YAML: {}", failure));
}

/// Reads YAML back into a value. A document that does not match the target type
/// is an error naming the field that did not fit.
pub fn yaml_deserialize<T: for<'de> Deserialize<'de>>(yaml_string: String) -> Result<T, String> {
    return serde_yaml::from_str(&yaml_string).map_err(|failure| {
        let detail = failure.to_string();
        if detail.contains("missing field") {
            format!("yaml_deserialize: {}. Every field of the target struct must be present in the document.", detail)
        } else if detail.contains("unknown field") {
            format!("yaml_deserialize: {}. The document has a key the target struct does not.", detail)
        } else if detail.contains("invalid type") {
            format!("yaml_deserialize: {}. A value in the document is not the type the target struct expects.", detail)
        } else {
            format!("yaml_deserialize: could not read the document: {}", detail)
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, PartialEq, Serialize, Deserialize)]
    struct Job {
        name: String,
        runs_on: String,
        timeout_minutes: i64,
        steps: Vec<String>,
    }

    fn example() -> Job {
        return Job { name: "build".to_string(), runs_on: "ubuntu-latest".to_string(), timeout_minutes: 30, steps: vec!["checkout".to_string(), "test".to_string()] };
    }

    #[test]
    fn a_value_round_trips_through_the_text() {
        let written = yaml_serialize(&example()).expect("a writable struct");
        let read: Job = yaml_deserialize(written).expect("what we just wrote");
        assert_eq!(read, example());
    }

    #[test]
    fn the_text_is_what_a_person_would_have_typed() {
        let written = yaml_serialize(&example()).expect("a writable struct");
        assert!(written.contains("name: build"), "got: {}", written);
        assert!(written.contains("timeout_minutes: 30"), "got: {}", written);
        assert!(written.contains("- checkout"), "got: {}", written);
    }

    #[test]
    fn indentation_and_comments_in_the_document_are_read_as_meant() {
        let document = "
            # which runner
            name: build
            runs_on: ubuntu-latest
            timeout_minutes: 30
            steps:
              - checkout
              - test
        ";
        let read: Job = yaml_deserialize(document.to_string()).expect("a valid document");
        assert_eq!(read, example());
    }

    #[test]
    fn the_flow_spelling_reads_the_same_as_the_block_one() {
        let document = "{name: build, runs_on: ubuntu-latest, timeout_minutes: 30, steps: [checkout, test]}";
        let read: Job = yaml_deserialize(document.to_string()).expect("a valid document");
        assert_eq!(read, example());
    }

    #[test]
    fn a_missing_field_names_the_field() {
        let document = "name: build\nruns_on: ubuntu-latest\ntimeout_minutes: 30\n";
        let failure = yaml_deserialize::<Job>(document.to_string()).unwrap_err();
        assert!(failure.contains("missing field"), "got: {}", failure);
        assert!(failure.contains("steps"), "got: {}", failure);
    }

    #[test]
    fn a_value_of_the_wrong_type_is_an_error() {
        let document = "name: build\nruns_on: ubuntu-latest\ntimeout_minutes: soon\nsteps: []\n";
        let failure = yaml_deserialize::<Job>(document.to_string()).unwrap_err();
        assert!(failure.contains("yaml_deserialize"), "got: {}", failure);
    }
}

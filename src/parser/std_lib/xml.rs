//! XML, for the documents that still arrive in it.
//!
//! Nothing new should be written in XML, and this module does not disagree -
//! but sitemaps, invoices, and half of everything enterprise still travel as
//! it. Same shape as the JSON, TOML and YAML modules: a struct out, a struct
//! back in, with the type on the left of the assignment saying what to read.
//!
//! Struct fields become child elements of the same name. Attributes in a
//! document being read map to fields when quick-xml can see how; a document
//! that leans heavily on attributes is better read with `html_select_*`, whose
//! selectors do not care.

use serde::{Deserialize, Serialize};

/// Writes a value out as XML, with the struct's own name unavailable at this
/// layer - the root element is named by the caller, since Nail values do not
/// carry their type's name at runtime.
pub fn xml_serialize<T: Serialize>(value: T, root_name: String) -> Result<String, String> {
    if root_name.trim().is_empty() {
        return Err("xml_serialize: the root element needs a name".to_string());
    }
    let mut out = String::new();
    let serializer = quick_xml::se::Serializer::with_root(&mut out, Some(root_name.trim())).map_err(|failure| format!("xml_serialize: `{}` is not a usable element name: {}", root_name, failure))?;
    value.serialize(serializer).map_err(|failure| format!("xml_serialize: only structs, hashmaps and arrays of them can be written as XML: {}", failure))?;
    return Ok(out);
}

/// Reads XML back into a value. A document that does not match the target type
/// is an error naming what did not fit.
pub fn xml_deserialize<T: for<'de> Deserialize<'de>>(xml_string: String) -> Result<T, String> {
    return quick_xml::de::from_str(&xml_string).map_err(|failure| {
        let detail = failure.to_string();
        if detail.contains("missing field") {
            format!("xml_deserialize: {}. Every field of the target struct must be present in the document.", detail)
        } else {
            format!("xml_deserialize: could not read the document: {}", detail)
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, PartialEq, Serialize, Deserialize)]
    struct Invoice {
        number: String,
        total_cents: i64,
        paid: bool,
    }

    fn example() -> Invoice {
        return Invoice { number: "INV-7".to_string(), total_cents: 12345, paid: false };
    }

    #[test]
    fn a_value_round_trips_through_the_text() {
        let written = xml_serialize(&example(), "invoice".to_string()).expect("a writable struct");
        assert!(written.contains("<invoice>"), "got: {}", written);
        assert!(written.contains("<number>INV-7</number>"), "got: {}", written);
        let read: Invoice = xml_deserialize(written).expect("what we just wrote");
        assert_eq!(read, example());
    }

    #[test]
    fn a_hand_written_document_reads_the_same() {
        let document = "<invoice><number>INV-9</number><total_cents>500</total_cents><paid>true</paid></invoice>";
        let read: Invoice = xml_deserialize(document.to_string()).expect("a valid document");
        assert_eq!(read.number, "INV-9");
        assert!(read.paid);
    }

    #[test]
    fn a_missing_field_names_the_field() {
        let failure = xml_deserialize::<Invoice>("<invoice><number>INV-9</number></invoice>".to_string()).unwrap_err();
        assert!(failure.contains("missing field"), "got: {}", failure);
    }

    #[test]
    fn text_that_is_not_xml_says_so() {
        assert!(xml_deserialize::<Invoice>("{\"number\": \"INV-9\"}".to_string()).is_err());
    }

    #[test]
    fn the_root_element_needs_a_name() {
        assert!(xml_serialize(&example(), "  ".to_string()).is_err());
    }
}

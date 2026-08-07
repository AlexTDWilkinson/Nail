//! Reading a yes or no out of a value, the way int_from reads a number.
//!
//! The module is called boolean rather than bool so the file name is not a
//! Rust primitive. Its one function is `bool_from` in Nail.

use std::fmt::Display;

/// The words a value may be written in when it means yes, and when it means
/// no. Case does not matter and surrounding space is ignored. These are the
/// spellings configuration files, environment variables and command lines
/// actually use, and nothing outside them is guessed at.
const TRUE_WORDS: &[&str] = &["true", "yes", "y", "on", "1"];
const FALSE_WORDS: &[&str] = &["false", "no", "n", "off", "0"];

/// Converts a value to true or false. A bool is already the answer, a number
/// is 1 or 0 and nothing else, and text is one of the words above. Anything
/// else is an error rather than a guess: a setting that reads "maybe" should
/// say so, not quietly become false.
pub fn from<T: Display>(v: T) -> Result<bool, String> {
    let written = v.to_string();
    let word = written.trim().to_lowercase();
    if TRUE_WORDS.contains(&word.as_str()) {
        return Ok(true);
    }
    if FALSE_WORDS.contains(&word.as_str()) {
        return Ok(false);
    }
    return Err(format!("bool_from: '{}' is neither true nor false. Write one of true, yes, y, on, 1 or false, no, n, off, 0", written));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_written_words_for_yes_and_no_are_read_either_case() {
        for yes in ["true", "TRUE", "True", "yes", "Y", "on", "1", " true "] {
            assert!(from(yes).unwrap_or_else(|_| panic!("'{}' means yes", yes)));
        }
        for no in ["false", "FALSE", "no", "N", "off", "0", " false "] {
            assert!(!from(no).unwrap_or_else(|_| panic!("'{}' means no", no)));
        }
    }

    #[test]
    fn a_bool_and_a_number_come_through_as_themselves() {
        assert!(from(true).expect("a bool"));
        assert!(!from(false).expect("a bool"));
        assert!(from(1i64).expect("one"));
        assert!(!from(0i64).expect("zero"));
    }

    #[test]
    fn anything_that_is_not_a_yes_or_a_no_is_an_error() {
        assert!(from("maybe").unwrap_err().contains("neither true nor false"));
        assert!(from("").unwrap_err().contains("neither true nor false"));
        assert!(from(2i64).unwrap_err().contains("neither true nor false"), "a number other than 1 or 0 is not a yes or a no");
        assert!(from(-1i64).unwrap_err().contains("neither true nor false"));
    }
}

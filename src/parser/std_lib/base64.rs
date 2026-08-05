//! Base64 text encoding.
//!
//! Two alphabets are exposed because the wire formats that use base64 disagree
//! about which one they mean. The standard alphabet (`+`, `/`, `=` padding) is
//! what Basic auth headers, data URIs and MIME bodies carry. The URL-safe
//! alphabet (`-`, `_`, no padding) is what JWTs, OAuth tokens and anything
//! travelling inside a URL or filename carry.
//!
//! Decoding accepts padding either way, so a JWT segment that arrived unpadded
//! and a MIME blob that arrived padded both work without the caller having to
//! patch up the string first.

use base64::alphabet;
use base64::engine::general_purpose::{GeneralPurpose, GeneralPurposeConfig, NO_PAD, STANDARD};
use base64::engine::DecodePaddingMode;
use base64::Engine;

/// Decoders ignore whether the input was padded; encoders still emit the
/// padding their format expects.
const INDIFFERENT: GeneralPurposeConfig = GeneralPurposeConfig::new().with_decode_padding_mode(DecodePaddingMode::Indifferent);

const STANDARD_DECODER: GeneralPurpose = GeneralPurpose::new(&alphabet::STANDARD, INDIFFERENT);
const URL_SAFE_DECODER: GeneralPurpose = GeneralPurpose::new(&alphabet::URL_SAFE, INDIFFERENT);

/// The decoded bytes have to become a Nail string, so they have to be text.
/// The error names the function because the caller sees only the message.
fn to_text(bytes: Vec<u8>, function: &str) -> Result<String, String> {
    return String::from_utf8(bytes).map_err(|e| format!("{}: the decoded bytes are not valid UTF-8: {}", function, e));
}

/// Encode text with the standard alphabet, padded.
pub fn encode(text: String) -> String {
    return STANDARD.encode(text.as_bytes());
}

/// Decode standard-alphabet base64 back to text.
pub fn decode(data: String) -> Result<String, String> {
    let bytes = STANDARD_DECODER.decode(data.as_bytes()).map_err(|e| format!("base64_decode: the input is not valid base64: {}", e))?;
    return to_text(bytes, "base64_decode");
}

/// Encode text with the URL-safe alphabet and no padding, which is what JWT
/// segments and OAuth parameters use.
pub fn encode_url(text: String) -> String {
    return GeneralPurpose::new(&alphabet::URL_SAFE, NO_PAD).encode(text.as_bytes());
}

/// Encode raw bytes with the URL-safe alphabet and no padding. For the stdlib
/// functions that carry bytes rather than text - a nonce and a ciphertext, say
/// - through a string.
pub fn encode_url_bytes(bytes: &[u8]) -> String {
    return GeneralPurpose::new(&alphabet::URL_SAFE, NO_PAD).encode(bytes);
}

/// Decode URL-safe base64 back to raw bytes, padded or not.
pub fn decode_url_bytes(data: &str) -> Result<Vec<u8>, String> {
    return URL_SAFE_DECODER.decode(data.as_bytes()).map_err(|e| format!("base64_decode_url: the input is not valid URL-safe base64: {}", e));
}

/// Decode URL-safe base64 back to text, padded or not.
pub fn decode_url(data: String) -> Result<String, String> {
    let bytes = URL_SAFE_DECODER.decode(data.as_bytes()).map_err(|e| format!("base64_decode_url: the input is not valid URL-safe base64: {}", e))?;
    return to_text(bytes, "base64_decode_url");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_round_trips_through_the_standard_alphabet() {
        assert_eq!(encode("hello".to_string()), "aGVsbG8=");
        assert_eq!(decode("aGVsbG8=".to_string()).expect("valid base64"), "hello");
        assert_eq!(decode(encode("The quick brown fox.".to_string())).expect("its own output"), "The quick brown fox.");
    }

    #[test]
    fn text_round_trips_through_the_url_safe_alphabet() {
        assert_eq!(encode_url("hello".to_string()), "aGVsbG8");
        assert_eq!(decode_url("aGVsbG8".to_string()).expect("valid base64"), "hello");
        assert_eq!(decode_url(encode_url("a JWT payload".to_string())).expect("its own output"), "a JWT payload");
    }

    /// The two alphabets disagree on the 62nd and 63rd symbols, which is the
    /// whole reason both are exposed: `>>>` hits symbol 62 and `???` hits 63.
    #[test]
    fn the_alphabets_differ_where_they_are_supposed_to() {
        assert_eq!(encode(">>>".to_string()), "Pj4+");
        assert_eq!(encode_url(">>>".to_string()), "Pj4-");
        assert_eq!(encode("???".to_string()), "Pz8/");
        assert_eq!(encode_url("???".to_string()), "Pz8_");
        assert_eq!(decode_url("Pj4-".to_string()).expect("valid URL-safe base64"), ">>>");
        assert_eq!(decode_url("Pz8_".to_string()).expect("valid URL-safe base64"), "???");
    }

    /// The byte 0xfb encodes to `-w` in the URL-safe alphabet where the
    /// standard one writes `+w==` - the difference a JWT carries.
    #[test]
    fn raw_bytes_round_trip_through_the_url_safe_alphabet() {
        assert_eq!(encode_url_bytes(&[0xfb]), "-w");
        assert_eq!(STANDARD.encode([0xfb]), "+w==");
        assert_eq!(decode_url_bytes("-w").expect("valid URL-safe base64"), vec![0xfb]);
    }

    #[test]
    fn decoders_forgive_padding_either_way() {
        assert_eq!(decode("aGVsbG8".to_string()).expect("unpadded standard base64"), "hello");
        assert_eq!(decode_url("aGVsbG8=".to_string()).expect("padded URL-safe base64"), "hello");
        assert_eq!(decode_url_bytes("-w==").expect("padded URL-safe base64"), vec![0xfb]);
    }

    #[test]
    fn each_decoder_rejects_the_other_alphabet_and_names_itself() {
        assert!(decode_url("Pj4+".to_string()).unwrap_err().contains("base64_decode_url"));
        assert!(decode("Pj4-".to_string()).unwrap_err().contains("base64_decode"));
        assert!(decode("not base64!".to_string()).unwrap_err().contains("not valid base64"));
    }
}

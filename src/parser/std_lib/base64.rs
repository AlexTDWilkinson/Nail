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

/// Decode URL-safe base64 back to text, padded or not.
pub fn decode_url(data: String) -> Result<String, String> {
    let bytes = URL_SAFE_DECODER.decode(data.as_bytes()).map_err(|e| format!("base64_decode_url: the input is not valid URL-safe base64: {}", e))?;
    return to_text(bytes, "base64_decode_url");
}

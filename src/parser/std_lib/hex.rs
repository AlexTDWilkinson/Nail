//! Hex (base16) text encoding.
//!
//! The companion to base64 for the places that spell bytes in hex instead:
//! digests, HMAC signatures, binary keys pasted into a config file. The
//! encoding is deliberately dull - two lower-case characters per byte, no
//! separators, no prefix.

/// Shared by `crypto_hash_*` and `crypto_hmac_*`, which hold bytes rather than
/// text and need the same spelling of them.
pub(crate) fn encode_bytes(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push_str(&format!("{:02x}", byte));
    }
    return out;
}

/// One hex character to its value.
fn digit_value(character: char) -> Option<u8> {
    return character.to_digit(16).map(|value| value as u8);
}

/// Encode text as hex, two characters per byte of UTF-8.
pub fn encode(text: String) -> String {
    return encode_bytes(text.as_bytes());
}

/// Decode hex back to text. Upper and lower case both decode; anything else,
/// including an odd number of characters, is an error rather than a guess.
pub fn decode(data: String) -> Result<String, String> {
    let characters: Vec<char> = data.chars().collect();
    if characters.len() % 2 != 0 {
        return Err(format!("hex_decode: the input has {} characters, and hex needs an even number", characters.len()));
    }

    let mut bytes = Vec::with_capacity(characters.len() / 2);
    let mut index = 0;
    while index < characters.len() {
        let high = digit_value(characters[index]);
        let low = digit_value(characters[index + 1]);
        match (high, low) {
            (Some(high), Some(low)) => bytes.push(high * 16 + low),
            _ => return Err(format!("hex_decode: '{}{}' at character {} is not a hex byte", characters[index], characters[index + 1], index)),
        }
        index += 2;
    }

    return String::from_utf8(bytes).map_err(|e| format!("hex_decode: the decoded bytes are not valid UTF-8: {}", e));
}

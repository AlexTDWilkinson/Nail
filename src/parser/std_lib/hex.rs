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

/// Hex to raw bytes for the functions that work on bytes rather than text.
/// The label names which argument was bad, so the error reads like a person
/// wrote it.
fn decode_bytes_labelled(function: &str, label: &str, data: &str) -> Result<Vec<u8>, String> {
    let characters: Vec<char> = data.chars().collect();
    if characters.len() % 2 != 0 {
        return Err(format!("{}: the {} has {} characters, and hex needs an even number", function, label, characters.len()));
    }

    let mut bytes = Vec::with_capacity(characters.len() / 2);
    let mut index = 0;
    while index < characters.len() {
        match (digit_value(characters[index]), digit_value(characters[index + 1])) {
            (Some(high), Some(low)) => bytes.push(high * 16 + low),
            _ => return Err(format!("{}: '{}{}' at character {} of the {} is not a hex byte", function, characters[index], characters[index + 1], index, label)),
        }
        index += 2;
    }
    return Ok(bytes);
}

/// Byte-wise xor of two equal-length hex strings, the building block under
/// one-time pads, IV masking and every crypto exercise. Errors name a length
/// mismatch or bad hex rather than guessing at either.
pub fn xor(first: String, second: String) -> Result<String, String> {
    let first_bytes = decode_bytes_labelled("hex_xor", "first input", &first)?;
    let second_bytes = decode_bytes_labelled("hex_xor", "second input", &second)?;
    if first_bytes.len() != second_bytes.len() {
        return Err(format!("hex_xor: the first input is {} bytes and the second is {} bytes, and xor needs the same length", first_bytes.len(), second_bytes.len()));
    }
    let combined: Vec<u8> = first_bytes.iter().zip(second_bytes.iter()).map(|(left, right)| left ^ right).collect();
    return Ok(encode_bytes(&combined));
}

/// The classic inspection layout for a person looking at bytes: an offset
/// column, 16 bytes of hex per line with a gap after the eighth, and an ASCII
/// gutter where anything non-printable is a dot. Lines are joined with
/// newlines, and empty input gives an empty string.
pub fn dump(hex: String) -> Result<String, String> {
    let bytes = decode_bytes_labelled("hex_dump", "input", &hex)?;
    let mut lines: Vec<String> = Vec::new();
    for (line_number, chunk) in bytes.chunks(16).enumerate() {
        let mut line = format!("{:08x}  ", line_number * 16);
        for slot in 0..16 {
            if slot == 8 {
                line.push(' ');
            }
            match chunk.get(slot) {
                Some(byte) => line.push_str(&format!("{:02x} ", byte)),
                None => line.push_str("   "),
            }
        }
        line.push(' ');
        line.push('|');
        for byte in chunk {
            if (0x20..0x7f).contains(byte) {
                line.push(*byte as char);
            } else {
                line.push('.');
            }
        }
        line.push('|');
        lines.push(line);
    }
    return Ok(lines.join("\n"));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn xor_combines_bytes_pairwise() {
        assert_eq!(xor("ff00".to_string(), "0f0f".to_string()).expect("equal lengths"), "f00f");
        assert_eq!(xor("deadbeef".to_string(), "deadbeef".to_string()).expect("equal lengths"), "00000000");
        assert_eq!(xor("".to_string(), "".to_string()).expect("equal lengths"), "");
    }

    #[test]
    fn xor_names_what_went_wrong() {
        assert!(xor("ff00".to_string(), "0f".to_string()).unwrap_err().contains("same length"));
        assert!(xor("ff0".to_string(), "0f0f".to_string()).unwrap_err().contains("even number"));
        assert!(xor("zzzz".to_string(), "0f0f".to_string()).unwrap_err().contains("not a hex byte"));
        assert!(xor("ff00".to_string(), "zz00".to_string()).unwrap_err().contains("second input"));
    }

    #[test]
    fn a_dump_lines_up_hex_and_ascii() {
        let expected = format!("00000000  68 65 6c 6c 6f{}|hello|", " ".repeat(36));
        assert_eq!(dump("68656c6c6f".to_string()).expect("valid hex"), expected);
    }

    #[test]
    fn a_full_line_holds_sixteen_bytes_with_a_gap_after_eight() {
        // Sixteen bytes of "Hello, world!" plus three non-printable, then one
        // more byte to start a second line at the next offset.
        let listing = dump("48656c6c6f2c20776f726c642100010241".to_string()).expect("valid hex");
        let first = "00000000  48 65 6c 6c 6f 2c 20 77  6f 72 6c 64 21 00 01 02  |Hello, world!...|";
        let second = format!("00000010  41{}|A|", " ".repeat(48));
        assert_eq!(listing, format!("{}\n{}", first, second));
    }

    #[test]
    fn a_dump_of_nothing_is_empty_and_bad_hex_is_an_error() {
        assert_eq!(dump("".to_string()).expect("valid hex"), "");
        assert!(dump("f".to_string()).unwrap_err().contains("even number"));
        assert!(dump("zz".to_string()).unwrap_err().contains("not a hex byte"));
    }
}

//! Base58, the Bitcoin alphabet, for ids and keys meant to be read aloud
//! and retyped. It leaves out 0, O, I and l so no character can be mistaken
//! for another, and leading zero bytes come out as leading `1`s.

const ALPHABET: &[u8; 58] = b"123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz";

pub(crate) fn encode_bytes(bytes: &[u8]) -> String {
    let zeros = bytes.iter().take_while(|byte| **byte == 0).count();
    let mut digits: Vec<u8> = Vec::new();
    for &byte in &bytes[zeros..] {
        let mut carry = u32::from(byte);
        for digit in digits.iter_mut() {
            carry += u32::from(*digit) << 8;
            *digit = (carry % 58) as u8;
            carry /= 58;
        }
        while carry > 0 {
            digits.push((carry % 58) as u8);
            carry /= 58;
        }
    }
    let mut out = String::with_capacity(zeros + digits.len());
    for _ in 0..zeros {
        out.push('1');
    }
    for &digit in digits.iter().rev() {
        out.push(ALPHABET[usize::from(digit)] as char);
    }
    return out;
}

pub(crate) fn decode_to_bytes(text: &str, what: &str) -> Result<Vec<u8>, String> {
    let cleaned: Vec<char> = text.chars().filter(|c| !c.is_whitespace()).collect();
    let ones = cleaned.iter().take_while(|c| **c == '1').count();
    let mut bytes: Vec<u8> = Vec::new();
    for &c in &cleaned[ones..] {
        let index = ALPHABET
            .iter()
            .position(|letter| *letter as char == c)
            .ok_or_else(|| format!("{}: `{}` is not a base58 character, the alphabet leaves out 0, O, I and l", what, c))?;
        let mut carry = index as u32;
        for byte in bytes.iter_mut() {
            carry += u32::from(*byte) * 58;
            *byte = (carry & 0xff) as u8;
            carry >>= 8;
        }
        while carry > 0 {
            bytes.push((carry & 0xff) as u8);
            carry >>= 8;
        }
    }
    let mut out = vec![0u8; ones];
    out.extend(bytes.iter().rev());
    return Ok(out);
}

/// Text bytes as base58, the way Bitcoin spells binary for human eyes.
pub fn encode(text: String) -> String {
    return encode_bytes(text.as_bytes());
}

/// Base58 back to text. Whitespace is forgiven. A character outside the
/// alphabet is named, as are decoded bytes that make no text.
pub fn decode(text: String) -> Result<String, String> {
    let bytes = decode_to_bytes(&text, "base58_decode")?;
    return String::from_utf8(bytes).map_err(|_| "base58_decode: the decoded bytes are not text".to_string());
}

/// Hex bytes as base58, how a binary id becomes something short enough to
/// read aloud. Leading zero bytes come out as leading `1`s.
pub fn encode_hex(hex: String) -> Result<String, String> {
    let cleaned: String = hex.chars().filter(|c| !c.is_whitespace()).collect();
    if let Some(bad) = cleaned.chars().find(|c| !c.is_ascii_hexdigit()) {
        return Err(format!("base58_encode_hex: `{}` is not a hex digit", bad));
    }
    if cleaned.len() % 2 != 0 {
        return Err(format!("base58_encode_hex: hex text needs an even number of digits, this has {}", cleaned.len()));
    }
    let bytes: Vec<u8> = (0..cleaned.len()).step_by(2).map(|i| u8::from_str_radix(&cleaned[i..i + 2], 16).expect("every digit was checked just above")).collect();
    return Ok(encode_bytes(&bytes));
}

/// Base58 back to hex bytes, for ids and keys that were never text.
pub fn decode_hex(text: String) -> Result<String, String> {
    let bytes = decode_to_bytes(&text, "base58_decode_hex")?;
    return Ok(bytes.iter().map(|byte| format!("{:02x}", byte)).collect());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_bitcoin_anchors_hold() {
        assert_eq!(encode("Hello World!".to_string()), "2NEpo7TZRRrLZSi2U");
        assert_eq!(decode("2NEpo7TZRRrLZSi2U".to_string()).unwrap(), "Hello World!");
        assert_eq!(encode_hex("0000287fb4cd".to_string()).unwrap(), "11233QC4");
        assert_eq!(decode_hex("11233QC4".to_string()).unwrap(), "0000287fb4cd");
    }

    #[test]
    fn empty_text_and_whitespace_are_forgiven() {
        assert_eq!(encode("".to_string()), "");
        assert_eq!(decode("".to_string()).unwrap(), "");
        assert_eq!(decode("2NEpo7TZ RRrLZSi2U".to_string()).unwrap(), "Hello World!");
    }

    #[test]
    fn the_lookalike_absentees_are_rejected_by_name() {
        assert!(decode("0abc".to_string()).unwrap_err().contains("`0` is not a base58 character"));
        assert!(decode("hello".to_string()).unwrap_err().contains("`l` is not a base58 character"));
        assert!(decode_hex("OI".to_string()).unwrap_err().contains("`O` is not a base58 character"));
    }

    #[test]
    fn bytes_that_are_not_text_are_refused() {
        let encoded = encode_hex("ff".to_string()).unwrap();
        assert_eq!(encoded, "5Q");
        assert!(decode(encoded).unwrap_err().contains("not text"));
    }

    #[test]
    fn hex_ids_round_trip() {
        assert_eq!(decode_hex(encode_hex("00deadbeef00".to_string()).unwrap()).unwrap(), "00deadbeef00");
    }

    #[test]
    fn bad_hex_is_named() {
        assert!(encode_hex("abc".to_string()).unwrap_err().contains("even number"));
        assert!(encode_hex("zz".to_string()).unwrap_err().contains("`z` is not a hex digit"));
    }
}

//! Base32 encoding, RFC 4648 - the alphabet authenticator secrets use.

const ALPHABET: &[u8; 32] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ234567";

pub(crate) fn encode_bytes(bytes: &[u8]) -> String {
    let mut out = String::new();
    for chunk in bytes.chunks(5) {
        let mut buffer = [0u8; 5];
        buffer[..chunk.len()].copy_from_slice(chunk);
        let value = u64::from(buffer[0]) << 32 | u64::from(buffer[1]) << 24 | u64::from(buffer[2]) << 16 | u64::from(buffer[3]) << 8 | u64::from(buffer[4]);
        let digits = match chunk.len() {
            1 => 2,
            2 => 4,
            3 => 5,
            4 => 7,
            _ => 8,
        };
        for i in 0..digits {
            let index = ((value >> (35 - i * 5)) & 0x1f) as usize;
            out.push(ALPHABET[index] as char);
        }
        for _ in digits..8 {
            out.push('=');
        }
    }
    return out;
}

pub(crate) fn decode_to_bytes(text: &str, what: &str) -> Result<Vec<u8>, String> {
    let cleaned: Vec<u8> = text.bytes().filter(|b| !b.is_ascii_whitespace() && *b != b'=').map(|b| b.to_ascii_uppercase()).collect();
    let mut out = Vec::new();
    for chunk in cleaned.chunks(8) {
        let mut value: u64 = 0;
        for b in chunk {
            let index = ALPHABET.iter().position(|a| a == b).ok_or_else(|| format!("{}: `{}` is not a base32 character", what, *b as char))?;
            value = (value << 5) | index as u64;
        }
        let bits = chunk.len() * 5;
        let bytes = bits / 8;
        if bytes == 0 {
            return Err(format!("{}: a base32 group of {} characters is cut short", what, chunk.len()));
        }
        let value = value << (40 - bits);
        let full = value.to_be_bytes();
        out.extend_from_slice(&full[3..3 + bytes]);
    }
    return Ok(out);
}

/// Text as base32, padded the way RFC 4648 asks.
pub fn encode(text: String) -> String {
    return encode_bytes(text.as_bytes());
}

/// Base32 back to text. Case and padding are forgiven; other characters are not.
pub fn decode(text: String) -> Result<String, String> {
    let bytes = decode_to_bytes(&text, "base32_decode")?;
    return String::from_utf8(bytes).map_err(|_| "base32_decode: the decoded bytes are not text".to_string());
}

/// Hex bytes as base32 - how a binary secret becomes an authenticator code.
pub fn encode_hex(hex: String) -> Result<String, String> {
    let cleaned: String = hex.chars().filter(|c| !c.is_whitespace()).collect();
    if cleaned.len() % 2 != 0 {
        return Err(format!("base32_encode_hex: hex text needs an even number of digits, this has {}", cleaned.len()));
    }
    let bytes: Result<Vec<u8>, String> = (0..cleaned.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&cleaned[i..i + 2], 16).map_err(|_| format!("base32_encode_hex: `{}` is not a hex byte", &cleaned[i..i + 2])))
        .collect();
    return Ok(encode_bytes(&bytes?));
}

/// Base32 back to hex bytes, for secrets that were never text.
pub fn decode_hex(text: String) -> Result<String, String> {
    let bytes = decode_to_bytes(&text, "base32_decode_hex")?;
    return Ok(bytes.iter().map(|b| format!("{:02x}", b)).collect());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_rfc_test_vectors_hold() {
        assert_eq!(encode("".to_string()), "");
        assert_eq!(encode("f".to_string()), "MY======");
        assert_eq!(encode("fo".to_string()), "MZXQ====");
        assert_eq!(encode("foo".to_string()), "MZXW6===");
        assert_eq!(encode("foob".to_string()), "MZXW6YQ=");
        assert_eq!(encode("fooba".to_string()), "MZXW6YTB");
        assert_eq!(encode("foobar".to_string()), "MZXW6YTBOI======");
    }

    #[test]
    fn decoding_forgives_case_and_padding_but_not_noise() {
        assert_eq!(decode("mzxw6ytboi".to_string()).unwrap(), "foobar");
        assert_eq!(decode("MZXW6YTBOI======".to_string()).unwrap(), "foobar");
        assert!(decode("MZ1W6===".to_string()).unwrap_err().contains("not a base32 character"));
    }

    #[test]
    fn hex_secrets_round_trip() {
        let encoded = encode_hex("48656c6c6f21deadbeef".to_string()).unwrap();
        assert_eq!(decode_hex(encoded).unwrap(), "48656c6c6f21deadbeef");
    }
}

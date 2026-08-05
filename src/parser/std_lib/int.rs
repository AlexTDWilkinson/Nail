// Integer conversion and utility functions
use std::fmt::Display;

// Convert value to integer
pub fn from<T: Display>(v: T) -> Result<i64, String> {
    v.to_string().parse::<i64>().map_err(|_| format!("int_from: could not parse '{}' as an integer", v))
}

// Integer power with overflow and negative-exponent checks
pub fn pow(base: i64, exp: i64) -> Result<i64, String> {
    if exp < 0 {
        return Err(format!("int_pow: exponent cannot be negative, got {}", exp));
    }
    let exp = u32::try_from(exp).map_err(|_| format!("int_pow: exponent {} is too large", exp))?;
    base.checked_pow(exp).ok_or_else(|| format!("int_pow: {}^{} overflows a 64-bit integer", base, exp))
}

/// Reads a hexadecimal number, with or without the `0x` in front, and with or
/// without a leading `-`. The form colour codes, byte dumps and file formats
/// are written in.
pub fn from_hex(text: String) -> Result<i64, String> {
    return from_radix(text, 16);
}

/// Reads a number written in any base from 2 to 36, where the digits above 9
/// are the letters, upper or lower case. Base 2 for bit patterns, base 16 for
/// hex, base 36 for short ids.
pub fn from_radix(text: String, radix: i64) -> Result<i64, String> {
    if !(2..=36).contains(&radix) {
        return Err(format!("int_from_radix: the base must be between 2 and 36, got {}", radix));
    }

    let trimmed = text.trim();
    let (sign, digits) = match trimmed.strip_prefix('-') {
        Some(rest) => (-1i64, rest),
        None => (1i64, trimmed.strip_prefix('+').unwrap_or(trimmed)),
    };
    let digits = match radix {
        16 => digits.strip_prefix("0x").or_else(|| digits.strip_prefix("0X")).unwrap_or(digits),
        2 => digits.strip_prefix("0b").or_else(|| digits.strip_prefix("0B")).unwrap_or(digits),
        8 => digits.strip_prefix("0o").or_else(|| digits.strip_prefix("0O")).unwrap_or(digits),
        _ => digits,
    };
    if digits.is_empty() {
        return Err(format!("int_from_radix: '{}' has no digits in it", text));
    }

    let magnitude = i64::from_str_radix(digits, radix as u32).map_err(|_| format!("int_from_radix: '{}' is not a base {} number that fits in a 64-bit integer", text, radix))?;
    return Ok(sign * magnitude);
}

/// Writes a number in any base from 2 to 36, using lower-case letters for the
/// digits above 9. A negative number keeps its `-`.
pub fn to_radix(value: i64, radix: i64) -> Result<String, String> {
    if !(2..=36).contains(&radix) {
        return Err(format!("int_to_radix: the base must be between 2 and 36, got {}", radix));
    }
    if value == 0 {
        return Ok("0".to_string());
    }

    const DIGITS: &[u8] = b"0123456789abcdefghijklmnopqrstuvwxyz";
    // Counted down in the negative half, since -9223372036854775808 has no
    // positive counterpart to work with.
    let mut remaining = if value < 0 { value } else { -value };
    let mut written = Vec::new();
    while remaining != 0 {
        let digit = (-(remaining % radix)) as usize;
        written.push(DIGITS[digit]);
        remaining /= radix;
    }
    if value < 0 {
        written.push(b'-');
    }
    written.reverse();
    return Ok(String::from_utf8(written).unwrap_or_default());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_is_read_with_or_without_the_prefix() {
        assert_eq!(from_hex("ff".to_string()).expect("hex"), 255);
        assert_eq!(from_hex("0xFF".to_string()).expect("hex"), 255);
        assert_eq!(from_hex("-0x10".to_string()).expect("hex"), -16);
    }

    #[test]
    fn other_bases_read_their_own_digits() {
        assert_eq!(from_radix("1011".to_string(), 2).expect("binary"), 11);
        assert_eq!(from_radix("0b1011".to_string(), 2).expect("binary"), 11);
        assert_eq!(from_radix("777".to_string(), 8).expect("octal"), 511);
        assert_eq!(from_radix("zz".to_string(), 36).expect("base 36"), 1295);
    }

    #[test]
    fn a_bad_base_or_bad_digits_are_errors() {
        assert!(from_radix("10".to_string(), 1).unwrap_err().contains("between 2 and 36"));
        assert!(from_radix("12".to_string(), 2).unwrap_err().contains("not a base 2 number"));
        assert!(from_radix("".to_string(), 16).unwrap_err().contains("no digits"));
        assert!(to_radix(10, 40).unwrap_err().contains("between 2 and 36"));
    }

    #[test]
    fn writing_and_reading_a_base_round_trips() {
        assert_eq!(to_radix(255, 16).expect("hex"), "ff");
        assert_eq!(to_radix(0, 2).expect("binary"), "0");
        assert_eq!(to_radix(-16, 16).expect("hex"), "-10");
        assert_eq!(to_radix(i64::MIN, 16).expect("hex"), "-8000000000000000");
        for value in [-1000i64, -7, 0, 1, 42, 123456789, i64::MAX] {
            for radix in [2i64, 8, 10, 16, 36] {
                let written = to_radix(value, radix).expect("a base in range");
                assert_eq!(from_radix(written, radix).expect("what we just wrote"), value);
            }
        }
    }
}

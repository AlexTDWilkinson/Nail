//! Bit-level operations on whole numbers.
//!
//! Nail's integers are 64-bit and signed, and every function here works on
//! that pattern of 64 bits directly. Shifts and rotations count from bit 0 as
//! the lowest bit. Anything that would shift or index past bit 63 is an error
//! rather than a silently different answer, because the machine's own
//! behaviour there varies and quietly wrong is the worst kind of wrong.

/// Shared guard for the functions that name a single bit.
fn check_index(function: &str, index: i64) -> Result<(), String> {
    if !(0..64).contains(&index) {
        return Err(format!("{}: bit {} is outside the 0 to 63 a whole number has", function, index));
    }
    return Ok(());
}

pub fn and(left: i64, right: i64) -> i64 {
    return left & right;
}

pub fn or(left: i64, right: i64) -> i64 {
    return left | right;
}

pub fn xor(left: i64, right: i64) -> i64 {
    return left ^ right;
}

pub fn not(value: i64) -> i64 {
    return !value;
}

/// Shift left, filling with zeros. Bits shifted past the top are discarded.
pub fn shift_left(value: i64, places: i64) -> Result<i64, String> {
    check_index("bits_shift_left", places)?;
    return Ok(((value as u64) << places as u32) as i64);
}

/// Shift right, filling with zeros rather than copies of the sign bit, so the
/// answer is the bit pattern moved over and nothing more.
pub fn shift_right(value: i64, places: i64) -> Result<i64, String> {
    check_index("bits_shift_right", places)?;
    return Ok(((value as u64) >> places as u32) as i64);
}

/// Rotate left: bits that fall off the top come back at the bottom.
pub fn rotate_left(value: i64, places: i64) -> Result<i64, String> {
    check_index("bits_rotate_left", places)?;
    return Ok((value as u64).rotate_left(places as u32) as i64);
}

/// Rotate right: bits that fall off the bottom come back at the top.
pub fn rotate_right(value: i64, places: i64) -> Result<i64, String> {
    check_index("bits_rotate_right", places)?;
    return Ok((value as u64).rotate_right(places as u32) as i64);
}

/// How many bits are set. The population count, which is what tells you the
/// size of a set held as a bitmask.
pub fn count_ones(value: i64) -> i64 {
    return value.count_ones() as i64;
}

pub fn count_zeros(value: i64) -> i64 {
    return value.count_zeros() as i64;
}

/// How many zero bits sit above the highest set bit. 64 for zero itself.
pub fn leading_zeros(value: i64) -> i64 {
    return value.leading_zeros() as i64;
}

/// How many zero bits sit below the lowest set bit. 64 for zero itself.
pub fn trailing_zeros(value: i64) -> i64 {
    return value.trailing_zeros() as i64;
}

/// Whether one particular bit is set, counting from 0 at the lowest.
pub fn get(value: i64, index: i64) -> Result<bool, String> {
    check_index("bits_get", index)?;
    return Ok((value as u64) >> index as u32 & 1 == 1);
}

/// The number with one particular bit turned on or off.
pub fn set(value: i64, index: i64, on: bool) -> Result<i64, String> {
    check_index("bits_set", index)?;
    let mask = 1u64 << index as u32;
    if on {
        return Ok((value as u64 | mask) as i64);
    }
    return Ok((value as u64 & !mask) as i64);
}

/// The bit pattern written out, most significant bit first, with no leading
/// zeros - so 5 is "101" and 0 is "0".
pub fn to_binary(value: i64) -> String {
    return format!("{:b}", value as u64);
}

/// Reads a string of ones and zeros back into a number. Underscores are
/// allowed as separators, because 64 characters in a row are unreadable.
pub fn from_binary(text: String) -> Result<i64, String> {
    let digits: String = text.chars().filter(|character| *character != '_').collect();
    if digits.is_empty() {
        return Err("bits_from_binary: the text has no digits in it".to_string());
    }
    if digits.len() > 64 {
        return Err(format!("bits_from_binary: {} digits is more than the 64 a whole number holds", digits.len()));
    }
    if let Some(bad) = digits.chars().find(|character| *character != '0' && *character != '1') {
        return Err(format!("bits_from_binary: '{}' is not a binary digit", bad));
    }
    return u64::from_str_radix(&digits, 2).map(|value| value as i64).map_err(|e| format!("bits_from_binary: could not read '{}': {}", text, e));
}

/// The bit pattern in hex, most significant digit first, with no leading
/// zeros. The companion to `bits_to_binary` when 64 bits is too many to read.
pub fn to_hex(value: i64) -> String {
    return format!("{:x}", value as u64);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_boolean_operations_combine_patterns() {
        assert_eq!(and(0b1100, 0b1010), 0b1000);
        assert_eq!(or(0b1100, 0b1010), 0b1110);
        assert_eq!(xor(0b1100, 0b1010), 0b0110);
        assert_eq!(not(0), -1);
    }

    #[test]
    fn shifts_move_the_pattern() {
        assert_eq!(shift_left(1, 4).expect("in range"), 16);
        assert_eq!(shift_right(16, 4).expect("in range"), 1);
    }

    #[test]
    fn shifting_right_fills_with_zeros_even_for_negatives() {
        // -1 is all ones; shifting right by 63 leaves exactly one bit.
        assert_eq!(shift_right(-1, 63).expect("in range"), 1);
    }

    #[test]
    fn rotations_bring_bits_back_round() {
        assert_eq!(rotate_left(1, 64 - 1).expect("in range"), i64::MIN);
        assert_eq!(rotate_right(1, 1).expect("in range"), i64::MIN);
        assert_eq!(rotate_left(0b1011, 0).expect("in range"), 0b1011);
    }

    #[test]
    fn shifting_past_the_top_is_an_error() {
        assert!(shift_left(1, 64).unwrap_err().contains("outside the 0 to 63"));
        assert!(shift_right(1, -1).unwrap_err().contains("outside the 0 to 63"));
        assert!(rotate_left(1, 100).unwrap_err().contains("outside the 0 to 63"));
    }

    #[test]
    fn counting_bits() {
        assert_eq!(count_ones(0b1011), 3);
        assert_eq!(count_zeros(0), 64);
        assert_eq!(leading_zeros(0), 64);
        assert_eq!(leading_zeros(1), 63);
        assert_eq!(trailing_zeros(0), 64);
        assert_eq!(trailing_zeros(0b1000), 3);
        assert_eq!(count_ones(-1), 64);
    }

    #[test]
    fn single_bits_can_be_read_and_written() {
        assert!(get(0b0100, 2).expect("in range"));
        assert!(!get(0b0100, 1).expect("in range"));
        assert_eq!(set(0, 3, true).expect("in range"), 8);
        assert_eq!(set(0b1111, 0, false).expect("in range"), 0b1110);
        assert_eq!(set(0, 63, true).expect("in range"), i64::MIN);
    }

    #[test]
    fn a_bit_outside_the_word_is_an_error() {
        assert!(get(0, 64).unwrap_err().contains("outside the 0 to 63"));
        assert!(set(0, -1, true).unwrap_err().contains("outside the 0 to 63"));
    }

    #[test]
    fn binary_text_round_trips() {
        assert_eq!(to_binary(5), "101");
        assert_eq!(to_binary(0), "0");
        assert_eq!(from_binary("101".to_string()).expect("binary digits"), 5);
        assert_eq!(from_binary("1010_1010".to_string()).expect("binary digits"), 170);
        assert_eq!(from_binary(to_binary(-1)).expect("binary digits"), -1);
    }

    #[test]
    fn binary_text_rejects_what_is_not_binary() {
        assert!(from_binary("".to_string()).unwrap_err().contains("no digits"));
        assert!(from_binary("102".to_string()).unwrap_err().contains("not a binary digit"));
        assert!(from_binary("1".repeat(65)).unwrap_err().contains("more than the 64"));
    }

    #[test]
    fn hex_writes_the_same_pattern() {
        assert_eq!(to_hex(255), "ff");
        assert_eq!(to_hex(-1), "ffffffffffffffff");
    }
}

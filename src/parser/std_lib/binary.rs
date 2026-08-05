//! Packing and unpacking fixed-width numbers in hex strings.
//!
//! Nail carries binary data as hex text, so these functions are how a program
//! reads a file header or builds a wire packet: unpack numbers out of the hex,
//! pack them back in, and slice or join the pieces in between.

fn hex_to_bytes(hex: &str, what: &str) -> Result<Vec<u8>, String> {
    let cleaned: String = hex.chars().filter(|c| !c.is_whitespace()).collect();
    if cleaned.len() % 2 != 0 {
        return Err(format!("{}: hex text needs an even number of digits, this has {}", what, cleaned.len()));
    }
    return (0..cleaned.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&cleaned[i..i + 2], 16).map_err(|_| format!("{}: `{}` is not a hex byte", what, &cleaned[i..i + 2])))
        .collect();
}

fn bytes_to_hex(bytes: &[u8]) -> String {
    return bytes.iter().map(|b| format!("{:02x}", b)).collect();
}

fn check_width(byte_count: i64, what: &str) -> Result<usize, String> {
    return match byte_count {
        1 | 2 | 4 | 8 => Ok(byte_count as usize),
        _ => Err(format!("{}: a number is 1, 2, 4 or 8 bytes wide, not {}", what, byte_count)),
    };
}

fn take_at<'a>(bytes: &'a [u8], offset: i64, width: usize, what: &str) -> Result<&'a [u8], String> {
    if offset < 0 {
        return Err(format!("{}: the offset can not be negative", what));
    }
    let start = offset as usize;
    if start + width > bytes.len() {
        return Err(format!("{}: wanted {} bytes at offset {}, but there are only {} bytes", what, width, offset, bytes.len()));
    }
    return Ok(&bytes[start..start + width]);
}

/// Pack an integer into its hex bytes: 1, 2, 4 or 8 of them, either endian.
pub fn pack_int(value: i64, byte_count: i64, big_endian: bool) -> Result<String, String> {
    let width = check_width(byte_count, "binary_pack_int")?;
    if width < 8 {
        let bits = width as u32 * 8;
        let lowest = -(1i64 << (bits - 1));
        let highest = (1i64 << bits) - 1;
        if value < lowest || value > highest {
            return Err(format!("binary_pack_int: {} does not fit in {} bytes", value, byte_count));
        }
    }
    let full = value.to_le_bytes();
    let mut bytes: Vec<u8> = full[..width].to_vec();
    if big_endian {
        bytes.reverse();
    }
    return Ok(bytes_to_hex(&bytes));
}

/// Read an integer out of hex bytes at an offset. Signed reads sign-extend;
/// unsigned reads of the full 8 bytes must still fit in Nail's integer.
pub fn unpack_int(hex: String, offset: i64, byte_count: i64, big_endian: bool, signed: bool) -> Result<i64, String> {
    let width = check_width(byte_count, "binary_unpack_int")?;
    let bytes = hex_to_bytes(&hex, "binary_unpack_int")?;
    let taken = take_at(&bytes, offset, width, "binary_unpack_int")?;
    let mut raw: u64 = 0;
    if big_endian {
        for b in taken {
            raw = (raw << 8) | *b as u64;
        }
    } else {
        for b in taken.iter().rev() {
            raw = (raw << 8) | *b as u64;
        }
    }
    if signed {
        let bits = width as u32 * 8;
        if bits < 64 && raw >= 1u64 << (bits - 1) {
            return Ok(raw as i64 - (1i64 << (bits - 1)) * 2);
        }
        return Ok(raw as i64);
    }
    if raw > i64::MAX as u64 {
        return Err(format!("binary_unpack_int: {} is too large for an unsigned read", raw));
    }
    return Ok(raw as i64);
}

/// A float as its 8 hex bytes, either endian.
pub fn pack_float(value: f64, big_endian: bool) -> String {
    let bytes = if big_endian { value.to_be_bytes() } else { value.to_le_bytes() };
    return bytes_to_hex(&bytes);
}

/// Read an 8-byte float out of hex bytes at an offset.
pub fn unpack_float(hex: String, offset: i64, big_endian: bool) -> Result<f64, String> {
    let bytes = hex_to_bytes(&hex, "binary_unpack_float")?;
    let taken = take_at(&bytes, offset, 8, "binary_unpack_float")?;
    let mut fixed = [0u8; 8];
    fixed.copy_from_slice(taken);
    return Ok(if big_endian { f64::from_be_bytes(fixed) } else { f64::from_le_bytes(fixed) });
}

/// A float as its 4 hex bytes - the single-precision form binary formats use.
pub fn pack_float32(value: f64, big_endian: bool) -> String {
    let narrowed = value as f32;
    let bytes = if big_endian { narrowed.to_be_bytes() } else { narrowed.to_le_bytes() };
    return bytes_to_hex(&bytes);
}

/// Read a 4-byte float out of hex bytes at an offset.
pub fn unpack_float32(hex: String, offset: i64, big_endian: bool) -> Result<f64, String> {
    let bytes = hex_to_bytes(&hex, "binary_unpack_float32")?;
    let taken = take_at(&bytes, offset, 4, "binary_unpack_float32")?;
    let mut fixed = [0u8; 4];
    fixed.copy_from_slice(taken);
    return Ok(if big_endian { f32::from_be_bytes(fixed) as f64 } else { f32::from_le_bytes(fixed) as f64 });
}

/// How many bytes a hex string holds.
pub fn byte_length(hex: String) -> Result<i64, String> {
    return Ok(hex_to_bytes(&hex, "binary_byte_length")?.len() as i64);
}

/// A run of bytes out of the middle: offset and length are in bytes, not digits.
pub fn slice(hex: String, offset: i64, length: i64) -> Result<String, String> {
    if length < 0 {
        return Err("binary_slice: the length can not be negative".to_string());
    }
    let bytes = hex_to_bytes(&hex, "binary_slice")?;
    let taken = take_at(&bytes, offset, length as usize, "binary_slice")?;
    return Ok(bytes_to_hex(taken));
}

/// Join hex pieces into one, validating each along the way.
pub fn concat(parts: Vec<String>) -> Result<String, String> {
    let mut joined = Vec::new();
    for part in &parts {
        joined.extend(hex_to_bytes(part, "binary_concat")?);
    }
    return Ok(bytes_to_hex(&joined));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn integers_go_in_and_come_back_out() {
        let packed = pack_int(4660, 2, true).unwrap();
        assert_eq!(packed, "1234");
        assert_eq!(unpack_int(packed, 0, 2, true, false).unwrap(), 4660);
        assert_eq!(pack_int(4660, 2, false).unwrap(), "3412");
        assert_eq!(unpack_int("3412".to_string(), 0, 2, false, false).unwrap(), 4660);
    }

    #[test]
    fn signed_reads_sign_extend() {
        let packed = pack_int(-2, 2, true).unwrap();
        assert_eq!(packed, "fffe");
        assert_eq!(unpack_int(packed.clone(), 0, 2, true, true).unwrap(), -2);
        assert_eq!(unpack_int(packed, 0, 2, true, false).unwrap(), 65534);
    }

    #[test]
    fn a_value_too_wide_for_its_bytes_is_refused() {
        assert!(pack_int(70000, 2, true).unwrap_err().contains("does not fit"));
    }

    #[test]
    fn floats_survive_the_round_trip() {
        let packed = pack_float(1.5, true);
        assert_eq!(unpack_float(packed, 0, true).unwrap(), 1.5);
        let narrow = pack_float32(0.25, false);
        assert_eq!(unpack_float32(narrow, 0, false).unwrap(), 0.25);
    }

    #[test]
    fn slicing_and_joining_work_in_bytes() {
        assert_eq!(slice("00112233".to_string(), 1, 2).unwrap(), "1122");
        assert_eq!(concat(vec!["dead".to_string(), "beef".to_string()]).unwrap(), "deadbeef");
        assert_eq!(byte_length("deadbeef".to_string()).unwrap(), 4);
    }

    #[test]
    fn a_png_header_reads_like_the_spec_says() {
        // Width lives at offset 16 of a PNG file, four bytes, big-endian.
        let header = "89504e470d0a1a0a0000000d49484452000001000000008008060000007f1f66df";
        assert_eq!(unpack_int(header.to_string(), 16, 4, true, false).unwrap(), 256);
    }
}

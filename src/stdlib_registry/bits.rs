//! Bits module stdlib registry entries.

use super::*;

pub(super) fn register(m: &mut HashMap<&'static str, StdlibFunction>) {
    simple_fns! { m, Bits:
        "bits_and" => "std_lib::bits::and", (left: i, right: i) -> i,
            "Returns the bits set in both numbers.",
            "shared:i = bits_and(flags, mask);";
        "bits_or" => "std_lib::bits::or", (left: i, right: i) -> i,
            "Returns the bits set in either number.",
            "combined:i = bits_or(flags, extra);";
        "bits_xor" => "std_lib::bits::xor", (left: i, right: i) -> i,
            "Returns the bits set in one number but not the other.",
            "changed:i = bits_xor(before, after);";
        "bits_not" => "std_lib::bits::not", (value: i) -> i,
            "Returns the number with every bit flipped.",
            "inverted:i = bits_not(flags);";
        "bits_shift_left" => "std_lib::bits::shift_left", (value: i, places: i) -> (i!e),
            "Shifts the bits up, filling with zeros; errors on a shift outside 0 to 63.",
            "doubled:i = danger(bits_shift_left(value, 1));";
        "bits_shift_right" => "std_lib::bits::shift_right", (value: i, places: i) -> (i!e),
            "Shifts the bits down, filling with zeros; errors on a shift outside 0 to 63.",
            "halved:i = danger(bits_shift_right(value, 1));";
        "bits_rotate_left" => "std_lib::bits::rotate_left", (value: i, places: i) -> (i!e),
            "Shifts the bits up, with the bits that fall off the top returning at the bottom.",
            "rotated:i = danger(bits_rotate_left(value, 8));";
        "bits_rotate_right" => "std_lib::bits::rotate_right", (value: i, places: i) -> (i!e),
            "Shifts the bits down, with the bits that fall off the bottom returning at the top.",
            "rotated:i = danger(bits_rotate_right(value, 8));";
        "bits_count_ones" => "std_lib::bits::count_ones", (value: i) -> i,
            "Returns how many bits are set, which is the size of a set held as a bitmask.",
            "members:i = bits_count_ones(mask);";
        "bits_count_zeros" => "std_lib::bits::count_zeros", (value: i) -> i,
            "Returns how many bits are clear.",
            "free:i = bits_count_zeros(mask);";
        "bits_leading_zeros" => "std_lib::bits::leading_zeros", (value: i) -> i,
            "Returns how many zero bits sit above the highest set bit, and 64 for zero itself.",
            "headroom:i = bits_leading_zeros(value);";
        "bits_trailing_zeros" => "std_lib::bits::trailing_zeros", (value: i) -> i,
            "Returns how many zero bits sit below the lowest set bit, and 64 for zero itself.",
            "alignment:i = bits_trailing_zeros(address);";
        "bits_get" => "std_lib::bits::get", (value: i, index: i) -> (b!e),
            "Returns whether one particular bit is set, counting from 0 at the lowest; errors outside 0 to 63.",
            "enabled:b = danger(bits_get(flags, 3));";
        "bits_set" => "std_lib::bits::set", (value: i, index: i, on: b) -> (i!e),
            "Returns the number with one particular bit turned on or off; errors outside 0 to 63.",
            "flags:i = danger(bits_set(flags, 3, true));";
        "bits_to_binary" => "std_lib::bits::to_binary", (value: i) -> s,
            "Writes the bit pattern as ones and zeros, highest bit first, with no leading zeros.",
            "pattern:s = bits_to_binary(5);";
        "bits_from_binary" => "std_lib::bits::from_binary", (text: s) -> (i!e),
            "Reads a string of ones and zeros back into a number; underscores are allowed as separators.",
            "value:i = danger(bits_from_binary(`1010_1010`));";
        "bits_to_hex" => "std_lib::bits::to_hex", (value: i) -> s,
            "Writes the bit pattern in hex, highest digit first, with no leading zeros.",
            "pattern:s = bits_to_hex(255);";
    }
}

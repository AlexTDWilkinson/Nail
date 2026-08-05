//! Hex module stdlib registry entries.

use super::*;

pub(super) fn register(m: &mut HashMap<&'static str, StdlibFunction>) {
    simple_fns! { m, Hex:
        "hex_encode" => "std_lib::hex::encode", (text: s) -> s,
            "Encodes text as hex, two lower-case characters per byte.",
            "encoded:s = hex_encode(`hello`);";
        "hex_decode" => "std_lib::hex::decode", (data: s) -> (s!e),
            "Decodes hex back to text. Errors on non-hex characters or an odd length.",
            "plain:s = danger(hex_decode(`68656c6c6f`));";
        "hex_xor" => "std_lib::hex::xor", (first: s, second: s) -> (s!e),
            "Returns the byte-wise xor of two hex strings of equal length. Errors name a length mismatch or bad hex.",
            "masked:s = danger(hex_xor(`ff00`, `0f0f`));";
        "hex_dump" => "std_lib::hex::dump", (hex: s) -> (s!e),
            "Lays bytes out for a person: an offset column, 16 bytes of hex per line and an ASCII gutter with dots for the non-printable. Errors when the input is not hex.",
            "listing:s = danger(hex_dump(payload));";
    }
}

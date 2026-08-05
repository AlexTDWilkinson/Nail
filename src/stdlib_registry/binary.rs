//! Binary module stdlib registry entries.

use super::*;

pub(super) fn register(m: &mut HashMap<&'static str, StdlibFunction>) {
    simple_fns! { m, Binary:
        "binary_pack_int" => "std_lib::binary::pack_int", (value: i, byte_count: i, big_endian: b) -> (s!e),
            "An integer as hex bytes: 1, 2, 4 or 8 of them, big- or little-endian. Refuses a value that does not fit the width.",
            "length_field:s = danger(binary_pack_int(256, 4, true));";
        "binary_unpack_int" => "std_lib::binary::unpack_int", (hex: s, offset: i, byte_count: i, big_endian: b, signed: b) -> (i!e),
            "Reads an integer out of hex bytes at a byte offset. Signed reads sign-extend two's-complement values.",
            "width:i = danger(binary_unpack_int(png_header, 16, 4, true, false));";
        "binary_pack_float" => "std_lib::binary::pack_float", (value: f, big_endian: b) -> s,
            "A float as its 8 hex bytes, IEEE 754 double precision.",
            "packed:s = binary_pack_float(1.5, true);";
        "binary_unpack_float" => "std_lib::binary::unpack_float", (hex: s, offset: i, big_endian: b) -> (f!e),
            "Reads an 8-byte float out of hex bytes at a byte offset.",
            "reading:f = danger(binary_unpack_float(sample, 0, true));";
        "binary_pack_float32" => "std_lib::binary::pack_float32", (value: f, big_endian: b) -> s,
            "A float as its 4 hex bytes - the single-precision form binary formats mostly use.",
            "packed:s = binary_pack_float32(0.25, false);";
        "binary_unpack_float32" => "std_lib::binary::unpack_float32", (hex: s, offset: i, big_endian: b) -> (f!e),
            "Reads a 4-byte float out of hex bytes at a byte offset.",
            "reading:f = danger(binary_unpack_float32(sample, 12, false));";
        "binary_byte_length" => "std_lib::binary::byte_length", (hex: s) -> (i!e),
            "How many bytes a hex string holds - half its digit count.",
            "size:i = danger(binary_byte_length(header));";
        "binary_slice" => "std_lib::binary::slice", (hex: s, offset: i, length: i) -> (s!e),
            "A run of bytes out of the middle of hex data. Offset and length count bytes, not digits.",
            "magic:s = danger(binary_slice(header, 0, 4));";
        "binary_concat" => "std_lib::binary::concat", (parts: [s]) -> (s!e),
            "Joins hex pieces into one, checking each is real hex on the way.",
            "packet:s = danger(binary_concat([header, payload]));";
    }
}

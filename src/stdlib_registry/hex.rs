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
    }
}

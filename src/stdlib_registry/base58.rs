//! Base58 module stdlib registry entries.

use super::*;

pub(super) fn register(m: &mut HashMap<&'static str, StdlibFunction>) {
    simple_fns! { m, Base58:
        "base58_encode" => "std_lib::base58::encode", (text: s) -> s,
            "Text as base58, the Bitcoin alphabet that leaves out 0, O, I and l so nothing is misread.",
            "encoded:s = base58_encode(`hello`);";
        "base58_decode" => "std_lib::base58::decode", (text: s) -> (s!e),
            "Base58 back to text. Whitespace is forgiven. Characters outside the alphabet are not.",
            "decoded:s = danger(base58_decode(encoded));";
        "base58_encode_hex" => "std_lib::base58::encode_hex", (hex: s) -> (s!e),
            "Hex bytes as base58, how a binary id becomes something short enough to read aloud. Leading zero bytes come out as leading 1s.",
            "id:s = danger(base58_encode_hex(crypto_random_hex(16)));";
        "base58_decode_hex" => "std_lib::base58::decode_hex", (text: s) -> (s!e),
            "Base58 back to hex bytes, for ids and keys that were never text.",
            "raw:s = danger(base58_decode_hex(id));";
    }
}

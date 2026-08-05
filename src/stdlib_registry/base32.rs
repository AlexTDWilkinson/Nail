//! Base32 module stdlib registry entries.

use super::*;

pub(super) fn register(m: &mut HashMap<&'static str, StdlibFunction>) {
    simple_fns! { m, Base32:
        "base32_encode" => "std_lib::base32::encode", (text: s) -> s,
            "Text as base32, RFC 4648 - the alphabet authenticator apps and DNS records use.",
            "encoded:s = base32_encode(`hello`);";
        "base32_decode" => "std_lib::base32::decode", (text: s) -> (s!e),
            "Base32 back to text. Case and padding are forgiven; characters outside the alphabet are not.",
            "decoded:s = danger(base32_decode(encoded));";
        "base32_encode_hex" => "std_lib::base32::encode_hex", (hex: s) -> (s!e),
            "Hex bytes as base32 - how a binary secret becomes the code an authenticator app accepts.",
            "secret:s = danger(base32_encode_hex(crypto_random_hex(20)));";
        "base32_decode_hex" => "std_lib::base32::decode_hex", (text: s) -> (s!e),
            "Base32 back to hex bytes, for secrets that were never text.",
            "raw:s = danger(base32_decode_hex(secret));";
    }
}

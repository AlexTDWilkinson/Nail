//! Base64 module stdlib registry entries.

use super::*;

pub(super) fn register(m: &mut HashMap<&'static str, StdlibFunction>) {
    simple_fns! { m, Base64:
        "base64_encode" [Base64] => "std_lib::base64::encode", (text: s) -> s,
            "Encodes text as base64 with the standard alphabet and padding.",
            "encoded:s = base64_encode(`hello`);";
        "base64_decode" [Base64] => "std_lib::base64::decode", (data: s) -> (s!e),
            "Decodes standard base64 back to text; errors on invalid base64 or non-text bytes.",
            "plain:s = danger(base64_decode(`aGVsbG8=`));";
        "base64_encode_url" [Base64] => "std_lib::base64::encode_url", (text: s) -> s,
            "Encodes text as URL-safe base64 without padding, the form JWTs and URLs use.",
            "token:s = base64_encode_url(`hello`);";
        "base64_decode_url" [Base64] => "std_lib::base64::decode_url", (data: s) -> (s!e),
            "Decodes URL-safe base64, padded or not, back to text.",
            "payload:s = danger(base64_decode_url(`aGVsbG8`));";
    }
}

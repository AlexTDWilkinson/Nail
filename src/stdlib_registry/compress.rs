//! Compress module stdlib registry entries.

use super::*;

pub(super) fn register(m: &mut HashMap<&'static str, StdlibFunction>) {
    simple_fns! { m, Compress:
        "compress_gzip" [Flate2, Base64] => "std_lib::compress::gzip_compress", (data: s) -> (s!e),
            "Gzip-compresses a string and returns it base64-encoded.",
            "packed:s = danger(compress_gzip(payload));";
        "compress_gunzip" [Flate2, Base64] => "std_lib::compress::gzip_decompress", (data: s) -> (s!e),
            "Decompresses a base64-encoded gzip string back to the original text.",
            "original:s = danger(compress_gunzip(packed));";
    }
}

//! Compress module stdlib registry entries.

use super::*;

pub(super) fn register(m: &mut HashMap<&'static str, StdlibFunction>) {
    simple_fns! { m, Compress:
        "compress_gzip" [Flate2, Base64] => "std_lib::compress::gzip_compress", (data: s) -> (s!e),
            "Gzip-compresses a string and returns it base64-encoded.",
            "payload:s = `the same sentence over and over and over again`;\npacked:s = danger(compress_gzip(payload));";
        "compress_gunzip" [Flate2, Base64] => "std_lib::compress::gzip_decompress", (data: s) -> (s!e),
            "Decompresses a base64-encoded gzip string back to the original text.",
            "payload:s = `the same sentence over and over and over again`;\npacked:s = danger(compress_gzip(payload));\noriginal:s = danger(compress_gunzip(packed));";
        "compress_zstd" [Zstd, Base64] => "std_lib::compress::zstd_compress", (data: s) -> (s!e),
            "Zstd-compresses a string and returns it base64-encoded - the modern format for stored data, faster and tighter than gzip.",
            "payload:s = `the same sentence over and over and over again`;\npacked:s = danger(compress_zstd(payload));";
        "compress_unzstd" [Zstd, Base64] => "std_lib::compress::zstd_decompress", (data: s) -> (s!e),
            "Decompresses a base64-encoded zstd string back to the original text.",
            "payload:s = `the same sentence over and over and over again`;\npacked:s = danger(compress_zstd(payload));\noriginal:s = danger(compress_unzstd(packed));";
        "compress_brotli" [Brotli, Base64] => "std_lib::compress::brotli_compress", (data: s) -> (s!e),
            "Brotli-compresses a string and returns it base64-encoded - what the web's `content-encoding: br` carries.",
            "page:s = `<html><body>hello hello hello</body></html>`;\npacked:s = danger(compress_brotli(page));";
        "compress_unbrotli" [Brotli, Base64] => "std_lib::compress::brotli_decompress", (data: s) -> (s!e),
            "Decompresses a base64-encoded brotli string back to the original text.",
            "page:s = `<html><body>hello hello hello</body></html>`;\npacked:s = danger(compress_brotli(page));\noriginal:s = danger(compress_unbrotli(packed));";
    }
}

//! Crypto module stdlib registry entries.

use super::*;

pub(super) fn register(m: &mut HashMap<&'static str, StdlibFunction>) {
    simple_fns! { m, Crypto:
        "crypto_hash_sha256" [Sha2] => "std_lib::crypto::hash_sha256", (input: s) -> s,
            "Returns the SHA-256 hash of the input as a hex string.",
            "digest:s = crypto_hash_sha256(`hello`);";
        "crypto_hash_md5" [Md5] => "std_lib::crypto::hash_md5", (input: s) -> s,
            "Returns the MD5 hash of the input as a hex string (not for security-sensitive uses).",
            "digest:s = crypto_hash_md5(`hello`);";
        "crypto_uuid_v4" [Uuid] => "std_lib::crypto::uuid_v4", () -> s,
            "Generates a random version 4 UUID string.",
            "id:s = crypto_uuid_v4();";
    }
}

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
        "crypto_random_hex" [Rand] => "std_lib::crypto::random_hex", (bytes: i) -> (s!e),
            "Returns the given number of operating-system random bytes as hex. Use this, not math_random, for session ids, nonces and anything an attacker must not guess.",
            "session_id:s = danger(crypto_random_hex(16));";
        "crypto_secure_equal" => "std_lib::crypto::secure_equal", (left: s, right: s) -> b,
            "Compares two secrets in time that does not reveal how much of them matched. Use it instead of == for session ids, tokens and signatures.",
            "matches:b = crypto_secure_equal(presented_token, stored_token);";
        "crypto_hmac_sha256" [Hmac, Sha2] => "std_lib::crypto::hmac_sha256", (key: s, message: s) -> s,
            "Returns the HMAC-SHA256 of a message under a secret key, as hex. Verifies webhook signatures and signs values that pass through a browser.",
            "signature:s = crypto_hmac_sha256(secret, payload);";
    }
}

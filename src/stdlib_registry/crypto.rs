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
        "crypto_hash_sha512" [Sha2] => "std_lib::crypto::hash_sha512", (input: s) -> s,
            "Returns the SHA-512 hash of the input as a hex string.",
            "digest:s = crypto_hash_sha512(`hello`);";
        "crypto_uuid_v4" [Uuid] => "std_lib::crypto::uuid_v4", () -> s,
            "Generates a random version 4 UUID string.",
            "id:s = crypto_uuid_v4();";
        "crypto_uuid_v7" [Uuid] => "std_lib::crypto::uuid_v7", () -> s,
            "Generates a version 7 UUID: random, but with the time it was made in the leading bits, so sorting the ids sorts them by age. The one to use for a database key.",
            "id:s = crypto_uuid_v7();";
        "crypto_encrypt" [AesGcm, Sha2, Rand, Base64] => "std_lib::crypto::encrypt", (text: s, secret: s) -> (s!e),
            "Encrypts text with a secret so only somebody holding the same secret can read it back, using AES-256-GCM. The result is URL-safe base64 and is different every time, and text changed afterwards fails to decrypt rather than decrypting to something else. For data at rest - a session cookie, a stored token; passwords go through crypto_hash_password instead.",
            "sealed:s = danger(crypto_encrypt(token, danger(env_get(`SECRET_KEY`))));";
        "crypto_decrypt" [AesGcm, Sha2, Rand, Base64] => "std_lib::crypto::decrypt", (encrypted: s, secret: s) -> (s!e),
            "Reads back what crypto_encrypt wrote, with the same secret. The wrong secret, text that was tampered with, and text that was never encrypted are all errors.",
            "token:s = danger(crypto_decrypt(sealed, danger(env_get(`SECRET_KEY`))));";
        "crypto_hash_password" [Argon2] => "std_lib::crypto::hash_password", (password: s) -> (s!e),
            "Turns a password into something safe to store, using Argon2id with a fresh random salt. Never store a password with crypto_hash_sha256 - a graphics card guesses those billions of times a second.",
            "stored:s = danger(crypto_hash_password(password));";
        "crypto_verify_password" [Argon2] => "std_lib::crypto::verify_password", (password: s, stored_hash: s) -> b,
            "Checks a password against a hash from crypto_hash_password. False for a wrong password and false for a stored value that is not a hash.",
            "allowed:b = crypto_verify_password(attempt, stored);";
        "crypto_random_hex" [Rand] => "std_lib::crypto::random_hex", (bytes: i) -> (s!e),
            "Returns the given number of operating-system random bytes as hex. Use this, not math_random, for session ids, nonces and anything an attacker must not guess.",
            "session_id:s = danger(crypto_random_hex(16));";
        "crypto_secure_equal" => "std_lib::crypto::secure_equal", (left: s, right: s) -> b,
            "Compares two secrets in time that does not reveal how much of them matched. Use it instead of == for session ids, tokens and signatures.",
            "matches:b = crypto_secure_equal(presented_token, stored_token);";
        "crypto_hmac_sha256" [Hmac, Sha2] => "std_lib::crypto::hmac_sha256", (key: s, message: s) -> s,
            "Returns the HMAC-SHA256 of a message under a secret key, as hex. Verifies webhook signatures and signs values that pass through a browser.",
            "signature:s = crypto_hmac_sha256(secret, payload);";
        "crypto_ulid" [Rand] => "std_lib::crypto::ulid", () -> (s!e),
            "Generates a ULID: 26 typable characters, sorted by the time it was made, with no hyphens. The identifier to put in a URL.",
            "id:s = danger(crypto_ulid());";
        "crypto_random_id" [Rand] => "std_lib::crypto::random_id", (length: i) -> (s!e),
            "Generates a random identifier of the given length using letters, digits, hyphen and underscore, so it needs no escaping in a URL.",
            "invite_code:s = danger(crypto_random_id(12));";
        "crypto_hash_file_sha256" [Sha2, Tokio] => "std_lib::crypto::hash_file_sha256", (path: s) -> (s!e),
            "The SHA-256 of a file's contents as hex, read in blocks so the file never has to fit in memory. The checksum a download is verified against.",
            "digest:s = danger(crypto_hash_file_sha256(`release.tar.gz`));";
    }
}

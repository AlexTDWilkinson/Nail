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
        "crypto_uuid_v5" [Uuid, Sha1] => "std_lib::crypto::uuid_v5", (namespace_uuid: s, name: s) -> (s!e),
            "A version 5 UUID, RFC 4122: the SHA-1 of a namespace UUID and a name folded into UUID shape. The same namespace and name always give the same id, which is the point - it turns any stable name into a stable UUID. The well-known DNS namespace is 6ba7b810-9dad-11d1-80b4-00c04fd430c8. Errors on a namespace that is not a UUID.",
            "id:s = danger(crypto_uuid_v5(`6ba7b810-9dad-11d1-80b4-00c04fd430c8`, `www.example.com`));";
        "crypto_encrypt" [AesGcm, Sha2, Rand, Base64] => "std_lib::crypto::encrypt", (text: s, secret: s) -> (s!e),
            "Encrypts text with a secret so only somebody holding the same secret can read it back, using AES-256-GCM. The result is URL-safe base64 and is different every time, and text changed afterwards fails to decrypt rather than decrypting to something else. For data at rest - a session cookie, a stored token. Passwords go through crypto_hash_password instead.",
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
        "crypto_signing_key" [Ed25519, Rand] => "std_lib::crypto::signing_key", () -> s,
            "Makes a new Ed25519 signing key, as hex - the secret half, which must never be published. Signing is what HMAC cannot do: whoever checks a signature does not need the secret and so cannot forge one. Keep the key in a file the service user owns and read it with env_get.",
            "secret:s = crypto_signing_key();";
        "crypto_verifying_key" [Ed25519] => "std_lib::crypto::verifying_key", (signing_key: s) -> (s!e),
            "The verifying key that goes with a signing key, as hex. This is the half to publish: it goes in the program that checks signatures, or in the documentation of an API other people call.",
            "public:s = danger(crypto_verifying_key(secret));";
        "crypto_sign" [Ed25519] => "std_lib::crypto::sign", (signing_key: s, message: s) -> (s!e),
            "Signs a message with an Ed25519 signing key, as hex. The signature proves the message came from whoever holds that key and that not one character of it has changed since.",
            "signature:s = danger(crypto_sign(secret, manifest));";
        "crypto_verify_signature" [Ed25519] => "std_lib::crypto::verify_signature", (verifying_key: s, message: s, signature: s) -> (b!e),
            "Whether the signature really is this message signed by the holder of that verifying key. A key or signature of the wrong length is an error rather than a false, because that is a mistake in the program and not a message that failed its check.",
            "genuine:b = danger(crypto_verify_signature(public, manifest, signature));";
        "crypto_ulid" [Rand] => "std_lib::crypto::ulid", () -> (s!e),
            "Generates a ULID: 26 typable characters, sorted by the time it was made, with no hyphens. The identifier to put in a URL.",
            "id:s = danger(crypto_ulid());";
        "crypto_random_id" [Rand] => "std_lib::crypto::random_id", (length: i) -> (s!e),
            "Generates a random identifier of the given length using letters, digits, hyphen and underscore, so it needs no escaping in a URL.",
            "invite_code:s = danger(crypto_random_id(12));";
        "crypto_hash_file_sha256" [Sha2, Tokio] => "std_lib::crypto::hash_file_sha256", (path: s) -> (s!e),
            "The SHA-256 of a file's contents as hex, read in blocks so the file never has to fit in memory. The checksum a download is verified against.",
            "digest:s = danger(crypto_hash_file_sha256(`release.tar.gz`));";
        "crypto_crc32" [Crc32Fast] => "std_lib::crypto::crc32", (text: s) -> s,
            "The CRC32 of text as 8 hex digits - the fast checksum zip and png use. A checksum catches accidents, not tampering. For tampering use a hash.",
            "checksum:s = crypto_crc32(payload);";
        "crypto_hash_sha1" [Sha1] => "std_lib::crypto::hash_sha1", (input: s) -> s,
            "The SHA-1 of text as hex. Broken for new designs, still what git objects, OAuth 1 and older webhook signatures speak.",
            "digest:s = crypto_hash_sha1(content);";
        "crypto_hmac_sha1" [Sha1, Hmac] => "std_lib::crypto::hmac_sha1", (message: s, key: s) -> s,
            "HMAC-SHA1 of a message as hex, for the older signature schemes that still ask for it. New designs use crypto_hmac_sha256.",
            "signature:s = crypto_hmac_sha1(payload, secret);";
        "crypto_hash_blake3" [Blake3] => "std_lib::crypto::hash_blake3", (input: s) -> s,
            "The BLAKE3 of text as hex - the modern hash that is faster than the SHA family at the same strength. Good for content addressing and dedup keys.",
            "key:s = crypto_hash_blake3(document);";
        "crypto_totp_now" [Sha1, Hmac] => "std_lib::crypto::totp_now", (secret_base32: s) -> (s!e),
            "The six-digit authenticator code a base32 secret makes right now, RFC 6238 - the same one the phone app shows.",
            "code:s = danger(crypto_totp_now(user_secret));";
        "crypto_totp_at" [Sha1, Hmac] => "std_lib::crypto::totp_at", (secret_base32: s, timestamp: i) -> (s!e),
            "The code a secret made at a particular moment - what tests and audits ask for.",
            "code:s = danger(crypto_totp_at(user_secret, moment));";
        "crypto_totp_verify" [Sha1, Hmac] => "std_lib::crypto::totp_verify", (secret_base32: s, code: s) -> (b!e),
            "Whether a code someone typed is the secret's current one. One clock step of drift on either side is forgiven, since phones and servers disagree by seconds.",
            "valid:b = danger(crypto_totp_verify(user_secret, typed_code));";
        "crypto_hotp" [Sha1, Hmac] => "std_lib::crypto::hotp", (secret_base32: s, counter: i) -> (s!e),
            "The six-digit code a base32 secret makes for a counter, RFC 4226. The counter-stepped cousin of TOTP, for hardware tokens and printed back-up code lists. The counter must not be negative.",
            "code:s = danger(crypto_hotp(user_secret, 3));";
        "crypto_hash_file_blake3" [Blake3, Tokio] => "std_lib::crypto::hash_file_blake3", (path: s) -> (s!e),
            "The BLAKE3 of a file's contents as hex, read in blocks so the file never has to fit in memory. The fast fingerprint for content addressing and dedup keys.",
            "digest:s = danger(crypto_hash_file_blake3(`release.tar.gz`));";
    }
}

use hmac::{Hmac, Mac};
use md5;
use rand::RngCore;
use sha2::{Digest, Sha256};
use uuid::Uuid;

// SHA256 hash
pub fn hash_sha256(s: String) -> String {
    let mut hasher = Sha256::new();
    hasher.update(s.as_bytes());
    let result = hasher.finalize();
    format!("{:x}", result)
}

// MD5 hash (for checksums, not cryptographic security)
pub fn hash_md5(s: String) -> String {
    let digest = md5::compute(s.as_bytes());
    format!("{:x}", digest)
}

// Generate UUID v4
pub fn uuid_v4() -> String {
    Uuid::new_v4().to_string()
}

/// Random bytes from the operating system, as hex. This is the one to reach
/// for when an attacker must not be able to guess the answer: session ids,
/// OAuth state nonces, pairing codes, password reset links. `math_random` is
/// not that - it is a fast generator for simulations and shuffles, and its
/// output can be predicted from earlier output.
///
/// The count is in bytes, so the string is twice as long. 16 bytes (128 bits)
/// is the usual floor for a token nobody may guess.
pub fn random_hex(bytes: i64) -> Result<String, String> {
    if bytes < 1 {
        return Err(format!("crypto_random_hex: asked for {} bytes, which is not a usable amount of randomness", bytes));
    }
    if bytes > 1024 {
        return Err(format!("crypto_random_hex: asked for {} bytes, more than the 1024 byte limit", bytes));
    }

    let mut buffer = vec![0u8; bytes as usize];
    rand::rngs::OsRng.fill_bytes(&mut buffer);
    return Ok(super::hex::encode_bytes(&buffer));
}

/// Compare two secrets in time that does not depend on how much of them
/// matches. A normal `==` stops at the first differing byte, and the time it
/// took says how long the shared prefix was - enough, over many tries, to
/// recover a session id or a signature a byte at a time. The length of each
/// input is not hidden, only the contents.
pub fn secure_equal(left: String, right: String) -> bool {
    let left = left.as_bytes();
    let right = right.as_bytes();
    if left.len() != right.len() {
        return false;
    }

    let mut difference = 0u8;
    for index in 0..left.len() {
        difference |= left[index] ^ right[index];
    }
    return difference == 0;
}

/// HMAC-SHA256 of a message under a secret key, as hex. This is what proves a
/// message came from someone holding the key: incoming webhook signatures
/// (Stripe, GitHub, Discord) and cookies or links that must survive a round
/// trip through a browser without being edited. Compare the result with
/// `crypto_secure_equal`, never with `==`.
pub fn hmac_sha256(key: String, message: String) -> String {
    let mut mac = <Hmac<Sha256> as Mac>::new_from_slice(key.as_bytes()).expect("HMAC accepts a key of any length");
    mac.update(message.as_bytes());
    return super::hex::encode_bytes(&mac.finalize().into_bytes());
}
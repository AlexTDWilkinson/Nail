use hmac::{Hmac, Mac};
use md5;
use rand::RngCore;
use sha2::{Digest, Sha256, Sha512};
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

// SHA512 hash
pub fn hash_sha512(s: String) -> String {
    let mut hasher = Sha512::new();
    hasher.update(s.as_bytes());
    let result = hasher.finalize();
    format!("{:x}", result)
}

// Generate UUID v4
pub fn uuid_v4() -> String {
    Uuid::new_v4().to_string()
}

/// A UUID version 7: random like a v4, but with the time it was made in the
/// leading bits, so sorting the ids sorts them by age. That is what a database
/// wants from a primary key - rows written near each other in time land near
/// each other on disk, which a v4 deliberately prevents.
pub fn uuid_v7() -> String {
    Uuid::now_v7().to_string()
}

/// Turns a password into something safe to store, using Argon2id with a fresh
/// random salt each time.
///
/// This is not `crypto_hash_sha256`, and the difference matters more than any
/// other line in this file. A SHA-256 digest of a password is computed
/// billions of times a second on a graphics card, so a stolen table of them is
/// a stolen table of passwords. Argon2id is deliberately slow and deliberately
/// memory-hungry, which is what makes guessing at scale cost real money.
///
/// The answer carries its own salt and settings, so store the whole string and
/// hand it back to `crypto_verify_password` unchanged. Two calls with the same
/// password return different strings; that is the salt doing its job, and it
/// is why you must never compare these with `==`.
pub fn hash_password(password: String) -> Result<String, String> {
    use argon2::password_hash::{rand_core::OsRng, PasswordHasher, SaltString};
    use argon2::Argon2;

    let salt = SaltString::generate(&mut OsRng);
    return Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map(|hash| hash.to_string())
        .map_err(|e| format!("crypto_hash_password: could not hash the password: {}", e));
}

/// Checks a password against a stored hash from `crypto_hash_password`.
/// Returns false for a wrong password and false for a stored value that is not
/// a hash at all, so a corrupted row cannot let anyone in.
pub fn verify_password(password: String, stored_hash: String) -> bool {
    use argon2::password_hash::{PasswordHash, PasswordVerifier};
    use argon2::Argon2;

    return match PasswordHash::new(&stored_hash) {
        Ok(parsed) => Argon2::default().verify_password(password.as_bytes(), &parsed).is_ok(),
        Err(_) => false,
    };
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn digests_match_the_published_answers_for_the_empty_string() {
        assert_eq!(hash_sha256(String::new()), "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855");
        assert_eq!(hash_sha512(String::new()), "cf83e1357eefb8bdf1542850d66d8007d620e4050b5715dc83f4a921d36ce9ce47d0d13c5d85f2b0ff8318d2877eec2f63b931bd47417a81a538327af927da3e");
        assert_eq!(hash_md5(String::new()), "d41d8cd98f00b204e9800998ecf8427e");
    }

    #[test]
    fn hmac_matches_the_published_answer() {
        // RFC 4231 test case 1.
        assert_eq!(hmac_sha256("\x0b".repeat(20), "Hi There".to_string()), "b0344c61d8db38535ca8afceaf0bf12b881dc200c9833da726e9376c2e32cff7");
    }

    #[test]
    fn a_hashed_password_verifies_and_a_wrong_one_does_not() {
        let stored = hash_password("correct horse battery staple".to_string()).expect("a hashable password");
        assert!(verify_password("correct horse battery staple".to_string(), stored.clone()));
        assert!(!verify_password("Correct horse battery staple".to_string(), stored.clone()));
        assert!(!verify_password(String::new(), stored));
    }

    #[test]
    fn the_same_password_hashes_differently_every_time() {
        let first = hash_password("hunter2".to_string()).expect("a hashable password");
        let second = hash_password("hunter2".to_string()).expect("a hashable password");
        assert_ne!(first, second, "each hash must carry its own salt");
        // Both still verify, which is the whole point of storing the salt inside.
        assert!(verify_password("hunter2".to_string(), first));
        assert!(verify_password("hunter2".to_string(), second));
    }

    #[test]
    fn a_stored_value_that_is_not_a_hash_lets_nobody_in() {
        assert!(!verify_password("anything".to_string(), "not a hash at all".to_string()));
        assert!(!verify_password("anything".to_string(), String::new()));
    }

    #[test]
    fn version_seven_ids_sort_by_the_time_they_were_made() {
        let mut ids: Vec<String> = Vec::new();
        for _ in 0..8 {
            ids.push(uuid_v7());
            std::thread::sleep(std::time::Duration::from_millis(2));
        }
        let mut sorted = ids.clone();
        sorted.sort();
        assert_eq!(ids, sorted, "version 7 ids must already be in the order they were made");
    }

    #[test]
    fn version_four_ids_are_distinct() {
        let first = uuid_v4();
        assert_ne!(first, uuid_v4());
        assert_eq!(first.len(), 36);
    }

    #[test]
    fn random_hex_is_the_length_asked_for_and_refuses_nonsense() {
        assert_eq!(random_hex(16).expect("a usable amount").len(), 32);
        assert!(random_hex(0).unwrap_err().contains("not a usable amount"));
        assert!(random_hex(2048).unwrap_err().contains("1024 byte limit"));
    }

    #[test]
    fn secure_equal_agrees_with_equality_without_leaking_the_timing() {
        assert!(secure_equal("token".to_string(), "token".to_string()));
        assert!(!secure_equal("token".to_string(), "tokeN".to_string()));
        assert!(!secure_equal("token".to_string(), "token ".to_string()));
    }
}

/// Crockford's base32 alphabet, which a ULID is spelled in: no letters that
/// can be read as digits, so an id read aloud or typed in survives it.
const CROCKFORD: &[u8; 32] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";

/// A ULID: 26 characters, the first ten of them the time it was made, the rest
/// random. Sorts by age like a v7 UUID, but with no hyphens and in an alphabet
/// that can be typed - which is what makes it the identifier to put in a URL.
pub fn ulid() -> Result<String, String> {
    let milliseconds = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|_| "crypto_ulid: the system clock is set before 1970, so an id cannot be dated".to_string())?
        .as_millis();

    let mut out = String::with_capacity(26);
    // The timestamp is 48 bits written as ten base32 characters, most
    // significant first, so sorting the text sorts by time.
    for position in (0..10).rev() {
        let shift = position * 5;
        out.push(CROCKFORD[((milliseconds >> shift) & 31) as usize] as char);
    }

    let mut randomness = [0u8; 16];
    rand::rngs::OsRng.fill_bytes(&mut randomness);
    for byte in randomness.iter().take(16) {
        out.push(CROCKFORD[(*byte & 31) as usize] as char);
    }
    out.truncate(26);
    return Ok(out);
}

/// A random identifier of the given length in the URL-safe alphabet - letters,
/// digits, hyphen and underscore. Shorter than a UUID and needs no escaping,
/// for share links and invite codes.
///
/// Twelve characters is about as short as is sensible; below that, two ids
/// colliding stops being unlikely.
pub fn random_id(length: i64) -> Result<String, String> {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
    if length < 1 {
        return Err(format!("crypto_random_id: the length must be at least 1, got {}", length));
    }
    if length > 256 {
        return Err(format!("crypto_random_id: the length must be at most 256, got {}", length));
    }
    let mut bytes = vec![0u8; length as usize];
    rand::rngs::OsRng.fill_bytes(&mut bytes);
    // Each byte's low six bits index the alphabet, and 64 divides 256 exactly,
    // so no value is more likely than another.
    return Ok(bytes.iter().map(|byte| ALPHABET[(*byte & 63) as usize] as char).collect());
}

#[cfg(test)]
mod identifier_tests {
    use super::*;

    #[test]
    fn a_ulid_is_twenty_six_typable_characters() {
        let id = ulid().expect("a working clock");
        assert_eq!(id.chars().count(), 26);
        assert!(id.chars().all(|character| CROCKFORD.contains(&(character as u8))), "got: {}", id);
    }

    #[test]
    fn ulids_sort_by_the_time_they_were_made() {
        let first = ulid().expect("a working clock");
        std::thread::sleep(std::time::Duration::from_millis(2));
        let second = ulid().expect("a working clock");
        assert!(first < second, "{} should sort before {}", first, second);
    }

    #[test]
    fn two_ulids_made_together_still_differ() {
        let first = ulid().expect("a working clock");
        let second = ulid().expect("a working clock");
        assert_ne!(first, second);
    }

    #[test]
    fn a_random_id_is_the_length_asked_for_and_needs_no_escaping() {
        let id = random_id(12).expect("a sensible length");
        assert_eq!(id.chars().count(), 12);
        assert!(id.chars().all(|character| character.is_ascii_alphanumeric() || character == '-' || character == '_'), "got: {}", id);
        assert_ne!(random_id(21).expect("a sensible length"), random_id(21).expect("a sensible length"));
    }

    #[test]
    fn a_length_that_makes_no_sense_is_refused() {
        assert!(random_id(0).is_err());
        assert!(random_id(-1).is_err());
        assert!(random_id(1000).is_err());
    }
}

/// The SHA-256 of a file's contents, as hex - the checksum a download is
/// verified against, and the fingerprint that says whether two files are the
/// same one.
///
/// Takes a path rather than the contents because the contents may be a
/// gigabyte, and because a file of arbitrary bytes cannot be held in a Nail
/// string at all. Read in blocks, so the file never has to fit in memory.
pub async fn hash_file_sha256(path: String) -> Result<String, String> {
    use tokio::io::AsyncReadExt;

    let mut file = tokio::fs::File::open(&path).await.map_err(|failure| format!("crypto_hash_file_sha256: could not read '{}': {}", path, failure))?;
    let mut hasher = Sha256::new();
    let mut block = vec![0u8; 64 * 1024];
    loop {
        let read = file.read(&mut block).await.map_err(|failure| format!("crypto_hash_file_sha256: could not read '{}': {}", path, failure))?;
        if read == 0 {
            break;
        }
        hasher.update(&block[..read]);
    }
    return Ok(format!("{:x}", hasher.finalize()));
}

#[cfg(test)]
mod file_hash_tests {
    use super::*;

    #[tokio::test]
    async fn a_file_hashes_to_the_same_thing_as_its_contents() {
        let path = std::env::temp_dir().join("nail_hash_file_test.txt");
        std::fs::write(&path, "hello").expect("a writable file");
        let from_file = hash_file_sha256(path.to_string_lossy().to_string()).await.expect("a readable file");
        assert_eq!(from_file, hash_sha256("hello".to_string()));
        // The published SHA-256 of "hello".
        assert_eq!(from_file, "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824");
        let _ = std::fs::remove_file(&path);
    }

    /// Blocks are 64 KiB, so this crosses several of them.
    #[tokio::test]
    async fn a_file_larger_than_one_block_hashes_correctly() {
        let path = std::env::temp_dir().join("nail_hash_file_big_test.bin");
        let contents: Vec<u8> = (0..200_000).map(|index| (index % 251) as u8).collect();
        std::fs::write(&path, &contents).expect("a writable file");

        let from_file = hash_file_sha256(path.to_string_lossy().to_string()).await.expect("a readable file");
        let mut hasher = Sha256::new();
        hasher.update(&contents);
        assert_eq!(from_file, format!("{:x}", hasher.finalize()));
        let _ = std::fs::remove_file(&path);
    }

    #[tokio::test]
    async fn a_file_that_is_not_there_says_so() {
        let failure = hash_file_sha256("/tmp/nail_no_such_file_to_hash".to_string()).await.unwrap_err();
        assert!(failure.contains("crypto_hash_file_sha256"), "got: {}", failure);
        assert!(failure.contains("nail_no_such_file_to_hash"), "got: {}", failure);
    }
}

/// The 32-byte key AES-GCM needs, derived from whatever secret was passed in.
/// A passphrase of any length becomes a key of the right one, and the same
/// passphrase always gives the same key.
fn key_from_secret(secret: &str) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(secret.as_bytes());
    return hasher.finalize().into();
}

/// Encrypts text with a secret, so only somebody holding the same secret can
/// read it back. AES-256-GCM, which also authenticates: text that was tampered
/// with fails to decrypt rather than decrypting to something else.
///
/// The result is URL-safe base64 holding a fresh random nonce followed by the
/// ciphertext, so encrypting the same text twice gives two different answers -
/// which is what stops anybody from telling that they were the same.
///
/// The secret is hashed to make the key, so any length works; use something
/// long and random, such as `crypto_random_hex(32)` kept in the environment.
/// This is for data at rest - a session cookie, a stored token. Passwords go
/// through `crypto_hash_password` instead, which is not reversible at all.
pub fn encrypt(text: String, secret: String) -> Result<String, String> {
    use aes_gcm::aead::{Aead, KeyInit};

    if secret.is_empty() {
        return Err("crypto_encrypt: the secret is empty, so there is nothing keeping this text private".to_string());
    }

    let key = key_from_secret(&secret);
    let cipher = aes_gcm::Aes256Gcm::new_from_slice(&key).map_err(|e| format!("crypto_encrypt: could not use that secret: {}", e))?;

    let mut nonce_bytes = [0u8; 12];
    rand::rngs::OsRng.fill_bytes(&mut nonce_bytes);
    let nonce = aes_gcm::Nonce::from_slice(&nonce_bytes);

    let encrypted = cipher.encrypt(nonce, text.as_bytes()).map_err(|_| "crypto_encrypt: could not encrypt the text".to_string())?;

    let mut carried = nonce_bytes.to_vec();
    carried.extend_from_slice(&encrypted);
    return Ok(super::base64::encode_url_bytes(&carried));
}

/// Reads back what `crypto_encrypt` wrote, with the same secret. Errors when
/// the secret is wrong, when the text was changed after it was encrypted, or
/// when it was never encrypted at all - all of which are the same answer to
/// whoever handed it over: no.
pub fn decrypt(encrypted: String, secret: String) -> Result<String, String> {
    use aes_gcm::aead::{Aead, KeyInit};

    if secret.is_empty() {
        return Err("crypto_decrypt: the secret is empty, so there is nothing to decrypt with".to_string());
    }

    let carried = super::base64::decode_url_bytes(&encrypted).map_err(|_| "crypto_decrypt: this is not something crypto_encrypt wrote".to_string())?;
    if carried.len() <= 12 {
        return Err("crypto_decrypt: this is not something crypto_encrypt wrote".to_string());
    }

    let key = key_from_secret(&secret);
    let cipher = aes_gcm::Aes256Gcm::new_from_slice(&key).map_err(|e| format!("crypto_decrypt: could not use that secret: {}", e))?;
    let nonce = aes_gcm::Nonce::from_slice(&carried[..12]);

    let plain = cipher.decrypt(nonce, &carried[12..]).map_err(|_| "crypto_decrypt: the wrong secret, or the text was changed after it was encrypted".to_string())?;
    return String::from_utf8(plain).map_err(|_| "crypto_decrypt: what came out is not text".to_string());
}

#[cfg(test)]
mod encryption_tests {
    use super::*;

    #[test]
    fn what_was_encrypted_comes_back() {
        let sealed = encrypt("the secret plan".to_string(), "a long random key".to_string()).expect("a secret");
        assert_ne!(sealed, "the secret plan");
        assert_eq!(decrypt(sealed, "a long random key".to_string()).expect("the same secret"), "the secret plan");
    }

    #[test]
    fn the_same_text_encrypts_differently_every_time() {
        let first = encrypt("same".to_string(), "key".to_string()).expect("a secret");
        let second = encrypt("same".to_string(), "key".to_string()).expect("a secret");
        assert_ne!(first, second);
        assert_eq!(decrypt(first, "key".to_string()).expect("the key"), "same");
        assert_eq!(decrypt(second, "key".to_string()).expect("the key"), "same");
    }

    #[test]
    fn the_wrong_secret_does_not_open_it() {
        let sealed = encrypt("private".to_string(), "right".to_string()).expect("a secret");
        assert!(decrypt(sealed, "wrong".to_string()).unwrap_err().contains("the wrong secret"));
    }

    #[test]
    fn text_that_was_changed_afterwards_fails_rather_than_decrypting_to_something_else() {
        let sealed = encrypt("transfer 10".to_string(), "key".to_string()).expect("a secret");
        // Flip the last character to something else in the same alphabet.
        let mut tampered = sealed[..sealed.len() - 1].to_string();
        tampered.push(if sealed.ends_with('A') { 'B' } else { 'A' });
        assert!(decrypt(tampered, "key".to_string()).is_err());
    }

    #[test]
    fn nonsense_and_empty_secrets_are_refused() {
        assert!(decrypt("not encrypted".to_string(), "key".to_string()).unwrap_err().contains("not something crypto_encrypt wrote"));
        assert!(decrypt("".to_string(), "key".to_string()).unwrap_err().contains("not something crypto_encrypt wrote"));
        assert!(encrypt("text".to_string(), "".to_string()).unwrap_err().contains("the secret is empty"));
        assert!(decrypt("text".to_string(), "".to_string()).unwrap_err().contains("the secret is empty"));
    }

    #[test]
    fn a_long_text_and_unicode_survive_the_trip() {
        let long = "héllo, 世界 ".repeat(500);
        let sealed = encrypt(long.clone(), "key".to_string()).expect("a secret");
        assert_eq!(decrypt(sealed, "key".to_string()).expect("the key"), long);
    }
}

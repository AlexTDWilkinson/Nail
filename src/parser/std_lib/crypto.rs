use sha2::{Sha256, Digest};
use md5;
use uuid::Uuid;

// SHA256 hash
pub async fn hash_sha256(s: String) -> String {
    let mut hasher = Sha256::new();
    hasher.update(s.as_bytes());
    let result = hasher.finalize();
    format!("{:x}", result)
}

// MD5 hash (for checksums, not cryptographic security)
pub async fn hash_md5(s: String) -> String {
    let digest = md5::compute(s.as_bytes());
    format!("{:x}", digest)
}

// Generate UUID v4
pub async fn uuid_v4() -> String {
    Uuid::new_v4().to_string()
}
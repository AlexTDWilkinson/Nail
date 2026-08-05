//! Signed tokens, for a server that has to recognise a visitor it has seen
//! before.
//!
//! A cookie a browser can edit is worth nothing on its own, so the server
//! signs what it puts there and checks the signature when it comes back. That
//! is all a JWT is: some JSON, a signature over it, and both spelled in
//! base64url so they survive a header or a cookie.
//!
//! Only HS256 is offered - one shared secret, HMAC-SHA256 - because that is
//! what a program signing its own tokens needs, and offering a choice of
//! algorithm is how JWT libraries end up accepting `alg: none`. A token
//! arriving with any other algorithm is refused rather than trusted.
//!
//! The claims go in and come out as JSON text, which `json_serialize` and
//! `json_deserialize` turn into a struct of the program's own.

use base64::Engine;
use hmac::{Hmac, Mac};
use sha2::Sha256;

/// base64url without padding, which is what the JWT format specifies.
const ENCODING: base64::engine::general_purpose::GeneralPurpose = base64::engine::general_purpose::URL_SAFE_NO_PAD;

/// The one header this module writes, and the only one it accepts back.
const HEADER: &str = "{\"alg\":\"HS256\",\"typ\":\"JWT\"}";

fn sign_parts(payload: &str, secret: &str) -> String {
    let mut mac = <Hmac<Sha256> as Mac>::new_from_slice(secret.as_bytes()).expect("HMAC accepts a key of any length");
    mac.update(payload.as_bytes());
    return ENCODING.encode(mac.finalize().into_bytes());
}

/// Seconds since the epoch, which is what the `exp` and `iat` claims count.
fn now_in_seconds() -> Result<i64, String> {
    return std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|elapsed| elapsed.as_secs() as i64)
        .map_err(|_| "the system clock is set before 1970, so a token cannot be dated".to_string());
}

/// A signed token carrying the given claims, expiring the given number of
/// seconds from now. An expiry of zero or less makes a token that does not
/// expire, for the cases - a machine-to-machine key, a signed download link
/// checked another way - where a deadline is the wrong tool.
///
/// The claims must be a JSON object, because `exp` and `iat` are added to it.
pub fn sign(claims_json: String, secret: String, expires_in_seconds: i64) -> Result<String, String> {
    if secret.is_empty() {
        return Err("jwt_sign: the secret cannot be empty, or anyone could sign a token".to_string());
    }
    let mut claims: serde_json::Value = serde_json::from_str(&claims_json).map_err(|failure| format!("jwt_sign: the claims are not JSON: {}", failure))?;
    let object = match claims.as_object_mut() {
        Some(object) => object,
        None => return Err("jwt_sign: the claims must be a JSON object, since the issued and expiry times are added to them".to_string()),
    };

    let issued_at = now_in_seconds().map_err(|detail| format!("jwt_sign: {}", detail))?;
    object.insert("iat".to_string(), serde_json::json!(issued_at));
    if expires_in_seconds > 0 {
        object.insert("exp".to_string(), serde_json::json!(issued_at + expires_in_seconds));
    }

    let payload = format!("{}.{}", ENCODING.encode(HEADER), ENCODING.encode(serde_json::to_string(&claims).map_err(|failure| format!("jwt_sign: the claims could not be written back out: {}", failure))?));
    let signature = sign_parts(&payload, &secret);
    return Ok(format!("{}.{}", payload, signature));
}

/// The claims of a token whose signature checks out and whose expiry has not
/// passed, as JSON text. Anything else is an error saying which of those went
/// wrong, and a program should treat every one of them the same way: as a
/// visitor it does not recognise.
pub fn verify(token: String, secret: String) -> Result<String, String> {
    let parts: Vec<&str> = token.split('.').collect();
    if parts.len() != 3 {
        return Err("jwt_verify: this is not a token - a token has three parts separated by dots".to_string());
    }

    let header_bytes = ENCODING.decode(parts[0]).map_err(|_| "jwt_verify: the header is not base64url".to_string())?;
    let header: serde_json::Value = serde_json::from_slice(&header_bytes).map_err(|_| "jwt_verify: the header is not JSON".to_string())?;
    // Checked before the signature so a token asking for `alg: none` is refused
    // outright rather than being handed to a verifier that might honour it.
    if header.get("alg").and_then(|algorithm| algorithm.as_str()) != Some("HS256") {
        return Err("jwt_verify: this token was not signed with HS256, which is the only algorithm accepted".to_string());
    }

    let payload = format!("{}.{}", parts[0], parts[1]);
    let expected = sign_parts(&payload, &secret);
    // Compared byte by byte in constant time, so how much of a forged signature
    // was right cannot be learned from how long the comparison took.
    if !constant_time_equal(expected.as_bytes(), parts[2].as_bytes()) {
        return Err("jwt_verify: the signature does not match, so this token was not signed with that secret".to_string());
    }

    let claims_bytes = ENCODING.decode(parts[1]).map_err(|_| "jwt_verify: the claims are not base64url".to_string())?;
    let claims: serde_json::Value = serde_json::from_slice(&claims_bytes).map_err(|_| "jwt_verify: the claims are not JSON".to_string())?;
    if let Some(expiry) = claims.get("exp").and_then(|expiry| expiry.as_i64()) {
        if now_in_seconds().map_err(|detail| format!("jwt_verify: {}", detail))? >= expiry {
            return Err("jwt_verify: this token has expired".to_string());
        }
    }

    return serde_json::to_string(&claims).map_err(|failure| format!("jwt_verify: the claims could not be written back out: {}", failure));
}

/// Whether the token's expiry has passed. A token with no expiry is never
/// expired. This asks nothing about the signature - use it to decide whether to
/// refresh a token you already trust, not whether to trust one.
pub fn is_expired(token: String) -> Result<bool, String> {
    let claims_json = read_unverified(token)?;
    let claims: serde_json::Value = serde_json::from_str(&claims_json).map_err(|failure| format!("jwt_is_expired: the claims are not JSON: {}", failure))?;
    return match claims.get("exp").and_then(|expiry| expiry.as_i64()) {
        Some(expiry) => Ok(now_in_seconds().map_err(|detail| format!("jwt_is_expired: {}", detail))? >= expiry),
        None => Ok(false),
    };
}

/// The claims of a token without checking the signature, for reading which user
/// a token names before deciding what to do with it - logging, say, or picking
/// which secret to verify against.
///
/// Nothing this returns has been verified. A program that acts on it is trusting
/// text a browser could have written.
pub fn read_unverified(token: String) -> Result<String, String> {
    let parts: Vec<&str> = token.split('.').collect();
    if parts.len() != 3 {
        return Err("jwt_read_unverified: this is not a token - a token has three parts separated by dots".to_string());
    }
    let claims_bytes = ENCODING.decode(parts[1]).map_err(|_| "jwt_read_unverified: the claims are not base64url".to_string())?;
    let claims: serde_json::Value = serde_json::from_slice(&claims_bytes).map_err(|_| "jwt_read_unverified: the claims are not JSON".to_string())?;
    return serde_json::to_string(&claims).map_err(|failure| format!("jwt_read_unverified: the claims could not be written back out: {}", failure));
}

/// Compares two byte strings in time that does not depend on where they first
/// differ. The same reasoning as `crypto_secure_equal`, kept here so verifying
/// a token needs nothing from another module.
fn constant_time_equal(left: &[u8], right: &[u8]) -> bool {
    if left.len() != right.len() {
        return false;
    }
    let mut difference = 0u8;
    for (left_byte, right_byte) in left.iter().zip(right.iter()) {
        difference |= left_byte ^ right_byte;
    }
    return difference == 0;
}

#[cfg(test)]
mod tests {
    use super::*;

    const SECRET: &str = "a-secret-nobody-else-has";

    #[test]
    fn a_token_signed_here_verifies_here() {
        let token = sign("{\"user\":\"alex\"}".to_string(), SECRET.to_string(), 3600).expect("signable claims");
        let claims: serde_json::Value = serde_json::from_str(&verify(token, SECRET.to_string()).expect("our own token")).expect("JSON claims");
        assert_eq!(claims["user"], "alex");
        assert!(claims["exp"].as_i64().expect("an expiry") > claims["iat"].as_i64().expect("an issued time"));
    }

    #[test]
    fn a_token_has_three_parts() {
        let token = sign("{}".to_string(), SECRET.to_string(), 60).expect("signable claims");
        assert_eq!(token.split('.').count(), 3);
        // No padding, and nothing that would have to be escaped in a URL.
        assert!(!token.contains('='), "got: {}", token);
        assert!(!token.contains('+'), "got: {}", token);
    }

    #[test]
    fn another_secret_does_not_verify_it() {
        let token = sign("{\"user\":\"alex\"}".to_string(), SECRET.to_string(), 3600).expect("signable claims");
        let failure = verify(token, "some-other-secret".to_string()).unwrap_err();
        assert!(failure.contains("signature does not match"), "got: {}", failure);
    }

    #[test]
    fn an_edited_claim_does_not_verify() {
        let token = sign("{\"user\":\"alex\"}".to_string(), SECRET.to_string(), 3600).expect("signable claims");
        let parts: Vec<&str> = token.split('.').collect();
        let forged_claims = ENCODING.encode("{\"user\":\"root\",\"iat\":1,\"exp\":9999999999}");
        let forged = format!("{}.{}.{}", parts[0], forged_claims, parts[2]);
        assert!(verify(forged, SECRET.to_string()).is_err());
    }

    #[test]
    fn a_token_claiming_no_algorithm_is_refused() {
        let header = ENCODING.encode("{\"alg\":\"none\",\"typ\":\"JWT\"}");
        let claims = ENCODING.encode("{\"user\":\"root\"}");
        let token = format!("{}.{}.", header, claims);
        let failure = verify(token, SECRET.to_string()).unwrap_err();
        assert!(failure.contains("HS256"), "got: {}", failure);
    }

    #[test]
    fn an_expired_token_is_refused_even_though_it_is_properly_signed() {
        // Signed with a negative lifetime, so its expiry is already behind us.
        let issued = now_in_seconds().expect("a working clock");
        let claims = ENCODING.encode(format!("{{\"user\":\"alex\",\"iat\":{},\"exp\":{}}}", issued - 120, issued - 60));
        let payload = format!("{}.{}", ENCODING.encode(HEADER), claims);
        let token = format!("{}.{}", payload, sign_parts(&payload, SECRET));
        let failure = verify(token.clone(), SECRET.to_string()).unwrap_err();
        assert!(failure.contains("expired"), "got: {}", failure);
        assert!(is_expired(token).expect("readable claims"));
    }

    #[test]
    fn a_token_with_no_expiry_never_expires() {
        let token = sign("{\"user\":\"alex\"}".to_string(), SECRET.to_string(), 0).expect("signable claims");
        let claims: serde_json::Value = serde_json::from_str(&verify(token.clone(), SECRET.to_string()).expect("our own token")).expect("JSON claims");
        assert!(claims.get("exp").is_none(), "got: {}", claims);
        assert!(!is_expired(token).expect("readable claims"));
    }

    #[test]
    fn claims_can_be_read_without_the_secret() {
        let token = sign("{\"user\":\"alex\"}".to_string(), SECRET.to_string(), 60).expect("signable claims");
        let claims: serde_json::Value = serde_json::from_str(&read_unverified(token).expect("readable claims")).expect("JSON claims");
        assert_eq!(claims["user"], "alex");
    }

    #[test]
    fn things_that_are_not_tokens_say_so() {
        assert!(verify("not-a-token".to_string(), SECRET.to_string()).is_err());
        assert!(verify("one.two".to_string(), SECRET.to_string()).is_err());
        assert!(read_unverified("nope".to_string()).is_err());
        assert!(sign("[1,2,3]".to_string(), SECRET.to_string(), 60).is_err());
        assert!(sign("not json".to_string(), SECRET.to_string(), 60).is_err());
        assert!(sign("{}".to_string(), String::new(), 60).is_err());
    }
}

//! JWT module stdlib registry entries

use super::*;

pub(super) fn register(m: &mut HashMap<&'static str, StdlibFunction>) {
    simple_fns! { m, Jwt:
        "jwt_sign" [Hmac, Sha2, Base64, SerdeJson] => "std_lib::jwt::sign", (claims_json: s, secret: s, expires_in_seconds: i) -> (s!e),
            "Returns a signed HS256 token carrying the given JSON claims, expiring that many seconds from now, or never if the number is zero or less.",
            "struct Session { user:i }\n\nsession:Session = Session { user = 42 };\nsecret:s = `a long random string kept out of the source`;\ntoken:s = danger(jwt_sign(danger(json_serialize(session)), secret, 3600));";
        "jwt_verify" [Hmac, Sha2, Base64, SerdeJson] => "std_lib::jwt::verify", (token: s, secret: s) -> (s!e),
            "Returns the claims of a token whose signature checks out and whose expiry has not passed, as JSON text. Any other outcome is an error.",
            "secret:s = `a long random string kept out of the source`;\ntoken:s = danger(jwt_sign(`{\"user\":42}`, secret, 3600));\ncookie_value:s = token;\nclaims_json:s = danger(jwt_verify(cookie_value, secret));";
        "jwt_is_expired" [Base64, SerdeJson] => "std_lib::jwt::is_expired", (token: s) -> (b!e),
            "Returns true if the token's expiry has passed, without checking the signature. For deciding whether to refresh a token, not whether to trust one.",
            "secret:s = `a long random string kept out of the source`;\ntoken:s = danger(jwt_sign(`{\"user\":42}`, secret, 3600));\nstale:b = danger(jwt_is_expired(token));";
        "jwt_read_unverified" [Base64, SerdeJson] => "std_lib::jwt::read_unverified", (token: s) -> (s!e),
            "Returns the claims of a token as JSON text without checking the signature. Nothing it returns has been verified.",
            "secret:s = `a long random string kept out of the source`;\ntoken:s = danger(jwt_sign(`{\"user\":42}`, secret, 3600));\nclaims_json:s = danger(jwt_read_unverified(token));";
    }
}

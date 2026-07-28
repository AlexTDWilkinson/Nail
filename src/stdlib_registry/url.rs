//! Url module stdlib registry entries.

use super::*;

pub(super) fn register(m: &mut HashMap<&'static str, StdlibFunction>) {
    simple_fns! { m, Url:
        "url_encode" [UrlEncoding] => "std_lib::url::encode", (text: s) -> s,
            "Percent-encodes a string for safe use in a URL.",
            "safe:s = url_encode(`a b&c`);";
        "url_decode" [UrlEncoding] => "std_lib::url::decode", (text: s) -> (s!e),
            "Decodes a percent-encoded URL string; errors on invalid encoding.",
            "plain:s = danger(url_decode(`a%20b`));";
        "url_parse_query" [UrlEncoding, DashMap] => "std_lib::url::parse_query", (query: s) -> (h s s),
            "Parses a query string like a=1&b=2 into a hashmap.",
            "params:h<s,s> = url_parse_query(`page=2&sort=asc`);";
        "url_build_query" [UrlEncoding, DashMap] => "std_lib::url::build_query", (params: (&(h s s))) -> s,
            "Builds a percent-encoded query string from a hashmap.",
            "query:s = url_build_query(params);";
    }
}

//! Cache module stdlib registry entries.

use super::*;

pub(super) fn register(m: &mut HashMap<&'static str, StdlibFunction>) {
    simple_fns! { m, Cache:
        "cache_set" => "std_lib::cache::set", (cache: s, key: s, value: s) -> v,
            "Stores a value in a named in-memory cache. Caches live for the length of the process and are shared everywhere by name - this is how a web handler keeps something between requests. Values are strings: json_serialize anything richer first.",
            "cache_set(`pages`, `/about`, rendered);";
        "cache_set_ttl" => "std_lib::cache::set_ttl", (cache: s, key: s, value: s, ttl_seconds: i) -> v,
            "Stores a value that quietly disappears after the given number of seconds.",
            "cache_set_ttl(`weather`, city, report, 600);";
        "cache_get" => "std_lib::cache::get", (cache: s, key: s) -> (s!e),
            "The stored value. An error when nothing is there or it has expired.",
            "page:s = safe(cache_get(`pages`, `/about`), render_about);";
        "cache_get_or" => "std_lib::cache::get_or", (cache: s, key: s, fallback: s) -> s,
            "The stored value, or the fallback when nothing is there.",
            "greeting:s = cache_get_or(`session`, user_id, `stranger`);";
        "cache_has" => "std_lib::cache::has", (cache: s, key: s) -> b,
            "Whether a live value is stored under the key.",
            "warm:b = cache_has(`pages`, `/about`);";
        "cache_delete" => "std_lib::cache::delete", (cache: s, key: s) -> v,
            "Drops one key. Deleting what is not there is fine.",
            "cache_delete(`pages`, `/about`);";
        "cache_clear" => "std_lib::cache::clear", (cache: s) -> v,
            "Drops everything in one cache.",
            "cache_clear(`pages`);";
        "cache_len" => "std_lib::cache::len", (cache: s) -> i,
            "How many live values a cache holds.",
            "cached:i = cache_len(`pages`);";
    }
}

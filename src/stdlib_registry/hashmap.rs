//! HashMap module stdlib registry entries.

use super::*;

pub(super) fn register(m: &mut HashMap<&'static str, StdlibFunction>) {
    simple_fns! { m, HashMap:
        "hashmap_new" [DashMap] => "std_lib::hashmap::new", () -> (h K V),
            "Creates a new empty hashmap.",
            "scores:h<s,i> = hashmap_new();";
        "hashmap_set" [DashMap] => "std_lib::hashmap::insert", (map: (&(h K V)), key: K, value: V) -> v,
            "Inserts or updates a key-value pair in the hashmap.",
            "hashmap_set(scores, `alice`, 10);";
        "hashmap_get" [DashMap] => "std_lib::hashmap::get", (map: (&(h K V)), key: (&K)) -> (V!e),
            "Returns the value for a key, or an error if the key is missing.",
            "score:i = danger(hashmap_get(scores, `alice`));";
        "hashmap_remove" [DashMap] => "std_lib::hashmap::remove", (map: (&(h K V)), key: (&K)) -> (V!e),
            "Removes a key and returns its value, or an error if the key is missing.",
            "removed:i = danger(hashmap_remove(scores, `alice`));";
        "hashmap_contains_key" [DashMap] => "std_lib::hashmap::contains_key", (map: (&(h K V)), key: (&K)) -> b,
            "Returns true if the hashmap contains the given key.",
            "known:b = hashmap_contains_key(scores, `alice`);";
        "hashmap_len" [DashMap] => "std_lib::hashmap::len", (map: (&(h K V))) -> i,
            "Returns the number of entries in the hashmap.",
            "count:i = hashmap_len(scores);";
        "hashmap_is_empty" [DashMap] => "std_lib::hashmap::is_empty", (map: (&(h K V))) -> b,
            "Returns true if the hashmap has no entries.",
            "empty:b = hashmap_is_empty(scores);";
        "hashmap_clear" [DashMap] => "std_lib::hashmap::clear", (map: (&(h K V))) -> v,
            "Removes all entries from the hashmap.",
            "hashmap_clear(scores);";
        "hashmap_keys" [DashMap] => "std_lib::hashmap::keys", (map: (&(h K V))) -> [K],
            "Returns all keys in the hashmap as an array.",
            "names:a:s = hashmap_keys(scores);";
        "hashmap_values" [DashMap] => "std_lib::hashmap::values", (map: (&(h K V))) -> [V],
            "Returns all values in the hashmap as an array.",
            "points:a:i = hashmap_values(scores);";
        "hashmap_entry_or_insert" [DashMap] => "std_lib::hashmap::entry_or_insert", (map: (&(h K V)), key: K, default: V) -> V,
            "Returns the value for a key, inserting and returning the default if the key is missing.",
            "score:i = hashmap_entry_or_insert(scores, `alice`, 0);";
        "hashmap_merge" [DashMap] => "std_lib::hashmap::merge", (first: (&(h K V)), second: (&(h K V))) -> (h K V),
            "Returns a new hashmap with entries from both maps; the second map wins on duplicate keys.",
            "combined:h<s,i> = hashmap_merge(defaults, overrides);";
    }
}

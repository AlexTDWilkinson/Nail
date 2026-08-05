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
        "hashmap_increment" [DashMap] => "std_lib::hashmap::increment", (map: (&(h K i)), key: K) -> i,
            "Adds one to the count under a key, starting from zero, and returns the new count.",
            "seen:i = hashmap_increment(counts, word);";
        "hashmap_add_to" [DashMap] => "std_lib::hashmap::add_to", (map: (&(h K i)), key: K, amount: i) -> i,
            "Adds an amount to the running total under a key, starting from zero, and returns the new total. A negative amount subtracts.",
            "balance:i = hashmap_add_to(totals, account, -250);";
        "hashmap_from_arrays" [DashMap] => "std_lib::hashmap::from_arrays", (keys: [K], values: [V]) -> ((h K V)!e),
            "Builds a hashmap by pairing keys with values by position. Errors if the arrays are different lengths.",
            "settings:h<s,s> = danger(hashmap_from_arrays(names, values));";
        "hashmap_merge" [DashMap] => "std_lib::hashmap::merge", (first: (&(h K V)), second: (&(h K V))) -> (h K V),
            "Returns a new hashmap with entries from both maps. The second map wins on duplicate keys.",
            "combined:h<s,i> = hashmap_merge(defaults, overrides);";
        "hashmap_get_or" [DashMap] => "std_lib::hashmap::get_or", (map: (&(h K V)), key: (&K), fallback: V) -> V,
            "Returns the value for a key, or the fallback when the key is missing - never an error, and the map is not changed. The inserting cousin is hashmap_entry_or_insert.",
            "score:i = hashmap_get_or(scores, `alice`, 0);";
        "hashmap_key_of" [DashMap] => "std_lib::hashmap::key_of", (map: (&(h K V)), value: (&V)) -> (K!e),
            "Returns a key holding the given value, or an error when none does - the lookup run backwards. With several such keys, which one comes back is undefined. Meant for values that appear once, the way an id does.",
            "name:s = danger(hashmap_key_of(user_ids, 42));";
    }
}

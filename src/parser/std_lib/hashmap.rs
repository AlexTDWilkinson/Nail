use dashmap::DashMap;
use std::hash::Hash;

pub fn new<K: Hash + Eq + Clone, V: Clone>() -> DashMap<K, V> {
    DashMap::new()
}

pub fn insert<K: Hash + Eq + Clone, V: Clone>(map: &DashMap<K, V>, key: K, value: V) -> Option<V> {
    map.insert(key, value)
}

pub fn get<K: Hash + Eq + Clone + std::fmt::Debug, V: Clone>(map: &DashMap<K, V>, key: &K) -> Result<V, String> {
    map.get(key)
        .map(|v| v.clone())
        .ok_or_else(|| format!("hashmap_get: key {:?} not found in the hashmap", key))
}

pub fn remove<K: Hash + Eq + Clone + std::fmt::Debug, V: Clone>(map: &DashMap<K, V>, key: &K) -> Result<V, String> {
    map.remove(key)
        .map(|(_, v)| v)
        .ok_or_else(|| format!("hashmap_remove: key {:?} not found in the hashmap", key))
}

pub fn contains_key<K: Hash + Eq + Clone, V: Clone>(map: &DashMap<K, V>, key: &K) -> bool {
    map.contains_key(key)
}

pub fn len<K: Hash + Eq + Clone, V: Clone>(map: &DashMap<K, V>) -> i64 {
    map.len() as i64
}

pub fn is_empty<K: Hash + Eq + Clone, V: Clone>(map: &DashMap<K, V>) -> bool {
    map.is_empty()
}

pub fn clear<K: Hash + Eq + Clone, V: Clone>(map: &DashMap<K, V>) {
    map.clear()
}

pub fn keys<K: Hash + Eq + Clone, V: Clone>(map: &DashMap<K, V>) -> Vec<K> {
    map.iter().map(|entry| entry.key().clone()).collect()
}

pub fn values<K: Hash + Eq + Clone, V: Clone>(map: &DashMap<K, V>) -> Vec<V> {
    map.iter().map(|entry| entry.value().clone()).collect()
}

pub fn to_vec<K: Hash + Eq + Clone, V: Clone>(map: &DashMap<K, V>) -> Vec<(K, V)> {
    map.iter().map(|entry| (entry.key().clone(), entry.value().clone())).collect()
}

// Generic from_vec that handles tuples
pub fn from_vec<K: Hash + Eq + Clone, V: Clone>(pairs: Vec<(K, V)>) -> DashMap<K, V> {
    let map = DashMap::new();
    for (key, value) in pairs {
        map.insert(key, value);
    }
    map
}

// Since Nail doesn't have a standard way to handle generic struct-to-tuple conversion,
// the transpiler will need to generate the appropriate conversion code for each use case

pub fn entry_or_insert<K: Hash + Eq + Clone, V: Clone>(map: &DashMap<K, V>, key: K, default_value: V) -> V {
    map.entry(key).or_insert(default_value).clone()
}

pub fn merge<K: Hash + Eq + Clone, V: Clone>(map1: &DashMap<K, V>, map2: &DashMap<K, V>) -> DashMap<K, V> {
    let result = DashMap::new();
    
    for entry in map1.iter() {
        result.insert(entry.key().clone(), entry.value().clone());
    }
    
    for entry in map2.iter() {
        result.insert(entry.key().clone(), entry.value().clone());
    }
    
    result
}
/// Adds one to the count under a key, starting from zero if it is not there
/// yet, and returns the new count. Counting how often something happens is the
/// most common thing a hashmap is for, and this is that in one line.
pub fn increment<K: Hash + Eq + Clone>(map: &DashMap<K, i64>, key: K) -> i64 {
    let mut entry = map.entry(key).or_insert(0);
    *entry += 1;
    return *entry;
}

/// Adds a number to the running total under a key, starting from zero, and
/// returns the new total. Pass a negative number to subtract.
pub fn add_to<K: Hash + Eq + Clone>(map: &DashMap<K, i64>, key: K, amount: i64) -> i64 {
    let mut entry = map.entry(key).or_insert(0);
    *entry += amount;
    return *entry;
}

/// Builds a hashmap from a list of keys and a matching list of values, paired
/// up by position. The two must be the same length; a later duplicate key wins.
pub fn from_arrays<K: Hash + Eq + Clone, V: Clone>(keys: Vec<K>, values: Vec<V>) -> Result<DashMap<K, V>, String> {
    if keys.len() != values.len() {
        return Err(format!("hashmap_from_arrays: {} keys and {} values, and they must be the same length", keys.len(), values.len()));
    }
    let map = DashMap::new();
    for (key, value) in keys.into_iter().zip(values.into_iter()) {
        map.insert(key, value);
    }
    return Ok(map);
}

/// The value under a key, or the fallback when the key is not there. The map
/// is not changed - the fallback is returned, never inserted - which is the
/// difference between this and entry_or_insert.
pub fn get_or<K: Hash + Eq + Clone, V: Clone>(map: &DashMap<K, V>, key: &K, fallback: V) -> V {
    return map.get(key).map(|entry| entry.value().clone()).unwrap_or(fallback);
}

/// A key holding the given value, or an error when no key does - the lookup
/// run backwards. A hashmap has no order, so with several keys holding the
/// value, which one comes back is not defined; it is for values that appear
/// once, the way an id does.
pub fn key_of<K: Hash + Eq + Clone, V: Clone + PartialEq + std::fmt::Debug>(map: &DashMap<K, V>, value: &V) -> Result<K, String> {
    return map.iter().find(|entry| entry.value() == value).map(|entry| entry.key().clone()).ok_or_else(|| format!("hashmap_key_of: no key holds the value {:?}", value));
}

/// The keys in order. A hashmap has no order of its own, and the order
/// `hashmap_keys` happens to give back can differ between two runs of the same
/// program, so this is what to use anywhere the order is seen: a listing, a
/// report, anything compared against a previous run.
pub fn sorted_keys<K: Hash + Eq + Clone + Ord, V: Clone>(map: &DashMap<K, V>) -> Vec<K> {
    let mut found: Vec<K> = map.iter().map(|entry| entry.key().clone()).collect();
    found.sort();
    return found;
}

/// The keys ordered by the value each one holds, smallest first. Keys holding
/// equal values come back in their own order, so the answer is the same on
/// every run.
pub fn keys_by_value<K: Hash + Eq + Clone + Ord, V: Clone + PartialOrd>(map: &DashMap<K, V>) -> Vec<K> {
    let mut pairs: Vec<(K, V)> = to_vec(map);
    pairs.sort_by(|left, right| left.1.partial_cmp(&right.1).unwrap_or(std::cmp::Ordering::Equal).then_with(|| left.0.cmp(&right.0)));
    return pairs.into_iter().map(|(key, _)| key).collect();
}

/// The keys ordered by the value each one holds, largest first. This with
/// `array_take` is the top ten: count into a hashmap with `hashmap_increment`,
/// order it here, take the front. Keys holding equal values still come back in
/// their own order.
pub fn keys_by_value_descending<K: Hash + Eq + Clone + Ord, V: Clone + PartialOrd>(map: &DashMap<K, V>) -> Vec<K> {
    let mut pairs: Vec<(K, V)> = to_vec(map);
    pairs.sort_by(|left, right| right.1.partial_cmp(&left.1).unwrap_or(std::cmp::Ordering::Equal).then_with(|| left.0.cmp(&right.0)));
    return pairs.into_iter().map(|(key, _)| key).collect();
}

/// The key holding the largest value, or an error when the hashmap is empty.
/// With several keys tied for largest, the first in the keys' own order wins,
/// so the answer does not move between runs.
pub fn max_by_value<K: Hash + Eq + Clone + Ord, V: Clone + PartialOrd>(map: &DashMap<K, V>) -> Result<K, String> {
    return keys_by_value_descending(map).into_iter().next().ok_or_else(|| "hashmap_max_by_value: the hashmap is empty, so no key holds the largest value".to_string());
}

/// The key holding the smallest value, or an error when the hashmap is empty.
/// With several keys tied for smallest, the first in the keys' own order wins.
pub fn min_by_value<K: Hash + Eq + Clone + Ord, V: Clone + PartialOrd>(map: &DashMap<K, V>) -> Result<K, String> {
    return keys_by_value(map).into_iter().next().ok_or_else(|| "hashmap_min_by_value: the hashmap is empty, so no key holds the smallest value".to_string());
}

/// The values added together. An empty hashmap totals zero, the way an empty
/// list does.
pub fn sum_values<K: Hash + Eq + Clone, V: Clone + std::iter::Sum<V>>(map: &DashMap<K, V>) -> V {
    return map.iter().map(|entry| entry.value().clone()).sum();
}

/// The hashmap turned around, so what were the values are the keys. For this
/// to mean anything the values must be as unique as the keys were: where two
/// keys hold the same value, only one of them survives, and which one is not
/// defined. It is for the lookups that go both ways, a code and its name.
pub fn invert<K: Hash + Eq + Clone, V: Hash + Eq + Clone>(map: &DashMap<K, V>) -> DashMap<V, K> {
    let turned = DashMap::new();
    for entry in map.iter() {
        turned.insert(entry.value().clone(), entry.key().clone());
    }
    return turned;
}

/// A new hashmap holding only the named keys. A name that is not in the
/// hashmap is simply not in the answer, rather than an error, so a fixed list
/// of interesting keys can be used against any hashmap.
pub fn pick<K: Hash + Eq + Clone, V: Clone>(map: &DashMap<K, V>, keys: Vec<K>) -> DashMap<K, V> {
    let chosen = DashMap::new();
    for key in keys {
        if let Some(entry) = map.get(&key) {
            chosen.insert(key, entry.value().clone());
        }
    }
    return chosen;
}

/// A new hashmap holding everything except the named keys - the other half of
/// pick. Dropping a password or a token before something is logged or sent on
/// is what this is for.
pub fn omit<K: Hash + Eq + Clone, V: Clone>(map: &DashMap<K, V>, keys: Vec<K>) -> DashMap<K, V> {
    let dropped: std::collections::HashSet<K> = keys.into_iter().collect();
    let kept = DashMap::new();
    for entry in map.iter() {
        if !dropped.contains(entry.key()) {
            kept.insert(entry.key().clone(), entry.value().clone());
        }
    }
    return kept;
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The counted-words hashmap the ordering tests all work from.
    fn counts() -> DashMap<String, i64> {
        let map = DashMap::new();
        map.insert("pear".to_string(), 2);
        map.insert("apple".to_string(), 5);
        map.insert("fig".to_string(), 2);
        map.insert("plum".to_string(), 9);
        return map;
    }

    #[test]
    fn the_keys_come_back_in_order_however_they_went_in() {
        assert_eq!(sorted_keys(&counts()), vec!["apple".to_string(), "fig".to_string(), "pear".to_string(), "plum".to_string()]);
        let empty: DashMap<String, i64> = DashMap::new();
        assert_eq!(sorted_keys(&empty), Vec::<String>::new());
    }

    #[test]
    fn ordering_by_value_breaks_ties_on_the_key_so_two_runs_agree() {
        assert_eq!(keys_by_value(&counts()), vec!["fig".to_string(), "pear".to_string(), "apple".to_string(), "plum".to_string()]);
        assert_eq!(keys_by_value_descending(&counts()), vec!["plum".to_string(), "apple".to_string(), "fig".to_string(), "pear".to_string()]);
    }

    #[test]
    fn the_largest_and_smallest_are_the_ends_of_that_order() {
        assert_eq!(max_by_value(&counts()).expect("a hashmap with entries"), "plum");
        assert_eq!(min_by_value(&counts()).expect("a hashmap with entries"), "fig");

        let empty: DashMap<String, i64> = DashMap::new();
        assert!(max_by_value(&empty).unwrap_err().contains("the hashmap is empty"));
        assert!(min_by_value(&empty).unwrap_err().contains("the hashmap is empty"));
    }

    #[test]
    fn values_add_up_whole_or_fractional() {
        assert_eq!(sum_values(&counts()), 18);
        let empty: DashMap<String, i64> = DashMap::new();
        assert_eq!(sum_values(&empty), 0, "nothing totals zero");

        let prices: DashMap<String, f64> = DashMap::new();
        prices.insert("tea".to_string(), 2.5);
        prices.insert("jam".to_string(), 4.25);
        assert_eq!(sum_values(&prices), 6.75);
    }

    #[test]
    fn turning_a_hashmap_around_swaps_the_keys_and_the_values() {
        let codes: DashMap<String, i64> = DashMap::new();
        codes.insert("not found".to_string(), 404);
        codes.insert("teapot".to_string(), 418);
        let names = invert(&codes);
        assert_eq!(names.len(), 2);
        assert_eq!(names.get(&404).expect("the code").value().clone(), "not found");
        assert_eq!(names.get(&418).expect("the code").value().clone(), "teapot");
    }

    #[test]
    fn a_repeated_value_costs_a_key_when_the_hashmap_is_turned_around() {
        let map: DashMap<String, i64> = DashMap::new();
        map.insert("one".to_string(), 1);
        map.insert("uno".to_string(), 1);
        assert_eq!(invert(&map).len(), 1, "two keys held the same value, so only one survives");
    }

    #[test]
    fn picking_and_omitting_are_the_two_halves_of_the_same_choice() {
        let map = counts();
        let wanted = vec!["apple".to_string(), "plum".to_string(), "ghost".to_string()];

        let chosen = pick(&map, wanted.clone());
        assert_eq!(sorted_keys(&chosen), vec!["apple".to_string(), "plum".to_string()], "a key that is not there is simply not in the answer");
        assert_eq!(chosen.get("apple").expect("a chosen key").value().clone(), 5);

        let kept = omit(&map, wanted);
        assert_eq!(sorted_keys(&kept), vec!["fig".to_string(), "pear".to_string()]);
        assert_eq!(map.len(), 4, "neither one changes the hashmap it read");
    }

    #[test]
    fn incrementing_counts_from_zero() {
        let map: DashMap<String, i64> = DashMap::new();
        assert_eq!(increment(&map, "hit".to_string()), 1);
        assert_eq!(increment(&map, "hit".to_string()), 2);
        assert_eq!(increment(&map, "miss".to_string()), 1);
        assert_eq!(map.get("hit").expect("the key").value().clone(), 2);
    }

    #[test]
    fn adding_to_a_total_can_go_both_ways() {
        let map: DashMap<String, i64> = DashMap::new();
        assert_eq!(add_to(&map, "balance".to_string(), 100), 100);
        assert_eq!(add_to(&map, "balance".to_string(), -30), 70);
    }

    #[test]
    fn two_arrays_pair_up_by_position() {
        let map = from_arrays(vec!["a".to_string(), "b".to_string()], vec![1i64, 2]).expect("matching lengths");
        assert_eq!(map.len(), 2);
        assert_eq!(map.get("b").expect("the key").value().clone(), 2);
    }

    #[test]
    fn mismatched_lengths_are_an_error() {
        let error = from_arrays(vec!["a".to_string()], vec![1i64, 2]).unwrap_err();
        assert!(error.contains("same length"));
    }

    #[test]
    fn a_missing_key_falls_back_without_changing_the_map() {
        let map: DashMap<String, i64> = DashMap::new();
        map.insert("alice".to_string(), 10);
        assert_eq!(get_or(&map, &"alice".to_string(), 0), 10);
        assert_eq!(get_or(&map, &"bob".to_string(), 0), 0);
        assert_eq!(map.len(), 1, "the fallback is returned, not inserted");
    }

    #[test]
    fn a_value_can_be_looked_up_backwards() {
        let map: DashMap<String, i64> = DashMap::new();
        map.insert("alice".to_string(), 10);
        map.insert("bob".to_string(), 20);
        assert_eq!(key_of(&map, &20).expect("a held value"), "bob");
        assert!(key_of(&map, &99).unwrap_err().contains("no key holds"));
    }
}

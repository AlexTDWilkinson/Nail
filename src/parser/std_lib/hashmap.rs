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

#[cfg(test)]
mod tests {
    use super::*;

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

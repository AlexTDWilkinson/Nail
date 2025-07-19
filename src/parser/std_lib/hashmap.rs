use dashmap::DashMap;
use std::hash::Hash;

pub fn new<K: Hash + Eq + Clone, V: Clone>() -> DashMap<K, V> {
    DashMap::new()
}

pub fn insert<K: Hash + Eq + Clone, V: Clone>(map: &DashMap<K, V>, key: K, value: V) -> Option<V> {
    map.insert(key, value)
}

pub fn get<K: Hash + Eq + Clone, V: Clone>(map: &DashMap<K, V>, key: &K) -> Option<V> {
    map.get(key).map(|v| v.clone())
}

pub fn remove<K: Hash + Eq + Clone, V: Clone>(map: &DashMap<K, V>, key: &K) -> Option<V> {
    map.remove(key).map(|(_, v)| v)
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

pub fn from_vec<K: Hash + Eq + Clone, V: Clone>(pairs: Vec<(K, V)>) -> DashMap<K, V> {
    let map = DashMap::new();
    for (key, value) in pairs {
        map.insert(key, value);
    }
    map
}

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
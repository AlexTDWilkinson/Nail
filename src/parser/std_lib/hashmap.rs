use dashmap::DashMap;
use std::hash::Hash;

pub async fn new<K: Hash + Eq + Clone, V: Clone>() -> DashMap<K, V> {
    DashMap::new()
}

pub async fn insert<K: Hash + Eq + Clone, V: Clone>(map: &DashMap<K, V>, key: K, value: V) -> Option<V> {
    map.insert(key, value)
}

pub async fn get<K: Hash + Eq + Clone, V: Clone>(map: &DashMap<K, V>, key: &K) -> Result<V, String> {
    map.get(key)
        .map(|v| v.clone())
        .ok_or_else(|| "Key not found".to_string())
}

pub async fn remove<K: Hash + Eq + Clone, V: Clone>(map: &DashMap<K, V>, key: &K) -> Result<V, String> {
    map.remove(key)
        .map(|(_, v)| v)
        .ok_or_else(|| "Key not found".to_string())
}

pub async fn contains_key<K: Hash + Eq + Clone, V: Clone>(map: &DashMap<K, V>, key: &K) -> bool {
    map.contains_key(key)
}

pub async fn len<K: Hash + Eq + Clone, V: Clone>(map: &DashMap<K, V>) -> i64 {
    map.len() as i64
}

pub async fn is_empty<K: Hash + Eq + Clone, V: Clone>(map: &DashMap<K, V>) -> bool {
    map.is_empty()
}

pub async fn clear<K: Hash + Eq + Clone, V: Clone>(map: &DashMap<K, V>) {
    map.clear()
}

pub async fn keys<K: Hash + Eq + Clone, V: Clone>(map: &DashMap<K, V>) -> Vec<K> {
    map.iter().map(|entry| entry.key().clone()).collect()
}

pub async fn values<K: Hash + Eq + Clone, V: Clone>(map: &DashMap<K, V>) -> Vec<V> {
    map.iter().map(|entry| entry.value().clone()).collect()
}

pub async fn to_vec<K: Hash + Eq + Clone, V: Clone>(map: &DashMap<K, V>) -> Vec<(K, V)> {
    map.iter().map(|entry| (entry.key().clone(), entry.value().clone())).collect()
}

// Generic from_vec that handles tuples
pub async fn from_vec<K: Hash + Eq + Clone, V: Clone>(pairs: Vec<(K, V)>) -> DashMap<K, V> {
    let map = DashMap::new();
    for (key, value) in pairs {
        map.insert(key, value);
    }
    map
}

// Since Nail doesn't have a standard way to handle generic struct-to-tuple conversion,
// the transpiler will need to generate the appropriate conversion code for each use case

pub async fn entry_or_insert<K: Hash + Eq + Clone, V: Clone>(map: &DashMap<K, V>, key: K, default_value: V) -> V {
    map.entry(key).or_insert(default_value).clone()
}

pub async fn merge<K: Hash + Eq + Clone, V: Clone>(map1: &DashMap<K, V>, map2: &DashMap<K, V>) -> DashMap<K, V> {
    let result = DashMap::new();
    
    for entry in map1.iter() {
        result.insert(entry.key().clone(), entry.value().clone());
    }
    
    for entry in map2.iter() {
        result.insert(entry.key().clone(), entry.value().clone());
    }
    
    result
}
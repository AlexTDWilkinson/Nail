//! Named in-memory caches with optional expiry.
//!
//! Nail programs have no global variables, so this is where a web handler
//! keeps something between requests without reaching for a database: caches
//! live for the length of the process and are shared by name. Values are
//! strings - json_serialize anything richer on the way in.

use std::collections::HashMap;
use std::sync::{OnceLock, RwLock};
use std::time::{Duration, Instant};

struct Entry {
    value: String,
    expires: Option<Instant>,
}

impl Entry {
    fn is_expired(&self) -> bool {
        return match self.expires {
            Some(deadline) => Instant::now() >= deadline,
            None => false,
        };
    }
}

static CACHES: OnceLock<RwLock<HashMap<String, HashMap<String, Entry>>>> = OnceLock::new();

fn caches() -> &'static RwLock<HashMap<String, HashMap<String, Entry>>> {
    return CACHES.get_or_init(|| RwLock::new(HashMap::new()));
}

/// Store a value that stays until it is deleted or the process ends.
pub fn set(cache: String, key: String, value: String) {
    let mut all = caches().write().unwrap();
    all.entry(cache).or_default().insert(key, Entry { value, expires: None });
}

/// Store a value that quietly disappears after the given number of seconds.
pub fn set_ttl(cache: String, key: String, value: String, ttl_seconds: i64) {
    let deadline = Instant::now() + Duration::from_secs(ttl_seconds.max(0) as u64);
    let mut all = caches().write().unwrap();
    all.entry(cache).or_default().insert(key, Entry { value, expires: Some(deadline) });
}

/// The stored value; an error when nothing is there or it has expired.
pub fn get(cache: String, key: String) -> Result<String, String> {
    let mut all = caches().write().unwrap();
    if let Some(entries) = all.get_mut(&cache) {
        if let Some(entry) = entries.get(&key) {
            if entry.is_expired() {
                entries.remove(&key);
            } else {
                return Ok(entry.value.clone());
            }
        }
    }
    return Err(format!("cache_get: nothing stored under `{}` in the `{}` cache", key, cache));
}

/// The stored value, or the fallback when nothing is there.
pub fn get_or(cache: String, key: String, fallback: String) -> String {
    return get(cache, key).unwrap_or(fallback);
}

/// Whether a live value is stored under the key.
pub fn has(cache: String, key: String) -> bool {
    return get(cache, key).is_ok();
}

/// Drop one key. Deleting what is not there is fine.
pub fn delete(cache: String, key: String) {
    let mut all = caches().write().unwrap();
    if let Some(entries) = all.get_mut(&cache) {
        entries.remove(&key);
    }
}

/// Drop everything in one cache.
pub fn clear(cache: String) {
    let mut all = caches().write().unwrap();
    all.remove(&cache);
}

/// How many live values a cache holds. Expired entries are swept on the way.
pub fn len(cache: String) -> i64 {
    let mut all = caches().write().unwrap();
    if let Some(entries) = all.get_mut(&cache) {
        entries.retain(|_, entry| !entry.is_expired());
        return entries.len() as i64;
    }
    return 0;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_value_set_is_a_value_got() {
        set("test_basics".to_string(), "greeting".to_string(), "hello".to_string());
        assert_eq!(get("test_basics".to_string(), "greeting".to_string()).unwrap(), "hello");
        assert!(has("test_basics".to_string(), "greeting".to_string()));
        assert_eq!(len("test_basics".to_string()), 1);
    }

    #[test]
    fn missing_keys_read_as_errors_and_fallbacks() {
        assert!(get("test_missing".to_string(), "nope".to_string()).unwrap_err().contains("nothing stored"));
        assert_eq!(get_or("test_missing".to_string(), "nope".to_string(), "default".to_string()), "default");
    }

    #[test]
    fn an_expired_value_is_gone() {
        set_ttl("test_expiry".to_string(), "flash".to_string(), "now".to_string(), 0);
        assert!(get("test_expiry".to_string(), "flash".to_string()).is_err());
        assert_eq!(len("test_expiry".to_string()), 0);
    }

    #[test]
    fn caches_are_separate_and_clearable() {
        set("test_one".to_string(), "k".to_string(), "1".to_string());
        set("test_two".to_string(), "k".to_string(), "2".to_string());
        assert_eq!(get("test_one".to_string(), "k".to_string()).unwrap(), "1");
        clear("test_one".to_string());
        assert!(get("test_one".to_string(), "k".to_string()).is_err());
        assert_eq!(get("test_two".to_string(), "k".to_string()).unwrap(), "2");
        delete("test_two".to_string(), "k".to_string());
        assert!(!has("test_two".to_string(), "k".to_string()));
    }
}

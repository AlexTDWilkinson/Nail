//! Message catalogs for speaking the reader's language. A directory of flat
//! JSON files - en.json, fr.json, de.json - loads once at startup, and
//! i18n_translate picks the right text at request time.

use std::collections::HashMap;
use std::sync::{OnceLock, RwLock};

static CATALOGS: OnceLock<RwLock<HashMap<String, HashMap<String, String>>>> = OnceLock::new();

fn catalogs() -> &'static RwLock<HashMap<String, HashMap<String, String>>> {
    return CATALOGS.get_or_init(|| RwLock::new(HashMap::new()));
}

/// Load every .json catalog in a directory. The file stem is the locale:
/// en.json holds `en`, pt-BR.json holds `pt-BR`. Each file is one flat object
/// of key to text. Returns how many messages were loaded across all locales.
pub fn load(directory: String) -> Result<i64, String> {
    let entries = std::fs::read_dir(&directory).map_err(|e| format!("i18n_load: could not read `{}`: {}", directory, e))?;
    let mut loaded: i64 = 0;
    let mut all = catalogs().write().unwrap();
    for entry in entries {
        let path = entry.map_err(|e| format!("i18n_load: could not read `{}`: {}", directory, e))?.path();
        if path.extension().map(|x| x != "json").unwrap_or(true) {
            continue;
        }
        let locale = path.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
        let text = std::fs::read_to_string(&path).map_err(|e| format!("i18n_load: could not read `{}`: {}", path.display(), e))?;
        let parsed: serde_json::Value = serde_json::from_str(&text).map_err(|e| format!("i18n_load: `{}` is not JSON: {}", path.display(), e))?;
        let object = parsed.as_object().ok_or_else(|| format!("i18n_load: `{}` must be one flat object of key to text", path.display()))?;
        let catalog = all.entry(locale).or_default();
        for (key, value) in object {
            let message = value.as_str().ok_or_else(|| format!("i18n_load: `{}`: the value of `{}` must be text - nested objects are not supported, use dotted keys like `menu.title`", path.display(), key))?;
            catalog.insert(key.clone(), message.to_string());
            loaded += 1;
        }
    }
    if loaded == 0 {
        return Err(format!("i18n_load: `{}` holds no .json catalogs", directory));
    }
    return Ok(loaded);
}

fn lookup(locale: &str, key: &str) -> Option<String> {
    let all = catalogs().read().unwrap();
    if let Some(message) = all.get(locale).and_then(|catalog| catalog.get(key)) {
        return Some(message.clone());
    }
    // pt-BR falls back to pt, and anything still missing falls back to en.
    if let Some((language, _)) = locale.split_once('-') {
        if let Some(message) = all.get(language).and_then(|catalog| catalog.get(key)) {
            return Some(message.clone());
        }
    }
    return all.get("en").and_then(|catalog| catalog.get(key)).cloned();
}

/// The message for a key in a locale. `pt-BR` falls back to `pt`, then to
/// `en`, and a key nobody defines comes back as itself - visible in the page,
/// greppable, and never a crash.
pub fn translate(locale: String, key: String) -> String {
    return lookup(locale.trim(), &key).unwrap_or_else(|| key.clone());
}

/// The message for a count, English-plural style: `<key>.one` when the count
/// is 1, `<key>.other` otherwise, with `{count}` in the text replaced by the
/// number. Falls back like i18n_translate.
pub fn translate_count(locale: String, key: String, count: i64) -> String {
    let form = if count == 1 { format!("{}.one", key) } else { format!("{}.other", key) };
    let message = lookup(locale.trim(), &form).unwrap_or_else(|| format!("{{count}} {}", key));
    return message.replace("{count}", &count.to_string());
}

/// Every locale with a loaded catalog.
pub fn locales() -> Vec<String> {
    let mut names: Vec<String> = catalogs().read().unwrap().keys().cloned().collect();
    names.sort();
    return names;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn load_test_catalogs() {
        let directory = std::env::temp_dir().join(format!("nail_i18n_test_{}", std::process::id()));
        std::fs::create_dir_all(&directory).unwrap();
        std::fs::write(directory.join("en.json"), r#"{"greeting": "Hello", "items.one": "{count} item", "items.other": "{count} items"}"#).unwrap();
        std::fs::write(directory.join("fr.json"), r#"{"greeting": "Bonjour"}"#).unwrap();
        std::fs::write(directory.join("pt.json"), r#"{"greeting": "Olá"}"#).unwrap();
        load(directory.to_string_lossy().to_string()).unwrap();
    }

    #[test]
    fn translation_walks_the_fallback_chain() {
        load_test_catalogs();
        assert_eq!(translate("fr".to_string(), "greeting".to_string()), "Bonjour");
        assert_eq!(translate("pt-BR".to_string(), "greeting".to_string()), "Olá");
        assert_eq!(translate("de".to_string(), "greeting".to_string()), "Hello");
        assert_eq!(translate("fr".to_string(), "missing.key".to_string()), "missing.key");
        assert!(locales().contains(&"fr".to_string()));
    }

    #[test]
    fn counts_pick_their_plural_form() {
        load_test_catalogs();
        assert_eq!(translate_count("en".to_string(), "items".to_string(), 1), "1 item");
        assert_eq!(translate_count("en".to_string(), "items".to_string(), 7), "7 items");
        assert_eq!(translate_count("en".to_string(), "unknown".to_string(), 3), "3 unknown");
    }

    #[test]
    fn an_empty_or_missing_directory_is_an_error() {
        assert!(load("/no/such/dir/anywhere".to_string()).unwrap_err().contains("could not read"));
    }
}

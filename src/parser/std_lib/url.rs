use dashmap::DashMap;

/// URL encode a string
pub fn encode(text: String) -> String {
    urlencoding::encode(&text).to_string()
}

/// URL decode a string
pub fn decode(text: String) -> Result<String, String> {
    urlencoding::decode(&text)
        .map(|s| s.to_string())
        .map_err(|e| format!("url_decode: could not decode '{}': {}", text, e))
}

/// Percent-decoding for one query or form field. Browsers encode a space as
/// '+' in both query strings and form bodies, and percent-decoding alone does
/// not undo that, so it is translated first.
fn decode_field(field: &str) -> String {
    let plus_decoded = field.replace('+', " ");
    urlencoding::decode(&plus_decoded).map(|decoded| decoded.to_string()).unwrap_or(plus_decoded)
}

/// Parse a query string into a hashmap. A POST form body uses this same
/// encoding, so it parses submitted forms too.
pub fn parse_query(query: String) -> DashMap<String, String> {
    let map = DashMap::new();

    // Remove leading ? if present
    let query = query.trim_start_matches('?');

    for pair in query.split('&') {
        if pair.is_empty() {
            continue;
        }

        let parts: Vec<&str> = pair.splitn(2, '=').collect();
        let key = parts[0];
        let value = parts.get(1).unwrap_or(&"");

        map.insert(decode_field(key), decode_field(value));
    }

    map
}

/// Build a query string from a hashmap
pub fn build_query(params: &DashMap<String, String>) -> String {
    let mut parts = Vec::new();
    
    for entry in params.iter() {
        let key = urlencoding::encode(entry.key());
        let value = urlencoding::encode(entry.value());
        parts.push(format!("{}={}", key, value));
    }
    
    parts.join("&")
}
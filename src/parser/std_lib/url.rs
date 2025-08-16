use dashmap::DashMap;
use std::sync::Arc;

/// URL encode a string
pub async fn encode(text: String) -> String {
    urlencoding::encode(&text).to_string()
}

/// URL decode a string
pub async fn decode(text: String) -> Result<String, String> {
    urlencoding::decode(&text)
        .map(|s| s.to_string())
        .map_err(|e| format!("Failed to decode URL: {}", e))
}

/// Parse a query string into a hashmap
pub async fn parse_query(query: String) -> Arc<DashMap<String, String>> {
    let map = Arc::new(DashMap::new());
    
    // Remove leading ? if present
    let query = query.trim_start_matches('?');
    
    for pair in query.split('&') {
        if pair.is_empty() {
            continue;
        }
        
        let parts: Vec<&str> = pair.splitn(2, '=').collect();
        let key = parts[0];
        let value = parts.get(1).unwrap_or(&"");
        
        // Decode both key and value
        if let Ok(decoded_key) = urlencoding::decode(key) {
            if let Ok(decoded_value) = urlencoding::decode(value) {
                map.insert(decoded_key.to_string(), decoded_value.to_string());
            }
        }
    }
    
    map
}

/// Build a query string from a hashmap
pub async fn build_query(params: Arc<DashMap<String, String>>) -> String {
    let mut parts = Vec::new();
    
    for entry in params.iter() {
        let key = urlencoding::encode(entry.key());
        let value = urlencoding::encode(entry.value());
        parts.push(format!("{}={}", key, value));
    }
    
    parts.join("&")
}
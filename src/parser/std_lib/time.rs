use std::time::{SystemTime, UNIX_EPOCH, Duration};
use tokio::time::sleep as tokio_sleep;
use serde::{Deserialize, Serialize};

// TIME_Format enum for parsing and formatting time
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TIME_Format {
    Unix,        // Unix timestamp (seconds since epoch)
    UnixMillis,  // Unix timestamp in milliseconds
    ISO8601,     // ISO 8601 format: 2024-01-15T10:30:00Z
    RFC3339,     // RFC 3339 format (similar to ISO8601)
    RFC2822,     // RFC 2822 format: Mon, 15 Jan 2024 10:30:00 +0000
    Custom(String), // Custom format string (for future extension)
}

// Current Unix timestamp in seconds. Total (never panics): a system clock set
// before 1970 yields a negative timestamp instead of crashing the program.
pub fn now() -> i64 {
    match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(duration) => duration.as_secs() as i64,
        Err(e) => -(e.duration().as_secs() as i64),
    }
}

// time_sleep is already correct in the registry
pub async fn sleep(seconds: f64) {
    let duration = Duration::from_secs_f64(seconds);
    tokio_sleep(duration).await;
}

// time_format is already correct in the registry
pub fn format(timestamp: i64, format: TIME_Format) -> String {
    match format {
        TIME_Format::Unix => timestamp.to_string(),
        TIME_Format::UnixMillis => (timestamp * 1000).to_string(),
        TIME_Format::ISO8601 => {
            // Simple implementation - would use chrono for proper formatting
            format!("{}Z", timestamp)
        },
        TIME_Format::RFC3339 => {
            // Simple implementation
            format!("{}+00:00", timestamp)
        },
        TIME_Format::RFC2822 => {
            // Simple implementation
            format!("Unix: {}", timestamp)
        },
        TIME_Format::Custom(fmt) => {
            format!("Timestamp: {} (format: {})", timestamp, fmt)
        }
    }
}

// These new functions need time_ prefix in the registry
pub fn parse(time_str: String, format: TIME_Format) -> Result<i64, String> {
    match format {
        TIME_Format::Unix => {
            time_str.parse::<i64>()
                .map_err(|_| format!("time_parse: could not parse '{}' as a Unix timestamp", time_str))
        },
        TIME_Format::UnixMillis => {
            time_str.parse::<i64>()
                .map(|ms| ms / 1000)
                .map_err(|_| format!("time_parse: could not parse '{}' as Unix milliseconds", time_str))
        },
        TIME_Format::ISO8601 | TIME_Format::RFC3339 | TIME_Format::RFC2822 => {
            // For now, try to parse as a number if it looks like one
            // In production, would use chrono to parse these formats properly
            time_str.parse::<i64>()
                .map_err(|_| format!("time_parse: could not parse '{}' with format {:?}", time_str, format))
        },
        TIME_Format::Custom(fmt) => {
            // Custom parsing logic would go here
            time_str.parse::<i64>()
                .map_err(|_| format!("time_parse: could not parse '{}' with custom format '{}'", time_str, fmt))
        }
    }
}

pub fn add_seconds(timestamp: i64, seconds: i64) -> i64 {
    timestamp + seconds
}

pub fn diff(t1: i64, t2: i64) -> i64 {
    (t1 - t2).abs()
}

// Current Unix timestamp in milliseconds. Total like now() above.
pub fn now_millis() -> i64 {
    match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(duration) => duration.as_millis() as i64,
        Err(e) => -(e.duration().as_millis() as i64),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_time_format_unix() {
        let timestamp = 1234567890;
        let formatted = format(timestamp, TIME_Format::Unix);
        assert_eq!(formatted, "1234567890");
    }
    
    #[tokio::test]
    async fn test_time_format_unix_millis() {
        let timestamp = 1234567890;
        let formatted = format(timestamp, TIME_Format::UnixMillis);
        assert_eq!(formatted, "1234567890000");
    }
    
    #[tokio::test]
    async fn test_time_format_iso8601() {
        let timestamp = 1234567890;
        let formatted = format(timestamp, TIME_Format::ISO8601);
        assert!(formatted.ends_with("Z"));
        assert!(formatted.contains("1234567890"));
    }
    
    #[tokio::test]
    async fn test_time_format_custom() {
        let timestamp = 1234567890;
        let formatted = format(timestamp, TIME_Format::Custom("MyFormat".to_string()));
        assert!(formatted.contains("1234567890"));
        assert!(formatted.contains("MyFormat"));
    }
    
    #[tokio::test]
    async fn test_time_parse_unix() {
        let result = parse("1234567890".to_string(), TIME_Format::Unix);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 1234567890);
    }
    
    #[tokio::test]
    async fn test_time_parse_unix_invalid() {
        let result = parse("not_a_number".to_string(), TIME_Format::Unix);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("could not parse"));
    }
    
    #[tokio::test]
    async fn test_time_parse_unix_millis() {
        let result = parse("1234567890000".to_string(), TIME_Format::UnixMillis);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 1234567890); // Should convert from millis to seconds
    }
    
    #[tokio::test]
    async fn test_time_parse_custom() {
        let result = parse("1234567890".to_string(), TIME_Format::Custom("MyFormat".to_string()));
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 1234567890);
        
        let result_err = parse("invalid".to_string(), TIME_Format::Custom("MyFormat".to_string()));
        assert!(result_err.is_err());
        let error_message = result_err.unwrap_err();
        assert!(error_message.contains("custom format"));
        assert!(error_message.contains("MyFormat"));
    }
    
    #[tokio::test]
    async fn test_time_add_seconds() {
        let base = 1000;
        let added = add_seconds(base, 3600);
        assert_eq!(added, 4600);
        
        let subtracted = add_seconds(base, -500);
        assert_eq!(subtracted, 500);
    }
    
    #[tokio::test]
    async fn test_time_diff() {
        let t1 = 1000;
        let t2 = 2000;
        assert_eq!(diff(t1, t2), 1000);
        assert_eq!(diff(t2, t1), 1000); // Should be absolute difference
        assert_eq!(diff(t1, t1), 0);
    }
}
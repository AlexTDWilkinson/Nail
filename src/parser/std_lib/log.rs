//! Logging: the running commentary a program leaves behind.
//!
//! `print` is for the answer a program was asked for. Logging is for
//! everything else - what it tried, what was slow, what went wrong - and it
//! goes to standard error so the answer on standard output stays pipeable.
//!
//! Two knobs, both process-wide because a log level that varied by call site
//! would be useless: `log_set_level` drops everything below a threshold, and
//! `log_set_json` switches the line format from something a person reads to
//! something a log collector parses.

use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};

/// How serious a message is. Ordered: setting the level to Warn hides Info and
/// Debug and keeps Warn and Error.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LOG_Level {
    Debug,
    Info,
    Warn,
    Error,
}

impl LOG_Level {
    fn severity(&self) -> u8 {
        match self {
            LOG_Level::Debug => 0,
            LOG_Level::Info => 1,
            LOG_Level::Warn => 2,
            LOG_Level::Error => 3,
        }
    }

    fn label(&self) -> &'static str {
        match self {
            LOG_Level::Debug => "DEBUG",
            LOG_Level::Info => "INFO",
            LOG_Level::Warn => "WARN",
            LOG_Level::Error => "ERROR",
        }
    }
}

/// Messages below this severity are dropped. Info by default, so a program
/// that has not thought about logging still says what it is doing without
/// drowning anyone in Debug.
static MINIMUM_SEVERITY: AtomicU8 = AtomicU8::new(1);
static WRITE_JSON: AtomicBool = AtomicBool::new(false);

/// Hide everything below this level for the rest of the run.
pub fn set_level(level: LOG_Level) {
    MINIMUM_SEVERITY.store(level.severity(), Ordering::Relaxed);
}

/// Write one JSON object per line instead of a human-readable line. What log
/// collectors want, and what makes fields queryable rather than greppable.
pub fn set_json(enabled: bool) {
    WRITE_JSON.store(enabled, Ordering::Relaxed);
}

fn enabled(level: LOG_Level) -> bool {
    return level.severity() >= MINIMUM_SEVERITY.load(Ordering::Relaxed);
}

/// JSON string escaping for the handful of characters that must not appear
/// raw. Local rather than serde_json so the log module costs no dependency -
/// a program that only logs should not pull in a serializer.
fn escape_json(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 2);
    for character in text.chars() {
        match character {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            control if (control as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", control as u32)),
            other => out.push(other),
        }
    }
    return out;
}

/// Writes one line, in whichever format is switched on.
fn emit(level: LOG_Level, message: &str, fields: &[(String, String)]) {
    if !enabled(level) {
        return;
    }

    let timestamp = super::time::now();
    let line = if WRITE_JSON.load(Ordering::Relaxed) {
        let mut out = format!("{{\"time\":{},\"level\":\"{}\",\"message\":\"{}\"", timestamp, level.label(), escape_json(message));
        for (key, value) in fields {
            out.push_str(&format!(",\"{}\":\"{}\"", escape_json(key), escape_json(value)));
        }
        out.push('}');
        out
    } else {
        let mut out = format!("{} {:<5} {}", timestamp, level.label(), message);
        for (key, value) in fields {
            out.push_str(&format!(" {}={}", key, value));
        }
        out
    };

    eprintln!("{}", line);
}

pub fn debug(message: String) {
    emit(LOG_Level::Debug, &message, &[]);
}

pub fn info(message: String) {
    emit(LOG_Level::Info, &message, &[]);
}

pub fn warn(message: String) {
    emit(LOG_Level::Warn, &message, &[]);
}

pub fn error(message: String) {
    emit(LOG_Level::Error, &message, &[]);
}

/// A message plus named values. The fields are what turns a log line from
/// prose into something searchable: `request_id`, `duration_ms`, `path`.
/// They are written in a stable order so two runs of the same code produce
/// comparable lines.
pub fn with_fields(level: LOG_Level, message: String, fields: &dashmap::DashMap<String, String>) {
    if !enabled(level) {
        return;
    }
    let mut pairs: Vec<(String, String)> = fields.iter().map(|entry| (entry.key().clone(), entry.value().clone())).collect();
    pairs.sort_by(|left, right| left.0.cmp(&right.0));
    emit(level, &message, &pairs);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn levels_are_ordered_by_severity() {
        assert!(LOG_Level::Debug.severity() < LOG_Level::Info.severity());
        assert!(LOG_Level::Info.severity() < LOG_Level::Warn.severity());
        assert!(LOG_Level::Warn.severity() < LOG_Level::Error.severity());
    }

    #[test]
    fn escaping_covers_quotes_newlines_and_control_characters() {
        assert_eq!(escape_json("a\"b"), "a\\\"b");
        assert_eq!(escape_json("a\nb"), "a\\nb");
        assert_eq!(escape_json("a\\b"), "a\\\\b");
        assert_eq!(escape_json("a\u{1}b"), "a\\u0001b");
    }

    /// One test for the level filter, because the threshold is process-wide and
    /// concurrent tests would fight over it.
    #[test]
    fn the_level_threshold_hides_quieter_messages() {
        set_level(LOG_Level::Warn);
        assert!(!enabled(LOG_Level::Debug));
        assert!(!enabled(LOG_Level::Info));
        assert!(enabled(LOG_Level::Warn));
        assert!(enabled(LOG_Level::Error));

        set_level(LOG_Level::Debug);
        assert!(enabled(LOG_Level::Debug));

        // Back to the default so nothing else observes the change.
        set_level(LOG_Level::Info);
    }
}

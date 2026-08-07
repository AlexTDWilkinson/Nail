//! Regex module stdlib registry entries

use super::*;

pub(super) fn register(m: &mut HashMap<&'static str, StdlibFunction>) {
    simple_fns! { m, Regex:
        "regex_match" [Regex] => "std_lib::regex::match_pattern", (pattern: s, text: s) -> (b!e),
            "Returns true if the regex pattern matches anywhere in the text. Errors on an invalid pattern.",
            "matched:b = danger(regex_match(`\\d+`, `abc123`));";
        "regex_replace" [Regex] => "std_lib::regex::replace", (pattern: s, text: s, replacement: s) -> (s!e),
            "Replaces all regex matches in the text with the replacement. Errors on an invalid pattern.",
            "clean:s = danger(regex_replace(`\\d+`, `abc123xyz`, `NUM`));";
        "regex_find" [Regex] => "std_lib::regex::find", (pattern: s, text: s) -> (s!e),
            "Returns the first regex match in the text. Errors if the pattern is invalid or nothing matches.",
            "first:s = danger(regex_find(`\\d+`, `abc123xyz`));";
        "regex_find_all" [Regex] => "std_lib::regex::find_all", (pattern: s, text: s) -> ([s]!e),
            "Returns all regex matches in the text. Errors if the pattern is invalid or nothing matches.",
            "numbers:a:s = danger(regex_find_all(`\\d+`, `a1b2c3`));";
        "regex_split" [Regex] => "std_lib::regex::split", (pattern: s, text: s) -> ([s]!e),
            "Splits the text by a regex pattern. Errors on an invalid pattern.",
            "words:a:s = danger(regex_split(`\\s+`, `hello world test`));";
        "regex_captures" [Regex] => "std_lib::regex::captures", (pattern: s, text: s) -> ([s]!e),
            "Returns the capture groups of the first match with the whole match first. Errors if the pattern is invalid or nothing matches.",
            "parts:a:s = danger(regex_captures(`(\\d+)-(\\d+)`, `12-34`));";
        "regex_capture_named" [Regex] => "std_lib::regex::capture_named", (pattern: s, text: s, name: s) -> (s!e),
            "Returns one named capture group of the first match, for patterns written with (?<name>...).",
            "year:s = danger(regex_capture_named(`(?<year>\\d{4})`, `2026-08`, `year`));";
        "regex_replace_first" [Regex] => "std_lib::regex::replace_first", (pattern: s, text: s, replacement: s) -> (s!e),
            "Replaces only the first regex match, where regex_replace replaces every one.",
            "once:s = danger(regex_replace_first(`\\d`, `a1b2`, `#`));";
        "regex_count" [Regex] => "std_lib::regex::count", (pattern: s, text: s) -> (i!e),
            "Returns how many times the pattern matches, which may be zero.",
            "digits:i = danger(regex_count(`\\d`, `a1b2`));";
        "regex_is_valid" [Regex] => "std_lib::regex::is_valid", (pattern: (&s)) -> b,
            "Returns true if the text is a usable regex pattern, for checking a search a visitor typed before running it.",
            "search_term:s = `^na+il$`;\nusable:b = regex_is_valid(search_term);";
        "regex_escape" [Regex] => "std_lib::regex::escape", (text: s) -> s,
            "Escapes every regex character in the text so it can be put inside a pattern and match only itself.",
            "search_term:s = `1+1 (exactly)`;\nliteral:s = regex_escape(search_term);";
    }
}

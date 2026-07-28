//! Regex module stdlib registry entries

use super::*;

pub(super) fn register(m: &mut HashMap<&'static str, StdlibFunction>) {
    simple_fns! { m, Regex:
        "regex_match" [Regex] => "std_lib::regex::match_pattern", (pattern: s, text: s) -> (b!e),
            "Returns true if the regex pattern matches anywhere in the text; errors on an invalid pattern.",
            "matched:b = danger(regex_match(`\\d+`, `abc123`));";
        "regex_replace" [Regex] => "std_lib::regex::replace", (pattern: s, text: s, replacement: s) -> (s!e),
            "Replaces all regex matches in the text with the replacement; errors on an invalid pattern.",
            "clean:s = danger(regex_replace(`\\d+`, `abc123xyz`, `NUM`));";
        "regex_find" [Regex] => "std_lib::regex::find", (pattern: s, text: s) -> (s!e),
            "Returns the first regex match in the text; errors if the pattern is invalid or nothing matches.",
            "first:s = danger(regex_find(`\\d+`, `abc123xyz`));";
        "regex_find_all" [Regex] => "std_lib::regex::find_all", (pattern: s, text: s) -> ([s]!e),
            "Returns all regex matches in the text; errors if the pattern is invalid or nothing matches.",
            "all:a:s = danger(regex_find_all(`\\d+`, `a1b2c3`));";
        "regex_split" [Regex] => "std_lib::regex::split", (pattern: s, text: s) -> ([s]!e),
            "Splits the text by a regex pattern; errors on an invalid pattern.",
            "words:a:s = danger(regex_split(`\\s+`, `hello world test`));";
    }
}

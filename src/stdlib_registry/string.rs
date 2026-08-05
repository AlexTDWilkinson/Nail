//! String module stdlib registry entries

use super::*;

pub(super) fn register(m: &mut HashMap<&'static str, StdlibFunction>) {
    simple_fns! { m, String:
        "string_from" => "std_lib::string::from", (value: T) -> s,
            "Converts any value (int, float, bool, struct, etc.) to its string representation.",
            "text:s = string_from(42);";
        "string_from_array_i64" => "std_lib::string::from_array_i64", (array: [i]) -> s,
            "Converts an array of integers to a string like [1, 2, 3].",
            "text:s = string_from_array_i64(numbers);";
        "string_from_array_f64" => "std_lib::string::from_array_f64", (array: [f]) -> s,
            "Converts an array of floats to a string like [1.5, 2.5].",
            "text:s = string_from_array_f64(prices);";
        "string_from_array_string" => "std_lib::string::from_array_string", (array: [s]) -> s,
            "Converts an array of strings to a string like [a, b, c].",
            "text:s = string_from_array_string(names);";
        "string_from_array_bool" => "std_lib::string::from_array_bool", (array: [b]) -> s,
            "Converts an array of booleans to a string like [true, false].",
            "text:s = string_from_array_bool(flags);";
        "string_concat" => "std_lib::string::concat", (strings: [s]) -> s,
            "Concatenates an array of strings into a single string.",
            "joined:s = string_concat([`Hello`, ` `, `World`]);";
        "string_split" => "std_lib::string::split", (input: s, delimiter: s) -> [s],
            "Splits a string into an array of substrings by a delimiter.",
            "parts:a:s = string_split(`a,b,c`, `,`);";
        "string_trim" => "std_lib::string::trim", (input: s) -> s,
            "Removes leading and trailing whitespace.",
            "trimmed:s = string_trim(`  hi  `);";
        "string_contains" => "std_lib::string::contains", (input: (&s), pattern: s) -> b,
            "Returns true if the string contains the given substring.",
            "found:b = string_contains(`hello`, `ell`);";
        "string_replace" => "std_lib::string::replace", (input: s, from: s, to: s) -> s,
            "Replaces every occurrence of a substring with another string.",
            "fixed:s = string_replace(`a-b-c`, `-`, `_`);";
        "string_length" => "std_lib::string::len", (input: (&s)) -> i,
            "Returns the number of characters in the string.",
            "length:i = string_length(`hello`);";
        "string_to_uppercase" => "std_lib::string::to_uppercase", (input: s) -> s,
            "Converts all characters to uppercase.",
            "shout:s = string_to_uppercase(`hi`);";
        "string_to_lowercase" => "std_lib::string::to_lowercase", (input: s) -> s,
            "Converts all characters to lowercase.",
            "quiet:s = string_to_lowercase(`HI`);";
        "string_starts_with" => "std_lib::string::starts_with", (input: (&s), prefix: s) -> b,
            "Returns true if the string starts with the given prefix.",
            "yes:b = string_starts_with(`hello`, `he`);";
        "string_ends_with" => "std_lib::string::ends_with", (input: (&s), suffix: s) -> b,
            "Returns true if the string ends with the given suffix.",
            "yes:b = string_ends_with(`hello`, `lo`);";
        "string_index_of" => "std_lib::string::index_of", (input: (&s), substring: s) -> (i!e),
            "Returns the index of the first occurrence of a substring, or an error if not found.",
            "index:i = danger(string_index_of(`hello`, `ll`));";
        "string_last_index_of" => "std_lib::string::last_index_of", (input: (&s), substring: s) -> (i!e),
            "Returns the index of the last occurrence of a substring, or an error if not found.",
            "index:i = danger(string_last_index_of(`aXbX`, `X`));";
        "string_substring" => "std_lib::string::substring", (input: s, start: i, end: i) -> (s!e),
            "Returns the substring from start (inclusive) to end (exclusive), or an error if out of bounds.",
            "part:s = danger(string_substring(`hello`, 1, 3));";
        "string_repeat" => "std_lib::string::repeat", (input: s, count: i) -> s,
            "Repeats the string count times.",
            "line:s = string_repeat(`-`, 10);";
        "string_reverse" => "std_lib::string::reverse", (input: s) -> s,
            "Reverses the order of characters in the string.",
            "backwards:s = string_reverse(`abc`);";
        "string_minify" => "std_lib::string::minify", (input: s) -> s,
            "Removes all whitespace outside of quoted strings (useful for minifying JSON).",
            "small:s = string_minify(json_string);";
        "string_join" => "std_lib::string::join", (array: [s], separator: s) -> s,
            "Joins an array of strings with a separator between elements.",
            "csv:s = string_join(names, `, `);";
        "string_chars" => "std_lib::string::chars", (input: s) -> [s],
            "Splits a string into an array of single-character strings.",
            "letters:a:s = string_chars(`abc`);";
        "string_is_empty" => "std_lib::string::is_empty", (input: (&s)) -> b,
            "Returns true if the string has no characters.",
            "empty:b = string_is_empty(``);";
        "string_pad_start" => "std_lib::string::pad_start", (input: s, target_length: i, pad_str: s) -> s,
            "Pads the start of the string with pad_str until it reaches target_length.",
            "padded:s = string_pad_start(`7`, 3, `0`);";
        "string_pad_end" => "std_lib::string::pad_end", (input: s, target_length: i, pad_str: s) -> s,
            "Pads the end of the string with pad_str until it reaches target_length.",
            "padded:s = string_pad_end(`7`, 3, ` `);";
        "string_trim_start" => "std_lib::string::trim_start", (input: s) -> s,
            "Removes leading whitespace.",
            "trimmed:s = string_trim_start(`  hi`);";
        "string_trim_end" => "std_lib::string::trim_end", (input: s) -> s,
            "Removes trailing whitespace.",
            "trimmed:s = string_trim_end(`hi  `);";
        "string_replace_first" => "std_lib::string::replace_first", (input: s, from: s, to: s) -> s,
            "Replaces only the first occurrence of a substring with another string.",
            "fixed:s = string_replace_first(`a-b-c`, `-`, `_`);";
        "string_to_snake_case" => "std_lib::string::to_snake_case", (input: s) -> s,
            "Converts a string to snake_case.",
            "snake:s = string_to_snake_case(`helloWorld`);";
        "string_to_kebab_case" => "std_lib::string::to_kebab_case", (input: s) -> s,
            "Converts a string to kebab-case.",
            "kebab:s = string_to_kebab_case(`helloWorld`);";
        "string_to_title_case" => "std_lib::string::to_title_case", (input: s) -> s,
            "Capitalizes the first letter of every word.",
            "title:s = string_to_title_case(`hello world`);";
        "string_to_sentence_case" => "std_lib::string::to_sentence_case", (input: s) -> s,
            "Capitalizes only the first letter of the string, lowercasing the rest.",
            "sentence:s = string_to_sentence_case(`HELLO WORLD`);";
        "string_split_lines" => "std_lib::string::split_lines", (input: s) -> [s],
            "Splits a string into an array of lines.",
            "lines:a:s = string_split_lines(file_content);";
        "string_split_whitespace" => "std_lib::string::split_whitespace", (input: s) -> [s],
            "Splits a string on runs of whitespace, dropping empty entries.",
            "words:a:s = string_split_whitespace(`a  b c`);";
        "string_count" => "std_lib::string::count", (input: (&s), substring: s) -> i,
            "Counts non-overlapping occurrences of a substring.",
            "hits:i = string_count(`banana`, `an`);";
        "string_capitalize" => "std_lib::string::capitalize", (input: s) -> s,
            "Uppercases the first character of the string.",
            "name:s = string_capitalize(`alice`);";
        "string_slice" => "std_lib::string::slice", (input: s, start: i, end: i) -> (s!e),
            "Returns the substring from start (inclusive) to end (exclusive), or an error if out of bounds.",
            "part:s = danger(string_slice(`hello`, 0, 2));";
        "string_is_alphabetic" => "std_lib::string::is_alphabetic", (input: (&s)) -> b,
            "Returns true if the string is non-empty and contains only alphabetic characters.",
            "alpha:b = string_is_alphabetic(`abc`);";
        "string_is_digits_only" => "std_lib::string::is_digits_only", (input: (&s)) -> b,
            "Returns true if the string is non-empty and contains only digits 0-9.",
            "digits:b = string_is_digits_only(`123`);";
        "string_is_alphanumeric" => "std_lib::string::is_alphanumeric", (input: (&s)) -> b,
            "Returns true if the string is non-empty and contains only letters and digits.",
            "alnum:b = string_is_alphanumeric(`abc123`);";
        "string_is_numeric" => "std_lib::string::is_numeric", (input: (&s)) -> b,
            "Returns true if the string parses as a number (including floats and negatives).",
            "numeric:b = string_is_numeric(`-1.5`);";
        "string_escape_html" => "std_lib::string::escape_html", (text: (&s)) -> s,
            "Escapes &, <, >, \" and ' so text a visitor supplied can be put in a page without becoming markup.",
            "safe_name:s = string_escape_html(player_name);";
        "string_unescape_html" => "std_lib::string::unescape_html", (text: s) -> s,
            "Turns HTML entities such as &amp;lt; and &amp;#39; back into the characters they stood for.",
            "plain:s = string_unescape_html(comment_html);";
        "string_to_camel_case" => "std_lib::string::to_camel_case", (input: s) -> s,
            "Converts to camelCase, the spelling JSON keys and JavaScript APIs use.",
            "key:s = string_to_camel_case(`user id number`);";
        "string_to_pascal_case" => "std_lib::string::to_pascal_case", (input: s) -> s,
            "Converts to PascalCase, the spelling type names use.",
            "type_name:s = string_to_pascal_case(`user account`);";
        "string_slugify" => "std_lib::string::slugify", (input: s) -> s,
            "Turns a title into a URL part: lowercase words joined by single hyphens.",
            "slug:s = string_slugify(`Hello, World!`);";
        "string_truncate" => "std_lib::string::truncate", (input: s, max_length: i, ellipsis: s) -> s,
            "Cuts text to a maximum length, counting the ellipsis as part of that length.",
            "summary:s = string_truncate(article, 80, `...`);";
        "string_word_wrap" => "std_lib::string::word_wrap", (input: s, width: i) -> s,
            "Breaks text into lines no wider than the given number of characters, splitting between words.",
            "paragraph:s = string_word_wrap(notice, 72);";
        "string_levenshtein" => "std_lib::string::levenshtein", (first: (&s), second: (&s)) -> i,
            "Returns how many single-character edits turn one string into the other.",
            "distance:i = string_levenshtein(`kitten`, `sitting`);";
        "string_similarity" => "std_lib::string::similarity", (first: (&s), second: (&s)) -> f,
            "Returns how alike two strings are, from 0.0 for nothing in common to 1.0 for identical.",
            "score:f = string_similarity(`transpile`, `transpiles`);";
        "string_closest" => "std_lib::string::closest", (input: s, candidates: [s]) -> (s!e),
            "Returns the candidate most like the input, for answering a typo with a suggestion, or an error if there are no candidates.",
            "suggestion:s = danger(string_closest(typed, commands));";
        "string_word_count" => "std_lib::string::word_count", (input: (&s)) -> i,
            "Returns how many whitespace-separated words the text holds.",
            "words:i = string_word_count(essay);";
        "string_indent" => "std_lib::string::indent", (input: s, prefix: s) -> s,
            "Puts the prefix in front of every non-blank line.",
            "quoted:s = string_indent(reply, `> `);";
        "string_dedent" => "std_lib::string::dedent", (input: s) -> s,
            "Removes the leading whitespace every non-blank line shares, keeping the relative shape.",
            "flush:s = string_dedent(block);";
        "string_normalize_whitespace" => "std_lib::string::normalize_whitespace", (input: s) -> s,
            "Collapses every run of whitespace to one space and trims the ends.",
            "clean:s = string_normalize_whitespace(scraped_text);";
        "string_trim_chars" => "std_lib::string::trim_chars", (input: s, characters: s) -> s,
            "Removes any of the given characters from both ends, the way string_trim removes whitespace.",
            "path:s = string_trim_chars(`/blog/`, `/`);";
        "string_trim_start_chars" => "std_lib::string::trim_start_chars", (input: s, characters: s) -> s,
            "Removes any of the given characters from the start only.",
            "digits:s = string_trim_start_chars(`00042`, `0`);";
        "string_trim_end_chars" => "std_lib::string::trim_end_chars", (input: s, characters: s) -> s,
            "Removes any of the given characters from the end only.",
            "sentence:s = string_trim_end_chars(`wait...`, `.`);";
        "string_split_once" => "std_lib::string::split_once", (input: s, separator: s) -> ([s]!e),
            "Splits at the first separator only and returns the two halves, so a value containing the separator stays whole; errors if the separator is not there.",
            "pair:a:s = danger(string_split_once(`key=a=b`, `=`));";
        "string_split_last" => "std_lib::string::split_last", (input: s, separator: s) -> ([s]!e),
            "Splits at the last separator and returns the two halves; errors if the separator is not there.",
            "pair:a:s = danger(string_split_last(`host:port`, `:`));";
        "string_char_code" => "std_lib::string::char_code", (input: s, index: i) -> (i!e),
            "Returns the Unicode code point of the character at the index, so A is 65; errors if the index is out of bounds.",
            "code:i = danger(string_char_code(`A`, 0));";
        "string_from_char_code" => "std_lib::string::from_char_code", (code: i) -> (s!e),
            "Returns the one-character string for a Unicode code point; errors on a number that is not one.",
            "letter:s = danger(string_from_char_code(65));";
        "string_char_at" => "std_lib::string::char_at", (input: s, index: i) -> (s!e),
            "Returns the single character at the index as a string, or an error if the index is out of bounds.",
            "initial:s = danger(string_char_at(name, 0));";
        "string_common_prefix" => "std_lib::string::common_prefix", (strings: [s]) -> s,
            "Returns the beginning that all the strings share, or the empty string if they share none.",
            "shared:s = string_common_prefix(paths);";
        "string_strip_prefix" => "std_lib::string::strip_prefix", (input: s, prefix: s) -> s,
            "Removes the prefix if the string starts with it, and returns the string unchanged otherwise.",
            "body:s = string_strip_prefix(line, `> `);";
        "string_strip_suffix" => "std_lib::string::strip_suffix", (input: s, suffix: s) -> s,
            "Removes the suffix if the string ends with it, and returns the string unchanged otherwise.",
            "stem:s = string_strip_suffix(file_name, `.nail`);";
        "string_mask" => "std_lib::string::mask", (input: s, visible_tail: i, mask_character: s) -> s,
            "Replaces all but the last few characters, for showing which secret is in use without printing it.",
            "shown:s = string_mask(api_key, 4, `*`);";
    }
}

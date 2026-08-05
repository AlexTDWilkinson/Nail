//! Checking that text is the shape it claims to be.
//!
//! Every program taking input from outside itself asks the same handful of
//! questions - is this an email address, is this a port number, is this a
//! credit card someone typed correctly - and each one is a small pile of
//! fiddly rules that is easy to write almost right. They are written here
//! once, so a program can ask instead of guess.
//!
//! Each function answers with a plain boolean rather than an error, because a
//! failed check is an expected answer for a program that is validating a form,
//! not an exceptional one. What to say about it is the program's business.

/// Whether the text is an email address that could be delivered to. The check
/// is the practical one - one `@`, something before it, and a domain with a dot
/// in it - not the full grammar, which allows quoted spaces and comments that
/// no signup form should be accepting anyway.
pub fn email(text: &String) -> bool {
    let (local, domain) = match text.split_once('@') {
        Some(halves) => halves,
        None => return false,
    };
    if local.is_empty() || local.len() > 64 || domain.is_empty() || domain.len() > 255 {
        return false;
    }
    if text.contains(char::is_whitespace) || text.contains("..") || domain.contains('@') {
        return false;
    }
    if local.starts_with('.') || local.ends_with('.') {
        return false;
    }
    // A domain has to have a dot and a couple of letters after it, or it is a
    // hostname on the local network rather than an address on the internet.
    let last_dot = match domain.rfind('.') {
        Some(position) => position,
        None => return false,
    };
    let extension = &domain[last_dot + 1..];
    if extension.len() < 2 || !extension.chars().all(|character| character.is_ascii_alphabetic()) {
        return false;
    }
    return hostname(&domain.to_string());
}

/// Whether the text is a URL with a scheme and a host. Relative paths are not
/// URLs by this reckoning: the question being asked is whether this can be
/// linked to or fetched on its own.
pub fn url(text: &String) -> bool {
    let (scheme, rest) = match text.split_once("://") {
        Some(halves) => halves,
        None => return false,
    };
    if scheme.is_empty() || !scheme.chars().all(|character| character.is_ascii_alphanumeric() || character == '+' || character == '-' || character == '.') {
        return false;
    }
    if !scheme.chars().next().expect("checked above").is_ascii_alphabetic() {
        return false;
    }
    // The host runs to the first slash, question mark or hash; anything after
    // that is the path and query, which may hold almost anything.
    let host = rest.split(['/', '?', '#']).next().unwrap_or("");
    let host = host.rsplit('@').next().unwrap_or("");
    let host_without_port = match host.rsplit_once(':') {
        Some((before, port_text)) if port_text.chars().all(|character| character.is_ascii_digit()) && !port_text.is_empty() => before,
        _ => host,
    };
    if host_without_port.is_empty() || host_without_port.contains(char::is_whitespace) {
        return false;
    }
    return hostname(&host_without_port.to_string()) || ipv4(&host_without_port.to_string()) || ipv6(&host_without_port.trim_matches(['[', ']']).to_string());
}

/// Whether the text is a hostname: dot-separated labels of letters, digits and
/// hyphens, with no label starting or ending in a hyphen.
pub fn hostname(text: &String) -> bool {
    if text.is_empty() || text.len() > 253 {
        return false;
    }
    // A trailing dot is legal in a fully-qualified name, so it is not a label.
    let name = text.strip_suffix('.').unwrap_or(text);
    return name.split('.').all(|label| {
        !label.is_empty()
            && label.len() <= 63
            && !label.starts_with('-')
            && !label.ends_with('-')
            && label.chars().all(|character| character.is_ascii_alphanumeric() || character == '-')
    });
}

/// Whether the text is a UUID in the usual 8-4-4-4-12 spelling.
pub fn uuid(text: &String) -> bool {
    let groups: Vec<&str> = text.split('-').collect();
    if groups.len() != 5 {
        return false;
    }
    let expected = [8, 4, 4, 4, 12];
    for (group, length) in groups.iter().zip(expected.iter()) {
        if group.len() != *length || !group.chars().all(|character| character.is_ascii_hexdigit()) {
            return false;
        }
    }
    return true;
}

/// Whether the text is an IPv4 address.
pub fn ipv4(text: &String) -> bool {
    return text.parse::<std::net::Ipv4Addr>().is_ok();
}

/// Whether the text is an IPv6 address.
pub fn ipv6(text: &String) -> bool {
    return text.parse::<std::net::Ipv6Addr>().is_ok();
}

/// Whether the number is a port a program could bind or connect to. Zero is
/// refused: the kernel reads it as "pick one for me", which is never what a
/// configuration file means to say.
pub fn port(number: i64) -> bool {
    return (1..=65535).contains(&number);
}

/// Whether the digits pass the Luhn check every card number is built to pass.
/// Spaces and hyphens are ignored, because that is how people type them. This
/// catches a mistyped digit before a payment is attempted; it says nothing
/// about whether the card exists.
pub fn credit_card(text: &String) -> bool {
    let digits: Vec<u32> = text.chars().filter(|character| !character.is_whitespace() && *character != '-').map(|character| character.to_digit(10).unwrap_or(10)).collect();
    if digits.len() < 12 || digits.len() > 19 || digits.iter().any(|digit| *digit > 9) {
        return false;
    }
    let mut total = 0;
    for (position, digit) in digits.iter().rev().enumerate() {
        // Every second digit from the right is doubled, and a doubled digit
        // over nine has nine taken off it - the same as adding its two digits.
        let contribution = if position % 2 == 1 {
            let doubled = digit * 2;
            if doubled > 9 {
                doubled - 9
            } else {
                doubled
            }
        } else {
            *digit
        };
        total += contribution;
    }
    return total % 10 == 0;
}

/// Whether the text is a colour a stylesheet would accept: a hash and then
/// three, six or eight hex digits.
pub fn hex_color(text: &String) -> bool {
    let digits = match text.strip_prefix('#') {
        Some(digits) => digits,
        None => return false,
    };
    return matches!(digits.len(), 3 | 4 | 6 | 8) && digits.chars().all(|character| character.is_ascii_hexdigit());
}

/// Whether the text is a slug: lowercase letters, digits and single hyphens,
/// with no hyphen at either end. What `string_slugify` produces.
pub fn slug(text: &String) -> bool {
    if text.is_empty() || text.starts_with('-') || text.ends_with('-') || text.contains("--") {
        return false;
    }
    return text.chars().all(|character| character.is_ascii_lowercase() || character.is_ascii_digit() || character == '-');
}

/// Whether the text has between the given numbers of characters, counted as
/// characters rather than bytes so a name in any alphabet is measured the same.
pub fn length_between(text: &String, minimum: i64, maximum: i64) -> bool {
    let length = text.chars().count() as i64;
    return length >= minimum && length <= maximum;
}

/// How strong a password is, from 0 to 4. One point each for being long enough
/// to be worth attacking, for lower and upper case, for a digit, and for
/// anything else - and nothing at all for one of the passwords everybody tries
/// first, however it is spelled.
pub fn password_strength(text: &String) -> i64 {
    const ALWAYS_TRIED_FIRST: [&str; 10] = ["password", "123456", "qwerty", "letmein", "welcome", "admin", "iloveyou", "monkey", "dragon", "abc123"];
    let lowered = text.to_lowercase();
    if ALWAYS_TRIED_FIRST.iter().any(|common| lowered == *common) {
        return 0;
    }

    let mut score = 0;
    if text.chars().count() >= 12 {
        score += 1;
    }
    if text.chars().any(|character| character.is_lowercase()) && text.chars().any(|character| character.is_uppercase()) {
        score += 1;
    }
    if text.chars().any(|character| character.is_numeric()) {
        score += 1;
    }
    if text.chars().any(|character| !character.is_alphanumeric()) {
        score += 1;
    }
    // However varied it is, something this short is not a password.
    if text.chars().count() < 8 {
        return score.min(1);
    }
    return score;
}

/// Whether the text is a JSON document. Answered by parsing it, so a document
/// this accepts is one `json_deserialize` can read.
pub fn json(text: &String) -> bool {
    return serde_json::from_str::<serde_json::Value>(text).is_ok();
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text(value: &str) -> String {
        return value.to_string();
    }

    #[test]
    fn an_address_needs_a_local_part_and_a_domain_with_an_extension() {
        assert!(email(&text("alex@houski.ca")));
        assert!(email(&text("first.last+tag@sub.example.co.uk")));
        assert!(!email(&text("alex")));
        assert!(!email(&text("alex@")));
        assert!(!email(&text("@houski.ca")));
        assert!(!email(&text("alex@localhost")));
        assert!(!email(&text("alex@@houski.ca")));
        assert!(!email(&text("alex houski@example.com")));
        assert!(!email(&text("alex..b@example.com")));
        assert!(!email(&text("alex@example.c")));
    }

    #[test]
    fn a_url_needs_a_scheme_and_a_host() {
        assert!(url(&text("https://nail-lang.org")));
        assert!(url(&text("http://127.0.0.1:8080/health?x=1")));
        assert!(url(&text("https://user:pass@example.com/path#top")));
        assert!(!url(&text("/relative/path")));
        assert!(!url(&text("example.com")));
        assert!(!url(&text("https://")));
        assert!(!url(&text("https:// space.com")));
    }

    #[test]
    fn a_hostname_is_labels_of_letters_digits_and_hyphens() {
        assert!(hostname(&text("example.com")));
        assert!(hostname(&text("my-host")));
        assert!(hostname(&text("example.com.")));
        assert!(!hostname(&text("-leading.com")));
        assert!(!hostname(&text("trailing-.com")));
        assert!(!hostname(&text("two..dots")));
        assert!(!hostname(&text("")));
    }

    #[test]
    fn a_uuid_is_recognised_by_its_groups() {
        assert!(uuid(&text("f47ac10b-58cc-4372-a567-0e02b2c3d479")));
        assert!(!uuid(&text("f47ac10b58cc4372a5670e02b2c3d479")));
        assert!(!uuid(&text("f47ac10b-58cc-4372-a567-0e02b2c3d47")));
        assert!(!uuid(&text("g47ac10b-58cc-4372-a567-0e02b2c3d479")));
    }

    #[test]
    fn addresses_of_both_kinds_are_recognised() {
        assert!(ipv4(&text("192.168.0.1")));
        assert!(!ipv4(&text("192.168.0.256")));
        assert!(!ipv4(&text("192.168.0")));
        assert!(ipv6(&text("::1")));
        assert!(ipv6(&text("2001:db8::8a2e:370:7334")));
        assert!(!ipv6(&text("192.168.0.1")));
    }

    #[test]
    fn a_port_is_one_through_sixty_five_thousand() {
        assert!(port(80));
        assert!(port(65535));
        assert!(!port(0));
        assert!(!port(65536));
        assert!(!port(-1));
    }

    #[test]
    fn a_card_number_passes_the_luhn_check_or_it_was_mistyped() {
        // The published test numbers every payment processor documents.
        assert!(credit_card(&text("4242424242424242")));
        assert!(credit_card(&text("4242 4242 4242 4242")));
        assert!(credit_card(&text("5555-5555-5555-4444")));
        assert!(credit_card(&text("378282246310005")));
        assert!(!credit_card(&text("4242424242424243")));
        assert!(!credit_card(&text("1234")));
        assert!(!credit_card(&text("4242424242424l42")));
    }

    #[test]
    fn a_colour_is_a_hash_and_hex_digits() {
        assert!(hex_color(&text("#fff")));
        assert!(hex_color(&text("#1a2b3c")));
        assert!(hex_color(&text("#1a2b3c80")));
        assert!(!hex_color(&text("fff")));
        assert!(!hex_color(&text("#ff")));
        assert!(!hex_color(&text("#gggggg")));
    }

    #[test]
    fn a_slug_is_what_slugify_produces() {
        assert!(slug(&text("hello-world")));
        assert!(slug(&text("nail-1-0-released")));
        assert!(!slug(&text("Hello-World")));
        assert!(!slug(&text("-leading")));
        assert!(!slug(&text("trailing-")));
        assert!(!slug(&text("double--hyphen")));
        assert!(!slug(&text("")));
    }

    #[test]
    fn length_is_counted_in_characters() {
        assert!(length_between(&text("alex"), 2, 20));
        assert!(!length_between(&text("a"), 2, 20));
        assert!(!length_between(&text("this name is far too long"), 2, 20));
        // Four characters, more than four bytes.
        assert!(length_between(&text("héllo"), 5, 5));
    }

    #[test]
    fn password_strength_rises_with_length_and_variety() {
        assert_eq!(password_strength(&text("password")), 0);
        assert_eq!(password_strength(&text("PASSWORD")), 0);
        assert!(password_strength(&text("short1!")) <= 1);
        assert!(password_strength(&text("correcthorsebattery")) >= 1);
        assert_eq!(password_strength(&text("Tr0ubador&horse!")), 4);
    }

    #[test]
    fn a_json_document_is_recognised_by_parsing_it() {
        assert!(json(&text("{\"a\": 1}")));
        assert!(json(&text("[1, 2, 3]")));
        assert!(!json(&text("{a: 1}")));
        assert!(!json(&text("not json")));
    }
}

/// Check a JSON document against a JSON Schema. The answer is the list of
/// problems, one message each with the path where it sits - an empty list
/// means the document passes. The error case is for a schema or document that
/// does not even parse.
#[cfg(feature = "jsonschema")]
pub fn schema(json_text: String, schema_text: String) -> Result<Vec<String>, String> {
    let schema_value: serde_json::Value = serde_json::from_str(&schema_text).map_err(|e| format!("validate_schema: the schema is not JSON: {}", e))?;
    let instance: serde_json::Value = serde_json::from_str(&json_text).map_err(|e| format!("validate_schema: the document is not JSON: {}", e))?;
    let compiled = jsonschema::JSONSchema::compile(&schema_value).map_err(|e| format!("validate_schema: the schema is not a JSON Schema: {}", e))?;
    let problems = match compiled.validate(&instance) {
        Ok(()) => Vec::new(),
        Err(failures) => failures
            .map(|failure| {
                let place = failure.instance_path.to_string();
                if place.is_empty() {
                    failure.to_string()
                } else {
                    format!("{}: {}", place, failure)
                }
            })
            .collect(),
    };
    return Ok(problems);
}

#[cfg(all(test, feature = "jsonschema"))]
mod schema_tests {
    use super::schema;

    const PERSON: &str = r#"{
        "type": "object",
        "required": ["name", "age"],
        "properties": {
            "name": {"type": "string", "minLength": 1},
            "age": {"type": "integer", "minimum": 0}
        }
    }"#;

    #[test]
    fn a_good_document_has_no_problems() {
        let problems = schema(r#"{"name": "Ada", "age": 36}"#.to_string(), PERSON.to_string()).unwrap();
        assert!(problems.is_empty());
    }

    #[test]
    fn each_failure_names_its_place() {
        let problems = schema(r#"{"name": "", "age": -3}"#.to_string(), PERSON.to_string()).unwrap();
        assert_eq!(problems.len(), 2);
        assert!(problems.iter().any(|p| p.contains("/name")));
        assert!(problems.iter().any(|p| p.contains("/age")));
    }

    #[test]
    fn broken_inputs_are_errors_not_problem_lists() {
        assert!(schema("not json".to_string(), PERSON.to_string()).unwrap_err().contains("document is not JSON"));
        assert!(schema("{}".to_string(), "not json".to_string()).unwrap_err().contains("schema is not JSON"));
    }
}

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

/// Collects the digits of a number typed with spaces or hyphens in it, the
/// way people type card numbers. Anything else in the text means it is not a
/// digit string at all, and the answer is None rather than a guess.
fn typed_digits(text: &str) -> Option<Vec<u32>> {
    let mut digits = Vec::new();
    for character in text.chars() {
        if character.is_whitespace() || character == '-' {
            continue;
        }
        match character.to_digit(10) {
            Some(digit) => digits.push(digit),
            None => return None,
        }
    }
    return Some(digits);
}

/// The Luhn check itself: every second digit from the right is doubled, a
/// doubled digit over nine has nine taken off it - the same as adding its two
/// digits - and the total has to land on a multiple of ten.
fn luhn_checksum_passes(digits: &[u32]) -> bool {
    let mut total = 0;
    for (position, digit) in digits.iter().rev().enumerate() {
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

/// Whether the digits pass the Luhn check every card number is built to pass.
/// Spaces and hyphens are ignored, because that is how people type them. This
/// catches a mistyped digit before a payment is attempted; it says nothing
/// about whether the card exists.
pub fn credit_card(text: &String) -> bool {
    let digits = match typed_digits(text) {
        Some(digits) => digits,
        None => return false,
    };
    if digits.len() < 12 || digits.len() > 19 {
        return false;
    }
    return luhn_checksum_passes(&digits);
}

/// The bare Luhn check over any digit string, with spaces and hyphens
/// forgiven. Card numbers are 12 to 19 digits and validate_credit_card checks
/// that too; this one takes IMEIs, Canadian social insurance numbers and every
/// other identifier built on the same checksum, whatever their length.
pub fn luhn(digits: &String) -> bool {
    let collected = match typed_digits(digits) {
        Some(collected) => collected,
        None => return false,
    };
    if collected.is_empty() {
        return false;
    }
    return luhn_checksum_passes(&collected);
}

/// Whether the text is an IBAN: the right length for its country and passing
/// the mod-97 check the format is built on. Spaces are forgiven, since an
/// IBAN is printed in groups of four. Only the common countries are known;
/// a country not in the table is refused rather than half-checked.
pub fn iban(text: &String) -> bool {
    let compact: String = text.chars().filter(|character| !character.is_whitespace()).collect::<String>().to_uppercase();
    if compact.len() < 4 || !compact.chars().all(|character| character.is_ascii_alphanumeric()) {
        return false;
    }
    let country = &compact[..2];
    if !country.chars().all(|character| character.is_ascii_alphabetic()) || !compact[2..4].chars().all(|character| character.is_ascii_digit()) {
        return false;
    }
    // Each country fixes its own length, and a wrong length is the easiest
    // mistake to catch. Canada has no IBAN at all, so it is not here.
    const LENGTHS: [(&str, usize); 16] =
        [("AT", 20), ("BE", 16), ("CH", 21), ("DE", 22), ("DK", 18), ("ES", 24), ("FI", 18), ("FR", 27), ("GB", 22), ("IE", 22), ("IT", 27), ("NL", 18), ("NO", 15), ("PL", 28), ("PT", 25), ("SE", 24)];
    let expected = match LENGTHS.iter().find(|(code, _)| *code == country) {
        Some((_, length)) => *length,
        None => return false,
    };
    if compact.len() != expected {
        return false;
    }
    // The country and check digits move to the end, letters read as 10 to 35,
    // and the whole number has to leave remainder 1 when divided by 97.
    let rearranged = std::format!("{}{}", &compact[4..], &compact[..4]);
    let mut remainder: u64 = 0;
    for character in rearranged.chars() {
        let value = character.to_digit(36).expect("checked alphanumeric above") as u64;
        remainder = if value < 10 { (remainder * 10 + value) % 97 } else { (remainder * 100 + value) % 97 };
    }
    return remainder == 1;
}

/// Whether the text is a MAC address: six pairs of hex digits separated by
/// colons or dashes, the two spellings hardware actually prints.
pub fn mac_address(text: &String) -> bool {
    // One separator or the other, not a mixture.
    let separator = if text.contains(':') { ':' } else { '-' };
    if text.contains(':') && text.contains('-') {
        return false;
    }
    let pairs: Vec<&str> = text.split(separator).collect();
    if pairs.len() != 6 {
        return false;
    }
    return pairs.iter().all(|pair| pair.len() == 2 && pair.chars().all(|character| character.is_ascii_hexdigit()));
}

/// Whether the text could be a phone number: 7 to 15 digits once the +,
/// spaces, dashes, parentheses and dots people format numbers with are
/// stripped. The E.164-flavoured sanity check a signup form wants - it says
/// the field holds a number, not that anyone answers it.
pub fn phone_loose(text: &String) -> bool {
    let mut digits = 0;
    for character in text.chars() {
        if character.is_ascii_digit() {
            digits += 1;
        } else if !matches!(character, '+' | ' ' | '-' | '(' | ')' | '.') {
            return false;
        }
    }
    return (7..=15).contains(&digits);
}

/// Whether the text is an ISBN with a correct checksum, in either the old
/// ten-digit or the current thirteen-digit shape. Hyphens and spaces are
/// forgiven, since an ISBN is printed with its parts separated.
pub fn isbn(text: &String) -> bool {
    let compact: Vec<char> = text.chars().filter(|character| *character != '-' && !character.is_whitespace()).collect();
    if compact.len() == 10 {
        // Positions weigh 10 down to 1, an X in the last place stands for
        // ten, and the total has to divide by eleven.
        let mut total: u32 = 0;
        for (position, character) in compact.iter().enumerate() {
            let value = if position == 9 && (*character == 'X' || *character == 'x') {
                10
            } else {
                match character.to_digit(10) {
                    Some(digit) => digit,
                    None => return false,
                }
            };
            total += value * (10 - position as u32);
        }
        return total % 11 == 0;
    }
    if compact.len() == 13 {
        // Positions alternate weights one and three, and the total has to
        // divide by ten - the same EAN check as any barcode.
        let mut total: u32 = 0;
        for (position, character) in compact.iter().enumerate() {
            let value = match character.to_digit(10) {
                Some(digit) => digit,
                None => return false,
            };
            total += value * if position % 2 == 0 { 1 } else { 3 };
        }
        return total % 10 == 0;
    }
    return false;
}

/// The loose shape of a UK postcode: one or two letters, a digit, possibly
/// one more digit or letter, then a digit and two letters. The real rules
/// about which letters go where are the Royal Mail's problem, not a form's.
fn uk_postcode(code: &str) -> bool {
    let compact: String = code.replace(' ', "").to_uppercase();
    let characters: Vec<char> = compact.chars().collect();
    if !(5..=7).contains(&characters.len()) {
        return false;
    }
    let (outward, inward) = characters.split_at(characters.len() - 3);
    if !inward[0].is_ascii_digit() || !inward[1].is_ascii_alphabetic() || !inward[2].is_ascii_alphabetic() {
        return false;
    }
    if !outward[0].is_ascii_alphabetic() {
        return false;
    }
    let mut position = 1;
    if position < outward.len() && outward[position].is_ascii_alphabetic() {
        position += 1;
    }
    if position >= outward.len() || !outward[position].is_ascii_digit() {
        return false;
    }
    position += 1;
    return outward.len() - position <= 1 && outward[position..].iter().all(|character| character.is_ascii_alphanumeric());
}

/// A country whose postal code shape validate_postal_code knows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum VALIDATE_Country {
    UnitedStates,
    Canada,
    UnitedKingdom,
    Germany,
    France,
    Netherlands,
    Australia,
}

/// Whether the text is a postal code shaped the way the given country shapes
/// them. The country is a VALIDATE_Country variant, so there is no unknown
/// country to have an opinion about, and the answer is a plain boolean.
pub fn postal_code(text: &String, country: VALIDATE_Country) -> bool {
    let code = text.trim();
    let characters: Vec<char> = code.chars().collect();
    let all_digits = |chars: &[char]| chars.iter().all(|character| character.is_ascii_digit());

    return match country {
        // 12345, with the optional four-digit ZIP+4 tail.
        VALIDATE_Country::UnitedStates => (characters.len() == 5 && all_digits(&characters)) || (characters.len() == 10 && all_digits(&characters[..5]) && characters[5] == '-' && all_digits(&characters[6..])),
        // A1A 1A1, with the space optional because people leave it out.
        VALIDATE_Country::Canada => {
            let compact: Vec<char> = characters.iter().filter(|character| **character != ' ').copied().collect();
            compact.len() == 6 && compact.iter().enumerate().all(|(position, character)| if position % 2 == 0 { character.is_ascii_alphabetic() } else { character.is_ascii_digit() })
        }
        VALIDATE_Country::UnitedKingdom => uk_postcode(code),
        VALIDATE_Country::Germany | VALIDATE_Country::France => characters.len() == 5 && all_digits(&characters),
        // 1234 AB, with the space optional.
        VALIDATE_Country::Netherlands => {
            let compact: Vec<char> = characters.iter().filter(|character| **character != ' ').copied().collect();
            compact.len() == 6 && all_digits(&compact[..4]) && compact[4..].iter().all(|character| character.is_ascii_alphabetic())
        }
        VALIDATE_Country::Australia => characters.len() == 4 && all_digits(&characters),
    };
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

    #[test]
    fn the_bare_luhn_check_takes_any_digit_string() {
        assert!(luhn(&text("4539578763621486")));
        assert!(luhn(&text("4539 5787 6362-1486")));
        // A valid IMEI - fifteen digits, outside any card's shape.
        assert!(luhn(&text("490154203237518")));
        // Short identifiers still checksum.
        assert!(luhn(&text("79927398713")));
        assert!(!luhn(&text("4539578763621487")));
        assert!(!luhn(&text("490154203237519")));
        assert!(!luhn(&text("4539a578763621486")));
        assert!(!luhn(&text("")));
        assert!(!luhn(&text(" - ")));
    }

    #[test]
    fn an_iban_passes_the_mod_97_check_at_its_country_length() {
        assert!(iban(&text("DE89370400440532013000")));
        assert!(iban(&text("DE89 3704 0044 0532 0130 00")));
        assert!(iban(&text("de89370400440532013000")));
        assert!(iban(&text("GB29NWBK60161331926819")));
        // One digit off fails the checksum.
        assert!(!iban(&text("DE89370400440532013001")));
        // The right checksum at the wrong length is still wrong.
        assert!(!iban(&text("DE8937040044053201300")));
        // Canada has no IBAN, and made-up countries have none either.
        assert!(!iban(&text("CA89370400440532013000")));
        assert!(!iban(&text("XX89370400440532013000")));
        assert!(!iban(&text("DE8937040044053201300!")));
        assert!(!iban(&text("")));
    }

    #[test]
    fn a_mac_address_is_six_pairs_with_one_separator() {
        assert!(mac_address(&text("00:1A:2B:3C:4D:5E")));
        assert!(mac_address(&text("00-1a-2b-3c-4d-5e")));
        assert!(!mac_address(&text("00:1A-2B:3C:4D:5E")));
        assert!(!mac_address(&text("00:1A:2B:3C:4D")));
        assert!(!mac_address(&text("00:1A:2B:3C:4D:5E:6F")));
        assert!(!mac_address(&text("00:1A:2B:3C:4D:GG")));
        assert!(!mac_address(&text("001A2B3C4D5E")));
    }

    #[test]
    fn a_phone_number_is_seven_to_fifteen_digits_under_its_formatting() {
        assert!(phone_loose(&text("+1 (403) 555-0123")));
        assert!(phone_loose(&text("555-0123")));
        assert!(phone_loose(&text("+441632960961")));
        assert!(phone_loose(&text("1.403.555.0123")));
        assert!(!phone_loose(&text("555-012")));
        assert!(!phone_loose(&text("1234567890123456")));
        assert!(!phone_loose(&text("555-CALL")));
        assert!(!phone_loose(&text("")));
    }

    #[test]
    fn an_isbn_checksums_in_both_its_shapes() {
        assert!(isbn(&text("0306406152")));
        assert!(isbn(&text("0-306-40615-2")));
        assert!(isbn(&text("9780306406157")));
        assert!(isbn(&text("978-0-306-40615-7")));
        assert!(isbn(&text("978 0 306 40615 7")));
        // An X in the last place of an ISBN-10 stands for ten.
        assert!(isbn(&text("097522980X")));
        assert!(isbn(&text("097522980x")));
        assert!(!isbn(&text("0306406153")));
        assert!(!isbn(&text("9780306406158")));
        assert!(!isbn(&text("030640615")));
        assert!(!isbn(&text("X306406152")));
        assert!(!isbn(&text("")));
    }

    fn postal(code: &str, country: VALIDATE_Country) -> bool {
        return postal_code(&text(code), country);
    }

    #[test]
    fn each_country_recognises_its_own_postal_codes() {
        assert!(postal("12345", VALIDATE_Country::UnitedStates));
        assert!(postal("12345-6789", VALIDATE_Country::UnitedStates));
        assert!(!postal("1234", VALIDATE_Country::UnitedStates));
        assert!(!postal("12345-678", VALIDATE_Country::UnitedStates));
        assert!(postal("T2N 1N4", VALIDATE_Country::Canada));
        assert!(postal("t2n 1n4", VALIDATE_Country::Canada));
        assert!(postal("T2N1N4", VALIDATE_Country::Canada));
        assert!(!postal("T2N-1N4", VALIDATE_Country::Canada));
        assert!(!postal("123 456", VALIDATE_Country::Canada));
        assert!(postal("SW1A 1AA", VALIDATE_Country::UnitedKingdom));
        assert!(postal("sw1a 1aa", VALIDATE_Country::UnitedKingdom));
        assert!(postal("M1 1AE", VALIDATE_Country::UnitedKingdom));
        assert!(postal("EC1A1BB", VALIDATE_Country::UnitedKingdom));
        assert!(!postal("12345", VALIDATE_Country::UnitedKingdom));
        assert!(postal("10115", VALIDATE_Country::Germany));
        assert!(postal("75008", VALIDATE_Country::France));
        assert!(!postal("1011", VALIDATE_Country::Germany));
        assert!(postal("1234 AB", VALIDATE_Country::Netherlands));
        assert!(postal("1234ab", VALIDATE_Country::Netherlands));
        assert!(!postal("12345", VALIDATE_Country::Netherlands));
        assert!(postal("2000", VALIDATE_Country::Australia));
        assert!(!postal("200", VALIDATE_Country::Australia));
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

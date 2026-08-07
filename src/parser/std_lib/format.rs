//! Turning numbers into the text a person reads.
//!
//! Nail has no format strings - `string_from` gives the debug spelling of a
//! value and nothing more - so every program that shows a price, a file size
//! or a count has been writing its own rounding and comma insertion. That is
//! the same work every time and it is easy to get subtly wrong, so it lives
//! here once.
//!
//! Everything in this module returns text meant for a reader, never text meant
//! to be parsed again. Round trips go through `int_from` and `float_from`.

/// Inserts thousands separators into the digits of an already-formatted number.
/// Works on the integer part only, which is why it takes the whole rendered
/// number and finds the decimal point itself.
fn group_digits(rendered: &str, separator: char) -> String {
    let (sign, unsigned) = match rendered.strip_prefix('-') {
        Some(rest) => ("-", rest),
        None => ("", rendered),
    };
    let (whole, fraction) = match unsigned.split_once('.') {
        Some((whole, fraction)) => (whole, Some(fraction)),
        None => (unsigned, None),
    };

    let digits: Vec<char> = whole.chars().collect();
    let mut grouped = String::with_capacity(whole.len() + whole.len() / 3);
    for (position, digit) in digits.iter().enumerate() {
        if position > 0 && (digits.len() - position) % 3 == 0 {
            grouped.push(separator);
        }
        grouped.push(*digit);
    }

    let mut out = String::with_capacity(sign.len() + grouped.len() + 1 + fraction.map_or(0, |f| f.len()));
    out.push_str(sign);
    out.push_str(&grouped);
    if let Some(fraction) = fraction {
        out.push('.');
        out.push_str(fraction);
    }
    return out;
}

/// A float rounded to a fixed number of decimal places, always showing them
/// all: `decimals(3.5, 2)` is `3.50`, not `3.5`. Prices and measurements line
/// up in a column this way.
pub fn decimals(value: f64, places: i64) -> String {
    let places = places.clamp(0, 17) as usize;
    return format!("{:.*}", places, value);
}

/// An integer with thousands separators: `1234567` becomes `1,234,567`.
pub fn thousands(value: i64) -> String {
    return group_digits(&value.to_string(), ',');
}

/// A float with both thousands separators and fixed decimals, which together
/// are what a money column looks like: `1234.5` at 2 places is `1,234.50`.
pub fn thousands_float(value: f64, places: i64) -> String {
    return group_digits(&decimals(value, places), ',');
}

/// An amount of money: the symbol, then the grouped digits, then two decimals.
/// The symbol goes in front because that is where every currency Nail is
/// likely to be printing one puts it; a program needing another arrangement
/// can concatenate its own.
pub fn currency(amount: f64, symbol: String) -> String {
    let rendered = thousands_float(amount.abs(), 2);
    let sign = if amount < 0.0 { "-" } else { "" };
    return format!("{}{}{}", sign, symbol, rendered);
}

/// A fraction as a percentage: `0.125` at one place is `12.5%`. The input is a
/// fraction rather than an already-multiplied number because that is what
/// dividing two counts gives you.
pub fn percent(fraction: f64, places: i64) -> String {
    return format!("{}%", decimals(fraction * 100.0, places));
}

/// A byte count in the largest unit that leaves a number below 1024, the way
/// `ls -h` shows it. The steps are 1024 rather than 1000 because these are
/// bytes on a disk; a plain count of things wants `compact` instead.
pub fn bytes(count: i64) -> String {
    const UNITS: [&str; 6] = ["B", "KB", "MB", "GB", "TB", "PB"];
    let negative = count < 0;
    let mut size = count.unsigned_abs() as f64;
    let mut unit = 0;
    while size >= 1024.0 && unit + 1 < UNITS.len() {
        size /= 1024.0;
        unit += 1;
    }
    let sign = if negative { "-" } else { "" };
    // Whole bytes have no fractional part worth showing - "512 B", not "512.0 B".
    if unit == 0 {
        return format!("{}{} {}", sign, size as i64, UNITS[unit]);
    }
    return format!("{}{} {}", sign, decimals(size, 1), UNITS[unit]);
}

/// A large count shortened for a headline: `1200` is `1.2k`, `3_400_000` is
/// `3.4M`. Steps of 1000, and no decimal when the number is already small.
pub fn compact(value: i64) -> String {
    const UNITS: [&str; 5] = ["", "k", "M", "B", "T"];
    let negative = value < 0;
    let magnitude = value.unsigned_abs();
    let sign = if negative { "-" } else { "" };
    if magnitude < 1000 {
        return format!("{}{}", sign, magnitude);
    }
    let mut size = magnitude as f64;
    let mut unit = 0;
    while size >= 1000.0 && unit + 1 < UNITS.len() {
        size /= 1000.0;
        unit += 1;
    }
    return format!("{}{}{}", sign, decimals(size, 1), UNITS[unit]);
}

/// The English ordinal for a number: 1st, 2nd, 3rd, 4th, and the teens that
/// break the pattern - 11th, 12th, 13th - handled as the exceptions they are.
pub fn ordinal(number: i64) -> String {
    let magnitude = number.unsigned_abs();
    let suffix = match (magnitude % 100, magnitude % 10) {
        (11 | 12 | 13, _) => "th",
        (_, 1) => "st",
        (_, 2) => "nd",
        (_, 3) => "rd",
        _ => "th",
    };
    return format!("{}{}", number, suffix);
}

/// A count with the right form of the word after it: `plural(1, "file",
/// "files")` is `1 file`. Both forms are given rather than guessed at, because
/// English guesses wrongly often enough - one mouse, two mice.
pub fn plural(count: i64, singular: String, plural_form: String) -> String {
    if count == 1 || count == -1 {
        return format!("{} {}", count, singular);
    }
    return format!("{} {}", count, plural_form);
}

/// Items written as a sentence would write them: `a, b and c`, with the given
/// conjunction. Two items get no comma; one item is itself; none is empty.
pub fn list(items: Vec<String>, conjunction: String) -> String {
    match items.len() {
        0 => return String::new(),
        1 => return items[0].clone(),
        2 => return format!("{} {} {}", items[0], conjunction, items[1]),
        _ => {}
    }
    let (last, leading) = items.split_last().expect("more than two items");
    return format!("{} {} {}", leading.join(", "), conjunction, last);
}

/// A number from 1 to 3999 as a Roman numeral: 1994 is `MCMXCIV`. Outside that
/// range is an error, because Roman numerals have no zero, no negatives, and
/// no standard way past MMMCMXCIX.
pub fn roman(value: i64) -> Result<String, String> {
    if !(1..=3999).contains(&value) {
        return Err(format!("format_roman: {} is outside 1 to 3999, the range Roman numerals can write", value));
    }
    const NUMERALS: [(i64, &str); 13] =
        [(1000, "M"), (900, "CM"), (500, "D"), (400, "CD"), (100, "C"), (90, "XC"), (50, "L"), (40, "XL"), (10, "X"), (9, "IX"), (5, "V"), (4, "IV"), (1, "I")];
    let mut remaining = value;
    let mut numeral = String::new();
    for (step, letters) in NUMERALS {
        while remaining >= step {
            numeral.push_str(letters);
            remaining -= step;
        }
    }
    return Ok(numeral);
}

/// A duration in seconds as clock digits: `m:ss` under an hour, `h:mm:ss` from
/// there, so 125 is `2:05` and 3661 is `1:01:01`. A negative duration gets a
/// leading minus.
pub fn clock(seconds: i64) -> String {
    let sign = if seconds < 0 { "-" } else { "" };
    let total = seconds.unsigned_abs();
    let hours = total / 3600;
    let minutes = (total % 3600) / 60;
    let rest = total % 60;
    if hours == 0 {
        return format!("{}{}:{:02}", sign, minutes, rest);
    }
    return format!("{}{}:{:02}:{:02}", sign, hours, minutes, rest);
}

/// A number rounded to the given significant figures for display: `1234.5` at
/// 2 figures is `1200`, and `0.00012345` at 3 is `0.000123`. Trailing zeros
/// inside the figures are kept - `1.5` at 3 figures is `1.50` - because they
/// are part of what the figures claim. Figures outside 1 to 12 are an error.
pub fn significant(value: f64, figures: i64) -> Result<String, String> {
    if !(1..=12).contains(&figures) {
        return Err(format!("format_significant: {} is not a number of significant figures between 1 and 12", figures));
    }
    if !value.is_finite() {
        return Err(format!("format_significant: {} is not a finite number", value));
    }
    if value == 0.0 {
        return Ok("0".to_string());
    }

    // Rust's scientific formatting does the rounding, including the carry that
    // turns 9.99 at two figures into 10; this only moves the point back.
    let scientific = format!("{:.*e}", (figures - 1) as usize, value);
    let (mantissa, exponent) = scientific.split_once('e').expect("scientific notation always has an e");
    let exponent: i64 = exponent.parse().expect("the exponent of a float is a whole number");
    let digits: String = mantissa.chars().filter(|character| character.is_ascii_digit()).collect();

    // How many of the digits sit left of the decimal point.
    let whole = exponent + 1;
    let unsigned = if whole <= 0 {
        format!("0.{}{}", "0".repeat(-whole as usize), digits)
    } else if whole as usize >= digits.len() {
        format!("{}{}", digits, "0".repeat(whole as usize - digits.len()))
    } else {
        format!("{}.{}", &digits[..whole as usize], &digits[whole as usize..])
    };
    if mantissa.starts_with('-') {
        return Ok(format!("-{}", unsigned));
    }
    return Ok(unsigned);
}

/// A North American phone number formatted the standard way: ten digits as
/// `(780) 555-0100`. Formatting characters already in the input are forgiven,
/// and so is an eleventh leading 1, the long-distance prefix, which is
/// dropped. Any other count of digits is an error saying how many were found.
pub fn phone_na(digits: String) -> Result<String, String> {
    let found: Vec<char> = digits.chars().filter(|character| character.is_ascii_digit()).collect();
    let ten: &[char] = if found.len() == 11 && found[0] == '1' { &found[1..] } else { &found[..] };
    if ten.len() != 10 {
        return Err(format!("format_phone_na: a North American number has ten digits and `{}` has {}", digits.trim(), found.len()));
    }
    let text: String = ten.iter().collect();
    return Ok(format!("({}) {}-{}", &text[..3], &text[3..6], &text[6..]));
}

/// The words for a number from 1 to 999, the group the bigger scales repeat.
fn words_under_thousand(n: u64) -> String {
    const ONES: [&str; 20] = [
        "", "one", "two", "three", "four", "five", "six", "seven", "eight", "nine", "ten", "eleven", "twelve", "thirteen", "fourteen", "fifteen", "sixteen",
        "seventeen", "eighteen", "nineteen",
    ];
    const TENS: [&str; 10] = ["", "", "twenty", "thirty", "forty", "fifty", "sixty", "seventy", "eighty", "ninety"];

    let mut out = String::new();
    if n >= 100 {
        out.push_str(ONES[(n / 100) as usize]);
        out.push_str(" hundred");
    }
    let rest = n % 100;
    if rest == 0 {
        return out;
    }
    if !out.is_empty() {
        out.push(' ');
    }
    if rest < 20 {
        out.push_str(ONES[rest as usize]);
    } else {
        out.push_str(TENS[(rest / 10) as usize]);
        if rest % 10 != 0 {
            out.push('-');
            out.push_str(ONES[(rest % 10) as usize]);
        }
    }
    return out;
}

/// A whole number in English words: 42 is `forty-two` and -8000 is `negative
/// eight thousand`. American style with no `and`, so 105 is `one hundred
/// five`, and the tens are hyphenated from twenty-one through ninety-nine.
/// Reaches the quintillions, which is as far as a Nail integer goes.
pub fn number_words(value: i64) -> String {
    if value == 0 {
        return "zero".to_string();
    }
    const SCALES: [&str; 7] = ["", " thousand", " million", " billion", " trillion", " quadrillion", " quintillion"];

    // unsigned_abs keeps the most negative integer honest - its magnitude does
    // not fit back into the signed type, but it fits here.
    let mut remaining = value.unsigned_abs();
    let mut groups: Vec<u64> = Vec::new();
    while remaining > 0 {
        groups.push(remaining % 1000);
        remaining /= 1000;
    }

    let mut parts: Vec<String> = Vec::new();
    if value < 0 {
        parts.push("negative".to_string());
    }
    for (scale, group) in groups.iter().enumerate().rev() {
        if *group == 0 {
            continue;
        }
        parts.push(format!("{}{}", words_under_thousand(*group), SCALES[scale]));
    }
    return parts.join(" ");
}

/// The byte count a size written for people means: `1.5 KB` is 1536, `2 GB` is
/// two gigabytes in bytes, `900` on its own is 900 bytes. This reads back what
/// `format_bytes` writes, and also what a person types into a config file for a
/// size limit, so a program can take `max_upload = 20 MB` and compare it with
/// what `fs_size` returned.
///
/// The steps are 1024, matching `format_bytes`, and `KiB` is accepted as
/// another spelling of `KB`. Case and the space do not matter.
pub fn parse_bytes(text: String) -> Result<i64, String> {
    const UNITS: [(&str, f64); 6] = [("b", 1.0), ("kb", 1024.0), ("mb", 1_048_576.0), ("gb", 1_073_741_824.0), ("tb", 1_099_511_627_776.0), ("pb", 1_125_899_906_842_624.0)];

    let trimmed = text.trim().to_lowercase();
    if trimmed.is_empty() {
        return Err("format_parse_bytes: there is no size in an empty string".to_string());
    }

    let digits_end = trimmed.find(|character: char| !character.is_ascii_digit() && character != '.' && character != ',' && character != '-' && character != '+').unwrap_or(trimmed.len());
    let (number, unit) = trimmed.split_at(digits_end);
    // `1.5 KiB` and `1.5 kb` name the same size - the i is a spelling of the
    // same 1024 step, not a different one.
    let unit = unit.trim().replace("ib", "b");
    let unit = if unit.is_empty() { "b" } else { unit.as_str() };

    let scale = match UNITS.iter().find(|(name, _)| *name == unit) {
        Some((_, scale)) => *scale,
        None => return Err(format!("format_parse_bytes: `{}` is not a size this knows - use B, KB, MB, GB, TB or PB", text.trim())),
    };

    let value: f64 = number.trim().replace(',', "").parse().map_err(|_| format!("format_parse_bytes: `{}` does not start with a number", text.trim()))?;
    let bytes = value * scale;
    if !bytes.is_finite() || bytes.abs() >= i64::MAX as f64 {
        return Err(format!("format_parse_bytes: `{}` is a larger size than a count of bytes can hold", text.trim()));
    }
    // A fraction of a byte is not a thing to have, so the count rounds the way
    // a size written to one decimal place was rounded when it was written.
    return Ok(bytes.round() as i64);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decimals_always_shows_the_places_asked_for() {
        assert_eq!(decimals(3.5, 2), "3.50");
        assert_eq!(decimals(3.567, 2), "3.57");
        assert_eq!(decimals(3.567, 0), "4");
        assert_eq!(decimals(-1.26, 1), "-1.3");
    }

    /// A value sitting exactly on a half goes to the even digit, which is what
    /// Rust's own float formatting does and what avoids a long column of
    /// roundings all drifting the same way.
    #[test]
    fn an_exact_half_rounds_to_the_even_digit() {
        assert_eq!(decimals(1.25, 1), "1.2");
        assert_eq!(decimals(1.35, 1), "1.4");
    }

    #[test]
    fn thousands_groups_from_the_right() {
        assert_eq!(thousands(1), "1");
        assert_eq!(thousands(999), "999");
        assert_eq!(thousands(1000), "1,000");
        assert_eq!(thousands(1234567), "1,234,567");
        assert_eq!(thousands(-1234567), "-1,234,567");
    }

    #[test]
    fn a_grouped_float_keeps_its_decimals_ungrouped() {
        assert_eq!(thousands_float(1234.5, 2), "1,234.50");
        assert_eq!(thousands_float(-1234567.891, 3), "-1,234,567.891");
    }

    #[test]
    fn money_puts_the_symbol_before_the_sign_free_digits() {
        assert_eq!(currency(1234.5, "$".to_string()), "$1,234.50");
        assert_eq!(currency(-9.99, "$".to_string()), "-$9.99");
        assert_eq!(currency(0.0, "€".to_string()), "€0.00");
    }

    #[test]
    fn a_fraction_becomes_a_percentage() {
        assert_eq!(percent(0.125, 1), "12.5%");
        assert_eq!(percent(1.0, 0), "100%");
        assert_eq!(percent(0.0, 2), "0.00%");
    }

    #[test]
    fn byte_counts_step_in_units_of_1024() {
        assert_eq!(bytes(512), "512 B");
        assert_eq!(bytes(1024), "1.0 KB");
        assert_eq!(bytes(1536), "1.5 KB");
        assert_eq!(bytes(1048576), "1.0 MB");
        assert_eq!(bytes(0), "0 B");
        assert_eq!(bytes(-2048), "-2.0 KB");
    }

    #[test]
    fn compact_counts_step_in_units_of_1000() {
        assert_eq!(compact(999), "999");
        assert_eq!(compact(1200), "1.2k");
        assert_eq!(compact(3400000), "3.4M");
        assert_eq!(compact(-1500), "-1.5k");
    }

    #[test]
    fn ordinals_handle_the_teens() {
        assert_eq!(ordinal(1), "1st");
        assert_eq!(ordinal(2), "2nd");
        assert_eq!(ordinal(3), "3rd");
        assert_eq!(ordinal(4), "4th");
        assert_eq!(ordinal(11), "11th");
        assert_eq!(ordinal(12), "12th");
        assert_eq!(ordinal(13), "13th");
        assert_eq!(ordinal(21), "21st");
        assert_eq!(ordinal(111), "111th");
        assert_eq!(ordinal(101), "101st");
    }

    #[test]
    fn plural_picks_the_form_by_count() {
        assert_eq!(plural(1, "file".to_string(), "files".to_string()), "1 file");
        assert_eq!(plural(0, "file".to_string(), "files".to_string()), "0 files");
        assert_eq!(plural(2, "mouse".to_string(), "mice".to_string()), "2 mice");
    }

    #[test]
    fn a_list_reads_as_a_sentence() {
        assert_eq!(list(vec![], "and".to_string()), "");
        assert_eq!(list(vec!["a".to_string()], "and".to_string()), "a");
        assert_eq!(list(vec!["a".to_string(), "b".to_string()], "and".to_string()), "a and b");
        assert_eq!(list(vec!["a".to_string(), "b".to_string(), "c".to_string()], "or".to_string()), "a, b or c");
    }

    /// The subtractive pairs are where hand-rolled Roman numeral code goes
    /// wrong, so each one is pinned.
    #[test]
    fn roman_numerals_handle_the_subtractive_pairs() {
        assert_eq!(roman(1).expect("in range"), "I");
        assert_eq!(roman(4).expect("in range"), "IV");
        assert_eq!(roman(9).expect("in range"), "IX");
        assert_eq!(roman(40).expect("in range"), "XL");
        assert_eq!(roman(90).expect("in range"), "XC");
        assert_eq!(roman(400).expect("in range"), "CD");
        assert_eq!(roman(900).expect("in range"), "CM");
        assert_eq!(roman(1994).expect("in range"), "MCMXCIV");
        assert_eq!(roman(2026).expect("in range"), "MMXXVI");
        assert_eq!(roman(3999).expect("in range"), "MMMCMXCIX");
    }

    #[test]
    fn roman_numerals_stop_at_their_range() {
        assert!(roman(0).unwrap_err().contains("outside 1 to 3999"));
        assert!(roman(4000).unwrap_err().contains("outside 1 to 3999"));
        assert!(roman(-7).unwrap_err().contains("outside 1 to 3999"));
    }

    #[test]
    fn clock_digits_change_shape_at_the_hour() {
        assert_eq!(clock(0), "0:00");
        assert_eq!(clock(59), "0:59");
        assert_eq!(clock(60), "1:00");
        assert_eq!(clock(125), "2:05");
        assert_eq!(clock(3599), "59:59");
        assert_eq!(clock(3600), "1:00:00");
        assert_eq!(clock(3661), "1:01:01");
    }

    #[test]
    fn a_negative_duration_keeps_its_minus() {
        assert_eq!(clock(-125), "-2:05");
        assert_eq!(clock(-3661), "-1:01:01");
    }

    #[test]
    fn significant_figures_round_for_display() {
        assert_eq!(significant(1234.5, 2).expect("valid figures"), "1200");
        assert_eq!(significant(1234.5, 6).expect("valid figures"), "1234.50");
        assert_eq!(significant(0.00012345, 3).expect("valid figures"), "0.000123");
        assert_eq!(significant(-1234.5, 2).expect("valid figures"), "-1200");
        assert_eq!(significant(0.0, 3).expect("valid figures"), "0");
    }

    /// Rounding at the top of a decade carries into an extra digit, and
    /// trailing zeros inside the figures are kept.
    #[test]
    fn significant_figures_carry_and_keep_their_zeros() {
        assert_eq!(significant(9.99, 2).expect("valid figures"), "10");
        assert_eq!(significant(0.999, 2).expect("valid figures"), "1.0");
        assert_eq!(significant(1.5, 3).expect("valid figures"), "1.50");
    }

    #[test]
    fn significant_figures_outside_one_to_twelve_are_an_error() {
        assert!(significant(1.5, 0).unwrap_err().contains("between 1 and 12"));
        assert!(significant(1.5, 13).unwrap_err().contains("between 1 and 12"));
    }

    #[test]
    fn a_phone_number_reads_the_same_however_it_arrived() {
        assert_eq!(phone_na("7805550100".to_string()).expect("ten digits"), "(780) 555-0100");
        assert_eq!(phone_na("780-555-0100".to_string()).expect("ten digits"), "(780) 555-0100");
        assert_eq!(phone_na("(780) 555-0100".to_string()).expect("ten digits"), "(780) 555-0100");
        assert_eq!(phone_na("780.555.0100".to_string()).expect("ten digits"), "(780) 555-0100");
    }

    #[test]
    fn a_leading_one_is_forgiven_and_dropped() {
        assert_eq!(phone_na("17805550100".to_string()).expect("eleven digits with the prefix"), "(780) 555-0100");
        assert_eq!(phone_na("1-780-555-0100".to_string()).expect("eleven digits with the prefix"), "(780) 555-0100");
        assert_eq!(phone_na("+1 780 555 0100".to_string()).expect("eleven digits with the prefix"), "(780) 555-0100");
    }

    #[test]
    fn the_wrong_number_of_digits_says_how_many_it_found() {
        assert!(phone_na("780555010".to_string()).unwrap_err().contains("has 9"));
        assert!(phone_na("78055501000".to_string()).unwrap_err().contains("has 11"), "eleven digits without the 1 prefix are not a number");
        assert!(phone_na("call me".to_string()).unwrap_err().contains("has 0"));
    }

    #[test]
    fn numbers_read_as_english_words() {
        assert_eq!(number_words(0), "zero");
        assert_eq!(number_words(7), "seven");
        assert_eq!(number_words(21), "twenty-one");
        assert_eq!(number_words(42), "forty-two");
        assert_eq!(number_words(100), "one hundred");
        assert_eq!(number_words(105), "one hundred five", "American style has no `and`");
        assert_eq!(number_words(999), "nine hundred ninety-nine");
        assert_eq!(number_words(1000), "one thousand");
        assert_eq!(number_words(1000000), "one million");
        assert_eq!(number_words(-8000), "negative eight thousand");
    }

    /// A group of zero contributes no words, so the scales around it must
    /// still read correctly.
    #[test]
    fn empty_groups_drop_out_of_the_words() {
        assert_eq!(number_words(1000001), "one million one");
        assert_eq!(number_words(2000000000), "two billion");
    }

    /// The most negative integer has no positive twin, so it is the value a
    /// naive `abs` breaks on. Both ends of the range are pinned in full.
    #[test]
    fn the_edges_of_the_integer_range_spell_out_fully() {
        assert_eq!(
            number_words(i64::MAX),
            "nine quintillion two hundred twenty-three quadrillion three hundred seventy-two trillion thirty-six billion eight hundred fifty-four million seven hundred seventy-five thousand eight hundred seven"
        );
        assert_eq!(
            number_words(i64::MIN),
            "negative nine quintillion two hundred twenty-three quadrillion three hundred seventy-two trillion thirty-six billion eight hundred fifty-four million seven hundred seventy-five thousand eight hundred eight"
        );
    }
}

#[cfg(test)]
mod size_parsing_tests {
    use super::*;

    #[test]
    fn a_size_reads_back_into_the_bytes_it_was_written_from() {
        assert_eq!(parse_bytes("1.5 KB".to_string()).expect("a real size"), 1536);
        assert_eq!(parse_bytes("512 B".to_string()).expect("a real size"), 512);
        assert_eq!(parse_bytes("900".to_string()).expect("a real size"), 900, "no unit is bytes");
        assert_eq!(parse_bytes("2gb".to_string()).expect("a real size"), 2_147_483_648);
        assert_eq!(parse_bytes("1 KiB".to_string()).expect("a real size"), 1024, "KiB is another spelling of KB");
        assert_eq!(parse_bytes("  20 MB  ".to_string()).expect("a real size"), 20_971_520);
        assert_eq!(parse_bytes("-1 KB".to_string()).expect("a real size"), -1024);
    }

    #[test]
    fn what_format_bytes_writes_is_what_parse_bytes_reads() {
        for count in [0i64, 512, 1024, 1536, 1_048_576, 3_221_225_472] {
            let written = bytes(count);
            let read_back = parse_bytes(written.clone()).expect("our own writing");
            // One decimal place is all format_bytes keeps, so the round trip is
            // exact only to that - a tenth of the unit it chose.
            let tolerance = (count as f64 * 0.05).max(1.0);
            assert!((read_back - count).abs() as f64 <= tolerance, "{} wrote {} and read back {}", count, written, read_back);
        }
    }

    #[test]
    fn a_size_that_is_not_one_says_so() {
        assert!(parse_bytes("".to_string()).unwrap_err().contains("empty"));
        assert!(parse_bytes("MB".to_string()).unwrap_err().contains("does not start with a number"));
        assert!(parse_bytes("lots".to_string()).unwrap_err().contains("not a size this knows"));
        assert!(parse_bytes("5 furlongs".to_string()).unwrap_err().contains("not a size this knows"));
        assert!(parse_bytes("99999 PB".to_string()).unwrap_err().contains("larger size than"));
    }
}

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
}

//! Money, counted in whole cents.
//!
//! A float cannot hold 0.10, so a program that adds ten dimes with floats does
//! not get a dollar, and a program that invoices with floats eventually sends
//! someone a bill for $19.999999999999998. Every accounting system solves this
//! the same way: count the smallest unit as a whole number and only turn it
//! into text at the end.
//!
//! So an amount here is an integer of cents - 1234 is $12.34 - and the
//! functions are the operations that cannot be done safely with `+` and `*`:
//! turning text into an amount, turning an amount into text, taking a
//! percentage, and dividing an amount up without losing or inventing a cent.
//!
//! Adding and subtracting need nothing from this module: cents are integers, so
//! `total:i = subtotal + shipping` is exact already.

/// The amount an entered number of dollars comes to in cents: `12.34` becomes
/// `1234`. Rounds to the nearest cent, so this is the boundary where a float
/// stops being involved - and the only place a rounding happens.
pub fn from_dollars(dollars: f64) -> i64 {
    return (dollars * 100.0).round() as i64;
}

/// The amount as a number of dollars, for handing to something that insists on
/// one. Anything over about 90 trillion dollars loses precision, which is a
/// long way past where this should have stopped being a float.
pub fn to_dollars(cents: i64) -> f64 {
    return cents as f64 / 100.0;
}

/// The amount written in text, in cents. Accepts what people type: a leading
/// symbol, thousands separators, a minus sign or brackets for a negative, and
/// either one or two decimal places. More than two decimal places is an error,
/// because rounding away someone's money silently is not something to do on a
/// guess.
pub fn parse(text: String) -> Result<i64, String> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Err("money_parse: there was no amount to read".to_string());
    }

    // Accountants write a negative amount in brackets, and a form will happily
    // pass that straight through.
    let (bracketed, without_brackets) = match trimmed.strip_prefix('(').and_then(|rest| rest.strip_suffix(')')) {
        Some(inner) => (true, inner.trim()),
        None => (false, trimmed),
    };
    let (negative, unsigned) = match without_brackets.strip_prefix('-') {
        Some(rest) => (true, rest.trim()),
        None => (false, without_brackets),
    };

    // Whatever currency symbol is in front, and whatever separators are inside,
    // the amount is the digits and the one decimal point.
    let digits_only: String = unsigned.chars().filter(|character| character.is_ascii_digit() || *character == '.').collect();
    if digits_only.is_empty() || digits_only == "." {
        return Err(format!("money_parse: `{}` has no digits in it", text));
    }
    if unsigned.chars().any(|character| character.is_alphabetic()) {
        return Err(format!("money_parse: `{}` has letters in it, so it is not an amount", text));
    }

    let (whole_text, fraction_text) = match digits_only.split_once('.') {
        Some((whole, fraction)) => {
            if fraction.contains('.') {
                return Err(format!("money_parse: `{}` has more than one decimal point", text));
            }
            (whole, fraction)
        }
        None => (digits_only.as_str(), ""),
    };
    if fraction_text.len() > 2 {
        return Err(format!("money_parse: `{}` is more precise than a cent, and rounding it away is not something to guess at", text));
    }

    let whole: i64 = if whole_text.is_empty() { 0 } else { whole_text.parse().map_err(|_| format!("money_parse: `{}` is a larger number than can be counted in cents", text))? };
    let cents_in_fraction: i64 = match fraction_text.len() {
        0 => 0,
        1 => fraction_text.parse::<i64>().map_err(|_| format!("money_parse: `{}` is not an amount", text))? * 10,
        _ => fraction_text.parse::<i64>().map_err(|_| format!("money_parse: `{}` is not an amount", text))?,
    };

    let total = whole.checked_mul(100).and_then(|whole_cents| whole_cents.checked_add(cents_in_fraction)).ok_or_else(|| format!("money_parse: `{}` is a larger number than can be counted in cents", text))?;
    return Ok(if negative || bracketed { -total } else { total });
}

/// The amount written out with a symbol, thousands separators and two decimal
/// places: `1234` with `$` is `$12.34`.
pub fn format(cents: i64, symbol: String) -> String {
    return crate::parser::std_lib::format::currency(to_dollars(cents), symbol);
}

/// A percentage of an amount, rounded to the nearest cent: tax, a discount, a
/// commission. The rate is a percentage rather than a fraction, because that is
/// how a tax rate is written down - 5.0 is five percent.
pub fn percent_of(cents: i64, rate: f64) -> i64 {
    return (cents as f64 * rate / 100.0).round() as i64;
}

/// An amount multiplied by a count, which is what a line item is.
pub fn times(cents: i64, count: i64) -> Result<i64, String> {
    return cents.checked_mul(count).ok_or_else(|| format!("money_times: {} cents {} times is more money than can be counted", cents, count));
}

/// An amount split as evenly as it can be, with the leftover cents given out
/// one each from the start rather than dropped. Three ways of $10.00 gives
/// $3.34, $3.33, $3.33 - which adds back up to exactly $10.00, and that is the
/// whole point.
pub fn split(cents: i64, ways: i64) -> Result<Vec<i64>, String> {
    if ways < 1 {
        return Err(format!("money_split: an amount cannot be split {} ways", ways));
    }
    let each = cents / ways;
    let mut remainder = cents % ways;
    let mut shares = Vec::with_capacity(ways as usize);
    // A negative amount has a negative remainder, so the leftover goes out in
    // the same direction the amount does.
    let step = if remainder < 0 { -1 } else { 1 };
    for _ in 0..ways {
        if remainder != 0 {
            shares.push(each + step);
            remainder -= step;
        } else {
            shares.push(each);
        }
    }
    return Ok(shares);
}

/// An amount divided in proportion to the given weights, again with every cent
/// accounted for. A 50/50 split of an odd number of cents has to give one side
/// the extra penny, and this decides which rather than leaving it to rounding:
/// the earliest weights get it.
pub fn allocate(cents: i64, weights: Vec<i64>) -> Result<Vec<i64>, String> {
    if weights.is_empty() {
        return Err("money_allocate: there were no shares to allocate to".to_string());
    }
    if weights.iter().any(|weight| *weight < 0) {
        return Err("money_allocate: a share cannot be a negative part of the whole".to_string());
    }
    let total_weight: i64 = weights.iter().sum();
    if total_weight == 0 {
        return Err("money_allocate: the shares add up to nothing, so there is no way to divide by them".to_string());
    }

    let mut shares: Vec<i64> = weights.iter().map(|weight| cents * weight / total_weight).collect();
    let mut remainder = cents - shares.iter().sum::<i64>();
    let step = if remainder < 0 { -1 } else { 1 };
    let mut position = 0;
    while remainder != 0 {
        // Only shares that were allocated something take part in the leftover,
        // so a weight of zero stays at zero.
        if weights[position] > 0 {
            shares[position] += step;
            remainder -= step;
        }
        position = (position + 1) % shares.len();
    }
    return Ok(shares);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dollars_and_cents_convert_both_ways() {
        assert_eq!(from_dollars(12.34), 1234);
        assert_eq!(from_dollars(0.1), 10);
        assert_eq!(from_dollars(-9.99), -999);
        assert_eq!(to_dollars(1234), 12.34);
    }

    /// The reason the module exists: ten dimes are a dollar here, and are not
    /// with floats.
    #[test]
    fn ten_dimes_are_exactly_a_dollar() {
        let dime = from_dollars(0.10);
        let total: i64 = (0..10).map(|_| dime).sum();
        assert_eq!(total, 100);
        assert_eq!(format(total, "$".to_string()), "$1.00");

        let float_total: f64 = (0..10).map(|_| 0.10).sum();
        assert_ne!(float_total, 1.0);
    }

    #[test]
    fn what_people_type_is_read_as_an_amount() {
        assert_eq!(parse("12.34".to_string()).expect("an amount"), 1234);
        assert_eq!(parse("$12.34".to_string()).expect("an amount"), 1234);
        assert_eq!(parse("$1,234.50".to_string()).expect("an amount"), 123450);
        assert_eq!(parse("12".to_string()).expect("an amount"), 1200);
        assert_eq!(parse("12.5".to_string()).expect("an amount"), 1250);
        assert_eq!(parse(" €99.99 ".to_string()).expect("an amount"), 9999);
        assert_eq!(parse(".50".to_string()).expect("an amount"), 50);
    }

    #[test]
    fn both_spellings_of_a_negative_amount_are_read() {
        assert_eq!(parse("-12.34".to_string()).expect("an amount"), -1234);
        assert_eq!(parse("($12.34)".to_string()).expect("an amount"), -1234);
    }

    #[test]
    fn something_that_is_not_an_amount_says_so() {
        assert!(parse("".to_string()).is_err());
        assert!(parse("free".to_string()).is_err());
        assert!(parse("12 dollars".to_string()).is_err());
        assert!(parse("$".to_string()).is_err());
        assert!(parse("1.2.3".to_string()).is_err());
    }

    /// Silently turning $1.005 into $1.00 is how a program loses half a cent a
    /// thousand times, so it is refused instead.
    #[test]
    fn more_precision_than_a_cent_is_refused_rather_than_rounded() {
        let failure = parse("1.005".to_string()).unwrap_err();
        assert!(failure.contains("more precise than a cent"), "got: {}", failure);
    }

    #[test]
    fn an_amount_is_written_the_way_a_receipt_writes_it() {
        assert_eq!(format(1234, "$".to_string()), "$12.34");
        assert_eq!(format(123450, "$".to_string()), "$1,234.50");
        assert_eq!(format(-999, "$".to_string()), "-$9.99");
        assert_eq!(format(0, "$".to_string()), "$0.00");
        assert_eq!(format(5, "$".to_string()), "$0.05");
    }

    #[test]
    fn text_and_amount_round_trip() {
        for cents in [0, 5, 99, 1234, 123450, -999] {
            let written = format(cents, "$".to_string());
            assert_eq!(parse(written.clone()).expect("our own output"), cents, "from: {}", written);
        }
    }

    #[test]
    fn a_percentage_is_taken_to_the_nearest_cent() {
        assert_eq!(percent_of(1000, 5.0), 50);
        assert_eq!(percent_of(999, 5.0), 50);
        assert_eq!(percent_of(1234, 13.0), 160);
        assert_eq!(percent_of(0, 20.0), 0);
    }

    #[test]
    fn a_line_item_is_an_amount_times_a_count() {
        assert_eq!(times(1234, 3).expect("a small total"), 3702);
        assert_eq!(times(0, 100).expect("a small total"), 0);
        assert!(times(i64::MAX, 2).is_err());
    }

    #[test]
    fn splitting_an_amount_loses_no_cents() {
        let shares = split(1000, 3).expect("three ways");
        assert_eq!(shares, vec![334, 333, 333]);
        assert_eq!(shares.iter().sum::<i64>(), 1000);

        let even = split(1000, 4).expect("four ways");
        assert_eq!(even, vec![250, 250, 250, 250]);

        let negative = split(-1000, 3).expect("three ways");
        assert_eq!(negative.iter().sum::<i64>(), -1000);

        assert!(split(100, 0).is_err());
    }

    #[test]
    fn an_allocation_follows_the_weights_and_keeps_every_cent() {
        let two_to_one = allocate(1000, vec![2, 1]).expect("two shares");
        assert_eq!(two_to_one, vec![667, 333]);
        assert_eq!(two_to_one.iter().sum::<i64>(), 1000);

        let odd_split = allocate(101, vec![1, 1]).expect("two shares");
        assert_eq!(odd_split, vec![51, 50]);

        // A share weighted at nothing gets nothing, even when cents are left
        // over to hand out.
        let with_zero = allocate(100, vec![1, 0, 1]).expect("three shares");
        assert_eq!(with_zero, vec![50, 0, 50]);
        let uneven_with_zero = allocate(101, vec![1, 0, 1]).expect("three shares");
        assert_eq!(uneven_with_zero, vec![51, 0, 50]);

        assert!(allocate(100, vec![]).is_err());
        assert!(allocate(100, vec![0, 0]).is_err());
        assert!(allocate(100, vec![1, -1]).is_err());
    }
}

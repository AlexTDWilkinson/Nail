//! Money module stdlib registry entries.
//!
//! An amount is an integer of cents, so adding and subtracting need nothing
//! from here - these are the operations that cannot be done safely with plain
//! arithmetic.

use super::*;

pub(super) fn register(m: &mut HashMap<&'static str, StdlibFunction>) {
    simple_fns! { m, Money:
        "money_from_dollars" => "std_lib::money::from_dollars", (dollars: f) -> i,
            "Returns the number of cents an amount of dollars comes to, rounded to the nearest cent. The one place a float is involved.",
            "cents:i = money_from_dollars(12.34);";
        "money_to_dollars" => "std_lib::money::to_dollars", (cents: i) -> f,
            "Returns an amount of cents as a number of dollars, for handing to something that insists on one.",
            "dollars:f = money_to_dollars(1234);";
        "money_parse" => "std_lib::money::parse", (text: s) -> (i!e),
            "Reads what a person typed as a number of cents, accepting a currency symbol, thousands separators, a minus sign or brackets. More precision than a cent is an error.",
            "cents:i = danger(money_parse(`$1,234.50`));";
        "money_format" => "std_lib::money::format", (cents: i, symbol: s) -> s,
            "Writes an amount of cents with a symbol, thousands separators and two decimal places.",
            "price:s = money_format(123450, `$`);";
        "money_percent_of" => "std_lib::money::percent_of", (cents: i, rate: f) -> i,
            "Returns a percentage of an amount, rounded to the nearest cent. The rate is a percentage, so 5.0 is five percent.",
            "tax:i = money_percent_of(subtotal, 5.0);";
        "money_times" => "std_lib::money::times", (cents: i, count: i) -> (i!e),
            "Returns an amount multiplied by a count, which is what a line item comes to. Errors if the total is larger than can be counted.",
            "line_total:i = danger(money_times(unit_price, quantity));";
        "money_split" => "std_lib::money::split", (cents: i, ways: i) -> ([i]!e),
            "Splits an amount as evenly as it can be, handing the leftover cents out one each from the start so the shares add back up to the whole.",
            "shares:a:i = danger(money_split(1000, 3));";
        "money_allocate" => "std_lib::money::allocate", (cents: i, weights: [i]) -> ([i]!e),
            "Divides an amount in proportion to the given weights, with every cent accounted for and the earliest weights taking any leftover.",
            "shares:a:i = danger(money_allocate(1000, [2, 1]));";
        "money_loan_payment" => "std_lib::money::loan_payment", (principal_cents: i, annual_rate_percent: f, months: i) -> (i!e),
            "The fixed monthly payment that clears a loan, in cents - the amortization formula every mortgage and car payment comes from. The rate is the yearly percentage as people quote it: 6.0 means six percent.",
            "monthly:i = danger(money_loan_payment(20000000, 6.0, 360));";
    }
}

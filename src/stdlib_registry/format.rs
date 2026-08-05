//! Format module stdlib registry entries

use super::*;

pub(super) fn register(m: &mut HashMap<&'static str, StdlibFunction>) {
    simple_fns! { m, Format:
        "format_decimals" => "std_lib::format::decimals", (value: f, places: i) -> s,
            "Formats a float with exactly the given number of decimal places, keeping trailing zeros.",
            "price:s = format_decimals(3.5, 2);";
        "format_thousands" => "std_lib::format::thousands", (value: i) -> s,
            "Formats an integer with comma thousands separators.",
            "population:s = format_thousands(1234567);";
        "format_thousands_float" => "std_lib::format::thousands_float", (value: f, places: i) -> s,
            "Formats a float with comma thousands separators and a fixed number of decimal places.",
            "total:s = format_thousands_float(1234.5, 2);";
        "format_currency" => "std_lib::format::currency", (amount: f, symbol: s) -> s,
            "Formats an amount of money as the symbol followed by grouped digits and two decimals.",
            "cost:s = format_currency(1234.5, `$`);";
        "format_percent" => "std_lib::format::percent", (fraction: f, places: i) -> s,
            "Formats a fraction as a percentage, so 0.125 becomes 12.5%.",
            "share:s = format_percent(0.125, 1);";
        "format_bytes" => "std_lib::format::bytes", (count: i) -> s,
            "Formats a byte count in the largest unit under 1024, like 1.5 KB.",
            "size:s = format_bytes(1536);";
        "format_compact" => "std_lib::format::compact", (value: i) -> s,
            "Shortens a large count for display, so 1200 becomes 1.2k.",
            "views:s = format_compact(3400000);";
        "format_ordinal" => "std_lib::format::ordinal", (number: i) -> s,
            "Returns the English ordinal for a number, like 1st, 2nd or 13th.",
            "place:s = format_ordinal(21);";
        "format_plural" => "std_lib::format::plural", (count: i, singular: s, plural: s) -> s,
            "Returns the count followed by the singular or plural word, whichever the count calls for.",
            "label:s = format_plural(2, `file`, `files`);";
        "format_list" => "std_lib::format::list", (items: [s], conjunction: s) -> s,
            "Joins items the way a sentence would, as `a, b and c`, using the given conjunction.",
            "sentence:s = format_list([`a`, `b`, `c`], `and`);";
    }
}

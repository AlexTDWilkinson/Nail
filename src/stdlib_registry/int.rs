//! Int module stdlib registry entries. Integer arithmetic beyond these lives
//! in the Math module (math_gcd, math_lcm, math_factorial, ...).

use super::*;

pub(super) fn register(m: &mut HashMap<&'static str, StdlibFunction>) {
    simple_fns! { m, Int:
        "int_from" => "std_lib::int::from", (value: (T: i|f|s|b)) -> (i!e),
            "Converts a value (string, float, etc.) to an integer. Errors if it cannot be parsed.",
            "age:i = danger(int_from(`42`));";
        "int_pow" => "std_lib::int::pow", (base: i, exponent: i) -> (i!e),
            "Raises base to an integer power. Errors on negative exponents or overflow.",
            "big:i = danger(int_pow(2, 10));";
        "int_from_hex" => "std_lib::int::from_hex", (text: s) -> (i!e),
            "Reads a hexadecimal number, with or without the 0x in front. Errors if it is not one.",
            "red:i = danger(int_from_hex(`0xFF`));";
        "int_from_radix" => "std_lib::int::from_radix", (text: s, base: i) -> (i!e),
            "Reads a number written in any base from 2 to 36, where digits above 9 are letters. Errors if it is not one.",
            "flags:i = danger(int_from_radix(`1011`, 2));";
        "int_to_radix" => "std_lib::int::to_radix", (value: i, base: i) -> (s!e),
            "Writes a number in any base from 2 to 36, using lower-case letters for digits above 9.",
            "hex:s = danger(int_to_radix(255, 16));";
        "int_is_even" => "std_lib::int::is_even", (value: i) -> b,
            "Returns whether the integer divides evenly by two. Zero is even.",
            "paired:b = int_is_even(4);";
        "int_is_odd" => "std_lib::int::is_odd", (value: i) -> b,
            "Returns whether the integer leaves a remainder when divided by two.",
            "alone:b = int_is_odd(7);";
        "int_clamp" => "std_lib::int::clamp", (value: i, low: i, high: i) -> (i!e),
            "Restricts an integer to the range low..high, both included. Errors if low is above high.",
            "bounded:i = danger(int_clamp(15, 0, 10));";
    }
}

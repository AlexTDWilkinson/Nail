//! Int module stdlib registry entries. Integer arithmetic beyond these lives
//! in the Math module (math_gcd, math_lcm, math_factorial, ...).

use super::*;

pub(super) fn register(m: &mut HashMap<&'static str, StdlibFunction>) {
    simple_fns! { m, Int:
        "int_from" => "std_lib::int::from", (value: any) -> (i!e),
            "Converts a value (string, float, etc.) to an integer; errors if it cannot be parsed.",
            "age:i = danger(int_from(`42`));";
        "int_pow" => "std_lib::int::pow", (base: i, exponent: i) -> (i!e),
            "Raises base to an integer power; errors on negative exponents or overflow.",
            "big:i = danger(int_pow(2, 10));";
    }
}

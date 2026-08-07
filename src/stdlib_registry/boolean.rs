//! Bool module stdlib registry entries - the yes-or-no half of the
//! conversions int_from and float_from cover for numbers.

use super::*;

pub(super) fn register(m: &mut HashMap<&'static str, StdlibFunction>) {
    simple_fns! { m, Bool:
        "bool_from" => "std_lib::boolean::from", (value: (T: i|f|s|b)) -> (b!e),
            "Reads a value as true or false. Text may be true, yes, y, on or 1 and their opposites false, no, n, off or 0, in any case. A number must be 1 or 0. Anything else is an error rather than a guess.",
            "enabled:b = danger(bool_from(danger(env_get(`FEATURE_ON`))));";
    }
}

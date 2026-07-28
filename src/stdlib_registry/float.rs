//! Float module stdlib registry entries. Float arithmetic (abs, sqrt, round,
//! etc.) lives in the Math module - this module only converts to floats.

use super::*;

pub(super) fn register(m: &mut HashMap<&'static str, StdlibFunction>) {
    simple_fns! { m, Float:
        "float_from" => "std_lib::float::from", (value: (T: i|f|s|b)) -> (f!e),
            "Converts a value (string, int, etc.) to a float; errors if it cannot be parsed.",
            "price:f = danger(float_from(`19.99`));";
    }
}
